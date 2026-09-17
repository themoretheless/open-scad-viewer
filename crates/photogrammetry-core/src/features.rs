use crate::{Image, Result};

/// Harris pyramid and gradient histograms, not a DoG/SIFT scale-space detector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FeatureOptions {
    pub subpixel: bool,
    pub interpolated_histograms: bool,
    pub root_hellinger: bool,
    /// Opt-in relaxed-ratio second pass in `matches_with_options` for mutual
    /// nearest neighbors rejected by the strict ratio test (symmetric ratio
    /// below 0.9 in both directions). Off by default; the default path is
    /// bit-identical to plain `matches`.
    pub second_chance: bool,
    /// Opt-in GPU descriptor distance grid (requires the `gpu` crate feature;
    /// falls back to the CPU scan when no adapter is available). GPU float
    /// contraction means results are not bit-identical to the CPU reference.
    pub acceleration: crate::Acceleration,
}
impl Default for FeatureOptions {
    fn default() -> Self {
        Self::ROOT
    }
}
impl FeatureOptions {
    pub const BASELINE: Self = Self {
        subpixel: false,
        interpolated_histograms: false,
        root_hellinger: false,
        second_chance: false,
        acceleration: crate::Acceleration::Cpu,
    };
    pub const REFINED: Self = Self {
        subpixel: true,
        interpolated_histograms: true,
        root_hellinger: false,
        second_chance: false,
        acceleration: crate::Acceleration::Cpu,
    };
    pub const ROOT: Self = Self {
        root_hellinger: true,
        ..Self::REFINED
    };
}

/// Quadratic Harris peak localization; reject non-maxima and unstable offsets.
fn localize(scores: &[f32], w: usize, x: usize, y: usize) -> (f64, f64) {
    let i = y * w + x;
    let c = scores[i] as f64;
    let gx = (scores[i + 1] as f64 - scores[i - 1] as f64) * 0.5;
    let gy = (scores[i + w] as f64 - scores[i - w] as f64) * 0.5;
    let xx = scores[i + 1] as f64 + scores[i - 1] as f64 - 2. * c;
    let yy = scores[i + w] as f64 + scores[i - w] as f64 - 2. * c;
    let xy = (scores[i + w + 1] as f64 - scores[i + w - 1] as f64 - scores[i - w + 1] as f64
        + scores[i - w - 1] as f64)
        * 0.25;
    let det = xx * yy - xy * xy;
    if xx >= 0. || yy >= 0. || det <= 1e-10 * (xx * yy).abs() {
        return (x as f64, y as f64);
    }
    let dx = (xy * gy - yy * gx) / det;
    let dy = (xy * gx - xx * gy) / det;
    if dx.is_finite() && dy.is_finite() && dx.abs() <= 0.75 && dy.abs() <= 0.75 {
        (x as f64 + dx, y as f64 + dy)
    } else {
        (x as f64, y as f64)
    }
}

