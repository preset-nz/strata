mod db;
mod gestures;
mod ingest;
mod palette;
mod server;
mod store;

use std::path::PathBuf;
use std::sync::Arc;

use duckdb::params;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
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

    camera_make: Option<String>,
    camera_model: Option<String>,
    lens_make: Option<String>,
    lens_model: Option<String>,
    focal_length_mm: Option<f32>,
    focal_length_35mm: Option<f32>,

    iso: Option<i32>,
    f_number: Option<f32>,
    exposure_time_sec: Option<f32>,
    exposure_bias: Option<f32>,
    exposure_program: Option<String>,
    metering_mode: Option<String>,
    flash_fired: Option<bool>,

    pixel_width: Option<i32>,
    pixel_height: Option<i32>,
    orientation: Option<i32>,
    color_space: Option<String>,

    gps_latitude: Option<f32>,
    gps_longitude: Option<f32>,
    gps_altitude_m: Option<f32>,

    iptc_title: Option<String>,
    iptc_caption: Option<String>,
    iptc_byline: Option<String>,
    iptc_copyright: Option<String>,
    iptc_city: Option<String>,
    iptc_state: Option<String>,
    iptc_country: Option<String>,
    iptc_date_created: Option<String>,

    software: Option<String>,
    keywords: Vec<String>,
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
    deleted_at: Option<String>,
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

fn default_direction(sort: &str) -> &'static str {
    match sort {
        "filename" => "ASC",
        "colour" => "ASC",
        _ => "DESC",
    }
}

fn sort_clause(sort: &str, direction: Option<&str>) -> String {
    let dir = match direction {
        Some("asc") => "ASC",
        Some("desc") => "DESC",
        _ => default_direction(sort),
    };
    match sort {
        "filename" => format!("i.original_filename {dir}"),
        "created" => format!("COALESCE(i.exif_created_at, i.fs_mtime, i.imported_at) {dir}"),
        "updated" => format!("COALESCE(p.extracted_at, i.imported_at) {dir}"),
        "colour" => format!(
            "p.dominant_l IS NULL {dir}, \
             CASE WHEN p.dominant_c < 10 THEN 0 ELSE 1 END {dir}, \
             CASE WHEN p.dominant_c < 10 THEN p.dominant_l ELSE p.dominant_h END {dir}"
        ),
        "deleted" => format!("i.deleted_at {dir}"),
        _ => format!("i.imported_at {dir}"),
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
        deleted_at: row
            .get::<_, Option<chrono::DateTime<chrono::Utc>>>(8)?
            .map(|d| d.to_rfc3339()),
    })
}

const ROW_COLUMNS: &str = "i.id, i.content_hash, i.original_filename, i.thumbnails_status, \
                           p.dominant_bucket, p.dominant_l, p.dominant_c, p.dominant_h, \
                           i.deleted_at";

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

#[derive(Clone, Copy)]
enum DeletedFilter {
    HideDeleted,
    OnlyDeleted,
    IncludeAll,
}

fn resolve_deleted(include_deleted: Option<bool>, only_deleted: Option<bool>) -> DeletedFilter {
    if only_deleted.unwrap_or(false) {
        DeletedFilter::OnlyDeleted
    } else if include_deleted.unwrap_or(false) {
        DeletedFilter::IncludeAll
    } else {
        DeletedFilter::HideDeleted
    }
}

