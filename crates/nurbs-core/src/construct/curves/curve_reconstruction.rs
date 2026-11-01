//! Planar reconstruction from piecewise-linear sampled signed curvature.
//! Each curvature panel has a quadratic tangent angle, integrated exactly in
//! real arithmetic. Displacement uses two half-panel Simpson steps. The sum
//! of local fine/coarse differences divided by 15 is a diagnostic estimate,
//! not a bound or a protection against oscillatory aliasing. The resulting
//! sites are fitted numerically to a NURBS with explicit convergence status.
use crate::{Result, check, curve::Curve, numeric, resource};

/// Shared maximum number of curvature integration panels per call.
pub const MAX_INTEGRATION_PANELS: usize = 4096;

#[derive(Clone, Debug)]
pub struct ReconstructionReport {
    /// Planar NURBS fitted to the reconstructed sites (z = 0).
    pub curve: Curve,
    /// Maximum defined curvature residual at parameters linearly mapped from
    /// input s to the fit domain. This is not true arc-length inversion.
    /// Inspect undefined_curvature_sites before using this diagnostic.
    pub max_residual_curvature: f64,
    /// Total arc length covered by the samples (`s_last - s_first`).
    pub length: f64,
    /// Sum of local two-level Simpson displacement error estimates divided
    /// by 15. Diagnostic only, not a continuous or certified error bound.
    pub quadrature_error_estimate: f64,
    /// Integrated sites before NURBS fitting, in input station order.
    pub sites: Vec<[f64; 3]>,
    /// Integrated tangent angles at the input stations.
    pub tangent_angles: Vec<f64>,
    /// Maximum fitting deviation at data sites, not a continuous error bound.
    pub fitted_site_max_deviation: f64,
    /// Whether the numerical fitter reached the requested data-site tolerance.
    pub fit_converged: bool,
    /// Sites where the fitted curve has no defined nonzero-speed curvature.
    pub undefined_curvature_sites: usize,
    /// Number of integration panels consumed from MAX_INTEGRATION_PANELS.
    pub integration_panels: usize,
}

