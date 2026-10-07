mod collection;
mod curation;
mod db;
mod gestures;
mod history;
mod ingest;
mod library;
mod menu;
pub mod naming;
mod palette;
mod preferences;
mod project;
pub mod resolver;
mod saved_search;
mod search;
mod server;
mod store;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex as AsyncMutex;

use preset_preferences::Preferences;

use crate::db::Db;
use crate::ingest::job::{run_batch, IngestOptions, StartBatchResult};
use crate::ingest::scan::{prescan, FolderRow, PrescanResult};
use crate::library::Library;
use crate::store::StoreRoot;

pub struct AppState {
    pub db: Arc<Db>,
    pub store: StoreRoot,
    pub job_lock: Arc<AsyncMutex<()>>,
    pub prefs: Preferences,
    /// The last pre-scan, kept so the import takes exactly the files the
    /// confirmation showed.
    pub last_scan: Arc<std::sync::Mutex<Option<PrescanResult>>>,
    /// Where projects live: `~/preset-nz/Projects` (work-projects.md).
    pub project_roots: Vec<PathBuf>,
}

fn ingest_options(prefs: &Preferences) -> IngestOptions {
    IngestOptions {
        concurrency: prefs
            .get_int(preferences::CONCURRENCY)
            .map(|n| n.max(1) as usize)
            .unwrap_or_else(|| (num_cpus::get() / 2).max(1)),
        extension_allow_list: prefs
            .get_list(preferences::EXTENSION_ALLOW_LIST)
            .unwrap_or_else(|| vec!["jpg".into(), "jpeg".into(), "png".into()]),
        keep_source_files: prefs
            .get_bool(preferences::KEEP_SOURCE_FILES)
            .unwrap_or(true),
    }
}

#[derive(Serialize)]
struct PrescanSummary {
    root: String,
    total: usize,
    by_extension: std::collections::HashMap<String, usize>,
    folders: Vec<FolderRow>,
    /// Whether source files stay put, so the confirmation's wording is true.
    keep_source_files: bool,
}

#[tauri::command]
async fn ingest_prescan(
    state: State<'_, AppState>,
    path: String,
) -> Result<PrescanSummary, String> {
    let root = PathBuf::from(&path);
    if !root.is_dir() {
        return Err(format!("not a directory: {path}"));
    }
    let options = ingest_options(&state.prefs);
    let allow = options.extension_allow_list;
    let result = tokio::task::spawn_blocking(move || prescan(&root, &allow))
        .await
        .map_err(|e| e.to_string())?;
    let summary = PrescanSummary {
        root: result.root.clone(),
        total: result.total,
        by_extension: result.by_extension.clone(),
        folders: result.folders(),
        keep_source_files: options.keep_source_files,
    };
    *state.last_scan.lock().unwrap() = Some(result);
    Ok(summary)
}

#[tauri::command]
async fn ingest_start(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    folders: Vec<String>,
) -> Result<StartBatchResult, String> {
    let root = PathBuf::from(&path);
    let files = {
        let last = state.last_scan.lock().unwrap();
        match last.as_ref() {
            Some(scan) if Path::new(&scan.root) == root => scan.select(&folders),
            _ => return Err("the folder changed since it was scanned; add it again".into()),
        }
    };
    let lock = state.job_lock.clone();
    let owned = lock
        .try_lock_owned()
        .map_err(|_| "job already in progress".to_string())?;

    let db = state.db.clone();
    let store = state.store.clone();
    let options = ingest_options(&state.prefs);
    run_batch(app, db, store, root, files, options, owned)
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

    title: String,
    generation: Option<GenerationDetails>,
}

/// How a generated image was made, from its provenance record.
#[derive(Serialize)]
struct GenerationDetails {
    producer: String,
    prompt: String,
    negative: Option<String>,
    model: Option<String>,
    seed: Option<i64>,
    /// The producer's own settings, as it wrote them.
    settings: serde_json::Value,
}

#[derive(Serialize)]
struct ImportedRow {
    id: String,
    content_hash: String,
    original_filename: String,
    title: String,
    favourite: bool,
    label: Option<String>,
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
        "orientation" => "ASC",
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
        // Bucket order is wider -> equal -> taller; unbucketed rows trail.
        // Ties fall back to newest-imported so each block reads like the
        // default sheet.
        "orientation" => format!(
            "i.orientation IS NULL ASC, \
             CASE i.orientation WHEN 'landscape' THEN 0 WHEN 'square' THEN 1 ELSE 2 END {dir}, \
             i.imported_at DESC"
        ),
        _ => format!("i.imported_at {dir}"),
    }
}