#[derive(Clone)]
pub struct Feature {
    pub x: f64,
    pub y: f64,
    pub descriptor: [f32; 128],
}
fn sample(p: &[f32], w: usize, h: usize, x: f64, y: f64) -> f32 {
    let x = x.clamp(0., (w - 1) as f64);
    let y = y.clamp(0., (h - 1) as f64);
    let (ix, iy) = (x as usize, y as usize);
    let (jx, jy) = ((ix + 1).min(w - 1), (iy + 1).min(h - 1));
    let (a, b) = ((x - ix as f64) as f32, (y - iy as f64) as f32);
    (1. - b) * ((1. - a) * p[iy * w + ix] + a * p[iy * w + jx])
        + b * ((1. - a) * p[jy * w + ix] + a * p[jy * w + jx])
}
fn gradient(p: &[f32], w: usize, h: usize, x: f64, y: f64) -> (f32, f32) {
    (
        sample(p, w, h, x + 1., y) - sample(p, w, h, x - 1., y),
        sample(p, w, h, x, y + 1.) - sample(p, w, h, x, y - 1.),
    )
}
/// Interior variant of `gradient` without the edge clamps; same arithmetic,
/// valid when every sample has ix + 1 < w and iy + 1 < h.
fn gradient_interior(p: &[f32], w: usize, x: f64, y: f64) -> (f32, f32) {
    let at = |x: f64, y: f64| {
        let (ix, iy) = (x as usize, y as usize);
        let (a, b) = ((x - ix as f64) as f32, (y - iy as f64) as f32);
        (1. - b) * ((1. - a) * p[iy * w + ix] + a * p[iy * w + ix + 1])
            + b * ((1. - a) * p[(iy + 1) * w + ix] + a * p[(iy + 1) * w + ix + 1])
    };
    (at(x + 1., y) - at(x - 1., y), at(x, y + 1.) - at(x, y - 1.))
}
/// Interior gradient at an integer offset from (x0, y0); every sample shares
/// the bilinear fractions (a, b), so they are computed once per corner.
/// Arithmetic is identical to `gradient` on the same coordinates.
#[allow(clippy::too_many_arguments)]
fn gradient_offset(
    p: &[f32],
    w: usize,
    x0: usize,
    y0: usize,
    dx: i32,
    dy: i32,
    a: f32,
    b: f32,
) -> (f32, f32) {
    let at = |ix: usize, iy: usize| {
        (1. - b) * ((1. - a) * p[iy * w + ix] + a * p[iy * w + ix + 1])
            + b * ((1. - a) * p[(iy + 1) * w + ix] + a * p[(iy + 1) * w + ix + 1])
    };
    let (ix, iy) = ((x0 as i32 + dx) as usize, (y0 as i32 + dy) as usize);
    (
        at(ix + 1, iy) - at(ix - 1, iy),
        at(ix, iy + 1) - at(ix, iy - 1),
    )
}
/// f32 bits biased so ascending u32 order matches `total_cmp` (-0.0 and NaN included).
fn total_order_key(v: f32) -> u32 {
    let b = v.to_bits();
    if b & 0x8000_0000 != 0 {
        !b
    } else {
        b | 0x8000_0000
    }
}
fn blur(p: &[f32], w: usize, h: usize) -> Vec<f32> {
    let mut tmp = vec![0.; p.len()];
    let mut out = vec![0.; p.len()];
    for y in 0..h {
        for x in 0..w {
            tmp[y * w + x] = (p[y * w + x.saturating_sub(1)]
                + 2. * p[y * w + x]
                + p[y * w + (x + 1).min(w - 1)])
                * 0.25;
        }
    }
    for y in 0..h {
        for x in 0..w {
            out[y * w + x] = (tmp[y.saturating_sub(1) * w + x]
                + 2. * tmp[y * w + x]
                + tmp[(y + 1).min(h - 1) * w + x])
                * 0.25;
        }
    }
    out
}
pub fn extract(image: &Image, limit: usize) -> Result<Vec<Feature>> {
    extract_with_options(image, limit, &FeatureOptions::default())
}
pub fn extract_with_options(
    image: &Image,
    limit: usize,
    options: &FeatureOptions,
) -> Result<Vec<Feature>> {
    image.validate()?;
    if limit == 0 {
        return Ok(Vec::new());
    }
    let mut p = image.gray();
    let (mut w, mut h) = (image.width, image.height);
    let mut all = Vec::new();
    let mut pyramid = Vec::with_capacity(3);
    for level in 0..3 {
        if w < 48 || h < 48 {
            break;
        }
        p = blur(&p, w, h);
        let mut xx = vec![0.; w * h];
        let mut yy = vec![0.; w * h];
        let mut xy = vec![0.; w * h];
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let i = y * w + x;
                let gx = p[i + 1] - p[i - 1];
                let gy = p[i + w] - p[i - w];
                xx[i] = gx * gx;
                yy[i] = gy * gy;
                xy[i] = gx * gy;
            }
        }
        let xx = blur(&xx, w, h);
        let yy = blur(&yy, w, h);
        let xy = blur(&xy, w, h);
        let scores: Vec<_> = (0..w * h)
            .map(|i| xx[i] * yy[i] - xy[i] * xy[i] - 0.04 * (xx[i] + yy[i]).powi(2))
            .collect();
        let mut corners = Vec::new();
        for y in 14..h - 14 {
            for x in 14..w - 14 {
                let i = y * w + x;
                let s = scores[i];
                if s < 1e-7 {
                    continue;
                }
                if (-1isize..=1).all(|dy| {
                    (-1isize..=1).all(|dx| {
                        scores[((y as isize + dy) as usize) * w + (x as isize + dx) as usize] <= s
                    })
                }) {
                    // Score key orders like descending total_cmp; the inverted
                    // position in the low bits keeps ties in scan order.
                    corners.push((total_order_key(s) as u64) << 32 | !(y * w + x) as u32 as u64);
                }
            }
        }
        corners.sort_unstable_by(|a, b| b.cmp(a));
        let mut chosen = Vec::new();
        let mut suppressed = vec![false; w * h];
        for key in corners {
            let k = (key >> 32) as u32;
            let score = f32::from_bits(if k & 0x8000_0000 != 0 {
                k & 0x7fff_ffff
            } else {
                !k
            });
            let pos = !(key as u32) as usize;
            let (x, y) = (pos % w, pos / w);
            if suppressed[y * w + x] {
                continue;
            }
            for py in y.saturating_sub(6)..=(y + 6).min(h - 1) {
                for px in x.saturating_sub(6)..=(x + 6).min(w - 1) {
                    suppressed[py * w + px] = true;
                }
            }
            chosen.push((score, x, y));
            if chosen.len() >= limit {
                break;
            }
        }
        for (score, x, y) in chosen {
            let (x, y) = if options.subpixel {
                localize(&scores, w, x, y)
            } else {
                (x as f64, y as f64)
            };
            all.push((score, level, x, y));
        }
        let nw = w / 2;
        let nh = h / 2;
        let next = if level < 2 && nw >= 48 && nh >= 48 {
            let smoothed = blur(&p, w, h);
            (0..nw * nh)
                .map(|i| smoothed[(i / nw * 2) * w + (i % nw * 2)])
                .collect()
        } else {
            Vec::new()
        };
        // Selection depends on the corner score and location, never its
        // descriptor. Retain gray levels, then describe only surviving corners.
        pyramid.push((w, h, p));
        p = next;
        w = nw;
        h = nh;
    }
    all.sort_by(|a, b| b.0.total_cmp(&a.0));
    let weights = DescriptorWeights::new();
    let mut result: Vec<Feature> = Vec::new();
    let mut suppressed = vec![false; image.width * image.height];
    for (_, level, level_x, level_y) in all {
        let scale = (1 << level) as f64;
        let (fx, fy) = (level_x * scale, level_y * scale);
        let (x, y) = (fx as usize, fy as usize);
        if suppressed[y * image.width + x] {
            continue;
        }
        for py in y.saturating_sub(4)..=(y + 4).min(image.height - 1) {
            for px in x.saturating_sub(4)..=(x + 4).min(image.width - 1) {
                suppressed[py * image.width + px] = true;
            }
        }
        let (w, h, pixels) = &pyramid[level];
        result.push(Feature {
            x: fx,
            y: fy,
            descriptor: describe(pixels, *w, *h, level_x, level_y, options, &weights),
        });
        if result.len() >= limit {
            break;
        }
    }
    Ok(result)
}
/// Fixed sample weights are independent of the image, corner and orientation.
/// Compute them once per extraction instead of repeating 425 exponentials per corner.
struct DescriptorWeights {
    orientation: [f32; 169],
    descriptor: [f32; 256],
}
impl DescriptorWeights {
    fn new() -> Self {
        Self {
            orientation: std::array::from_fn(|i| {
                let (dx, dy) = ((i % 13) as i32 - 6, (i / 13) as i32 - 6);
                (-((dx * dx + dy * dy) as f32) / 32.).exp()
            }),
            descriptor: std::array::from_fn(|i| {
                let (u, v) = ((i % 16) as f64 - 7.5, (i / 16) as f64 - 7.5);
                (-((u * u + v * v) as f32) / 128.).exp()
            }),
        }
    }
}
fn describe(
    p: &[f32],
    w: usize,
    h: usize,
    x: f64,
    y: f64,
    options: &FeatureOptions,
    weights: &DescriptorWeights,
) -> [f32; 128] {
    // Sampling reaches at most 7.5 * sqrt(2) + 1 px from the corner; when that
    // window is fully interior the bilinear clamps are no-ops and can be skipped.
    const REACH: f64 = 7.5 * std::f64::consts::SQRT_2 + 1.;
    let interior =
        x >= REACH && y >= REACH && x + REACH < (w - 1) as f64 && y + REACH < (h - 1) as f64;
    // Integer offsets share the corner's fractions, hoisted as bilinear weights.
    let (x0, y0) = (x as usize, y as usize);
    let (fa, fb) = ((x - x0 as f64) as f32, (y - y0 as f64) as f32);
    let mut hist = [0f32; 36];
    for dy in -6..=6 {
        for dx in -6..=6 {
            let (gx, gy) = if interior {
                gradient_offset(p, w, x0, y0, dx, dy, fa, fb)
            } else {
                gradient(p, w, h, x + dx as f64, y + dy as f64)
            };
            let angle = gy.atan2(gx);
            let b = ((angle + std::f32::consts::PI) * 36. / std::f32::consts::TAU) as usize % 36;
            let mag =
                (gx * gx + gy * gy).sqrt() * weights.orientation[((dy + 6) * 13 + dx + 6) as usize];
            if options.interpolated_histograms {
                let position = (angle + std::f32::consts::PI) * 36. / std::f32::consts::TAU;
                let fraction = position - position.floor();
                hist[b] += mag * (1. - fraction);
                hist[(b + 1) % 36] += mag * fraction;
            } else {
                hist[b] += mag;
            }
        }
    }
    if options.interpolated_histograms {
        for _ in 0..2 {
            let old = hist;
            for i in 0..36 {
                hist[i] = (old[(i + 35) % 36] + 2. * old[i] + old[(i + 1) % 36]) * 0.25;
            }
        }
    }
    let peak = (0..36)
        .max_by(|&i, &j| hist[i].total_cmp(&hist[j]))
        .unwrap();
    let offset = if options.interpolated_histograms {
        let l = hist[(peak + 35) % 36] as f64;
        let c = hist[peak] as f64;
        let r = hist[(peak + 1) % 36] as f64;
        let curvature = l - 2. * c + r;
        if curvature < -1e-12 {
            (0.5 * (l - r) / curvature).clamp(-0.5, 0.5)
        } else {
            0.
        }
    } else {
        0.5
    };
    let angle = (peak as f64 + offset) * std::f64::consts::TAU / 36. - std::f64::consts::PI;
    let (c, s) = (angle.cos(), angle.sin());
    let mut d = [0f32; 128];
    for iy in 0..16 {
        for ix in 0..16 {
            let (u, v) = (ix as f64 - 7.5, iy as f64 - 7.5);
            let (sx, sy) = (x + c * u - s * v, y + s * u + c * v);
            let (gx, gy) = if interior {
                gradient_interior(p, w, sx, sy)
            } else {
                gradient(p, w, h, sx, sy)
            };
            let mag = (gx * gx + gy * gy).sqrt() * weights.descriptor[iy * 16 + ix];
            let a = (gy.atan2(gx) as f64 - angle).rem_euclid(std::f64::consts::TAU) * 8.
                / std::f64::consts::TAU;
            let b = a.floor() as usize % 8;
            let f = (a - a.floor()) as f32;
            if options.interpolated_histograms {
                let sx = (ix as f32 + 0.5) / 4. - 0.5;
                let sy = (iy as f32 + 0.5) / 4. - 0.5;
                let (cx, cy) = (sx.floor() as isize, sy.floor() as isize);
                for dy in 0..=1 {
                    for dx in 0..=1 {
                        let (xx, yy) = (cx + dx, cy + dy);
                        if !(0..4).contains(&xx) || !(0..4).contains(&yy) {
                            continue;
                        }
                        let weight = (1. - (sx - xx as f32).abs()) * (1. - (sy - yy as f32).abs());
                        let cell = (yy as usize * 4 + xx as usize) * 8;
                        d[cell + b] += mag * weight * (1. - f);
                        d[cell + (b + 1) % 8] += mag * weight * f;
                    }
                }
            } else {
                let cell = ((iy / 4) * 4 + ix / 4) * 8;
                d[cell + b] += mag * (1. - f);
                d[cell + (b + 1) % 8] += mag * f;
            }
        }
    }
    let norm = d.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-12);
    for v in &mut d {
        *v = (*v / norm).min(0.2);
    }
    let norm = d.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-12);
    for v in &mut d {
        *v /= norm;
    }
    if options.root_hellinger {
        let l1 = d.iter().sum::<f32>().max(1e-12);
        for v in &mut d {
            *v = (*v / l1).sqrt();
        }
    }
    d
}

