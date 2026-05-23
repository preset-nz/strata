mod db;
mod ingest;
mod palette;
mod server;
mod store;

use std::path::PathBuf;
use std::sync::Arc;

use duckdb::params;
use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tokio::sync::Mutex as AsyncMutex;

use crate::db::Db;
use crate::ingest::job::{run_batch, StartBatchResult};
use crate::ingest::scan::{prescan, PrescanResult};
use crate::store::StoreRoot;

pub struct AppState {
    pub db: Arc<Db>,
    pub store: StoreRoot,
    pub job_lock: Arc<AsyncMutex<()>>,
}

#[derive(Serialize)]
struct PrescanSummary {
    root: String,
    total: usize,
    by_extension: std::collections::HashMap<String, usize>,
}

impl From<&PrescanResult> for PrescanSummary {
    fn from(r: &PrescanResult) -> Self {
        Self {
            root: r.root.clone(),
            total: r.total,
            by_extension: r.by_extension.clone(),
        }
    }
}

#[tauri::command]
async fn ingest_prescan(path: String) -> Result<PrescanSummary, String> {
    let root = PathBuf::from(&path);
    if !root.is_dir() {
        return Err(format!("not a directory: {path}"));
    }
    let result = tokio::task::spawn_blocking(move || prescan(&root))
        .await
        .map_err(|e| e.to_string())?;
    Ok(PrescanSummary::from(&result))
}

#[tauri::command]
async fn ingest_start(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<StartBatchResult, String> {
    let root = PathBuf::from(&path);
    if !root.is_dir() {
        return Err(format!("not a directory: {path}"));
    }
    let lock = state.job_lock.clone();
    let owned = lock
        .try_lock_owned()
        .map_err(|_| "job already in progress".to_string())?;

    let db = state.db.clone();
    let store = state.store.clone();
    run_batch(app, db, store, root, owned)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct LastBatch {
    id: String,
    source_folder: String,
    started_at: String,
    finished_at: Option<String>,
    imported_count: i64,
    skipped_count: i64,
    failed_count: i64,
}

#[derive(Serialize)]
struct ImportedRow {
    id: String,
    content_hash: String,
    original_filename: String,
    thumbnails_status: String,
}

#[tauri::command]
fn last_batch(state: State<'_, AppState>) -> Result<Option<LastBatch>, String> {
    let conn = state.db.0.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT id, source_folder, started_at, finished_at, imported_count, skipped_count, failed_count
             FROM ingest_batches ORDER BY started_at DESC LIMIT 1",
        )
        .map_err(|e| e.to_string())?;
    let row = stmt
        .query_row([], |row| {
            Ok(LastBatch {
                id: row.get::<_, String>(0)?,
                source_folder: row.get::<_, String>(1)?,
                started_at: row.get::<_, chrono::DateTime<chrono::Utc>>(2)?.to_rfc3339(),
                finished_at: row
                    .get::<_, Option<chrono::DateTime<chrono::Utc>>>(3)?
                    .map(|d| d.to_rfc3339()),
                imported_count: row.get::<_, i64>(4)?,
                skipped_count: row.get::<_, i64>(5)?,
                failed_count: row.get::<_, i64>(6)?,
            })
        })
        .ok();
    Ok(row)
}

#[tauri::command]
fn list_images(
    state: State<'_, AppState>,
    offset: i64,
    limit: i64,
) -> Result<Vec<ImportedRow>, String> {
    let limit = limit.clamp(1, 1000);
    let offset = offset.max(0);
    let conn = state.db.0.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT id, content_hash, original_filename, thumbnails_status
             FROM images ORDER BY imported_at DESC LIMIT ? OFFSET ?",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![limit, offset], |row| {
            Ok(ImportedRow {
                id: row.get::<_, String>(0)?,
                content_hash: row.get::<_, String>(1)?,
                original_filename: row.get::<_, String>(2)?,
                thumbnails_status: row.get::<_, String>(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

#[tauri::command]
fn batch_imported(state: State<'_, AppState>, batch_id: String) -> Result<Vec<ImportedRow>, String> {
    let conn = state.db.0.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT id, content_hash, original_filename, thumbnails_status
             FROM images WHERE ingest_batch_id = ? ORDER BY imported_at ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![batch_id], |row| {
            Ok(ImportedRow {
                id: row.get::<_, String>(0)?,
                content_hash: row.get::<_, String>(1)?,
                original_filename: row.get::<_, String>(2)?,
                thumbnails_status: row.get::<_, String>(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");
            std::fs::create_dir_all(&app_data_dir)?;

            let store = StoreRoot::new(&app_data_dir);
            store.ensure()?;

            let db_path = app_data_dir.join("strata.duckdb");
            let db = Arc::new(Db::open(&db_path)?);

            app.manage(AppState {
                db,
                store,
                job_lock: Arc::new(AsyncMutex::new(())),
            });
            Ok(())
        })
        .register_uri_scheme_protocol(crate::server::thumbnails::SCHEME, crate::server::thumbnails::handler)
        .invoke_handler(tauri::generate_handler![
            ingest_prescan,
            ingest_start,
            last_batch,
            batch_imported,
            list_images,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
