use std::path::Path;

use anyhow::{anyhow, Result};
use chrono::Utc;
use rusqlite::params;
use image::imageops::FilterType;
use serde::Serialize;

use crate::db::Db;
use crate::palette::colour::{lab_chroma, lab_hue_deg, lab_to_srgb, srgb_to_lab, Lab, Srgb};
use crate::palette::kmeans::kmeans;
use crate::palette::vga16;
use crate::store::StoreRoot;

pub const STAGE_VERSION: &str = "palette-v1";

const RESAMPLE_SIZE: u32 = 64;
const K: usize = 3;
const MAX_ITERS: usize = 20;
const THUMB_SIZE: u32 = 256;

#[derive(Serialize)]
struct Swatch {
    rgb: [u8; 3],
    weight: f32,
    bucket: &'static str,
    lab: [f32; 3],
}

pub fn run(store: &StoreRoot, target: &Path, hash: &str, image_id: &str, db: &Db) -> Result<()> {
    let thumb_path = store.thumb_path(hash, THUMB_SIZE);
    let source_for_decode = if thumb_path.exists() {
        thumb_path
    } else {
        target.to_path_buf()
    };
    let img = image::open(&source_for_decode)?
        .resize(RESAMPLE_SIZE, RESAMPLE_SIZE, FilterType::Triangle);
    let rgb = img.to_rgb8();

    let pixels: Vec<Lab> = rgb
        .pixels()
        .map(|p| {
            srgb_to_lab(Srgb {
                r: p.0[0],
                g: p.0[1],
                b: p.0[2],
            })
        })
        .collect();

    let clusters = kmeans(&pixels, K, MAX_ITERS);
    if clusters.is_empty() {
        return Err(anyhow!("palette: no clusters extracted"));
    }

    let swatches: Vec<Swatch> = clusters
        .iter()
        .map(|c| {
            let bucket = vga16::nearest_bucket(c.centroid);
            let rgb = lab_to_srgb(c.centroid);
            Swatch {
                rgb: [rgb.r, rgb.g, rgb.b],
                weight: c.weight,
                bucket: bucket.name,
                lab: [c.centroid.l, c.centroid.a, c.centroid.b],
            }
        })
        .collect();

    let dominant = clusters[0].centroid;
    let dominant_bucket = vga16::nearest_bucket(dominant).name;
    let dominant_l = dominant.l;
    let dominant_c = lab_chroma(dominant);
    let dominant_h = lab_hue_deg(dominant);

    let swatches_json = serde_json::to_string(&swatches)?;
    let unique_buckets: std::collections::BTreeSet<&'static str> =
        swatches.iter().map(|s| s.bucket).collect();
    let extracted_at = Utc::now();

    let conn = db.0.lock().unwrap();
    conn.execute(
        "INSERT OR REPLACE INTO image_palette \
         (image_id, swatches, dominant_bucket, dominant_l, dominant_c, dominant_h, extracted_at, stage_version) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            image_id,
            swatches_json,
            dominant_bucket,
            dominant_l,
            dominant_c,
            dominant_h,
            extracted_at,
            STAGE_VERSION,
        ],
    )?;

    conn.execute(
        "DELETE FROM image_palette_bucket WHERE image_id = ?",
        params![image_id],
    )?;
    for bucket in unique_buckets {
        conn.execute(
            "INSERT INTO image_palette_bucket (image_id, bucket) VALUES (?, ?)",
            params![image_id, bucket],
        )?;
    }
    Ok(())
}
