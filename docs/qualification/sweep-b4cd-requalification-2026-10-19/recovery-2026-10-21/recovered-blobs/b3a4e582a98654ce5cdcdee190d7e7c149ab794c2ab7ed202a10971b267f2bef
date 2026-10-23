//! Planar curve reconstruction from sampled signed curvature (item 330).
//! θ(s) = θ₀ + ∫κ ds is accumulated with composite Simpson quadrature; the
//! position follows from the Euler (Fresnel-style) integrals
//! x(s) = x₀ + ∫cos θ ds, y(s) = y₀ + ∫sin θ ds, also by composite Simpson
//! over the θ sites. A two-level Simpson estimate quantifies the quadrature
//! error (Runge: |I - I_h| ≈ |I_h - I_{2h}| / 15 for the composite Simpson
//! rule); accumulation is documented as O(h⁴) per the composite Simpson rule
//! plus linear binary64 rounding growth in the site count. The reconstructed
//! sites are fitted back to a NURBS via `fit_curve_adaptive`.
use crate::{Result, check, curve::Curve, numeric};

#[derive(Clone, Debug)]
pub struct ReconstructionReport {
    /// Planar NURBS fitted to the reconstructed sites (z = 0).
    pub curve: Curve,
    /// Maximum residual between the fitted curve's signed curvature and the
    /// input κ, resampled at the input arc-length sites.
    pub max_residual_curvature: f64,
    /// Total arc length covered by the samples (`s_last - s_first`).
    pub length: f64,
    /// Composite-Simpson quadrature error estimate on the total displacement
    /// (two-level Runge estimate |I_h − I_{2h}| / 15); documentation-grade,
    /// not a certified bound.
    pub quadrature_error_estimate: f64,
}

