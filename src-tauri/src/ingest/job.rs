use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use chrono::Utc;
use duckdb::params;
use serde::Serialize;
use tauri::AppHandle;
use uuid::Uuid;

use crate::db::Db;
use crate::ingest::events::{
    emit_batch_done, emit_file, BatchDoneEvent, FileEvent, FileState,
};
use crate::ingest::scan::prescan;
use crate::ingest::trash::move_to_trash;
use crate::ingest::{
    exif as exif_read, extension_for, hash::hash_file, mime_for_ext, store::copy_and_verify,
};
use crate::store::StoreRoot;

#[derive(Debug, Clone, Serialize)]
pub struct StartBatchResult {
    pub batch_id: String,
    pub total: usize,
}

pub async fn run_batch(
    app: AppHandle,
    db: Arc<Db>,
    store: StoreRoot,
    source_root: PathBuf,
    job_guard: tokio::sync::OwnedMutexGuard<()>,
) -> Result<StartBatchResult> {
    let scan = tokio::task::spawn_blocking({
        let root = source_root.clone();
        move || prescan(&root)
    })
    .await?;

    let batch_id = Uuid::new_v4();
    let started_at = Utc::now();
    {
        let conn = db.0.lock().unwrap();
        conn.execute(
            "INSERT INTO ingest_batches (id, source_folder, started_at, imported_count, skipped_count, failed_count) VALUES (?, ?, ?, 0, 0, 0)",
            params![
                batch_id.to_string(),
                source_root.to_string_lossy().to_string(),
                started_at,
            ],
        )?;
    }

    let result = StartBatchResult {
        batch_id: batch_id.to_string(),
        total: scan.total,
    };

    let files = scan.files;
    let app_clone = app.clone();
    let db_clone = db.clone();
    let store_clone = store.clone();
    let batch_id_str = batch_id.to_string();

    tokio::spawn(async move {
        let _guard = job_guard;
        for path in files.iter() {
            emit_file(
                &app_clone,
                &FileEvent {
                    batch_id: batch_id_str.clone(),
                    source_path: path.to_string_lossy().to_string(),
                    original_filename: filename(path),
                    state: FileState::Queued,
                    content_hash: None,
                    image_id: None,
                    error: None,
                },
            );
        }

        let concurrency = std::cmp::max(1, num_cpus::get() / 2);
        let mut imported = 0usize;
        let mut skipped = 0usize;
        let mut failed = 0usize;

        let chunks = files.chunks(concurrency.max(1));
        for chunk in chunks {
            let mut handles = Vec::with_capacity(chunk.len());
            for path in chunk {
                let app2 = app_clone.clone();
                let db2 = db_clone.clone();
                let store2 = store_clone.clone();
                let path2 = path.clone();
                let bid = batch_id_str.clone();
                handles.push(tokio::task::spawn_blocking(move || {
                    process_one(app2, db2, store2, bid, &path2)
                }));
            }
            for handle in handles {
                match handle.await {
                    Ok(Ok(outcome)) => match outcome {
                        Outcome::Imported => imported += 1,
                        Outcome::Skipped => skipped += 1,
                        Outcome::Failed => failed += 1,
                    },
                    _ => failed += 1,
                }
            }
        }

        let finished_at = Utc::now();
        {
            let conn = db_clone.0.lock().unwrap();
            let _ = conn.execute(
                "UPDATE ingest_batches SET finished_at = ?, imported_count = ?, skipped_count = ?, failed_count = ? WHERE id = ?",
                params![
                    finished_at,
                    imported as i64,
                    skipped as i64,
                    failed as i64,
                    batch_id_str.clone(),
                ],
            );
        }

        emit_batch_done(
            &app_clone,
            &BatchDoneEvent {
                batch_id: batch_id_str,
                imported,
                skipped,
                failed,
            },
        );
    });

    Ok(result)
}

enum Outcome {
    Imported,
    Skipped,
    Failed,
}

