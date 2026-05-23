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
struct ImageDetails {
    id: String,
    content_hash: String,
    original_filename: String,
    original_path: String,
    byte_size: i64,
    mime: String,
    imported_at: String,
    ingest_batch_id: String,
    thumbnails_status: String,
    exif_created_at: Option<String>,
    fs_mtime: Option<String>,
    dominant_bucket: Option<String>,
    dominant_l: Option<f32>,
    dominant_c: Option<f32>,
    dominant_h: Option<f32>,
}

#[derive(Serialize)]
struct ImportedRow {
    id: String,
    content_hash: String,
    original_filename: String,
    thumbnails_status: String,
    dominant_bucket: Option<String>,
    dominant_l: Option<f32>,
    dominant_c: Option<f32>,
    dominant_h: Option<f32>,
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

fn sort_clause(sort: &str) -> &'static str {
    match sort {
        "filename" => "i.original_filename ASC",
        "created" => "COALESCE(i.exif_created_at, i.fs_mtime, i.imported_at) DESC",
        "updated" => "COALESCE(p.extracted_at, i.imported_at) DESC",
        "colour" => {
            "p.dominant_l IS NULL, \
             CASE WHEN p.dominant_c < 10 THEN 0 ELSE 1 END, \
             CASE WHEN p.dominant_c < 10 THEN p.dominant_l ELSE p.dominant_h END"
        }
        _ => "i.imported_at DESC",
    }
}

fn row_from(row: &duckdb::Row) -> duckdb::Result<ImportedRow> {
    Ok(ImportedRow {
        id: row.get::<_, String>(0)?,
        content_hash: row.get::<_, String>(1)?,
        original_filename: row.get::<_, String>(2)?,
        thumbnails_status: row.get::<_, String>(3)?,
        dominant_bucket: row.get::<_, Option<String>>(4)?,
        dominant_l: row.get::<_, Option<f32>>(5)?,
        dominant_c: row.get::<_, Option<f32>>(6)?,
        dominant_h: row.get::<_, Option<f32>>(7)?,
    })
}

const ROW_COLUMNS: &str = "i.id, i.content_hash, i.original_filename, i.thumbnails_status, \
                           p.dominant_bucket, p.dominant_l, p.dominant_c, p.dominant_h";

fn allowed_buckets(input: &[String]) -> Vec<String> {
    input
        .iter()
        .filter(|b| {
            crate::palette::vga16::VGA16
                .iter()
                .any(|v| v.name == b.as_str())
        })
        .cloned()
        .collect()
}

fn build_where(
    filter_buckets: &[String],
    batch_id: Option<&str>,
) -> (String, Vec<duckdb::types::Value>) {
    let mut clauses: Vec<String> = Vec::new();
    let mut bound: Vec<duckdb::types::Value> = Vec::new();

    if !filter_buckets.is_empty() {
        let placeholders = vec!["?"; filter_buckets.len()].join(", ");
        clauses.push(format!(
            "i.id IN (SELECT image_id FROM image_palette_bucket WHERE bucket IN ({placeholders}))"
        ));
        for b in filter_buckets {
            bound.push(duckdb::types::Value::Text(b.clone()));
        }
    }
    if let Some(b) = batch_id {
        clauses.push("i.ingest_batch_id = ?".to_string());
        bound.push(duckdb::types::Value::Text(b.to_string()));
    }

    let where_sql = if clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", clauses.join(" AND "))
    };
    (where_sql, bound)
}

