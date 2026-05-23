use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use anyhow::{anyhow, Result};
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{ColorType, GenericImageView};

use crate::store::StoreRoot;

const SIZES: [u32; 3] = [256, 512, 1024];
const JPEG_QUALITY: u8 = 85;

pub fn generate(store: &StoreRoot, stored_path: &Path, hash_hex: &str) -> Result<()> {
    let img = image::open(stored_path)?;
    let (w, h) = img.dimensions();
    let longest = w.max(h);

    for size in SIZES {
        let target = store.thumb_path(hash_hex, size);
        if target.exists() {
            continue;
        }

        let resized = if longest <= size {
            img.clone()
        } else {
            img.resize(size, size, FilterType::Triangle)
        };
        let rgb = resized.to_rgb8();
        let (rw, rh) = rgb.dimensions();

        let file = File::create(&target)?;
        let mut writer = BufWriter::new(file);
        let mut encoder = JpegEncoder::new_with_quality(&mut writer, JPEG_QUALITY);
        encoder
            .encode(rgb.as_raw(), rw, rh, ColorType::Rgb8.into())
            .map_err(|e| anyhow!("jpeg encode failed: {e}"))?;
    }

    Ok(())
}
