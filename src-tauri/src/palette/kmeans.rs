use super::colour::{lab_delta_e, Lab};

pub struct Cluster {
    pub centroid: Lab,
    pub weight: f32,
}

pub fn kmeans(points: &[Lab], k: usize, max_iters: usize) -> Vec<Cluster> {
    if points.is_empty() || k == 0 {
        return vec![];
    }
    let k = k.min(points.len());
    let n = points.len();

    let mut centroids: Vec<Lab> = (0..k).map(|j| points[j * n / k]).collect();

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
}
