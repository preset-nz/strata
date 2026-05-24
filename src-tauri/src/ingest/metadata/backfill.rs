use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use duckdb::params;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::db::Db;
use crate::ingest::metadata::{self, STAGE_VERSION};
use crate::store::StoreRoot;

#[derive(Serialize, Clone)]
pub struct BackfillProgress {
    pub done: usize,
    pub total: usize,
    pub failed: usize,
}

pub fn schedule(app: AppHandle, db: Arc<Db>, store: StoreRoot) {
    std::thread::spawn(move || {
        if let Err(e) = run(&app, &db, &store) {
            eprintln!("metadata backfill failed: {e}");
        }
    });
}

pub fn run(app: &AppHandle, db: &Db, store: &StoreRoot) -> Result<()> {
    let pending: Vec<(String, String)> = {
        let conn = db.0.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT i.id, i.store_path
             FROM images i
             LEFT JOIN image_metadata m ON m.image_id = i.id
             WHERE i.deleted_at IS NULL
               AND (m.image_id IS NULL OR m.stage_version <> ?)",
        )?;
        stmt.query_map(params![STAGE_VERSION], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .filter_map(Result::ok)
        .collect()
    };

    let total = pending.len();
    if total == 0 {
        return Ok(());
    }

    let _ = app.emit(
        "metadata://backfill-start",
        BackfillProgress { done: 0, total, failed: 0 },
    );

    let mut done = 0usize;
    let mut failed = 0usize;
    for (image_id, store_path) in pending {
        let target: PathBuf = store.root().join(&store_path);
        let extracted = metadata::extract(&target);
        if let Err(e) = metadata::write(db, &image_id, &extracted) {
            failed += 1;
            eprintln!("metadata backfill: {image_id} failed: {e}");
        }
        done += 1;
        let _ = app.emit(
            "metadata://backfill-progress",
            BackfillProgress { done, total, failed },
        );
    }

    let _ = app.emit(
        "metadata://backfill-done",
        BackfillProgress { done, total, failed },
    );
    Ok(())
}