#[derive(Clone, Copy, Debug)]
pub struct Match {
    pub a: usize,
    pub b: usize,
    /// Squared descriptor distance and nearest/second-nearest distance ratio.
    /// The ratio belongs to the canonical matching direction; reversal preserves it.
    pub distance_squared: f32,
    pub ratio: f32,
}
pub fn matches(a: &[Feature], b: &[Feature]) -> Vec<Match> {
    matches_with_options(a, b, &FeatureOptions::BASELINE)
}

/// Conservative placement recommendation for full descriptor matching.
///
/// Descriptor matching compares 128 floats per candidate pair and scans both
/// directions, so it has much higher arithmetic intensity than one-to-one
/// vector math. The portable wgpu path covers Metal/Vulkan/DX12; there is no
/// dedicated CUDA PTX matcher, so `Cuda` falls through to the same shader.
pub const fn recommended_for_descriptor_matching(
    a_count: usize,
    b_count: usize,
) -> crate::Acceleration {
    const GPU_WORK_THRESHOLD: usize = 10_000;
    let Some(work) = a_count.checked_mul(b_count) else {
        return crate::Acceleration::Gpu;
    };
    if work >= GPU_WORK_THRESHOLD {
        crate::Acceleration::Gpu
    } else {
        crate::Acceleration::Cpu
    }
}

