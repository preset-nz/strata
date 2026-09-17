use std::path::Path;
use std::sync::Mutex;

use anyhow::Result;
use chrono::Utc;
use duckdb::{params, Connection};

pub struct Db(pub Mutex<Connection>);

impl Db {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        apply_migrations(&conn)?;
        Ok(Self(Mutex::new(conn)))
    }
}

const MIGRATIONS: &[(i64, &str)] = &[
    (
        1,
        r#"
        CREATE TABLE IF NOT EXISTS images (
            id UUID PRIMARY KEY,
            content_hash TEXT NOT NULL UNIQUE,
            store_path TEXT NOT NULL,
            original_filename TEXT NOT NULL,
            original_path TEXT NOT NULL,
            byte_size BIGINT NOT NULL,
            mime TEXT NOT NULL,
            imported_at TIMESTAMP NOT NULL,
            ingest_batch_id UUID NOT NULL,
            thumbnails_status TEXT NOT NULL DEFAULT 'missing'
        );

        CREATE INDEX IF NOT EXISTS idx_images_batch ON images(ingest_batch_id);

        CREATE TABLE IF NOT EXISTS ingest_batches (
            id UUID PRIMARY KEY,
            source_folder TEXT NOT NULL,
            started_at TIMESTAMP NOT NULL,
            finished_at TIMESTAMP,
            imported_count INTEGER NOT NULL DEFAULT 0,
            skipped_count INTEGER NOT NULL DEFAULT 0,
            failed_count INTEGER NOT NULL DEFAULT 0
        );
        "#,
    ),
    (
        2,
        r#"
        ALTER TABLE images ADD COLUMN exif_created_at TIMESTAMP;
        ALTER TABLE images ADD COLUMN fs_mtime TIMESTAMP;

        CREATE TABLE IF NOT EXISTS image_palette (
            image_id UUID PRIMARY KEY REFERENCES images(id),
            swatches TEXT NOT NULL,
            dominant_bucket TEXT NOT NULL,
            dominant_l REAL NOT NULL,
            dominant_c REAL NOT NULL,
            dominant_h REAL NOT NULL,
            extracted_at TIMESTAMP NOT NULL,
            stage_version TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS image_palette_bucket (
            image_id UUID NOT NULL REFERENCES images(id),
            bucket TEXT NOT NULL,
            PRIMARY KEY (image_id, bucket)
        );

        CREATE INDEX IF NOT EXISTS idx_image_palette_dominant_bucket
            ON image_palette(dominant_bucket);
        CREATE INDEX IF NOT EXISTS idx_image_palette_bucket_bucket
            ON image_palette_bucket(bucket);
        "#,
    ),
    (
        3,
        r#"
        CREATE TABLE IF NOT EXISTS image_metadata (
            image_id UUID PRIMARY KEY REFERENCES images(id),

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
            flash_fired BOOLEAN,

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
            iptc_date_created TIMESTAMP,

            software TEXT,
            extracted_at TIMESTAMP NOT NULL,
            stage_version TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS image_keyword (
            image_id UUID NOT NULL REFERENCES images(id),
            keyword TEXT NOT NULL,
            PRIMARY KEY (image_id, keyword)
        );

        CREATE INDEX IF NOT EXISTS idx_image_keyword_keyword
            ON image_keyword(keyword);
        "#,
    ),
    // v4 re-applies v3's CREATE TABLE statements idempotently. A local dev DB
    // accumulated a stale v3 row (from an earlier placeholder migration) without
    // the corresponding tables; bumping the version lets the migrator make
    // forward progress without manually editing schema_version.
    (
        4,
        r#"
        CREATE TABLE IF NOT EXISTS image_metadata (
            image_id UUID PRIMARY KEY REFERENCES images(id),

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
            flash_fired BOOLEAN,

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
            iptc_date_created TIMESTAMP,

            software TEXT,
            extracted_at TIMESTAMP NOT NULL,
            stage_version TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS image_keyword (
            image_id UUID NOT NULL REFERENCES images(id),
            keyword TEXT NOT NULL,
            PRIMARY KEY (image_id, keyword)
        );

        CREATE INDEX IF NOT EXISTS idx_image_keyword_keyword
            ON image_keyword(keyword);
        "#,
    ),
    (
        5,
        r#"
        ALTER TABLE images ADD COLUMN deleted_at TIMESTAMP;
        ALTER TABLE images ADD COLUMN deleted_reason TEXT;

        CREATE INDEX IF NOT EXISTS idx_images_deleted_at ON images(deleted_at);
        "#,
    ),
    // v6 — drop foreign-key constraints from side tables.
    // DuckDB refuses UPDATE on a parent row that has FK-referenced children,
    // even when the referenced column (images.id) is unchanged. That breaks
    // soft-delete, which only touches images.deleted_at. Strata is single-
    // writer local-first and the hard-delete cascade is managed explicitly
    // in Rust, so the FKs were never carrying enforcement weight.
    // DuckDB's ALTER TABLE DROP CONSTRAINT support is patchy for unnamed FKs,
    // so we recreate each side table by copy-rename. Indexes are dropped
    // explicitly before the table swap — leaving them implicit triggered
    // `unbound_index.cpp` assertion failures from orphaned column-id mappings
    // on the persisted catalog after the rename.
    (
        6,
        r#"
        DROP INDEX IF EXISTS idx_image_palette_dominant_bucket;
        DROP INDEX IF EXISTS idx_image_palette_bucket_bucket;
        DROP INDEX IF EXISTS idx_image_keyword_keyword;

        CREATE TABLE image_palette_new (
            image_id UUID PRIMARY KEY,
            swatches TEXT NOT NULL,
            dominant_bucket TEXT NOT NULL,
            dominant_l REAL NOT NULL,
            dominant_c REAL NOT NULL,
            dominant_h REAL NOT NULL,
            extracted_at TIMESTAMP NOT NULL,
            stage_version TEXT NOT NULL
        );
        INSERT INTO image_palette_new SELECT * FROM image_palette;
        DROP TABLE image_palette;
        ALTER TABLE image_palette_new RENAME TO image_palette;
        CREATE INDEX idx_image_palette_dominant_bucket
            ON image_palette(dominant_bucket);

        CREATE TABLE image_palette_bucket_new (
            image_id UUID NOT NULL,
            bucket TEXT NOT NULL,
            PRIMARY KEY (image_id, bucket)
        );
        INSERT INTO image_palette_bucket_new SELECT * FROM image_palette_bucket;
        DROP TABLE image_palette_bucket;
        ALTER TABLE image_palette_bucket_new RENAME TO image_palette_bucket;
        CREATE INDEX idx_image_palette_bucket_bucket
            ON image_palette_bucket(bucket);

        CREATE TABLE image_metadata_new (
            image_id UUID PRIMARY KEY,

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
            flash_fired BOOLEAN,

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
            iptc_date_created TIMESTAMP,

            software TEXT,
            extracted_at TIMESTAMP NOT NULL,
            stage_version TEXT NOT NULL
        );
        INSERT INTO image_metadata_new SELECT * FROM image_metadata;
        DROP TABLE image_metadata;
        ALTER TABLE image_metadata_new RENAME TO image_metadata;

        CREATE TABLE image_keyword_new (
            image_id UUID NOT NULL,
            keyword TEXT NOT NULL,
            PRIMARY KEY (image_id, keyword)
        );
        INSERT INTO image_keyword_new SELECT * FROM image_keyword;
        DROP TABLE image_keyword;
        ALTER TABLE image_keyword_new RENAME TO image_keyword;
        CREATE INDEX idx_image_keyword_keyword ON image_keyword(keyword);
        "#,
    ),
    // v7 — orientation bucket (epic 14). Displayed pixel dimensions and the
    // landscape / square / portrait bucket live on `images` because every
    // list and count query already reads that table. Existing rows are
    // backfilled on the next app start by `ingest::orientation::schedule`.
    (
        7,
        r#"
        ALTER TABLE images ADD COLUMN width INTEGER;
        ALTER TABLE images ADD COLUMN height INTEGER;
        ALTER TABLE images ADD COLUMN orientation TEXT;
        CREATE INDEX IF NOT EXISTS idx_images_orientation ON images(orientation);
        "#,
    ),
];

fn apply_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TIMESTAMP NOT NULL
        );",
    )?;

    let current: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |r| r.get(0),
    )?;

    for (version, sql) in MIGRATIONS {
        if *version > current {
            conn.execute_batch(sql)?;
            conn.execute(
                "INSERT INTO schema_version (version, applied_at) VALUES (?, ?)",
                params![version, Utc::now()],
            )?;
        }
    }
    Ok(())
}
