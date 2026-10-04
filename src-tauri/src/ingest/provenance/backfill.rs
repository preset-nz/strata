use std::sync::Arc;

use anyhow::Result;
use rusqlite::params;
use tauri::{AppHandle, Emitter};

use crate::db::Db;
use crate::ingest::metadata::backfill::BackfillProgress;
use crate::ingest::provenance::{self, STAGE_VERSION};
use crate::store::StoreRoot;

/// Reads the record of every catalogued image not yet checked at the
/// current stage version. Idempotent: a second run finds nothing to do.
pub fn schedule(app: AppHandle, db: Arc<Db>, store: StoreRoot) {
    std::thread::spawn(move || {
        if let Err(e) = run(&app, &db, &store) {
            eprintln!("provenance backfill failed: {e}");
        }
    });
}

pub fn run(app: &AppHandle, db: &Db, store: &StoreRoot) -> Result<()> {
    let pending = pending(db)?;
    let total = pending.len();
    if total == 0 {
        return Ok(());
    }
    let _ = app.emit(
        "provenance://backfill-start",
        BackfillProgress { done: 0, total, failed: 0 },
    );
    let mut done = 0usize;
    let mut failed = 0usize;
    for (image_id, content_hash, store_path) in pending {
        if let Err(e) = provenance::run(db, &image_id, &content_hash, &store.root().join(&store_path)) {
            failed += 1;
            eprintln!("provenance backfill: {image_id} failed: {e}");
        }
        done += 1;
        let _ = app.emit(
            "provenance://backfill-progress",
            BackfillProgress { done, total, failed },
        );
    }
    let _ = app.emit(
        "provenance://backfill-done",
        BackfillProgress { done, total, failed },
    );
    Ok(())
}

fn pending(db: &Db) -> Result<Vec<(String, String, String)>> {
    let conn = db.0.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT i.id, i.content_hash, i.store_path
         FROM images i
         LEFT JOIN provenance_checked c ON c.image_id = i.id
         WHERE i.deleted_at IS NULL
           AND (c.image_id IS NULL OR c.stage_version <> ?)",
    )?;
    let rows = stmt
        .query_map(params![STAGE_VERSION], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}
