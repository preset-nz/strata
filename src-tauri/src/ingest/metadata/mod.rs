pub mod backfill;
pub mod exif;
pub mod iptc;

use std::path::Path;

use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::params;

use crate::db::Db;

pub const STAGE_VERSION: &str = "metadata-extract@1";

#[derive(Debug, Default, Clone)]
pub struct Extracted {
    pub exif: Option<exif::ExifData>,
    pub iptc: Option<iptc::Iptc>,
}

pub fn extract(path: &Path) -> Extracted {
    Extracted {
        exif: exif::read(path),
        iptc: iptc::read(path).ok().flatten(),
    }
}

pub fn date_time_original(extracted: &Extracted) -> Option<DateTime<Utc>> {
    extracted.exif.as_ref().and_then(|e| e.date_time_original)
}

pub fn write(db: &Db, image_id: &str, extracted: &Extracted) -> Result<()> {
    let e = extracted.exif.clone().unwrap_or_default();
    let i = extracted.iptc.clone().unwrap_or_default();
    let extracted_at = Utc::now();

    let conn = db.0.lock().unwrap();
    conn.execute(
        "INSERT OR REPLACE INTO image_metadata (
            image_id,
            camera_make, camera_model, lens_make, lens_model,
            focal_length_mm, focal_length_35mm,
            iso, f_number, exposure_time_sec, exposure_bias,
            exposure_program, metering_mode, flash_fired,
            pixel_width, pixel_height, orientation, color_space,
            gps_latitude, gps_longitude, gps_altitude_m,
            iptc_title, iptc_caption, iptc_byline, iptc_copyright,
            iptc_city, iptc_state, iptc_country, iptc_date_created,
            software, extracted_at, stage_version
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            image_id,
            e.camera_make, e.camera_model, e.lens_make, e.lens_model,
            e.focal_length_mm, e.focal_length_35mm,
            e.iso, e.f_number, e.exposure_time_sec, e.exposure_bias,
            e.exposure_program, e.metering_mode, e.flash_fired,
            e.pixel_width, e.pixel_height, e.orientation, e.color_space,
            e.gps_latitude, e.gps_longitude, e.gps_altitude_m,
            i.title, i.caption, i.byline, i.copyright,
            i.city, i.state, i.country, i.date_created,
            e.software, extracted_at, STAGE_VERSION,
        ],
    )?;

    conn.execute(
        "DELETE FROM image_keyword WHERE image_id = ?",
        params![image_id],
    )?;
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for kw in &i.keywords {
        let trimmed = kw.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !seen.insert(trimmed.to_string()) {
            continue;
        }
        conn.execute(
            "INSERT INTO image_keyword (image_id, keyword) VALUES (?, ?)",
            params![image_id, trimmed],
        )?;
    }
    Ok(())
}
