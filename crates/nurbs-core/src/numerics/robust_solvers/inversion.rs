use super::{ClosureProblem, SolverOptions, SolverReport, trust_region_solve};
use crate::foundation::guards::{require_finite_f64, require_finite_point, require_finite_slice};
use crate::{Result, check, curve::Curve, numeric_err, surface::Surface};

/// Invert a point on a curve: `min_u ‖C(u) − P‖₂` over the curve domain,
/// seeded at `u0`. Uses the crate's analytic first derivative; the parameter
/// is softly clamped to the domain by rejecting out-of-domain trials.
pub fn trust_region_point_inversion_curve(
    curve: &Curve,
    point: &[f64],
    u0: f64,
    options: SolverOptions,
) -> Result<(f64, SolverReport)> {
    curve.validate()?;
    let domain = curve.domain();
    check(
        point.len() == curve.control_points[0].len(),
        "Inversion point must match curve dimension",
    )?;
    require_finite_slice(point, "point")?;
    require_finite_f64(u0, "u0")?;
    let start = u0.clamp(domain[0], domain[1]);
    let clamped = |u: f64| u.clamp(domain[0], domain[1]);
    let residual = |x: &[f64]| -> Result<Vec<f64>> {
        let e = curve.evaluate(clamped(x[0]))?;
        Ok(e.point.iter().zip(point).map(|(c, p)| c - p).collect())
    };
    let jacobian = |x: &[f64]| -> Result<Vec<Vec<f64>>> {
        let e = curve.evaluate(clamped(x[0]))?;
        let d1 = e
            .d1
            .ok_or_else(|| numeric_err("Curve first derivative unavailable for inversion"))?;
        Ok(d1.iter().map(|d| vec![*d]).collect())
    };
    let (x, report) = trust_region_solve(
        &ClosureProblem { residual, jacobian },
        &[start],
        options,
    )?;
    Ok((clamped(x[0]), report))
}

/// Invert a point on a surface: `min_(u,v) ‖S(u,v) − P‖₂` over the surface
/// domain rectangle, seeded at `(u0, v0)`. Uses the crate's analytic first
/// partial derivatives.
pub fn trust_region_point_inversion_surface(
    surface: &Surface,
    point: [f64; 3],
    u0: f64,
    v0: f64,
    options: SolverOptions,
) -> Result<(f64, f64, SolverReport)> {
    surface.validate()?;
    require_finite_point(&point, "point")?;
    require_finite_f64(u0, "u0")?;
    require_finite_f64(v0, "v0")?;
    let du = crate::curve::validate_basis(
        surface.degree_u,
        &surface.knots_u,
        surface.control_points.len(),
    )?;
    let dv = crate::curve::validate_basis(
        surface.degree_v,
        &surface.knots_v,
        surface.control_points[0].len(),
    )?;
    let clamped = |u: f64, v: f64| (u.clamp(du[0], du[1]), v.clamp(dv[0], dv[1]));
    let (su, sv) = clamped(u0, v0);
    let residual = |x: &[f64]| -> Result<Vec<f64>> {
        let (u, v) = clamped(x[0], x[1]);
        let e = surface.evaluate(u, v)?;
        Ok(e.point.iter().zip(point).map(|(c, p)| c - p).collect())
    };
    let jacobian = |x: &[f64]| -> Result<Vec<Vec<f64>>> {
        let (u, v) = clamped(x[0], x[1]);
        let e = surface.evaluate(u, v)?;
        let (tu, tv) = e
            .first_derivatives()
            .ok_or_else(|| numeric_err("Surface first derivatives unavailable for inversion"))?;
        Ok((0..3).map(|axis| vec![tu[axis], tv[axis]]).collect())
    };
    let (x, report) = trust_region_solve(
        &ClosureProblem { residual, jacobian },
        &[su, sv],
        options,
    )?;
    let (u, v) = clamped(x[0], x[1]);
    Ok((u, v, report))
}

#[cfg(test)]
mod tests {
    use super::super::SolverOptions;
    use super::{trust_region_point_inversion_curve, trust_region_point_inversion_surface};
    use crate::curve::Curve;
    use crate::foundation::project_curve_report;
    use crate::surface::Surface;

