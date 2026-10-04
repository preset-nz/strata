//! Orientation bucket — landscape / portrait / square — derived from an
//! image's displayed dimensions.
//!
//! Dimensions come from the codec header only (`image::image_dimensions`), so
//! probing costs a few KB of I/O and no decode. EXIF orientation tags 5..8
//! rotate the stored pixels by 90 degrees, so those swap width and height
//! before bucketing: the bucket describes what the user sees, not how the
//! bytes are laid out.
//!
//! The 1.1 threshold is part of the schema. Anything within +-10% of 1:1 is
//! square; that keeps deliberate squares (1080x1080) together without
//! calling every slightly cropped photo a square.

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use rusqlite::params;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::db::Db;
use crate::store::StoreRoot;

pub const LANDSCAPE: &str = "landscape";
pub const PORTRAIT: &str = "portrait";
pub const SQUARE: &str = "square";

/// Every bucket, in the order the sort key and rail present them:
/// wider, equal, taller.
pub const BUCKETS: [&str; 3] = [LANDSCAPE, SQUARE, PORTRAIT];

const THRESHOLD: f64 = 1.1;

pub fn classify(width: u32, height: u32) -> &'static str {
    if width == 0 || height == 0 {
        return SQUARE;
    }
    let w = width as f64;
    let h = height as f64;
    if w / h > THRESHOLD {
        LANDSCAPE
    } else if h / w > THRESHOLD {
        PORTRAIT
    } else {
        SQUARE
    }
}

pub fn is_bucket(name: &str) -> bool {
    BUCKETS.contains(&name)
}

/// Displayed dimensions: header dimensions with EXIF rotation applied.
pub fn displayed_dimensions(path: &Path, exif_orientation: Option<i32>) -> Result<(u32, u32)> {
    let (w, h) = image::image_dimensions(path)?;
    Ok(apply_exif_rotation(w, h, exif_orientation))
}

fn apply_exif_rotation(w: u32, h: u32, exif_orientation: Option<i32>) -> (u32, u32) {
    match exif_orientation {
        Some(5..=8) => (h, w),
        _ => (w, h),
    }
}

/// Probe the stored binary and record width, height and bucket on the row.
pub fn run(db: &Db, image_id: &str, stored_path: &Path, exif_orientation: Option<i32>) -> Result<()> {
    let (w, h) = displayed_dimensions(stored_path, exif_orientation)?;
    let bucket = classify(w, h);
    let conn = db.0.lock().unwrap();
    conn.execute(
        "UPDATE images SET width = ?, height = ?, orientation = ? WHERE id = ?",
        params![w as i32, h as i32, bucket, image_id],
    )?;
    Ok(())
}

#[derive(Serialize, Clone)]
pub struct BackfillProgress {
    pub done: usize,
    pub total: usize,
    pub failed: usize,
}

/// One-shot on app start: bucket every live row that has no orientation yet.
/// Header-only reads make this fast enough to run inline on a background
/// thread; the rail refreshes on `orientation://backfill-done`.
pub fn schedule(app: AppHandle, db: Arc<Db>, store: StoreRoot) {
    std::thread::spawn(move || {
        if let Err(e) = backfill(&app, &db, &store) {
            eprintln!("orientation backfill failed: {e}");
        }
    });
}

fn backfill(app: &AppHandle, db: &Db, store: &StoreRoot) -> Result<()> {
    let pending: Vec<(String, String, Option<i32>)> = {
        let conn = db.0.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT i.id, i.store_path, m.orientation
             FROM images i
             LEFT JOIN image_metadata m ON m.image_id = i.id
             WHERE i.orientation IS NULL AND i.deleted_at IS NULL",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<i32>>(2)?,
            ))
        })?
        .filter_map(Result::ok)
        .collect();
        rows
    };

    let total = pending.len();
    if total == 0 {
        return Ok(());
    }
    let _ = app.emit(
        "orientation://backfill-start",
        BackfillProgress { done: 0, total, failed: 0 },
    );

    let mut done = 0usize;
    let mut failed = 0usize;
    for (image_id, store_path, exif_orientation) in pending {
        let target = store.root().join(&store_path);
        if let Err(e) = run(db, &image_id, &target, exif_orientation) {
            failed += 1;
            eprintln!("orientation backfill: {image_id} failed: {e}");
        }
        done += 1;
    }

    let _ = app.emit(
        "orientation://backfill-done",
        BackfillProgress { done, total, failed },
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buckets_common_aspects() {
        assert_eq!(classify(1920, 1080), LANDSCAPE); // 16:9
        assert_eq!(classify(4000, 3000), LANDSCAPE); // 4:3
        assert_eq!(classify(3000, 4000), PORTRAIT);
        assert_eq!(classify(1080, 1080), SQUARE);
        assert_eq!(classify(1080, 1000), SQUARE); // 1.08, inside the band
        assert_eq!(classify(1250, 1000), LANDSCAPE); // 5:4 is 1.25, outside
        assert_eq!(classify(0, 100), SQUARE);
    }

    #[test]
    fn exif_rotation_swaps_axes() {
        assert_eq!(apply_exif_rotation(4000, 3000, Some(6)), (3000, 4000));
        assert_eq!(apply_exif_rotation(4000, 3000, Some(8)), (3000, 4000));
        assert_eq!(apply_exif_rotation(4000, 3000, Some(1)), (4000, 3000));
        assert_eq!(apply_exif_rotation(4000, 3000, Some(3)), (4000, 3000));
        assert_eq!(apply_exif_rotation(4000, 3000, None), (4000, 3000));
    }
}