pub const fn resolve_for_descriptor_matching(
    acceleration: crate::Acceleration,
    a_count: usize,
    b_count: usize,
) -> crate::Acceleration {
    match acceleration {
        crate::Acceleration::Auto => recommended_for_descriptor_matching(a_count, b_count),
        explicit => explicit,
    }
}

pub fn matches_with_options(a: &[Feature], b: &[Feature], options: &FeatureOptions) -> Vec<Match> {
    let mut best_a = vec![(usize::MAX, f32::INFINITY, f32::INFINITY); a.len()];
    let mut best_b = vec![(usize::MAX, f32::INFINITY); b.len()];
    // Flatten descriptors to a dense 512-byte stride instead of the Feature layout.
    let da: Vec<[f32; 128]> = a.iter().map(|x| x.descriptor).collect();
    let db: Vec<[f32; 128]> = b.iter().map(|x| x.descriptor).collect();
    let acceleration = resolve_for_descriptor_matching(options.acceleration, da.len(), db.len());
    #[cfg(feature = "gpu")]
    let gpu_done = acceleration.is_gpu()
        && match crate::gpu::matching::match_pair(&da, &db) {
            Some((rows, cols)) => {
                for (i, row) in rows.iter().enumerate() {
                    best_a[i] = (row.j, row.d1, row.d2);
                }
                for (j, col) in cols.iter().enumerate() {
                    best_b[j] = (col.i, col.d1);
                }
                true
            }
            None => false,
        };
    #[cfg(not(feature = "gpu"))]
    let gpu_done = false;
    if !gpu_done {
        for (i, x) in da.iter().enumerate() {
            for (j, y) in db.iter().enumerate() {
                let mut d = 0.;
                let cutoff = best_a[i].2.max(best_b[j].1);
                // Partial sums never decrease, so checking the cutoff once per
                // 16-component chunk rejects the same pairs as checking every k.
                for (cx, cy) in x.as_chunks::<16>().0.iter().zip(y.chunks_exact(16)) {
                    for k in 0..16 {
                        let v = cx[k] - cy[k];
                        d += v * v;
                    }
                    if d > cutoff {
                        break;
                    }
                }
                if d < best_a[i].1 {
                    best_a[i].2 = best_a[i].1;
                    best_a[i].1 = d;
                    best_a[i].0 = j;
                } else if d < best_a[i].2 {
                    best_a[i].2 = d;
                }
                if d < best_b[j].1 {
                    best_b[j] = (i, d);
                }
            }
        }
    }
    let strict = |(i, &(j, d, second)): (usize, &(usize, f32, f32))| {
        if j != usize::MAX && best_b[j].0 == i && d < 0.64 * second && d < 0.9 {
            Some(Match {
                a: i,
                b: j,
                distance_squared: d,
                ratio: (d / second).sqrt(),
            })
        } else {
            None
        }
    };
    let mut accepted: Vec<Match> = best_a.iter().enumerate().filter_map(strict).collect();
    if options.second_chance {
        // Mutual nearest neighbors just above the strict ratio (0.8) get one
        // more chance at a relaxed symmetric ratio (0.9 in both directions)
        // under the same absolute gate; the reverse check rejects the
        // ambiguous self-similar patches that a one-sided relaxation admits.
        // A mutual pair is unique per endpoint, so these never duplicate or
        // conflict with first-pass matches. Sorted by distance with index
        // tie-breaks and appended, keeping first-pass order untouched. The
        // reverse second-best distance is recomputed exactly for the few band
        // candidates, leaving the hot loop (and its early-break cutoff) on the
        // default path untouched.
        const RELAXED_RATIO_SQUARED: f32 = 0.81;
        let mut extra: Vec<Match> = best_a
            .iter()
            .enumerate()
            .filter_map(|(i, &(j, d, second))| {
                if !(j != usize::MAX
                    && best_b[j].0 == i
                    && d >= 0.64 * second
                    && d < RELAXED_RATIO_SQUARED * second
                    && d < 0.9)
                {
                    return None;
                }
                // Reverse second-best: j's nearest `a` feature is i (mutual),
                // so the second nearest is the minimum over the rest.
                let reverse_second = da
                    .iter()
                    .enumerate()
                    .filter(|&(k, _)| k != i)
                    .map(|(_, x)| {
                        let mut rd = 0.;
                        for k in 0..128 {
                            let v = x[k] - db[j][k];
                            rd += v * v;
                        }
                        rd
                    })
                    .fold(f32::INFINITY, f32::min);
                if d < RELAXED_RATIO_SQUARED * reverse_second {
                    Some(Match {
                        a: i,
                        b: j,
                        distance_squared: d,
                        ratio: (d / second).sqrt(),
                    })
                } else {
                    None
                }
            })
            .collect();
        extra.sort_by(|x, y| {
            x.distance_squared
                .total_cmp(&y.distance_squared)
                .then(x.a.cmp(&y.a))
                .then(x.b.cmp(&y.b))
        });
        accepted.extend(extra);
    }
    accepted
}

#[cfg(test)]
#[path = "features_tests.rs"]
mod tests;