fn filename(path: &Path) -> String {
    path.file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn process_one(
    app: AppHandle,
    db: Arc<Db>,
    store: StoreRoot,
    batch_id: String,
    source: &Path,
) -> Result<Outcome> {
    let original_filename = filename(source);
    let source_path = source.to_string_lossy().to_string();

    let send = |state: FileState,
                hash: Option<String>,
                image_id: Option<String>,
                error: Option<String>| {
        emit_file(
            &app,
            &FileEvent {
                batch_id: batch_id.clone(),
                source_path: source_path.clone(),
                original_filename: original_filename.clone(),
                state,
                content_hash: hash,
                image_id,
                error,
            },
        );
    };

    let ext = extension_for(source).ok_or_else(|| anyhow!("non-allowlisted extension"))?;

    send(FileState::Hashing, None, None, None);
    let hash_hex = match hash_file(source) {
        Ok(h) => h,
        Err(e) => {
            send(FileState::Failed, None, None, Some(format!("hash failed: {e}")));
            return Ok(Outcome::Failed);
        }
    };

    let existing_id: Option<String> = {
        let conn = db.0.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id FROM images WHERE content_hash = ?")
            .ok();
        if let Some(ref mut s) = stmt {
            s.query_row(params![hash_hex], |row| row.get::<_, String>(0)).ok()
        } else {
            None
        }
    };

    if let Some(id) = existing_id {
        send(
            FileState::SkippedDuplicate,
            Some(hash_hex.clone()),
            Some(id),
            None,
        );
        send(FileState::Trashing, Some(hash_hex.clone()), None, None);
        if let Err(e) = move_to_trash(source) {
            send(
                FileState::Failed,
                Some(hash_hex),
                None,
                Some(format!("trash failed: {e}")),
            );
            return Ok(Outcome::Failed);
        }
        send(FileState::Done, Some(hash_hex), None, None);
        return Ok(Outcome::Skipped);
    }

    send(FileState::Copying, Some(hash_hex.clone()), None, None);
    let stored = match copy_and_verify(&store, source, &hash_hex, ext) {
        Ok(s) => s,
        Err(e) => {
            send(
                FileState::Failed,
                Some(hash_hex),
                None,
                Some(format!("store failed: {e}")),
            );
            return Ok(Outcome::Failed);
        }
    };
    send(FileState::Verifying, Some(hash_hex.clone()), None, None);

    let byte_size = std::fs::metadata(source)
        .map(|m| m.len() as i64)
        .unwrap_or(0);
    let mime = mime_for_ext(ext);
    let image_id = Uuid::new_v4().to_string();
    let store_root = store.root().to_path_buf();
    let relative = relative_to(&stored.target_path, &store_root)
        .to_string_lossy()
        .to_string();
    let imported_at = Utc::now();
    let exif_created_at = exif_read::read_created_at(source);
    let fs_mtime = exif_read::fs_mtime(source);

    send(FileState::Indexing, Some(hash_hex.clone()), Some(image_id.clone()), None);
    {
        let conn = db.0.lock().unwrap();
        if let Err(e) = conn.execute(
            "INSERT INTO images (id, content_hash, store_path, original_filename, original_path, byte_size, mime, imported_at, ingest_batch_id, thumbnails_status, exif_created_at, fs_mtime) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                image_id,
                hash_hex,
                relative,
                original_filename,
                source_path,
                byte_size,
                mime,
                imported_at,
                batch_id,
                "missing",
                exif_created_at,
                fs_mtime,
            ],
        ) {
            send(
                FileState::Failed,
                Some(hash_hex),
                Some(image_id),
                Some(format!("indexing failed: {e}")),
            );
            return Ok(Outcome::Failed);
        }
    }

    send(
        FileState::Thumbnailing,
        Some(hash_hex.clone()),
        Some(image_id.clone()),
        None,
    );
    let thumb_status = match crate::ingest::thumb::generate(&store, &stored.target_path, &hash_hex)
    {
        Ok(()) => "ready",
        Err(_) => "missing",
    };
    {
        let conn = db.0.lock().unwrap();
        let _ = conn.execute(
            "UPDATE images SET thumbnails_status = ? WHERE id = ?",
            params![thumb_status, image_id],
        );
    }

    if thumb_status == "ready" {
        let _ = crate::ingest::stages::palette::run(
            &store,
            &stored.target_path,
            &hash_hex,
            &image_id,
            &db,
        );
    }

    send(
        FileState::Trashing,
        Some(hash_hex.clone()),
        Some(image_id.clone()),
        None,
    );
    if let Err(e) = move_to_trash(source) {
        send(
            FileState::Failed,
            Some(hash_hex),
            Some(image_id),
            Some(format!("trash failed: {e}")),
        );
        return Ok(Outcome::Failed);
    }

    send(FileState::Done, Some(hash_hex), Some(image_id), None);
    Ok(Outcome::Imported)
}

fn relative_to(path: &Path, root: &Path) -> PathBuf {
    path.strip_prefix(root)
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|_| path.to_path_buf())
}