fn build_where(
    filter_buckets: &[String],
    batch_id: Option<&str>,
    deleted: DeletedFilter,
) -> (String, Vec<duckdb::types::Value>) {
    let mut clauses: Vec<String> = Vec::new();
    let mut bound: Vec<duckdb::types::Value> = Vec::new();

    match deleted {
        DeletedFilter::HideDeleted => clauses.push("i.deleted_at IS NULL".to_string()),
        DeletedFilter::OnlyDeleted => clauses.push("i.deleted_at IS NOT NULL".to_string()),
        DeletedFilter::IncludeAll => {}
    }

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
    direction: Option<String>,
    buckets: Option<Vec<String>>,
    batch_id: Option<String>,
    include_deleted: Option<bool>,
    only_deleted: Option<bool>,
) -> Result<Vec<ImportedRow>, String> {
    let limit = limit.clamp(1, 1000);
    let offset = offset.max(0);
    let sort_key = sort.as_deref().unwrap_or("imported");
    let filter_buckets = buckets
        .as_ref()
        .map(|b| allowed_buckets(b))
        .unwrap_or_default();
    let deleted = resolve_deleted(include_deleted, only_deleted);

    let (where_sql, mut bound) = build_where(&filter_buckets, batch_id.as_deref(), deleted);
    let sql = format!(
        "SELECT {ROW_COLUMNS} FROM images i LEFT JOIN image_palette p ON p.image_id = i.id{where_sql} ORDER BY {sort} LIMIT ? OFFSET ?",
        sort = sort_clause(sort_key, direction.as_deref())
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
             WHERE i.ingest_batch_id = ? AND i.deleted_at IS NULL \
             ORDER BY i.imported_at ASC"
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
                    p.dominant_bucket, p.dominant_l, p.dominant_c, p.dominant_h, \
                    m.camera_make, m.camera_model, m.lens_make, m.lens_model, \
                    m.focal_length_mm, m.focal_length_35mm, \
                    m.iso, m.f_number, m.exposure_time_sec, m.exposure_bias, \
                    m.exposure_program, m.metering_mode, m.flash_fired, \
                    m.pixel_width, m.pixel_height, m.orientation, m.color_space, \
                    m.gps_latitude, m.gps_longitude, m.gps_altitude_m, \
                    m.iptc_title, m.iptc_caption, m.iptc_byline, m.iptc_copyright, \
                    m.iptc_city, m.iptc_state, m.iptc_country, m.iptc_date_created, \
                    m.software \
             FROM images i \
             LEFT JOIN image_palette p ON p.image_id = i.id \
             LEFT JOIN image_metadata m ON m.image_id = i.id \
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

                    camera_make: row.get(15)?,
                    camera_model: row.get(16)?,
                    lens_make: row.get(17)?,
                    lens_model: row.get(18)?,
                    focal_length_mm: row.get(19)?,
                    focal_length_35mm: row.get(20)?,

                    iso: row.get(21)?,
                    f_number: row.get(22)?,
                    exposure_time_sec: row.get(23)?,
                    exposure_bias: row.get(24)?,
                    exposure_program: row.get(25)?,
                    metering_mode: row.get(26)?,
                    flash_fired: row.get(27)?,

                    pixel_width: row.get(28)?,
                    pixel_height: row.get(29)?,
                    orientation: row.get(30)?,
                    color_space: row.get(31)?,

                    gps_latitude: row.get(32)?,
                    gps_longitude: row.get(33)?,
                    gps_altitude_m: row.get(34)?,

                    iptc_title: row.get(35)?,
                    iptc_caption: row.get(36)?,
                    iptc_byline: row.get(37)?,
                    iptc_copyright: row.get(38)?,
                    iptc_city: row.get(39)?,
                    iptc_state: row.get(40)?,
                    iptc_country: row.get(41)?,
                    iptc_date_created: row
                        .get::<_, Option<chrono::DateTime<chrono::Utc>>>(42)?
                        .map(|d| d.to_rfc3339()),

                    software: row.get(43)?,
                    keywords: Vec::new(),
                })
            },
        )
        .ok();

    let result = match result {
        Some(mut details) => {
            let mut stmt = conn
                .prepare("SELECT keyword FROM image_keyword WHERE image_id = ? ORDER BY keyword")
                .map_err(|e| e.to_string())?;
            let keywords: Vec<String> = stmt
                .query_map(params![id], |row| row.get::<_, String>(0))
                .map_err(|e| e.to_string())?
                .filter_map(Result::ok)
                .collect();
            details.keywords = keywords;
            Some(details)
        }
        None => None,
    };
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
            "SELECT b.bucket, COUNT(*) AS n
             FROM image_palette_bucket b
             JOIN images i ON i.id = b.image_id
             WHERE i.deleted_at IS NULL
             GROUP BY b.bucket",
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
    include_deleted: Option<bool>,
    only_deleted: Option<bool>,
) -> Result<i64, String> {
    let filter_buckets = buckets
        .as_ref()
        .map(|b| allowed_buckets(b))
        .unwrap_or_default();
    let deleted = resolve_deleted(include_deleted, only_deleted);
    let (where_sql, bound) = build_where(&filter_buckets, batch_id.as_deref(), deleted);

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
fn get_batch(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<BatchSummary>, String> {
    let conn = state.db.0.lock().unwrap();
    let result = conn
        .query_row(
            "SELECT b.id, b.source_folder, b.started_at, b.finished_at,
                    b.imported_count, b.skipped_count, b.failed_count,
                    (SELECT COUNT(*) FROM images i WHERE i.ingest_batch_id = b.id AND i.deleted_at IS NULL) AS image_count
             FROM ingest_batches b WHERE b.id = ?",
            params![id],
            |row| {
                Ok(BatchSummary {
                    id: row.get::<_, String>(0)?,
                    source_folder: row.get::<_, String>(1)?,
                    started_at: row
                        .get::<_, chrono::DateTime<chrono::Utc>>(2)?
                        .to_rfc3339(),
                    finished_at: row
                        .get::<_, Option<chrono::DateTime<chrono::Utc>>>(3)?
                        .map(|d| d.to_rfc3339()),
                    imported_count: row.get::<_, i64>(4)?,
                    skipped_count: row.get::<_, i64>(5)?,
                    failed_count: row.get::<_, i64>(6)?,
                    image_count: row.get::<_, i64>(7)?,
                })
            },
        )
        .ok();
    Ok(result)
}

#[tauri::command]
fn list_batches(state: State<'_, AppState>) -> Result<Vec<BatchSummary>, String> {
    let conn = state.db.0.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT b.id, b.source_folder, b.started_at, b.finished_at,
                    b.imported_count, b.skipped_count, b.failed_count,
                    (SELECT COUNT(*) FROM images i WHERE i.ingest_batch_id = b.id AND i.deleted_at IS NULL) AS image_count
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

#[tauri::command]
fn delete_image(
    app: AppHandle,
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> Result<usize, String> {
    if ids.is_empty() {
        return Ok(0);
    }
    let placeholders = vec!["?"; ids.len()].join(", ");
    let sql = format!(
        "UPDATE images SET deleted_at = now(), deleted_reason = COALESCE(deleted_reason, 'user') \
         WHERE id IN ({placeholders}) AND deleted_at IS NULL"
    );

    let n = {
        let conn = state.db.0.lock().unwrap();
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let bound: Vec<duckdb::types::Value> = ids
            .iter()
            .map(|id| duckdb::types::Value::Text(id.clone()))
            .collect();
        stmt.execute(duckdb::params_from_iter(bound.iter()))
            .map_err(|e| e.to_string())?
    };
    if n > 0 {
        let _ = app.emit("library://images-trashed", &ids);
    }
    Ok(n)
}

/// Hard-delete the catalog rows and filesystem artefacts for a single image.
/// Side tables go first; the parent `images` row goes last so a partial failure
/// keeps the row soft-deleted (deleted_at IS NOT NULL) and a future sweep can
/// retry. Filesystem misses are logged but don't fail the operation — orphan
/// binaries on disk are cheaper than orphan catalog rows.
fn purge_one(
    conn: &duckdb::Connection,
    store: &crate::store::StoreRoot,
    id: &str,
) -> Result<(), String> {
    let (content_hash, store_path): (String, String) = conn
        .query_row(
            "SELECT content_hash, store_path FROM images WHERE id = ?",
            params![id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .map_err(|e| e.to_string())?;

    for sql in [
        "DELETE FROM image_palette_bucket WHERE image_id = ?",
        "DELETE FROM image_palette WHERE image_id = ?",
        "DELETE FROM image_metadata WHERE image_id = ?",
        "DELETE FROM image_keyword WHERE image_id = ?",
        "DELETE FROM images WHERE id = ?",
    ] {
        conn.execute(sql, params![id]).map_err(|e| e.to_string())?;
    }

    let binary = store.root().join(&store_path);
    remove_quiet(&binary);
    for size in [256u32, 512, 1024] {
        remove_quiet(&store.thumb_path(&content_hash, size));
    }
    Ok(())
}

fn remove_quiet(path: &std::path::Path) {
    if let Err(e) = std::fs::remove_file(path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            eprintln!("purge: could not remove {}: {}", path.display(), e);
        }
    }
}

#[tauri::command]
fn purge_image(
    app: AppHandle,
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> Result<usize, String> {
    if ids.is_empty() {
        return Ok(0);
    }
    let purged: Vec<String> = {
        let conn = state.db.0.lock().unwrap();
        let mut ok = Vec::with_capacity(ids.len());
        for id in &ids {
            match purge_one(&conn, &state.store, id) {
                Ok(()) => ok.push(id.clone()),
                Err(e) => eprintln!("purge_image({id}): {e}"),
            }
        }
        ok
    };
    if !purged.is_empty() {
        let _ = app.emit("library://images-purged", &purged);
    }
    Ok(purged.len())
}

#[tauri::command]
fn restore_image(
    app: AppHandle,
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> Result<usize, String> {
    if ids.is_empty() {
        return Ok(0);
    }
    let placeholders = vec!["?"; ids.len()].join(", ");
    let sql = format!(
        "UPDATE images SET deleted_at = NULL, deleted_reason = NULL \
         WHERE id IN ({placeholders}) AND deleted_at IS NOT NULL"
    );

    let n = {
        let conn = state.db.0.lock().unwrap();
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let bound: Vec<duckdb::types::Value> = ids
            .iter()
            .map(|id| duckdb::types::Value::Text(id.clone()))
            .collect();
        stmt.execute(duckdb::params_from_iter(bound.iter()))
            .map_err(|e| e.to_string())?
    };
    if n > 0 {
        let _ = app.emit("library://images-restored", &ids);
    }
    Ok(n)
}

const RETENTION_DAYS: i64 = 30;

/// One-shot at app start: hard-delete every image whose soft-delete window has
/// elapsed. Runs on a background thread so the UI isn't blocked while the
/// cascade walks the filesystem. Daily timer is deferred (epic 11 open Q).
fn purge_expired_on_start(db: Arc<Db>, store: crate::store::StoreRoot) {
    std::thread::spawn(move || {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(RETENTION_DAYS);
        let ids: Vec<String> = {
            let conn = db.0.lock().unwrap();
            let mut stmt = match conn.prepare(
                "SELECT id FROM images \
                 WHERE deleted_at IS NOT NULL AND deleted_at < ?",
            ) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("purge sweep: prepare failed: {e}");
                    return;
                }
            };
            let rows = stmt.query_map(params![cutoff], |row| row.get::<_, String>(0));
            match rows {
                Ok(it) => it.filter_map(Result::ok).collect(),
                Err(e) => {
                    eprintln!("purge sweep: query failed: {e}");
                    return;
                }
            }
        };
        if ids.is_empty() {
            return;
        }
        eprintln!("purge sweep: hard-deleting {} expired images", ids.len());
        let conn = db.0.lock().unwrap();
        for id in &ids {
            if let Err(e) = purge_one(&conn, &store, id) {
                eprintln!("purge sweep: {id}: {e}");
            }
        }
    });
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

            purge_expired_on_start(db.clone(), store.clone());

            #[cfg(target_os = "macos")]
            crate::gestures::force_touch::install(app.handle().clone());

            let palette_handle = app.handle().clone();
            let palette_db = db.clone();
            let palette_store = store.clone();
            palette::backfill::schedule(palette_handle, palette_db, palette_store);

            let metadata_handle = app.handle().clone();
            let metadata_db = db.clone();
            let metadata_store = store.clone();
            ingest::metadata::backfill::schedule(metadata_handle, metadata_db, metadata_store);

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
            get_batch,
            delete_image,
            restore_image,
            purge_image,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
