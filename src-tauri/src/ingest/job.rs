use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use chrono::Utc;
use rusqlite::params;
use serde::Serialize;
use tauri::AppHandle;
use uuid::Uuid;

use crate::db::Db;
use crate::ingest::events::{emit_batch_done, emit_file, BatchDoneEvent, FileEvent, FileState};
use crate::ingest::{
    extension_for, hash::hash_file, metadata, mime_for_ext, store::copy_and_verify,
};
use crate::store::StoreRoot;

/// Ingest knobs read from preferences once per batch, so a change mid-batch
/// waits for the next one.
#[derive(Debug, Clone)]
pub struct IngestOptions {
    pub concurrency: usize,
    pub extension_allow_list: Vec<String>,
    pub keep_source_files: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct StartBatchResult {
    pub batch_id: String,
    pub total: usize,
}

/// Imports exactly `files`, the selection the person confirmed from the
/// pre-scan. Nothing is walked again here, so what was shown is what moves.
pub async fn run_batch(
    app: AppHandle,
    db: Arc<Db>,
    store: StoreRoot,
    source_root: PathBuf,
    files: Vec<PathBuf>,
    options: IngestOptions,
    job_guard: tokio::sync::OwnedMutexGuard<()>,
) -> Result<StartBatchResult> {
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
        total: files.len(),
    };

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

        let concurrency = options.concurrency.max(1);
        let keep_source_files = options.keep_source_files;
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
                    process_one(app2, db2, store2, bid, &path2, keep_source_files)
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
    keep_source_files: bool,
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
            send(
                FileState::Failed,
                None,
                None,
                Some(format!("hash failed: {e}")),
            );
            return Ok(Outcome::Failed);
        }
    };

    let existing_id: Option<String> = {
        let conn = db.0.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id FROM images WHERE content_hash = ?")
            .ok();
        if let Some(ref mut s) = stmt {
            s.query_row(params![hash_hex], |row| row.get::<_, String>(0))
                .ok()
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
        send(FileState::Done, Some(hash_hex), None, None);
        release_source(source, keep_source_files);
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
    let extracted = metadata::extract(source);
    let exif_created_at = metadata::date_time_original(&extracted);
    let fs_mtime = metadata::exif::fs_mtime(source);

    send(
        FileState::Indexing,
        Some(hash_hex.clone()),
        Some(image_id.clone()),
        None,
    );
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

    if let Err(e) = metadata::write(&db, &image_id, &extracted) {
        eprintln!("metadata::write failed for {image_id}: {e:?}");
    }

    if let Err(e) = crate::ingest::provenance::run(&db, &image_id, &hash_hex, &stored.target_path) {
        eprintln!("provenance::run failed for {image_id}: {e:?}");
    }

    let exif_orientation = extracted.exif.as_ref().and_then(|e| e.orientation);
    if let Err(e) =
        crate::ingest::orientation::run(&db, &image_id, &stored.target_path, exif_orientation)
    {
        eprintln!("orientation::run failed for {image_id}: {e:?}");
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
        Err(e) => {
            eprintln!(
                "thumb::generate failed for {}: {e:?}",
                stored.target_path.display()
            );
            "missing"
        }
    };
    {
        let conn = db.0.lock().unwrap();
        if let Err(e) = conn.execute(
            "UPDATE images SET thumbnails_status = ? WHERE id = ?",
            params![thumb_status, image_id],
        ) {
            eprintln!("thumb status UPDATE failed for {image_id}: {e:?}");
        }
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

    send(FileState::Done, Some(hash_hex), Some(image_id), None);
    release_source(source, keep_source_files);
    Ok(Outcome::Imported)
}

/// With "leave source files in place" off, a source that is now in the
/// catalog (imported, or already there) goes to the system Trash. A failure
/// here is logged, never fatal: the catalog copy is safe either way.
fn release_source(source: &Path, keep_source_files: bool) {
    if keep_source_files {
        return;
    }
    if let Err(e) = crate::ingest::trash::move_to_trash(source) {
        eprintln!("could not move {} to Trash: {e}", source.display());
    }
}

fn relative_to(path: &Path, root: &Path) -> PathBuf {
    path.strip_prefix(root)
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|_| path.to_path_buf())
}