    /// Highly curved clamped cubic: tight hairpin around (0, 1).
    fn hairpin_curve() -> Curve {
        Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 0.5, 1., 1., 1., 1.],
            control_points: vec![
                vec![-2., 0., 0.],
                vec![-1., 3., 0.],
                vec![1., -3., 0.],
                vec![1., 3., 0.],
                vec![2., 0., 0.],
            ],
            weights: vec![1.; 5],
            periodic: false,
        }
    }

    #[test]
    fn inversion_rejects_non_finite_inputs_with_param_and_index() {
        let curve = hairpin_curve();
        let err =
            trust_region_point_inversion_curve(&curve, &[0., f64::NAN, 0.], 0.5, SolverOptions::default())
                .unwrap_err();
        assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
        assert!(err.contains("point[1]"), "{err}");
        let err = trust_region_point_inversion_curve(&curve, &[0., 0., 0.], f64::NAN, SolverOptions::default())
            .unwrap_err();
        assert!(err.contains("u0"), "{err}");
    }

    #[test]
    fn curve_point_inversion_matches_certified_projection() {
        let curve = hairpin_curve();
        // Sample a true curve point deep inside the high-curvature region.
        let u_true = 0.47;
        let p = curve.evaluate(u_true).unwrap().point;
        // Deliberately poor seed, far from the answer.
        let (u, report) = trust_region_point_inversion_curve(
            &curve,
            &p,
            0.05,
            SolverOptions::default(),
        )
        .unwrap();
        assert!(report.converged, "{report:?}");
        let reproduced = curve.evaluate(u).unwrap().point;
        let error: f64 = reproduced
            .iter()
            .zip(&p)
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f64>()
            .sqrt();
        assert!(error < 1e-9, "reprojection error {error}");
        // Cross-check against the crate's certified projection machinery.
        let certified = project_curve_report(&curve, &p, None).unwrap();
        let winner = certified.winner_index.map(|i| &certified.candidates[i]);
        if let Some(winner) = winner {
            let certified_u = (winner.parameter_interval[0] + winner.parameter_interval[1]) * 0.5;
            assert!(
                (u - certified_u).abs() < 1e-4,
                "trust-region u={u}, certified u={certified_u}"
            );
        }
    }

    #[test]
    fn curve_point_inversion_handles_off_curve_point() {
        let curve = hairpin_curve();
        let p = vec![0.25, 0.9, 0.3];
        let (u, report) =
            trust_region_point_inversion_curve(&curve, &p, 0.5, SolverOptions::default())
                .unwrap();
        assert!(report.converged, "{report:?}");
        // The residual at the reported parameter must be a local minimum:
        // first-order stationarity (C − P)·C′ = 0 in the domain interior.
        let e = curve.evaluate(u).unwrap();
        let stationarity: f64 = e
            .point
            .iter()
            .zip(&p)
            .zip(e.d1.unwrap().iter())
            .map(|((c, p), d)| (c - p) * d)
            .sum();
        assert!(stationarity.abs() < 1e-6, "stationarity {stationarity}");
    }

    #[test]
    fn surface_point_inversion_recovers_parameters() {
        // Curved (hyperbolic-paraboloid-like) biquadratic NURBS surface.
        let surface = Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![
                vec![vec![-1., -1., 0.], vec![-1., 0., 0.5], vec![-1., 1., 0.]],
                vec![vec![0., -1., 0.5], vec![0., 0., -1.], vec![0., 1., 0.5]],
                vec![vec![1., -1., 0.], vec![1., 0., 0.5], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        let (u_true, v_true) = (0.63, 0.37);
        let p = surface.evaluate(u_true, v_true).unwrap().point;
        let (u, v, report) = trust_region_point_inversion_surface(
            &surface,
            p,
            0.1,
            0.9,
            SolverOptions::default(),
        )
        .unwrap();
        assert!(report.converged, "{report:?}");
        let reproduced = surface.evaluate(u, v).unwrap().point;
        let error: f64 = reproduced
            .iter()
            .zip(p)
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f64>()
            .sqrt();
        assert!(error < 1e-9, "reprojection error {error}");
        assert!((u - u_true).abs() < 1e-6 && (v - v_true).abs() < 1e-6);
    }
}
