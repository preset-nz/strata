use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection};

pub struct Db(pub Mutex<Connection>);

impl Db {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        configure(&conn)?;
        apply_migrations(&conn)?;
        Ok(Self(Mutex::new(conn)))
    }
}

/// WAL lets the UI, the CLI and background jobs read while one of them
/// writes; the busy timeout makes a second writer wait its turn instead of
/// failing with SQLITE_BUSY.
fn configure(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.busy_timeout(Duration::from_secs(5))?;
    Ok(())
}

// Ids are UUIDs stored as TEXT. Timestamps are TEXT written by rusqlite's
// chrono encoder (RFC 3339, UTC), so they sort and compare as strings; every
// timestamp must come through that encoder, never from SQL's own clock.
// Side tables carry no foreign keys: the hard-delete cascade is explicit in
// Rust (`purge_one`).
const MIGRATIONS: &[(i64, &str)] = &[(
    1,
    r#"
    CREATE TABLE images (
        id TEXT PRIMARY KEY,
        content_hash TEXT NOT NULL UNIQUE,
        store_path TEXT NOT NULL,
        original_filename TEXT NOT NULL,
        original_path TEXT NOT NULL,
        byte_size INTEGER NOT NULL,
        mime TEXT NOT NULL,
        imported_at TEXT NOT NULL,
        ingest_batch_id TEXT NOT NULL,
        thumbnails_status TEXT NOT NULL DEFAULT 'missing',
        exif_created_at TEXT,
        fs_mtime TEXT,
        deleted_at TEXT,
        deleted_reason TEXT,
        -- Displayed pixel dimensions and the landscape / square / portrait
        -- bucket (epic 14), backfilled by `ingest::orientation::schedule`.
        width INTEGER,
        height INTEGER,
        orientation TEXT
    );

    CREATE INDEX idx_images_batch ON images(ingest_batch_id);
    CREATE INDEX idx_images_deleted_at ON images(deleted_at);

    CREATE TABLE ingest_batches (
        id TEXT PRIMARY KEY,
        source_folder TEXT NOT NULL,
        started_at TEXT NOT NULL,
        finished_at TEXT,
        imported_count INTEGER NOT NULL DEFAULT 0,
        skipped_count INTEGER NOT NULL DEFAULT 0,
        failed_count INTEGER NOT NULL DEFAULT 0
    );

    CREATE TABLE image_palette (
        image_id TEXT PRIMARY KEY,
        swatches TEXT NOT NULL,
        dominant_bucket TEXT NOT NULL,
        dominant_l REAL NOT NULL,
        dominant_c REAL NOT NULL,
        dominant_h REAL NOT NULL,
        extracted_at TEXT NOT NULL,
        stage_version TEXT NOT NULL
    );

    CREATE INDEX idx_image_palette_dominant_bucket
        ON image_palette(dominant_bucket);

    CREATE TABLE image_palette_bucket (
        image_id TEXT NOT NULL,
        bucket TEXT NOT NULL,
        PRIMARY KEY (image_id, bucket)
    );

    CREATE INDEX idx_image_palette_bucket_bucket
        ON image_palette_bucket(bucket);

    CREATE TABLE image_metadata (
        image_id TEXT PRIMARY KEY,

        camera_make TEXT,
        camera_model TEXT,
        lens_make TEXT,
        lens_model TEXT,
        focal_length_mm REAL,
        focal_length_35mm REAL,

        iso INTEGER,
        f_number REAL,
        exposure_time_sec REAL,
        exposure_bias REAL,
        exposure_program TEXT,
        metering_mode TEXT,
        flash_fired INTEGER,

        pixel_width INTEGER,
        pixel_height INTEGER,
        orientation INTEGER,
        color_space TEXT,

        gps_latitude REAL,
        gps_longitude REAL,
        gps_altitude_m REAL,

        iptc_title TEXT,
        iptc_caption TEXT,
        iptc_byline TEXT,
        iptc_copyright TEXT,
        iptc_city TEXT,
        iptc_state TEXT,
        iptc_country TEXT,
        iptc_date_created TEXT,

        software TEXT,
        extracted_at TEXT NOT NULL,
        stage_version TEXT NOT NULL
    );

    CREATE TABLE image_keyword (
        image_id TEXT NOT NULL,
        keyword TEXT NOT NULL,
        PRIMARY KEY (image_id, keyword)
    );

    CREATE INDEX idx_image_keyword_keyword ON image_keyword(keyword);
    "#,
),
// v2 — provenance (guidance `design/provenance-and-evals.md`). A prompt is
// its own entity, deduplicated by a hash of text and negative; a generation
// keeps the producer's settings as JSON; generation_output links it to the
// image. provenance_checked records which images have been read, found or
// not, so the backfill is idempotent.
(
    2,
    r#"
    CREATE TABLE prompt (
        prompt_id TEXT PRIMARY KEY,
        text TEXT NOT NULL,
        negative TEXT,
        first_seen TEXT NOT NULL
    );

    CREATE TABLE generation (
        generation_id TEXT PRIMARY KEY,
        prompt_id TEXT NOT NULL,
        producer TEXT NOT NULL,
        model TEXT,
        seed INTEGER,
        settings TEXT NOT NULL,
        at TEXT
    );

    CREATE INDEX idx_generation_prompt ON generation(prompt_id);

    CREATE TABLE generation_output (
        generation_id TEXT NOT NULL,
        image_id TEXT NOT NULL,
        kind TEXT NOT NULL,
        PRIMARY KEY (generation_id, image_id)
    );

    CREATE INDEX idx_generation_output_image ON generation_output(image_id);

    CREATE TABLE provenance_checked (
        image_id TEXT PRIMARY KEY,
        stage_version TEXT NOT NULL,
        checked_at TEXT NOT NULL
    );
    "#,
)];

fn apply_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL
        );",
    )?;

    let current: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |r| r.get(0),
    )?;

    for (version, sql) in MIGRATIONS {
        if *version > current {
            let tx = conn.unchecked_transaction()?;
            tx.execute_batch(sql)?;
            tx.execute(
                "INSERT INTO schema_version (version, applied_at) VALUES (?, ?)",
                params![version, Utc::now()],
            )?;
            tx.commit()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_in_wal_and_migrates_once() {
        let dir = std::env::temp_dir().join(format!("strata-db-{}", uuid::Uuid::new_v4()));
        let path = dir.join("catalog.sqlite");
        drop(Db::open(&path).unwrap());
        let db = Db::open(&path).unwrap();
        let conn = db.0.lock().unwrap();
        let mode: String = conn.query_row("PRAGMA journal_mode", [], |r| r.get(0)).unwrap();
        assert_eq!(mode, "wal");
        let applied: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(applied, MIGRATIONS.len() as i64);
        drop(conn);
        std::fs::remove_dir_all(dir).ok();
    }
}