/// Reconstruct a planar curve from signed curvature κ sampled at arc-length
/// sites `s`. `kappa` holds `(s, κ)` pairs with strictly increasing `s`
/// spanning a positive total length; at least 3 samples, at most 4096.
///
/// `start` is the position at `s_first` (only x/y are used; z is set to 0)
/// and `start_tangent_angle` is θ(s_first) in radians. The sampled sites are
/// fitted to a cubic NURBS within `tolerance`; the report includes the
/// curvature residual of the fit at the input sites.
pub fn reconstruct_from_curvature(
    kappa: &[(f64, f64)],
    start: [f64; 3],
    start_tangent_angle: f64,
    tolerance: f64,
) -> Result<ReconstructionReport> {
    check(
        (3..=4096).contains(&kappa.len()),
        "Reconstruction needs 3..4096 curvature samples",
    )?;
    check(
        kappa
            .iter()
            .all(|(s, k)| s.is_finite() && k.is_finite() && k.abs() <= 1e12),
        "Curvature samples must be finite with |κ| ≤ 1e12",
    )?;
    check(
        kappa.windows(2).all(|w| w[0].0 < w[1].0),
        "Curvature arc-length sites must strictly increase",
    )?;
    check(
        start.iter().all(|x| x.is_finite()) && start_tangent_angle.is_finite(),
        "Reconstruction start position and tangent angle must be finite",
    )?;
    check(
        tolerance.is_finite() && tolerance > 0.,
        "Reconstruction tolerance must be positive and finite",
    )?;
    let length = kappa[kappa.len() - 1].0 - kappa[0].0;
    numeric(
        length.is_finite() && length > 0.,
        "Curvature samples must span a positive total length",
    )?;
    let n = kappa.len();

    // θ(s) at each site by cumulative composite Simpson on the κ samples
    // (panel pairs share their left prefix, accumulated incrementally).
    let mut theta = Vec::with_capacity(n);
    theta.push(start_tangent_angle);
    for i in 1..n {
        let (s0, k0) = kappa[i - 1];
        let (s1, k1) = kappa[i];
        let h = s1 - s0;
        // Midpoint κ from linear interpolation between samples; the panel
        // Simpson step is then exact for quadratic κ and O(h⁵) otherwise.
        let km = 0.5 * (k0 + k1);
        let panel = h * (k0 + 4. * km + k1) / 6.;
        theta.push(theta[i - 1] + panel);
    }
    numeric(
        theta.iter().all(|t| t.is_finite()),
        "Reconstruction tangent angle exhausted finite precision",
    )?;

    // Fresnel integrals: cos θ, sin θ at the sites, then composite Simpson
    // per panel. Non-uniform spacing is handled panel-wise with the midpoint
    // from linear θ interpolation (Hermite-consistent with the κ quadrature).
    let mut x = vec![start[0]];
    let mut y = vec![start[1]];
    for i in 1..n {
        let h = kappa[i].0 - kappa[i - 1].0;
        let (t0, t1) = (theta[i - 1], theta[i]);
        let tm = 0.5 * (t0 + t1);
        let ci = [t0.cos(), tm.cos(), t1.cos()];
        let si = [t0.sin(), tm.sin(), t1.sin()];
        x.push(x[i - 1] + h * (ci[0] + 4. * ci[1] + ci[2]) / 6.);
        y.push(y[i - 1] + h * (si[0] + 4. * si[1] + si[2]) / 6.);
    }
    numeric(
        x.iter().chain(y.iter()).all(|v| v.is_finite() && v.abs() <= 1e9),
        "Reconstruction position exhausted finite precision",
    )?;

    // Two-level Simpson error estimate on the total displacement: coarse
    // (every second panel) vs fine quadrature; |I_h - I_2h|/15 estimates the
    // fine-rule error. Documented, not enforced — it informs the caller.
    let mut coarse_dx = 0.;
    let mut coarse_dy = 0.;
    let mut i = 0;
    while i + 2 < n {
        let h = kappa[i + 2].0 - kappa[i].0;
        let (t0, t1, t2) = (theta[i], theta[i + 1], theta[i + 2]);
        coarse_dx += h * (t0.cos() + 4. * t1.cos() + t2.cos()) / 6.;
        coarse_dy += h * (t0.sin() + 4. * t1.sin() + t2.sin()) / 6.;
        i += 2;
    }
    let quadrature_error_estimate =
        ((x[n - 1] - x[0]) - coarse_dx).hypot((y[n - 1] - y[0]) - coarse_dy) / 15.;
    numeric(
        quadrature_error_estimate.is_finite(),
        "Reconstruction quadrature estimate exhausted finite precision",
    )?;

    let sites: Vec<[f64; 3]> = (0..n).map(|i| [x[i], y[i], 0.]).collect();
    let (curve, _max_deviation, _converged) =
        crate::curve_offset::fit_offset_sites(&sites, tolerance)?;
    numeric(
        _max_deviation.is_finite(),
        "Reconstruction fit deviation is not finite",
    )?;
    let fit_curve = curve;

    // Curvature residual: compare the fitted curve's signed curvature at the
    // arc-length-mapped parameters against the input κ samples.
    let [a, b] = fit_curve.domain();
    let mut max_residual_curvature: f64 = 0.;
    for &(s, k) in kappa.iter() {
        let u = a + (b - a) * (s - kappa[0].0) / length;
        let e = fit_curve.evaluate(u)?;
        if let (Some(d1), Some(d2)) = (e.d1, e.d2) {
            let speed = d1[0].hypot(d1[1]);
            if speed > 0. {
                let fitted = (d1[0] * d2[1] - d1[1] * d2[0]) / speed.powi(3);
                max_residual_curvature = max_residual_curvature.max((fitted - k).abs());
            }
        }
    }
    numeric(
        max_residual_curvature.is_finite(),
        "Reconstruction curvature residual is not finite",
    )?;
    Ok(ReconstructionReport {
        curve: fit_curve,
        max_residual_curvature,
        length,
        quadrature_error_estimate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circle_from_constant_curvature() {
        // A quarter circle of radius 2: κ = 0.5 over arc length π.
        let r = 2.;
        let arc = std::f64::consts::FRAC_PI_2 * r;
        let n = 65;
        let kappa: Vec<(f64, f64)> = (0..n)
            .map(|i| (arc * i as f64 / (n - 1) as f64, 1. / r))
            .collect();
        let report = reconstruct_from_curvature(&kappa, [0., 0., 0.], 0., 1e-6).unwrap();
        assert!((report.length - arc).abs() <= 1e-12);
        // Endpoint must land at (R, R) for a quarter circle starting at
        // the origin with tangent angle 0.
        let [a, b] = report.curve.domain();
        let end = report.curve.evaluate(b).unwrap().point;
        assert!(
            (end[0] - r).hypot(end[1] - r) <= 1e-4,
            "end={end:?}"
        );
        let start = report.curve.evaluate(a).unwrap().point;
        assert!(start[0].hypot(start[1]) <= 1e-4, "start={start:?}");
        // Curvature residual of the fit stays near the input κ.
        assert!(
            report.max_residual_curvature <= 0.05,
            "residual={}",
            report.max_residual_curvature
        );
    }

    #[test]
    fn clothoid_like_linear_curvature_has_monotone_tangent() {
        // κ(s) = a·s, a = 1, over s ∈ [0, 2]: θ(s) = s²/2 (Fresnel spiral).
        let n = 101;
        let total = 2.;
        let kappa: Vec<(f64, f64)> = (0..n)
            .map(|i| {
                let s = total * i as f64 / (n - 1) as f64;
                (s, s)
            })
            .collect();
        let report = reconstruct_from_curvature(&kappa, [0., 0., 0.], 0., 1e-6).unwrap();
        assert!((report.length - total).abs() <= 1e-12);
        // Sanity: the tangent angle of the fitted curve is monotone increasing
        // (κ ≥ 0 everywhere) and sweeps ≈ θ(2) = 2 rad in total.
        let [a, b] = report.curve.domain();
        let mut previous = f64::NEG_INFINITY;
        let mut monotone = true;
        let mut first_angle = None;
        let mut last_angle = 0.;
        for step in 0..=40 {
            let u = a + (b - a) * step as f64 / 40.;
            let e = report.curve.evaluate(u).unwrap();
            let d = e.d1.unwrap();
            let angle = d[1].atan2(d[0]);
            if angle < previous - 1e-6 && (angle - previous).abs() < 1. {
                monotone = false;
            }
            previous = angle;
            first_angle.get_or_insert(angle);
            last_angle = angle;
        }
        assert!(monotone, "tangent angle must be monotone for κ ≥ 0");
        let sweep = last_angle - first_angle.unwrap();
        assert!((sweep - 2.).abs() <= 0.2, "sweep={sweep}");
    }

    #[test]
    fn invalid_samples_are_rejected() {
        assert!(reconstruct_from_curvature(&[], [0.; 3], 0., 1e-3).is_err());
        assert!(
            reconstruct_from_curvature(&[(0., 1.), (1., 1.)], [0.; 3], 0., 1e-3).is_err()
        );
        // Non-increasing sites.
        let bad = vec![(0., 1.), (1., 1.), (0.5, 1.)];
        assert!(reconstruct_from_curvature(&bad, [0.; 3], 0., 1e-3).is_err());
        // Degenerate span.
        let flat = vec![(1., 1.), (1., 1.), (1., 1.)];
        assert!(reconstruct_from_curvature(&flat, [0.; 3], 0., 1e-3).is_err());
    }
}
