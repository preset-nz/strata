//! On-demand palette with a chosen k (4, 8 or 16), for the palette tool in
//! the properties pane. Unlike the ingest stage (k = 3, VGA-16 bucket,
//! stored), this is transient: computed when asked and never written.

use std::path::Path;

use anyhow::{bail, Result};
use image::imageops::FilterType;
use serde::Serialize;

use super::colour::{lab_chroma, lab_hue_deg, lab_to_srgb, srgb_to_lab, Lab, Srgb};
use super::kmeans::kmeans_pp;
use super::vga16;

pub const ALLOWED_K: [usize; 3] = [4, 8, 16];
/// The longest side the image is read at; finer detail doesn't move a palette.
const SOURCE_SIDE: u32 = 1024;
/// At most this many pixels go into k-means, sampled evenly, which keeps
/// k = 16 well under a second.
const MAX_SAMPLES: usize = 65_536;
const MAX_ITERS: usize = 30;

#[derive(Debug, Clone, Serialize)]
pub struct ExtractedSwatch {
    pub hex: String,
    pub lab: [f32; 3],
    /// L*, C*, h° (degrees).
    pub lch: [f32; 3],
    /// Fraction of the sampled pixels in this cluster.
    pub weight: f32,
    pub bucket: &'static str,
}

/// `source` is the 1024 px thumbnail when there is one, else the original.
pub fn extract(source: &Path, k: usize) -> Result<Vec<ExtractedSwatch>> {
    if !ALLOWED_K.contains(&k) {
        bail!("k must be one of {ALLOWED_K:?}");
    }
    let img = image::open(source)?;
    let img = if img.width().max(img.height()) > SOURCE_SIDE {
        img.resize(SOURCE_SIDE, SOURCE_SIDE, FilterType::Triangle)
    } else {
        img
    };
    let rgb = img.to_rgb8();
    let total = rgb.pixels().len();
    let stride = total.div_ceil(MAX_SAMPLES).max(1);
    let points: Vec<Lab> = rgb
        .pixels()
        .step_by(stride)
        .map(|p| srgb_to_lab(Srgb { r: p.0[0], g: p.0[1], b: p.0[2] }))
        .collect();
    Ok(swatches(&points, k))
}

fn swatches(points: &[Lab], k: usize) -> Vec<ExtractedSwatch> {
    kmeans_pp(points, k, MAX_ITERS)
        .into_iter()
        .map(|c| {
            let Srgb { r, g, b } = lab_to_srgb(c.centroid);
            ExtractedSwatch {
                hex: format!("#{r:02x}{g:02x}{b:02x}"),
                lab: [c.centroid.l, c.centroid.a, c.centroid.b],
                lch: [c.centroid.l, lab_chroma(c.centroid), lab_hue_deg(c.centroid)],
                weight: c.weight,
                bucket: vga16::nearest_bucket(c.centroid).name,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swatches_are_ordered_by_weight_and_named() {
        // A 4 x 4 image, three quarters red, one quarter blue.
        let mut img = image::RgbImage::new(4, 4);
        for (x, _, p) in img.enumerate_pixels_mut() {
            *p = if x < 3 { image::Rgb([200, 20, 20]) } else { image::Rgb([20, 20, 200]) };
        }
        let path = std::env::temp_dir().join(format!("strata-extract-{}.png", uuid::Uuid::new_v4()));
        img.save(&path).unwrap();
        let swatches = extract(&path, 4).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(swatches.len(), 2, "k caps at the distinct colours present");
        assert!((swatches[0].weight - 0.75).abs() < 1e-6);
        assert_eq!(swatches[0].hex, "#c81414");
        assert!(swatches[0].weight >= swatches[1].weight);
        assert!(!swatches[1].bucket.is_empty());
    }

    #[test]
    fn k_must_be_offered() {
        assert!(extract(Path::new("/nonexistent.png"), 5).is_err());
    }
}
