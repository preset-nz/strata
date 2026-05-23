use std::path::Path;
use std::sync::Mutex;

use anyhow::Result;
use duckdb::Connection;

pub struct Db(pub Mutex<Connection>);

impl Db {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        init_schema(&conn)?;
        Ok(Self(Mutex::new(conn)))
    }
}

fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
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
    )?;
    Ok(())
}
