//! On-demand palette with a chosen k (4, 8 or 16), for the palette tool in
//! the properties pane. Unlike the ingest stage (k = 3, VGA-16 bucket,
//! stored), this is transient: computed when asked and never written.

use std::path::Path;

use anyhow::{bail, Result};
use image::imageops::FilterType;
use serde::Serialize;

use super::colour::{lab_chroma, lab_delta_e, lab_hue_deg, lab_to_srgb, srgb_to_lab, Lab, Srgb};
use super::kmeans::kmeans_pp;
use super::vga16;

pub const ALLOWED_K: [usize; 3] = [4, 8, 16];
/// Marker counts the palette inspector offers.
pub const ALLOWED_MARKERS: [usize; 3] = [3, 5, 7];
/// Regions are found on a grid this many cells on the longest side: coarse
/// enough that speckle doesn't split a region, fine enough to place a dot.
const REGION_SIDE: u32 = 192;
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

/// A palette colour placed on the image: where its largest region is.
#[derive(Debug, Clone, Serialize)]
pub struct Marker {
    #[serde(flatten)]
    pub swatch: ExtractedSwatch,
    /// Position as a fraction of width and height, from the top left.
    pub x: f32,
    pub y: f32,
}

/// `source` is the 1024 px thumbnail when there is one, else the original.
pub fn extract(source: &Path, k: usize) -> Result<Vec<ExtractedSwatch>> {
    if !ALLOWED_K.contains(&k) {
        bail!("k must be one of {ALLOWED_K:?}");
    }
    let points = sample(&image::open(source)?);
    Ok(swatches(&points, k))
}

/// The palette inspector: `n` colours, each placed on its largest
/// connected region (4-neighbour) of pixels nearest to it. The marker sits
/// on the region pixel closest to the region's mean, so it lands inside
/// even a ring or a crescent.
pub fn markers(source: &Path, n: usize) -> Result<Vec<Marker>> {
    if !ALLOWED_MARKERS.contains(&n) {
        bail!("markers must be one of {ALLOWED_MARKERS:?}");
    }
    let img = image::open(source)?;
    let found = swatches(&sample(&img), n);
    let grid = img
        .resize(REGION_SIDE, REGION_SIDE, FilterType::Triangle)
        .to_rgb8();
    let (w, h) = grid.dimensions();
    let centroids: Vec<Lab> = found
        .iter()
        .map(|s| Lab {
            l: s.lab[0],
            a: s.lab[1],
            b: s.lab[2],
        })
        .collect();
    let labels: Vec<usize> = grid
        .pixels()
        .map(|p| {
            let lab = srgb_to_lab(Srgb {
                r: p.0[0],
                g: p.0[1],
                b: p.0[2],
            });
            nearest(&centroids, lab)
        })
        .collect();
    Ok(found
        .into_iter()
        .enumerate()
        .filter_map(|(cluster, swatch)| {
            let (x, y) = largest_region_anchor(&labels, w as usize, h as usize, cluster)?;
            Some(Marker {
                swatch,
                x: (x as f32 + 0.5) / w as f32,
                y: (y as f32 + 0.5) / h as f32,
            })
        })
        .collect())
}

fn nearest(centroids: &[Lab], lab: Lab) -> usize {
    centroids
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| lab_delta_e(lab, **a).total_cmp(&lab_delta_e(lab, **b)))
        .map_or(0, |(i, _)| i)
}

/// The largest 4-connected run of `cluster` in a `w` x `h` label grid, and
/// the cell in it nearest the run's mean.
fn largest_region_anchor(
    labels: &[usize],
    w: usize,
    h: usize,
    cluster: usize,
) -> Option<(usize, usize)> {
    let mut seen = vec![false; labels.len()];
    let mut best: Vec<usize> = Vec::new();
    let mut stack = Vec::new();
    for start in 0..labels.len() {
        if seen[start] || labels[start] != cluster {
            continue;
        }
        let mut region = Vec::new();
        seen[start] = true;
        stack.push(start);
        while let Some(i) = stack.pop() {
            region.push(i);
            let (x, y) = (i % w, i / w);
            let neighbours = [
                (x > 0).then(|| i - 1),
                (x + 1 < w).then(|| i + 1),
                (y > 0).then(|| i - w),
                (y + 1 < h).then(|| i + w),
            ];
            for j in neighbours.into_iter().flatten() {
                if !seen[j] && labels[j] == cluster {
                    seen[j] = true;
                    stack.push(j);
                }
            }
        }
        if region.len() > best.len() {
            best = region;
        }
    }
    if best.is_empty() {
        return None;
    }
    let n = best.len() as f32;
    let mx = best.iter().map(|i| (i % w) as f32).sum::<f32>() / n;
    let my = best.iter().map(|i| (i / w) as f32).sum::<f32>() / n;
    best.into_iter().map(|i| (i % w, i / w)).min_by(|a, b| {
        let d = |(x, y): (usize, usize)| (x as f32 - mx).powi(2) + (y as f32 - my).powi(2);
        d(*a).total_cmp(&d(*b))
    })
}