#[tauri::command]
fn list_images(
    state: State<'_, AppState>,
    offset: i64,
    limit: i64,
    sort: Option<String>,
    buckets: Option<Vec<String>>,
    batch_id: Option<String>,
) -> Result<Vec<ImportedRow>, String> {
    let limit = limit.clamp(1, 1000);
    let offset = offset.max(0);
    let sort_key = sort.as_deref().unwrap_or("imported");
    let filter_buckets = buckets
        .as_ref()
        .map(|b| allowed_buckets(b))
        .unwrap_or_default();

    let (where_sql, mut bound) = build_where(&filter_buckets, batch_id.as_deref());
    let sql = format!(
        "SELECT {ROW_COLUMNS} FROM images i LEFT JOIN image_palette p ON p.image_id = i.id{where_sql} ORDER BY {sort} LIMIT ? OFFSET ?",
        sort = sort_clause(sort_key)
    );

    let conn = state.db.0.lock().unwrap();
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    bound.push(duckdb::types::Value::BigInt(limit));
    bound.push(duckdb::types::Value::BigInt(offset));
    let rows = stmt
        .query_map(duckdb::params_from_iter(bound.iter()), |row| row_from(row))
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

#[tauri::command]
fn batch_imported(state: State<'_, AppState>, batch_id: String) -> Result<Vec<ImportedRow>, String> {
    let conn = state.db.0.lock().unwrap();
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {ROW_COLUMNS} \
             FROM images i LEFT JOIN image_palette p ON p.image_id = i.id \
             WHERE i.ingest_batch_id = ? ORDER BY i.imported_at ASC"
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![batch_id], |row| row_from(row))
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

#[tauri::command]
fn get_image_details(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<ImageDetails>, String> {
    let conn = state.db.0.lock().unwrap();
    let result = conn
        .query_row(
            "SELECT i.id, i.content_hash, i.original_filename, i.original_path, \
                    i.byte_size, i.mime, i.imported_at, i.ingest_batch_id, \
                    i.thumbnails_status, i.exif_created_at, i.fs_mtime, \
                    p.dominant_bucket, p.dominant_l, p.dominant_c, p.dominant_h \
             FROM images i LEFT JOIN image_palette p ON p.image_id = i.id \
             WHERE i.id = ?",
            params![id],
            |row| {
                Ok(ImageDetails {
                    id: row.get(0)?,
                    content_hash: row.get(1)?,
                    original_filename: row.get(2)?,
                    original_path: row.get(3)?,
                    byte_size: row.get(4)?,
                    mime: row.get(5)?,
                    imported_at: row
                        .get::<_, chrono::DateTime<chrono::Utc>>(6)?
                        .to_rfc3339(),
                    ingest_batch_id: row.get(7)?,
                    thumbnails_status: row.get(8)?,
                    exif_created_at: row
                        .get::<_, Option<chrono::DateTime<chrono::Utc>>>(9)?
                        .map(|d| d.to_rfc3339()),
                    fs_mtime: row
                        .get::<_, Option<chrono::DateTime<chrono::Utc>>>(10)?
                        .map(|d| d.to_rfc3339()),
                    dominant_bucket: row.get(11)?,
                    dominant_l: row.get(12)?,
                    dominant_c: row.get(13)?,
                    dominant_h: row.get(14)?,
                })
            },
        )
        .ok();
    Ok(result)
}

#[derive(Serialize)]
struct BucketCount {
    bucket: String,
    count: i64,
}

#[tauri::command]
fn list_bucket_counts(state: State<'_, AppState>) -> Result<Vec<BucketCount>, String> {
    let conn = state.db.0.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT bucket, COUNT(*) AS n
             FROM image_palette_bucket
             GROUP BY bucket",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(BucketCount {
                bucket: row.get::<_, String>(0)?,
                count: row.get::<_, i64>(1)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

#[tauri::command]
fn library_count(
    state: State<'_, AppState>,
    buckets: Option<Vec<String>>,
    batch_id: Option<String>,
) -> Result<i64, String> {
    let filter_buckets = buckets
        .as_ref()
        .map(|b| allowed_buckets(b))
        .unwrap_or_default();
    let (where_sql, bound) = build_where(&filter_buckets, batch_id.as_deref());

    let sql = format!("SELECT COUNT(*) FROM images i{where_sql}");
    let conn = state.db.0.lock().unwrap();
    conn.query_row(&sql, duckdb::params_from_iter(bound.iter()), |row| {
        row.get::<_, i64>(0)
    })
    .map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct BatchSummary {
    id: String,
    source_folder: String,
    started_at: String,
    finished_at: Option<String>,
    imported_count: i64,
    skipped_count: i64,
    failed_count: i64,
    image_count: i64,
}

#[tauri::command]
fn list_batches(state: State<'_, AppState>) -> Result<Vec<BatchSummary>, String> {
    let conn = state.db.0.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT b.id, b.source_folder, b.started_at, b.finished_at,
                    b.imported_count, b.skipped_count, b.failed_count,
                    (SELECT COUNT(*) FROM images i WHERE i.ingest_batch_id = b.id) AS image_count
             FROM ingest_batches b
             ORDER BY b.started_at DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(BatchSummary {
                id: row.get::<_, String>(0)?,
                source_folder: row.get::<_, String>(1)?,
                started_at: row.get::<_, chrono::DateTime<chrono::Utc>>(2)?.to_rfc3339(),
                finished_at: row
                    .get::<_, Option<chrono::DateTime<chrono::Utc>>>(3)?
                    .map(|d| d.to_rfc3339()),
                imported_count: row.get::<_, i64>(4)?,
                skipped_count: row.get::<_, i64>(5)?,
                failed_count: row.get::<_, i64>(6)?,
                image_count: row.get::<_, i64>(7)?,
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

            let backfill_handle = app.handle().clone();
            let backfill_db = db.clone();
            let backfill_store = store.clone();
            palette::backfill::schedule(backfill_handle, backfill_db, backfill_store);

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
            list_bucket_counts,
            list_batches,
            library_count,
            get_image_details,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