/// Reconstruct a planar curve from signed curvature κ sampled at arc-length
/// sites `s`. `kappa` holds `(s, κ)` pairs with strictly increasing `s`
/// spanning a positive total length; at least 3 samples, at most 4096.
///
/// `start` is the position at `s_first` (only x/y are used; z is set to 0)
/// and `start_tangent_angle` is θ(s_first) in radians. The sampled sites are
/// fitted to a cubic NURBS (quadratic for three sites). `tolerance` is a
/// requested data-site fitting tolerance; inspect fit_converged and the
/// reported deviation. No continuous reconstruction error bound is returned.
/// Curvature between input stations is defined by linear interpolation.
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

    let mut theta = Vec::with_capacity(n);
    theta.push(start_tangent_angle);
    let mut x = vec![start[0]];
    let mut y = vec![start[1]];
    let mut quadrature_error_estimate = 0.;
    let mut integration_panels = 0;
    for i in 1..n {
        let (s0, k0) = kappa[i - 1];
        let (s1, k1) = kappa[i];
        let h = s1 - s0;
        let t0 = theta[i - 1];
        // Integral of k0 + (k1-k0)*t/h at t = fraction*h.
        // In particular the midpoint angle is not (theta0+theta1)/2.
        let angle = |f: f64| t0 + h * (k0 * f + (k1 - k0) * (0.5 * f * f));
        // Bound angle travel on each integration cell by 0.5 rad.
        // All source panels share one fixed work budget. Large oscillatory
        // inputs are refused instead of aliased into an apparently good fit.
        let turn = h * k0.abs().max(k1.abs());
        let parts_f = (turn / 0.5).ceil().max(1.);
        if !parts_f.is_finite()
            || parts_f > (MAX_INTEGRATION_PANELS - integration_panels) as f64 {
            return Err(resource("Curvature integration exceeds the shared panel budget"));
        }
        let parts = parts_f as usize;
        integration_panels += parts;
        let mut total_dx = 0.;
        let mut total_dy = 0.;
        for part in 0..parts {
            let f0 = part as f64 / parts as f64;
            let fraction = 1. / parts as f64;
            let angles = [angle(f0), angle(f0 + fraction * 0.25),
                angle(f0 + fraction * 0.5), angle(f0 + fraction * 0.75),
                angle((part + 1) as f64 / parts as f64)];
            numeric(angles.iter().all(|t| t.is_finite()),
                "Reconstruction tangent angle exhausted finite precision")?;
            numeric(turn == 0. || angles.iter().any(|&t| t != t0),
                "Curvature rotation is not resolvable at the supplied angle")?;
            let cs = angles.map(f64::cos);
            let sn = angles.map(f64::sin);
            let step = h / parts as f64;
            let fine = |v: [f64; 5]| step * ((v[0] + v[4]) / 12.
                + (v[1] + v[3]) / 3. + v[2] / 6.);
            let coarse = |v: [f64; 5]| step * ((v[0] + v[4]) / 6. + 2. * v[2] / 3.);
            let dx = fine(cs);
            let dy = fine(sn);
            total_dx += dx;
            total_dy += dy;
            quadrature_error_estimate += (dx - coarse(cs)).hypot(dy - coarse(sn)) / 15.;
        }
        x.push(x[i - 1] + total_dx);
        y.push(y[i - 1] + total_dy);
        theta.push(angle(1.));
    }
    numeric(
        x.iter().chain(y.iter()).all(|v| v.is_finite() && v.abs() <= 1e9),
        "Reconstruction position exhausted finite precision",
    )?;
    numeric(quadrature_error_estimate.is_finite(),
        "Reconstruction quadrature estimate exhausted finite precision")?;

    let sites: Vec<[f64; 3]> = (0..n).map(|i| [x[i], y[i], 0.]).collect();
    let (mut curve, mut fitted_site_max_deviation, mut fit_converged) = if n == 3 {
        let fit = crate::foundation::interpolation::fit_curve_adaptive(
            &sites, 2, tolerance,
            crate::foundation::interpolation::Parameterization::ChordLength, None, true,
        )?;
        (fit.curve, fit.max_deviation, fit.converged)
    } else {
        crate::curve_offset::fit_offset_sites(&sites, tolerance)?
    };
    // A position-only adaptive solve may stop unconverged with clustered
    // knots and poor curvature. Try the existing bounded global candidates
    // before returning that result. Never replace it with a worse site fit.
    if !fit_converged {
        let points: Vec<Vec<f64>> = sites.iter().map(|p| p.to_vec()).collect();
        for count in [8usize, 13, 19, 26] {
            if count > n { continue; }
            if let Ok(candidate) = crate::foundation::fitting::fit_curve_points_report(
                points.clone(), count, None,
            ) {
                let deviation = candidate.certificate.data_site_error_upper;
                if deviation.is_finite() && deviation < fitted_site_max_deviation {
                    curve = candidate.curve;
                    fitted_site_max_deviation = deviation;
                    fit_converged = deviation <= tolerance;
                    if fit_converged { break; }
                }
            }
        }
    }
    numeric(
        fitted_site_max_deviation.is_finite(),
        "Reconstruction fit deviation is not finite",
    )?;
    let fit_curve = curve;

    // Curvature residual: compare the fitted curve's signed curvature at the
    // arc-length-mapped parameters against the input κ samples.
    let [a, b] = fit_curve.domain();
    let mut max_residual_curvature: f64 = 0.;
    let mut undefined_curvature_sites = 0;
    for &(s, k) in kappa.iter() {
        let u = a + (b - a) * (s - kappa[0].0) / length;
        let e = fit_curve.evaluate(u)?;
        if let (Some(d1), Some(d2)) = (e.d1, e.d2) {
            let speed = d1[0].hypot(d1[1]);
            if speed > 0. {
                let fitted = (d1[0] * d2[1] - d1[1] * d2[0]) / speed.powi(3);
                if fitted.is_finite() {
                    max_residual_curvature = max_residual_curvature.max((fitted - k).abs());
                } else {
                    undefined_curvature_sites += 1;
                }
            } else {
                undefined_curvature_sites += 1;
            }
        } else {
            undefined_curvature_sites += 1;
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
        sites,
        tangent_angles: theta,
        fitted_site_max_deviation,
        fit_converged,
        undefined_curvature_sites,
        integration_panels,
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
