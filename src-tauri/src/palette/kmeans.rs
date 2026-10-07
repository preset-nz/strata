use super::colour::{lab_delta_e, Lab};

pub struct Cluster {
    pub centroid: Lab,
    pub weight: f32,
}

/// Seeds from evenly spaced points. The ingest stage's palettes were made
/// this way (`palette-v1`), so it stays as is.
pub fn kmeans(points: &[Lab], k: usize, max_iters: usize) -> Vec<Cluster> {
    if points.is_empty() || k == 0 {
        return vec![];
    }
    let k = k.min(points.len());
    let n = points.len();
    refine(
        points,
        (0..k).map(|j| points[j * n / k]).collect(),
        max_iters,
    )
}

/// Seeds by k-means++ (each next seed drawn with probability proportional
/// to its squared distance from the seeds so far), from a fixed seed so the
/// same image always gives the same palette. Even spacing in pixel order can
/// leave a whole region unseeded; this doesn't.
pub fn kmeans_pp(points: &[Lab], k: usize, max_iters: usize) -> Vec<Cluster> {
    if points.is_empty() || k == 0 {
        return vec![];
    }
    let k = k.min(points.len());
    let mut rng = 0x9e37_79b9_7f4a_7c15u64;
    let mut next_unit = move || {
        // xorshift64*: deterministic, good enough to pick seeds.
        rng ^= rng >> 12;
        rng ^= rng << 25;
        rng ^= rng >> 27;
        (rng.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 11) as f64 / (1u64 << 53) as f64
    };
    let mut centroids = vec![points[points.len() / 2]];
    let mut nearest: Vec<f64> = points
        .iter()
        .map(|p| (lab_delta_e(*p, centroids[0]) as f64).powi(2))
        .collect();
    while centroids.len() < k {
        let total: f64 = nearest.iter().sum();
        if total <= 0.0 {
            break; // every point sits on a seed: fewer colours than k
        }
        let mut target = next_unit() * total;
        let mut pick = points.len() - 1;
        for (i, d) in nearest.iter().enumerate() {
            target -= d;
            if target <= 0.0 {
                pick = i;
                break;
            }
        }
        let seed = points[pick];
        centroids.push(seed);
        for (d, p) in nearest.iter_mut().zip(points) {
            *d = d.min((lab_delta_e(*p, seed) as f64).powi(2));
        }
    }
    refine(points, centroids, max_iters)
}

fn refine(points: &[Lab], mut centroids: Vec<Lab>, max_iters: usize) -> Vec<Cluster> {
    let k = centroids.len();
    let n = points.len();

    let mut assignments = vec![0usize; n];
    for _ in 0..max_iters {
        let mut changed = false;
        for (i, p) in points.iter().enumerate() {
            let mut best = 0usize;
            let mut best_d = lab_delta_e(*p, centroids[0]);
            for (j, c) in centroids.iter().enumerate().skip(1) {
                let d = lab_delta_e(*p, *c);
                if d < best_d {
                    best_d = d;
                    best = j;
                }
            }
            if assignments[i] != best {
                changed = true;
                assignments[i] = best;
            }
        }
        if !changed {
            break;
        }

        let mut sums = vec![(0.0f32, 0.0f32, 0.0f32, 0u32); k];
        for (i, p) in points.iter().enumerate() {
            let entry = &mut sums[assignments[i]];
            entry.0 += p.l;
            entry.1 += p.a;
            entry.2 += p.b;
            entry.3 += 1;
        }
        for (j, sum) in sums.iter().enumerate() {
            if sum.3 == 0 {
                continue;
            }
            let count = sum.3 as f32;
            centroids[j] = Lab {
                l: sum.0 / count,
                a: sum.1 / count,
                b: sum.2 / count,
            };
        }
    }

    let mut counts = vec![0u32; k];
    for &a in &assignments {
        counts[a] += 1;
    }
    let total = n as f32;
    let mut clusters: Vec<Cluster> = (0..k)
        .map(|j| Cluster {
            centroid: centroids[j],
            weight: counts[j] as f32 / total,
        })
        .filter(|c| c.weight > 0.0)
        .collect();

    clusters.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap());
    clusters
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::colour::{srgb_to_lab, Srgb};

    #[test]
    fn three_distinct_colours_yield_three_clusters() {
        let mut pixels = Vec::new();
        for _ in 0..100 {
            pixels.push(srgb_to_lab(Srgb { r: 255, g: 0, b: 0 }));
        }
        for _ in 0..100 {
            pixels.push(srgb_to_lab(Srgb { r: 0, g: 255, b: 0 }));
        }
        for _ in 0..100 {
            pixels.push(srgb_to_lab(Srgb { r: 0, g: 0, b: 255 }));
        }
        let clusters = kmeans(&pixels, 3, 20);
        assert_eq!(clusters.len(), 3);
        for c in &clusters {
            assert!((c.weight - 1.0 / 3.0).abs() < 0.05);
        }
    }

    #[test]
    fn plus_plus_seeds_a_region_even_spacing_misses() {
        // Row-major 4 x 4, the last column blue: evenly spaced seeds
        // (indices 0, 4, 8, 12) all land on red.
        let red = srgb_to_lab(Srgb {
            r: 200,
            g: 20,
            b: 20,
        });
        let blue = srgb_to_lab(Srgb {
            r: 20,
            g: 20,
            b: 200,
        });
        let pixels: Vec<Lab> = (0..16)
            .map(|i| if i % 4 == 3 { blue } else { red })
            .collect();
        assert_eq!(kmeans(&pixels, 4, 20).len(), 1);
        let clusters = kmeans_pp(&pixels, 4, 20);
        assert_eq!(clusters.len(), 2);
        assert!((clusters[0].weight - 0.75).abs() < 1e-6);
    }
}
