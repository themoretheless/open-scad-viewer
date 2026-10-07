//! Discrete Fréchet distance between two curves.
//! Arc-length-uniform samples come from `curve_measure` enclosures; a dynamic
//! program over the free-space diagram then finds the coupling that minimises
//! the largest sampled pair distance. The discrete value is an upper
//! approximation of the continuous Fréchet distance: with `n` arc-length
//! samples, the true distance lies in `[d - δ, d]` where `δ` is bounded by the
//! largest sample gap over both curves.
use crate::{
    Result, check,
    curve::Curve,
    curve_measure::length,
    numeric, numeric_err,
};

#[derive(Clone, Debug)]
pub struct FrechetReport {
    /// Discrete Fréchet distance: the minimum over monotone couplings of the
    /// maximum sampled pair distance. Upper approximation of the continuous
    /// Fréchet distance (see module docs).
    pub distance: f64,
    /// Number of pairs visited by the realising coupling (path length in the
    /// free-space diagram; between `samples` and `2·samples - 1`).
    pub coupling_length: f64,
    /// Arc-length-uniform samples taken per curve (≤ 512 budget).
    pub samples: usize,
}

/// Arc-length-uniform samples as `(parameter, point)` pairs.
///
/// The certified total arc length comes from `curve_measure::length`; the
/// inversion uses a midpoint-rule prefix table on a uniform parameter grid
/// (second order, documented) rescaled to the certified total, so sample
/// positions carry the quadrature's honesty without a certified inversion
/// per sample.
pub(crate) fn arc_uniform_samples(
    curve: &Curve,
    samples: usize,
    tolerance: f64,
) -> Result<(f64, Vec<(f64, Vec<f64>)>)> {
    let total = length(curve, tolerance, 100_000)?;
    numeric(
        total.within_tolerance,
        "Arc-length sampling could not resolve the total arc length",
    )?;
    numeric(
        total.value > 0.,
        "Arc-length sampling needs curves of positive length",
    )?;
    let [a, b] = curve.domain();
    let cells = (samples * 16).clamp(1024, 65536);
    let h = (b - a) / cells as f64;
    let mut speed = Vec::with_capacity(cells);
    let mut prefix = Vec::with_capacity(cells + 1);
    prefix.push(0.);
    for i in 0..cells {
        // Cell midpoints lie strictly inside a knot span: d1 is available.
        let e = curve.evaluate(a + (i as f64 + 0.5) * h)?;
        let d1 = e
            .d1
            .ok_or_else(|| numeric_err("Arc-length sampling lost the tangent"))?;
        let s = d1.iter().map(|v| v * v).sum::<f64>().sqrt();
        numeric(
            s > 0.,
            "Arc-length sampling hit a stationary point on the curve",
        )?;
        speed.push(s);
        prefix.push(prefix[i] + s * h);
    }
    let raw = prefix[cells];
    numeric(
        raw.is_finite() && raw > 0.,
        "Arc-length quadrature exhausted finite precision",
    )?;
    let rescale = total.value / raw;
    let mut out = Vec::with_capacity(samples);
    for k in 0..samples {
        let target = if samples == 1 {
            0.
        } else {
            total.value * k as f64 / (samples - 1) as f64
        } / rescale;
        let j = prefix
            .partition_point(|&p| p <= target)
            .saturating_sub(1)
            .min(cells - 1);
        let u = (a + (j as f64 + (target - prefix[j]) / (speed[j] * h)) * h).clamp(a, b);
        out.push((u, curve.evaluate(u)?.point));
    }
    Ok((total.value, out))
}

/// Absolute sampling tolerance scaled to the curve extents, never degenerate.
pub(crate) fn sampling_tolerance(curve: &Curve) -> f64 {
    let scale = curve
        .control_points
        .iter()
        .flatten()
        .map(|x| x.abs())
        .fold(1., f64::max);
    (scale * 1e-6).max(1e-9)
}