fn row_from(row: &rusqlite::Row) -> rusqlite::Result<ImportedRow> {
    Ok(ImportedRow {
        id: row.get::<_, String>(0)?,
        content_hash: row.get::<_, String>(1)?,
        original_filename: row.get::<_, String>(2)?,
        title: display_title(
            row.get::<_, Option<String>>(9)?.as_deref(),
            &row.get::<_, String>(2)?,
        ),
        favourite: row.get::<_, Option<bool>>(10)?.unwrap_or(false),
        label: row.get(11)?,
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
                           i.deleted_at, pr.text, mk.favourite, mk.label";

/// The images a list reads, with the palette and the prompt behind each,
/// when it has one, for the display title.
const ROW_SOURCE: &str = "images i \
                          LEFT JOIN image_palette p ON p.image_id = i.id \
                          LEFT JOIN generation_output go ON go.image_id = i.id \
                          LEFT JOIN generation g ON g.generation_id = go.generation_id \
                          LEFT JOIN prompt pr ON pr.prompt_id = g.prompt_id \
                          LEFT JOIN image_mark mk ON mk.image_id = i.id";

/// With a typed query, joins the keyword index so only matches remain and
/// each row carries its BM25 score (lower is better). The join's parameter
/// comes before the WHERE clause's, so its binding goes first.
fn search_join(query: Option<&str>) -> (String, Vec<rusqlite::types::Value>) {
    match query.and_then(search::match_query) {
        Some(q) => (
            format!(
                " JOIN (SELECT image_id AS hit_id, bm25(image_search, {w}) AS score \
                 FROM image_search WHERE image_search MATCH ?) hit ON hit.hit_id = i.id",
                w = search::BM25_WEIGHTS
            ),
            vec![rusqlite::types::Value::Text(q)],
        ),
        None => (String::new(), Vec::new()),
    }
}

/// A generated image is titled from its prompt; anything else by its filename.
fn display_title(prompt: Option<&str>, filename: &str) -> String {
    prompt
        .and_then(ingest::provenance::title_from_prompt)
        .unwrap_or_else(|| filename.to_string())
}

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

fn allowed_orientations(input: &[String]) -> Vec<String> {
    input
        .iter()
        .filter(|o| crate::ingest::orientation::is_bucket(o))
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

fn allowed_labels(input: &[String]) -> Vec<String> {
    input
        .iter()
        .filter(|l| l.as_str() == "favourite" || curation::LABELS.contains(&l.as_str()))
        .cloned()
        .collect()
}

fn build_where(
    filter_buckets: &[String],
    filter_orientations: &[String],
    filter_labels: &[String],
    batch_id: Option<&str>,
    collection_id: Option<&str>,
    project: Option<(&str, Option<&Path>)>,
    deleted: DeletedFilter,
) -> (String, Vec<rusqlite::types::Value>) {
    let mut clauses: Vec<String> = Vec::new();
    let mut bound: Vec<rusqlite::types::Value> = Vec::new();

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
            bound.push(rusqlite::types::Value::Text(b.clone()));
        }
    }
    if !filter_orientations.is_empty() {
        let placeholders = vec!["?"; filter_orientations.len()].join(", ");
        clauses.push(format!("i.orientation IN ({placeholders})"));
        for o in filter_orientations {
            bound.push(rusqlite::types::Value::Text(o.clone()));
        }
    }
    // Favourited and the colours are one section in the rail, read as "any of".
    if !filter_labels.is_empty() {
        let mut any = Vec::new();
        if filter_labels.iter().any(|l| l == "favourite") {
            any.push("favourite = 1".to_string());
        }
        let colours: Vec<&String> = filter_labels.iter().filter(|l| *l != "favourite").collect();
        if !colours.is_empty() {
            any.push(format!(
                "label IN ({})",
                vec!["?"; colours.len()].join(", ")
            ));
            for c in colours {
                bound.push(rusqlite::types::Value::Text(c.clone()));
            }
        }
        clauses.push(format!(
            "i.id IN (SELECT image_id FROM image_mark WHERE {})",
            any.join(" OR ")
        ));
    }
    if let Some(b) = batch_id {
        clauses.push("i.ingest_batch_id = ?".to_string());
        bound.push(rusqlite::types::Value::Text(b.to_string()));
    }
    if let Some(c) = collection_id {
        clauses.push(
            "i.id IN (SELECT image_id FROM collection_member WHERE collection_id = ?)".to_string(),
        );
        bound.push(rusqlite::types::Value::Text(c.to_string()));
    }
    if let Some((key, folder)) = project {
        let (clause, values) = project::membership(key, folder);
        clauses.push(clause);
        bound.extend(values);
    }

    let where_sql = if clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", clauses.join(" AND "))
    };
    (where_sql, bound)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn list_images(
    state: State<'_, AppState>,
    offset: i64,
    limit: i64,
    sort: Option<String>,
    direction: Option<String>,
    buckets: Option<Vec<String>>,
    orientations: Option<Vec<String>>,
    labels: Option<Vec<String>>,
    batch_id: Option<String>,
    collection_id: Option<String>,
    project_key: Option<String>,
    include_deleted: Option<bool>,
    only_deleted: Option<bool>,
    query: Option<String>,
) -> Result<Vec<ImportedRow>, String> {
    let limit = limit.clamp(1, 1000);
    let offset = offset.max(0);
    let sort_key = sort.as_deref().unwrap_or("imported");
    let filter_buckets = buckets
        .as_ref()
        .map(|b| allowed_buckets(b))
        .unwrap_or_default();
    let filter_orientations = orientations
        .as_ref()
        .map(|o| allowed_orientations(o))
        .unwrap_or_default();
    let deleted = resolve_deleted(include_deleted, only_deleted);

    let (join_sql, mut bound) = search_join(query.as_deref());
    let filter_labels = labels
        .as_ref()
        .map(|l| allowed_labels(l))
        .unwrap_or_default();
    let project_folder = project_key
        .as_deref()
        .and_then(|k| project::folder_of(&state.project_roots, k));
    let (where_sql, where_bound) = build_where(
        &filter_buckets,
        &filter_orientations,
        &filter_labels,
        batch_id.as_deref(),
        collection_id.as_deref(),
        project_key
            .as_deref()
            .map(|k| (k, project_folder.as_deref())),
        deleted,
    );
    bound.extend(where_bound);
    // A search ranks by relevance; the sort control applies to browsing.
    let order = if join_sql.is_empty() {
        sort_clause(sort_key, direction.as_deref())
    } else {
        "hit.score ASC, i.imported_at DESC".to_string()
    };
    let sql = format!(
        "SELECT {ROW_COLUMNS} FROM {ROW_SOURCE}{join_sql}{where_sql} ORDER BY {order} LIMIT ? OFFSET ?"
    );

    let conn = state.db.0.lock().unwrap();
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    bound.push(rusqlite::types::Value::Integer(limit));
    bound.push(rusqlite::types::Value::Integer(offset));
    let rows = stmt
        .query_map(rusqlite::params_from_iter(bound.iter()), |row| {
            row_from(row)
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

#[tauri::command]
fn batch_imported(
    state: State<'_, AppState>,
    batch_id: String,
) -> Result<Vec<ImportedRow>, String> {
    let conn = state.db.0.lock().unwrap();
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {ROW_COLUMNS} \
             FROM {ROW_SOURCE} \
             WHERE i.ingest_batch_id = ? AND i.deleted_at IS NULL \
             ORDER BY i.imported_at ASC"
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![batch_id], row_from)
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
                    imported_at: row.get::<_, chrono::DateTime<chrono::Utc>>(6)?.to_rfc3339(),
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
                    title: String::new(),
                    generation: None,
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
            details.generation = conn
                .query_row(
                    "SELECT g.producer, pr.text, pr.negative, g.model, g.seed, g.settings \
                     FROM generation_output go \
                     JOIN generation g ON g.generation_id = go.generation_id \
                     JOIN prompt pr ON pr.prompt_id = g.prompt_id \
                     WHERE go.image_id = ? LIMIT 1",
                    params![id],
                    |row| {
                        Ok(GenerationDetails {
                            producer: row.get(0)?,
                            prompt: row.get(1)?,
                            negative: row.get(2)?,
                            model: row.get(3)?,
                            seed: row.get(4)?,
                            settings: serde_json::from_str(&row.get::<_, String>(5)?)
                                .unwrap_or(serde_json::Value::Null),
                        })
                    },
                )
                .optional()
                .map_err(|e| e.to_string())?;
            details.title = display_title(
                details.generation.as_ref().map(|g| g.prompt.as_str()),
                &details.original_filename,
            );
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

#[derive(Serialize)]
struct OrientationCount {
    orientation: String,
    count: i64,
}

#[tauri::command]
fn list_orientation_counts(state: State<'_, AppState>) -> Result<Vec<OrientationCount>, String> {
    let conn = state.db.0.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT orientation, COUNT(*) AS n
             FROM images
             WHERE deleted_at IS NULL AND orientation IS NOT NULL
             GROUP BY orientation",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(OrientationCount {
                orientation: row.get::<_, String>(0)?,
                count: row.get::<_, i64>(1)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn library_count(
    state: State<'_, AppState>,
    buckets: Option<Vec<String>>,
    orientations: Option<Vec<String>>,
    labels: Option<Vec<String>>,
    batch_id: Option<String>,
    collection_id: Option<String>,
    project_key: Option<String>,
    include_deleted: Option<bool>,
    only_deleted: Option<bool>,
    query: Option<String>,
) -> Result<i64, String> {
    let filter_buckets = buckets
        .as_ref()
        .map(|b| allowed_buckets(b))
        .unwrap_or_default();
    let filter_orientations = orientations
        .as_ref()
        .map(|o| allowed_orientations(o))
        .unwrap_or_default();
    let deleted = resolve_deleted(include_deleted, only_deleted);
    let (join_sql, mut bound) = search_join(query.as_deref());
    let filter_labels = labels
        .as_ref()
        .map(|l| allowed_labels(l))
        .unwrap_or_default();
    let project_folder = project_key
        .as_deref()
        .and_then(|k| project::folder_of(&state.project_roots, k));
    let (where_sql, where_bound) = build_where(
        &filter_buckets,
        &filter_orientations,
        &filter_labels,
        batch_id.as_deref(),
        collection_id.as_deref(),
        project_key
            .as_deref()
            .map(|k| (k, project_folder.as_deref())),
        deleted,
    );
    bound.extend(where_bound);

    let sql = format!("SELECT COUNT(*) FROM images i{join_sql}{where_sql}");
    let conn = state.db.0.lock().unwrap();
    conn.query_row(&sql, rusqlite::params_from_iter(bound.iter()), |row| {
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
fn get_batch(state: State<'_, AppState>, id: String) -> Result<Option<BatchSummary>, String> {
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
             WHERE EXISTS (SELECT 1 FROM images i
                           WHERE i.ingest_batch_id = b.id AND i.deleted_at IS NULL)
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
    let n = {
        let conn = state.db.0.lock().unwrap();
        soft_delete(&conn, &ids).map_err(|e| e.to_string())?
    };
    if n > 0 {
        let _ = app.emit("library://images-trashed", &ids);
    }
    Ok(n)
}

fn soft_delete(conn: &rusqlite::Connection, ids: &[String]) -> rusqlite::Result<usize> {
    let placeholders = vec!["?"; ids.len()].join(", ");
    let sql = format!(
        "UPDATE images SET deleted_at = ?, deleted_reason = COALESCE(deleted_reason, 'user') \
         WHERE id IN ({placeholders}) AND deleted_at IS NULL"
    );
    // The timestamp comes from chrono, like every other, so the purge
    // sweep's `deleted_at < ?` compares like with like.
    let deleted_at = chrono::Utc::now();
    let bound: Vec<&dyn rusqlite::ToSql> = std::iter::once(&deleted_at as &dyn rusqlite::ToSql)
        .chain(ids.iter().map(|id| id as &dyn rusqlite::ToSql))
        .collect();
    conn.prepare(&sql)?.execute(bound.as_slice())
}

/// Hard-delete the catalog rows and filesystem artefacts for a single image.
/// Side tables go first; the parent `images` row goes last so a partial failure
/// keeps the row soft-deleted (deleted_at IS NOT NULL) and a future sweep can
/// retry. Filesystem misses are logged but don't fail the operation — orphan
/// binaries on disk are cheaper than orphan catalog rows.
fn purge_one(
    conn: &rusqlite::Connection,
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
        "DELETE FROM image_search WHERE image_id = ?",
        "DELETE FROM image_mark WHERE image_id = ?",
        "DELETE FROM collection_member WHERE image_id = ?",
        "DELETE FROM project_member WHERE image_id = ?",
        // The prompt and generation stay: the record outlives the output.
        "DELETE FROM generation_output WHERE image_id = ?",
        "DELETE FROM provenance_checked WHERE image_id = ?",
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
        let bound: Vec<rusqlite::types::Value> = ids
            .iter()
            .map(|id| rusqlite::types::Value::Text(id.clone()))
            .collect();
        stmt.execute(rusqlite::params_from_iter(bound.iter()))
            .map_err(|e| e.to_string())?
    };
    if n > 0 {
        let _ = app.emit("library://images-restored", &ids);
    }
    Ok(n)
}

/// One-shot at app start: hard-delete every image whose soft-delete window has
/// elapsed. The window is the `library.retention_days` preference, read once
/// here. Runs on a background thread so the UI isn't blocked while the
/// cascade walks the filesystem. Daily timer is deferred (epic 11 open Q).
fn purge_expired_on_start(db: Arc<Db>, store: crate::store::StoreRoot, retention_days: i64) {
    std::thread::spawn(move || {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(retention_days);
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

/// Records a done step in the curation history and retitles Undo.
fn record(app: &AppHandle, history: &history::Curation, step: Option<history::Step>) {
    if let Some(step) = step {
        history.push(step);
        preset_app_kit::refresh_history(app);
    }
}

#[tauri::command]
fn project_list(state: State<'_, AppState>) -> Result<Vec<project::Project>, String> {
    let conn = state.db.0.lock().unwrap();
    project::list(&conn, &state.project_roots).map_err(|e| e.to_string())
}

/// File › New Project…: makes the folder and its marker. Files on disk aren't
/// an undo step; the catalog has nothing to take back.
#[tauri::command]
fn project_create(
    state: State<'_, AppState>,
    name: String,
    description: String,
) -> Result<String, String> {
    let root = state.project_roots.first().ok_or("no projects folder")?;
    let (marker, _) =
        project::create_on_disk(root, &name, &description).map_err(|e| e.to_string())?;
    Ok(marker.key)
}

/// The project's name for undo labels, from its marker.
fn project_name(state: &AppState, key: &str) -> String {
    project::discover(&state.project_roots)
        .into_iter()
        .find(|(m, _)| m.key == key)
        .map_or_else(|| key.to_string(), |(m, _)| m.name)
}

#[tauri::command]
fn project_set_favourite(
    app: AppHandle,
    state: State<'_, AppState>,
    history: State<'_, history::Curation>,
    key: String,
    on: bool,
) -> Result<(), String> {
    let name = project_name(&state, &key);
    let step = {
        let conn = state.db.0.lock().unwrap();
        project::set_favourite(&conn, &key, &name, on).map_err(|e| e.to_string())?
    };
    record(&app, &history, step);
    Ok(())
}

#[tauri::command]
fn project_set_archived(
    app: AppHandle,
    state: State<'_, AppState>,
    history: State<'_, history::Curation>,
    key: String,
    on: bool,
) -> Result<(), String> {
    let name = project_name(&state, &key);
    let step = {
        let conn = state.db.0.lock().unwrap();
        project::set_archived(&conn, &key, &name, on).map_err(|e| e.to_string())?
    };
    record(&app, &history, step);
    Ok(())
}

#[tauri::command]
fn project_add(
    app: AppHandle,
    state: State<'_, AppState>,
    history: State<'_, history::Curation>,
    key: String,
    images: Vec<String>,
) -> Result<(), String> {
    let name = project_name(&state, &key);
    let step = {
        let conn = state.db.0.lock().unwrap();
        project::add(&conn, &key, &name, &images).map_err(|e| e.to_string())?
    };
    record(&app, &history, step);
    Ok(())
}

/// Removes images added by hand. Returns false when nothing could be removed
/// (they belong because they sit under the project's folder).
#[tauri::command]
fn project_remove(
    app: AppHandle,
    state: State<'_, AppState>,
    history: State<'_, history::Curation>,
    key: String,
    images: Vec<String>,
) -> Result<bool, String> {
    let name = project_name(&state, &key);
    let step = {
        let conn = state.db.0.lock().unwrap();
        project::remove(&conn, &key, &name, &images).map_err(|e| e.to_string())?
    };
    let removed = step.is_some();
    record(&app, &history, step);
    Ok(removed)
}

#[tauri::command]
fn collection_list(state: State<'_, AppState>) -> Result<Vec<collection::Collection>, String> {
    let conn = state.db.0.lock().unwrap();
    collection::list(&conn).map_err(|e| e.to_string())
}

/// New collection holding `images` (possibly none); one undo step. Returns its id.
#[tauri::command]
fn collection_create(
    app: AppHandle,
    state: State<'_, AppState>,
    history: State<'_, history::Curation>,
    name: String,
    images: Vec<String>,
) -> Result<String, String> {
    let (id, step) = {
        let conn = state.db.0.lock().unwrap();
        collection::create(&conn, &name, &images).map_err(|e| e.to_string())?
    };
    record(&app, &history, Some(step));
    Ok(id)
}

#[tauri::command]
fn collection_rename(
    app: AppHandle,
    state: State<'_, AppState>,
    history: State<'_, history::Curation>,
    id: String,
    name: String,
) -> Result<(), String> {
    let step = {
        let conn = state.db.0.lock().unwrap();
        collection::rename(&conn, &id, &name).map_err(|e| e.to_string())?
    };
    record(&app, &history, step);
    Ok(())
}

#[tauri::command]
fn collection_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    history: State<'_, history::Curation>,
    id: String,
) -> Result<(), String> {
    let step = {
        let conn = state.db.0.lock().unwrap();
        collection::delete(&conn, &id).map_err(|e| e.to_string())?
    };
    record(&app, &history, Some(step));
    Ok(())
}

#[tauri::command]
fn collection_add(
    app: AppHandle,
    state: State<'_, AppState>,
    history: State<'_, history::Curation>,
    id: String,
    images: Vec<String>,
) -> Result<(), String> {
    let step = {
        let conn = state.db.0.lock().unwrap();
        collection::add(&conn, &id, &images).map_err(|e| e.to_string())?
    };
    record(&app, &history, step);
    Ok(())
}

#[tauri::command]
fn collection_remove(
    app: AppHandle,
    state: State<'_, AppState>,
    history: State<'_, history::Curation>,
    id: String,
    images: Vec<String>,
) -> Result<(), String> {
    let step = {
        let conn = state.db.0.lock().unwrap();
        collection::remove(&conn, &id, &images).map_err(|e| e.to_string())?
    };
    record(&app, &history, step);
    Ok(())
}

/// Hearts or un-hearts images; one undo step.
#[tauri::command]
fn set_favourite(
    app: AppHandle,
    state: State<'_, AppState>,
    history: State<'_, history::Curation>,
    ids: Vec<String>,
    on: bool,
) -> Result<Vec<curation::Marked>, String> {
    let action = if on { "Favourite" } else { "Unfavourite" };
    mark(&app, &state, &history, &ids, action, |m| curation::Mark {
        favourite: on,
        ..m.clone()
    })
}

/// Sets or clears the colour label of images; one undo step.
#[tauri::command]
fn set_colour_label(
    app: AppHandle,
    state: State<'_, AppState>,
    history: State<'_, history::Curation>,
    ids: Vec<String>,
    label: Option<String>,
) -> Result<Vec<curation::Marked>, String> {
    curation::check_label(label.as_deref()).map_err(|e| e.to_string())?;
    let action = match &label {
        Some(l) => format!("Label {}{}", l[..1].to_uppercase(), &l[1..]),
        None => "Clear Label".to_string(),
    };
    mark(&app, &state, &history, &ids, &action, |m| curation::Mark {
        label: label.clone(),
        ..m.clone()
    })
}

fn mark(
    app: &AppHandle,
    state: &AppState,
    history: &history::Curation,
    ids: &[String],
    action: &str,
    change: impl Fn(&curation::Mark) -> curation::Mark,
) -> Result<Vec<curation::Marked>, String> {
    let changed = {
        let conn = state.db.0.lock().unwrap();
        curation::change(&conn, ids, action, change).map_err(|e| e.to_string())?
    };
    let Some((shown, step)) = changed else {
        return Ok(Vec::new());
    };
    record(app, history, Some(step));
    Ok(shown)
}

/// Current marks, for cards that didn't come from a list query (the contact
/// sheet's skipped duplicates).
#[tauri::command]
fn image_marks(
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> Result<Vec<curation::Marked>, String> {
    let conn = state.db.0.lock().unwrap();
    let marks = curation::marks(&conn, &ids).map_err(|e| e.to_string())?;
    Ok(ids
        .into_iter()
        .map(|id| {
            let mark = marks.get(&id).cloned().unwrap_or_default();
            curation::Marked { id, mark }
        })
        .collect())
}

#[derive(Serialize)]
struct LabelCount {
    label: String,
    count: i64,
}

/// Counts for the rail's Label section: "favourite" and each colour, live images only.
#[tauri::command]
fn list_label_counts(state: State<'_, AppState>) -> Result<Vec<LabelCount>, String> {
    let conn = state.db.0.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT 'favourite', COUNT(*) FROM image_mark mk JOIN images i ON i.id = mk.image_id
              WHERE mk.favourite = 1 AND i.deleted_at IS NULL
             UNION ALL
             SELECT mk.label, COUNT(*) FROM image_mark mk JOIN images i ON i.id = mk.image_id
              WHERE mk.label IS NOT NULL AND i.deleted_at IS NULL GROUP BY mk.label",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(LabelCount {
                label: r.get(0)?,
                count: r.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

/// The palette tool: k swatches from one image, computed now, not stored.
#[tauri::command]
async fn extract_palette(
    state: State<'_, AppState>,
    id: String,
    k: usize,
) -> Result<Vec<palette::extract::ExtractedSwatch>, String> {
    let source = display_source(&state, &id)?;
    tokio::task::spawn_blocking(move || palette::extract::extract(&source, k))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// The palette inspector: n colours placed on the 1024 px display image.
#[tauri::command]
async fn palette_markers(
    state: State<'_, AppState>,
    id: String,
    n: usize,
) -> Result<Vec<palette::extract::Marker>, String> {
    let source = display_source(&state, &id)?;
    tokio::task::spawn_blocking(move || palette::extract::markers(&source, n))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// The 1024 px thumbnail Quickview shows, or the original if it's missing.
fn display_source(state: &AppState, id: &str) -> Result<PathBuf, String> {
    let (hash, store_path): (String, String) = {
        let conn = state.db.0.lock().unwrap();
        conn.query_row(
            "SELECT content_hash, store_path FROM images WHERE id = ?",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| e.to_string())?
    };
    let thumb = state.store.thumb_path(&hash, 1024);
    Ok(if thumb.exists() {
        thumb
    } else {
        state.store.root().join(store_path)
    })
}

#[tauri::command]
fn saved_search_list(state: State<'_, AppState>) -> Result<Vec<saved_search::SavedSearch>, String> {
    let conn = state.db.0.lock().unwrap();
    saved_search::list(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn saved_search_put(
    state: State<'_, AppState>,
    id: Option<String>,
    name: String,
    query: serde_json::Value,
    created_at: Option<String>,
    project_key: Option<String>,
) -> Result<saved_search::SavedSearch, String> {
    let conn = state.db.0.lock().unwrap();
    saved_search::put(
        &conn,
        id.as_deref(),
        &name,
        &query,
        created_at.as_deref(),
        project_key.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn saved_search_delete(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<saved_search::SavedSearch>, String> {
    let conn = state.db.0.lock().unwrap();
    saved_search::delete(&conn, &id).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let pictures_dir = app
                .path()
                .picture_dir()
                .expect("failed to resolve the Pictures folder");
            let library = Library::in_pictures(&pictures_dir);

            let store = library.store();
            store.ensure()?;

            app.manage(history::Curation::default());
            menu::install(app.handle())?;

            let db_path = library.catalog_path();
            let db = Arc::new(Db::open(&db_path)?);

            let prefs = Preferences::open(
                preferences::schema(store.root(), &db_path),
                preset_preferences::tauri::default_path(app.handle())?,
            );
            // The file keeps whatever paths were current when it was last
            // saved, and loading prefers the file over the schema default,
            // read-only or not. Restate where the library is now.
            preferences::restate_catalog_paths(&prefs, store.root(), &db_path);
            let retention_days = prefs.get_int(preferences::RETENTION_DAYS).unwrap_or(30);
            purge_expired_on_start(db.clone(), store.clone(), retention_days);

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

            ingest::orientation::schedule(app.handle().clone(), db.clone(), store.clone());
            ingest::provenance::backfill::schedule(app.handle().clone(), db.clone(), store.clone());
            {
                let db = db.clone();
                std::thread::spawn(move || {
                    let conn = db.0.lock().unwrap();
                    if let Err(e) = search::backfill(&conn) {
                        eprintln!("search backfill failed: {e}");
                    }
                });
            }

            app.manage(prefs.clone());
            app.manage(AppState {
                db,
                store,
                job_lock: Arc::new(AsyncMutex::new(())),
                prefs,
                last_scan: Arc::new(std::sync::Mutex::new(None)),
                project_roots: vec![app
                    .path()
                    .home_dir()
                    .expect("failed to resolve the home folder")
                    .join("preset-nz")
                    .join("Projects")],
            });
            Ok(())
        })
        .register_uri_scheme_protocol(
            crate::server::thumbnails::SCHEME,
            crate::server::thumbnails::handler,
        )
        .invoke_handler(tauri::generate_handler![
            ingest_prescan,
            ingest_start,
            last_batch,
            batch_imported,
            list_images,
            list_bucket_counts,
            list_orientation_counts,
            list_batches,
            library_count,
            get_image_details,
            get_batch,
            delete_image,
            restore_image,
            purge_image,
            extract_palette,
            set_favourite,
            set_colour_label,
            list_label_counts,
            image_marks,
            collection_list,
            project_list,
            project_create,
            project_set_favourite,
            project_set_archived,
            project_add,
            project_remove,
            collection_create,
            collection_rename,
            collection_delete,
            collection_add,
            collection_remove,
            palette_markers,
            saved_search_list,
            saved_search_put,
            saved_search_delete,
            preset_app_kit::app_kit_commands,
            preset_app_kit::app_kit_menu_state,
            preset_app_kit::app_kit_history,
            preset_app_kit::app_kit_undo,
            preset_app_kit::app_kit_redo,
            preset_app_kit::app_kit_text_menu,
            preset_preferences::tauri::preferences_get,
            preset_preferences::tauri::preferences_set,
            preset_preferences::tauri::preferences_reset,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// The library's SQL against a real SQLite catalog: what ingest writes,
/// the list queries read back through `row_from` for every sort and filter.
/// A row that fails to decode is dropped by `filter_map(Result::ok)` and
/// would look like an empty library, so each query asserts the row.
#[cfg(test)]
mod catalog_round_trip {
    use super::*;
    use chrono::Utc;
    use rusqlite::params;

    const IMAGE: &str = "00000000-0000-4000-8000-000000000001";
    const BATCH: &str = "00000000-0000-4000-8000-0000000000b1";

    fn catalog() -> (Db, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("strata-catalog-{}", uuid::Uuid::new_v4()));
        let db = Db::open(&dir.join("catalog.sqlite")).unwrap();
        (db, dir)
    }

    /// The writes ingest makes, in the order `job.rs` makes them.
    fn ingest_one(db: &Db, dir: &std::path::Path) {
        {
            let conn = db.0.lock().unwrap();
            conn.execute(
                "INSERT INTO ingest_batches (id, source_folder, started_at, imported_count, skipped_count, failed_count) VALUES (?, ?, ?, 0, 0, 0)",
                params![BATCH, "/inbox", Utc::now()],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO images (id, content_hash, store_path, original_filename, original_path, byte_size, mime, imported_at, ingest_batch_id, thumbnails_status, exif_created_at, fs_mtime) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    IMAGE, "abcd", "ab/cd/abcd.png", "mist.png", "/inbox/mist.png", 42i64,
                    "image/png", Utc::now(), BATCH, "ready", Some(Utc::now()), Some(Utc::now()),
                ],
            )
            .unwrap();
            conn.execute(
                "UPDATE ingest_batches SET finished_at = ?, imported_count = ?, skipped_count = ?, failed_count = ? WHERE id = ?",
                params![Utc::now(), 1i64, 0i64, 0i64, BATCH],
            )
            .unwrap();
            conn.execute(
                "INSERT OR REPLACE INTO image_palette \
                 (image_id, swatches, dominant_bucket, dominant_l, dominant_c, dominant_h, extracted_at, stage_version) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                params![IMAGE, "[]", "green", 40.5f32, 22.0f32, 130.0f32, Utc::now(), "palette@1"],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO image_palette_bucket (image_id, bucket) VALUES (?, ?)",
                params![IMAGE, "green"],
            )
            .unwrap();
        }

        let extracted = ingest::metadata::Extracted {
            exif: Some(ingest::metadata::exif::ExifData {
                camera_make: Some("Fujifilm".into()),
                iso: Some(400),
                f_number: Some(2.8),
                flash_fired: Some(false),
                gps_latitude: Some(-41.29),
                ..Default::default()
            }),
            iptc: Some(ingest::metadata::iptc::Iptc {
                date_created: Some(Utc::now()),
                keywords: vec!["mould".into(), "bark".into()],
                ..Default::default()
            }),
        };
        ingest::metadata::write(db, IMAGE, &extracted).unwrap();

        let png = dir.join("mist.png");
        image::RgbImage::new(4, 2).save(&png).unwrap();
        ingest::orientation::run(db, IMAGE, &png, None).unwrap();
    }

    fn list(
        conn: &rusqlite::Connection,
        where_sql: &str,
        bound: &[rusqlite::types::Value],
        sort: &str,
    ) -> Vec<ImportedRow> {
        let sql = format!(
            "SELECT {ROW_COLUMNS} FROM {ROW_SOURCE}{where_sql} ORDER BY {sort} LIMIT 10 OFFSET 0",
        );
        let mut stmt = conn.prepare(&sql).unwrap();
        let rows = stmt
            .query_map(rusqlite::params_from_iter(bound.iter()), |row| {
                row_from(row)
            })
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        rows
    }

    #[test]
    fn ingest_writes_read_back_through_every_sort_and_filter() {
        let (db, dir) = catalog();
        ingest_one(&db, &dir);
        let conn = db.0.lock().unwrap();

        for sort in [
            "imported",
            "filename",
            "created",
            "updated",
            "colour",
            "deleted",
            "orientation",
        ] {
            for direction in [Some("asc"), Some("desc"), None] {
                let (where_sql, bound) =
                    build_where(&[], &[], &[], None, None, None, DeletedFilter::HideDeleted);
                let rows = list(&conn, &where_sql, &bound, &sort_clause(sort, direction));
                assert_eq!(rows.len(), 1, "sort {sort} {direction:?}");
                assert_eq!(rows[0].dominant_bucket.as_deref(), Some("green"));
            }
        }

        let filtered = build_where(
            &["green".into()],
            &["landscape".into()],
            &[],
            Some(BATCH),
            None,
            None,
            DeletedFilter::HideDeleted,
        );
        assert_eq!(
            list(&conn, &filtered.0, &filtered.1, "i.imported_at").len(),
            1
        );
        let miss = build_where(
            &["red".into()],
            &[],
            &[],
            None,
            None,
            None,
            DeletedFilter::HideDeleted,
        );
        assert!(list(&conn, &miss.0, &miss.1, "i.imported_at").is_empty());

        let (w, h, bucket): (i32, i32, String) = conn
            .query_row(
                "SELECT width, height, orientation FROM images WHERE id = ?",
                params![IMAGE],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!((w, h, bucket.as_str()), (4, 2, "landscape"));

        #[allow(clippy::type_complexity)]
        let (iso, f_number, flash, lat, created): (Option<i32>, Option<f32>, Option<bool>, Option<f32>, Option<chrono::DateTime<Utc>>) = conn
            .query_row(
                "SELECT iso, f_number, flash_fired, gps_latitude, iptc_date_created FROM image_metadata WHERE image_id = ?",
                params![IMAGE],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .unwrap();
        assert_eq!((iso, f_number, flash), (Some(400), Some(2.8), Some(false)));
        assert!(lat.is_some() && created.is_some());

        let finished: Option<chrono::DateTime<Utc>> = conn
            .query_row(
                "SELECT finished_at FROM ingest_batches WHERE id = ?",
                params![BATCH],
                |r| r.get(0),
            )
            .unwrap();
        assert!(finished.is_some());

        drop(conn);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn provenance_titles_rows_and_rereads_replace() {
        let (db, dir) = catalog();
        ingest_one(&db, &dir);
        let record = ingest::provenance::drawthings::parse_record(
            r#"{"c":"Lichen colony, wet bark","uc":"blurry","model":"flux","seed":7,"steps":4}"#,
        )
        .unwrap();
        for _ in 0..2 {
            ingest::provenance::store(&db, IMAGE, "abcd", Some(&record)).unwrap();
        }
        let conn = db.0.lock().unwrap();
        let counts: (i64, i64, i64) = conn
            .query_row(
                "SELECT (SELECT COUNT(*) FROM prompt), (SELECT COUNT(*) FROM generation), (SELECT COUNT(*) FROM generation_output)",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(counts, (1, 1, 1));

        let (where_sql, bound) =
            build_where(&[], &[], &[], None, None, None, DeletedFilter::HideDeleted);
        let rows = list(&conn, &where_sql, &bound, &sort_clause("imported", None));
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title, "lichen-colony");
        assert_eq!(rows[0].original_filename, "mist.png");

        drop(conn);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn keyword_search_finds_prompt_keyword_and_filename_words() {
        let (db, dir) = catalog();
        ingest_one(&db, &dir);
        let record = ingest::provenance::drawthings::parse_record(
            r#"{"c":"Lichen colony, (mist:1.3) over wet bark","seed":7}"#,
        )
        .unwrap();
        ingest::provenance::store(&db, IMAGE, "abcd", Some(&record)).unwrap();
        let conn = db.0.lock().unwrap();

        let search = |typed: &str| -> Vec<ImportedRow> {
            let (join_sql, mut bound) = search_join(Some(typed));
            let (where_sql, where_bound) =
                build_where(&[], &[], &[], None, None, None, DeletedFilter::HideDeleted);
            bound.extend(where_bound);
            list(
                &conn,
                &format!("{join_sql}{where_sql}"),
                &bound,
                "hit.score ASC",
            )
        };
        assert_eq!(search("lich").len(), 1, "prefix of a prompt word");
        assert_eq!(search("mist bark").len(), 1, "weights stripped, every word");
        assert_eq!(search("mould").len(), 1, "IPTC keyword");
        assert_eq!(search("mist.png").len(), 1, "filename words");
        assert!(search("1.3").is_empty(), "weight numbers not indexed");
        assert!(search("fungal").is_empty());
        assert!(search("lichen fungal").is_empty(), "all words must match");

        assert_eq!(search::backfill(&conn).unwrap(), 0, "already indexed");
        drop(conn);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn marks_show_on_rows_and_filter_by_label() {
        let (db, dir) = catalog();
        ingest_one(&db, &dir);
        let conn = db.0.lock().unwrap();
        let ids = vec![IMAGE.to_string()];
        curation::change(&conn, &ids, "Favourite", |m| curation::Mark {
            favourite: true,
            ..m.clone()
        })
        .unwrap();
        curation::change(&conn, &ids, "Label Red", |m| curation::Mark {
            label: Some("red".into()),
            ..m.clone()
        })
        .unwrap();

        let by = |labels: &[&str]| {
            let labels: Vec<String> = labels.iter().map(|l| l.to_string()).collect();
            let (where_sql, bound) = build_where(
                &[],
                &[],
                &labels,
                None,
                None,
                None,
                DeletedFilter::HideDeleted,
            );
            list(&conn, &where_sql, &bound, "i.imported_at")
        };
        let all = by(&[]);
        assert!(all[0].favourite);
        assert_eq!(all[0].label.as_deref(), Some("red"));
        assert_eq!(by(&["favourite"]).len(), 1);
        assert_eq!(by(&["red"]).len(), 1);
        assert_eq!(by(&["blue"]).len(), 0);
        assert_eq!(by(&["blue", "favourite"]).len(), 1, "any of the selected");
        assert_eq!(allowed_labels(&["teal".into(), "grey".into()]), ["grey"]);

        let (coll, _) = collection::create(&conn, "Bark", &ids).unwrap();
        let in_coll = |c: &str| {
            let (where_sql, bound) = build_where(
                &[],
                &[],
                &[],
                None,
                Some(c),
                None,
                DeletedFilter::HideDeleted,
            );
            list(&conn, &where_sql, &bound, "i.imported_at").len()
        };
        assert_eq!(in_coll(&coll), 1);
        assert_eq!(in_coll("other"), 0);
        drop(conn);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn trash_then_purge_sweep_compares_timestamps() {
        let (db, dir) = catalog();
        ingest_one(&db, &dir);
        let conn = db.0.lock().unwrap();

        assert_eq!(soft_delete(&conn, &[IMAGE.to_string()]).unwrap(), 1);
        let (where_sql, bound) =
            build_where(&[], &[], &[], None, None, None, DeletedFilter::OnlyDeleted);
        let trashed = list(&conn, &where_sql, &bound, &sort_clause("deleted", None));
        assert_eq!(trashed.len(), 1);
        assert!(trashed[0].deleted_at.is_some());

        let expired = |cutoff: chrono::DateTime<Utc>| -> i64 {
            conn.query_row(
                "SELECT COUNT(*) FROM images WHERE deleted_at IS NOT NULL AND deleted_at < ?",
                params![cutoff],
                |r| r.get(0),
            )
            .unwrap()
        };
        assert_eq!(expired(Utc::now() + chrono::Duration::days(1)), 1);
        assert_eq!(expired(Utc::now() - chrono::Duration::days(1)), 0);

        drop(conn);
        std::fs::remove_dir_all(dir).ok();
    }
}