/// Lab points from the image at no more than `SOURCE_SIDE`, evenly sampled
/// down to `MAX_SAMPLES`.
fn sample(img: &image::DynamicImage) -> Vec<Lab> {
    let rgb = if img.width().max(img.height()) > SOURCE_SIDE {
        img.resize(SOURCE_SIDE, SOURCE_SIDE, FilterType::Triangle)
            .to_rgb8()
    } else {
        img.to_rgb8()
    };
    let total = rgb.pixels().len();
    let stride = total.div_ceil(MAX_SAMPLES).max(1);
    rgb.pixels()
        .step_by(stride)
        .map(|p| {
            srgb_to_lab(Srgb {
                r: p.0[0],
                g: p.0[1],
                b: p.0[2],
            })
        })
        .collect()
}

fn swatches(points: &[Lab], k: usize) -> Vec<ExtractedSwatch> {
    kmeans_pp(points, k, MAX_ITERS)
        .into_iter()
        .map(|c| {
            let Srgb { r, g, b } = lab_to_srgb(c.centroid);
            ExtractedSwatch {
                hex: format!("#{r:02x}{g:02x}{b:02x}"),
                lab: [c.centroid.l, c.centroid.a, c.centroid.b],
                lch: [
                    c.centroid.l,
                    lab_chroma(c.centroid),
                    lab_hue_deg(c.centroid),
                ],
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
            *p = if x < 3 {
                image::Rgb([200, 20, 20])
            } else {
                image::Rgb([20, 20, 200])
            };
        }
        let path =
            std::env::temp_dir().join(format!("strata-extract-{}.png", uuid::Uuid::new_v4()));
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
    fn markers_sit_inside_each_colours_largest_region() {
        // Left half red; right half blue with a small red island at the top.
        let mut img = image::RgbImage::new(40, 20);
        for (x, y, p) in img.enumerate_pixels_mut() {
            let red = x < 20 || (x >= 34 && y < 3);
            *p = if red {
                image::Rgb([200, 20, 20])
            } else {
                image::Rgb([20, 20, 200])
            };
        }
        let path =
            std::env::temp_dir().join(format!("strata-markers-{}.png", uuid::Uuid::new_v4()));
        img.save(&path).unwrap();
        let found = markers(&path, 3).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(found.len(), 2, "two colours present");
        let red = found.iter().find(|m| m.swatch.hex == "#c81414").unwrap();
        let blue = found.iter().find(|m| m.swatch.hex == "#1414c8").unwrap();
        assert!(
            red.x < 0.5,
            "red marker on the big left region, not the island: {}",
            red.x
        );
        assert!(blue.x > 0.5);
        assert!(markers(&std::path::PathBuf::from("/x.png"), 4).is_err());
    }

    #[test]
    fn region_anchor_lands_inside_a_ring() {
        // A 5 x 5 ring of label 1 around a label-0 centre.
        let w = 5;
        let labels: Vec<usize> = (0..25)
            .map(|i| {
                let (x, y) = (i % w, i / w);
                usize::from(x == 0 || y == 0 || x == 4 || y == 4)
            })
            .collect();
        let (x, y) = largest_region_anchor(&labels, 5, 5, 1).unwrap();
        assert_eq!(labels[y * w + x], 1);
    }

    #[test]
    fn k_must_be_offered() {
        assert!(extract(Path::new("/nonexistent.png"), 5).is_err());
    }
}