/// Discrete Fréchet distance between two curves (item 352).
///
/// Both curves are sampled at `samples` arc-length-uniform sites (certified by
/// `curve_measure`); the classic dynamic program over the free-space diagram
/// minimises the maximum pair distance over all monotone couplings and also
/// reports the realising coupling's path length.
///
/// Complexity is O(samples²) time and memory; `samples` is capped at 512 per
/// curve as the resource budget.
pub fn frechet_discrete(a: &Curve, b: &Curve, samples: usize) -> Result<FrechetReport> {
    a.validate()?;
    b.validate()?;
    check(
        (2..=512).contains(&samples),
        "Fréchet sample count must be in 2..=512 per curve",
    )?;
    let pa = arc_uniform_samples(a, samples, sampling_tolerance(a))?.1;
    let pb = arc_uniform_samples(b, samples, sampling_tolerance(b))?.1;
    let n = samples;
    let mut distance_at = vec![0.; n * n];
    for (i, (_, x)) in pa.iter().enumerate() {
        for (j, (_, y)) in pb.iter().enumerate() {
            distance_at[i * n + j] = x
                .iter()
                .zip(y)
                .map(|(u, v)| (u - v) * (u - v))
                .sum::<f64>()
                .sqrt();
        }
    }
    numeric(
        distance_at.iter().all(|d| d.is_finite()),
        "Fréchet pair distances exhausted finite precision",
    )?;
    // ca[i][j] = min over monotone couplings ending at (i, j) of the max pair
    // distance along the coupling.
    let mut ca = vec![0.; n * n];
    for i in 0..n {
        for (j, &d) in distance_at[i * n..(i + 1) * n].iter().enumerate() {
            let best = match (i > 0, j > 0) {
                (false, false) => d,
                (true, false) => ca[(i - 1) * n],
                (false, true) => ca[j - 1],
                (true, true) => ca[(i - 1) * n + j]
                    .min(ca[(i - 1) * n + j - 1])
                    .min(ca[i * n + j - 1]),
            };
            ca[i * n + j] = best.max(d);
        }
    }
    let distance = ca[n * n - 1];
    // Backtrack the realising coupling to measure its path length in pairs.
    let (mut i, mut j) = (n - 1, n - 1);
    let mut steps = 0usize;
    while i > 0 || j > 0 {
        steps += 1;
        let up = if i > 0 { ca[(i - 1) * n + j] } else { f64::INFINITY };
        let diag = if i > 0 && j > 0 {
            ca[(i - 1) * n + j - 1]
        } else {
            f64::INFINITY
        };
        let left = if j > 0 { ca[i * n + j - 1] } else { f64::INFINITY };
        if diag <= up && diag <= left {
            i -= 1;
            j -= 1;
        } else if up <= left {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    numeric(
        distance.is_finite(),
        "Fréchet coupling distance exhausted finite precision",
    )?;
    Ok(FrechetReport {
        distance,
        coupling_length: (steps + 1) as f64,
        samples,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(a: [f64; 3], b: [f64; 3]) -> Curve {
        Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap()
    }

    fn circle(radius: f64) -> Curve {
        let w = std::f64::consts::FRAC_1_SQRT_2;
        let r = radius;
        let controls = [
            [r, 0., 0.],
            [r, r, 0.],
            [0., r, 0.],
            [-r, r, 0.],
            [-r, 0., 0.],
            [-r, -r, 0.],
            [0., -r, 0.],
            [r, -r, 0.],
            [r, 0., 0.],
        ];
        Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.],
            control_points: controls.iter().map(|p| p.to_vec()).collect(),
            weights: vec![1., w, 1., w, 1., w, 1., w, 1.],
            periodic: false,
        }
    }

    #[test]
    fn identical_curves_have_zero_distance() {
        let curve = circle(2.);
        let report = frechet_discrete(&curve, &curve, 64).unwrap();
        assert!(report.distance <= 1e-6, "distance={}", report.distance);
        assert_eq!(report.samples, 64);
        assert!(report.coupling_length >= 64.);
        assert!(report.coupling_length <= 127.);
    }

    #[test]
    fn parallel_lines_separate_by_the_shift() {
        let base = line([0., 0., 0.], [3., 0., 0.]);
        let shifted = line([0., 0.5, 0.], [3., 0.5, 0.]);
        let report = frechet_discrete(&base, &shifted, 33).unwrap();
        assert!((report.distance - 0.5).abs() <= 1e-9);
    }

    #[test]
    fn concentric_circles_separate_by_radius_difference() {
        let inner = circle(2.);
        let outer = circle(3.);
        let report = frechet_discrete(&inner, &outer, 128).unwrap();
        // The continuous distance is exactly |ΔR| = 1; the discrete value is
        // an upper approximation, honest to the sampling gap.
        assert!(report.distance >= 1. - 1e-6, "distance={}", report.distance);
        assert!(report.distance <= 1. + 0.02, "distance={}", report.distance);
    }

    #[test]
    fn sample_budget_is_enforced() {
        let a = line([0., 0., 0.], [1., 0., 0.]);
        let b = line([0., 0., 0.], [1., 1., 0.]);
        assert!(frechet_discrete(&a, &b, 1).is_err());
        assert!(frechet_discrete(&a, &b, 513).is_err());
    }
}
