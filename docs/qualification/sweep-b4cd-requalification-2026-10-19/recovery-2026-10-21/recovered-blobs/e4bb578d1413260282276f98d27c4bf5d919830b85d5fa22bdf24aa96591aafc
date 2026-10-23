//! Circular-profile pipe with shared rotation-minimizing sampled sweep evidence.
use crate::{Result, check, curve::Curve, foundation::guards::{require_finite_f64, require_finite_point}, framed_sweep::CheckedSweep};

/// Radius is constant at every authored circular section. Between sections,
/// the surface is a linear rational loft, not an exact constant-radius offset.
/// The refinement report is sampled only, not a continuous tolerance bound.
pub fn checked(
    path: &Curve,
    radius: f64,
    initial_normal: [f64; 3],
    sections: usize,
    max_deviation: f64,
) -> Result<CheckedSweep> {
    path.validate()?;
    check(path.control_points[0].len() == 3, "Pipe path must be 3D")?;
    require_finite_f64(radius, "radius")?;
    check(radius > 0., "Pipe radius must be positive")?;
    require_finite_f64(max_deviation, "max_deviation")?;
    check(max_deviation > 0., "Pipe deviation budget must be positive")?;
    require_finite_point(&initial_normal, "initial_normal")?;
    let (center, tangent) = crate::framed_sweep::sample(path, path.domain()[0])?;
    let profile = crate::primitives::circle(center, tangent, radius)?;
    crate::framed_sweep::checked_sweep(&profile, path, initial_normal, sections, max_deviation)
}

/// Radius law is a positive rational scalar curve encoded as [radius,0,0].
/// Both domains normalize independently. The refinement report is sampled, not a continuous certificate.
pub fn checked_variable(
    path: &Curve,
    radius: &Curve,
    normal: [f64; 3],
    sections: usize,
    budget: f64,
) -> Result<CheckedSweep> {
    path.validate()?;
    check(path.control_points[0].len() == 3, "Pipe path must be 3D")?;
    require_finite_point(&normal, "normal")?;
    let (start, tangent) = crate::framed_sweep::sample(path, path.domain()[0])?;
    let profile = crate::primitives::circle(start, tangent, 1.)?;
    crate::framed_sweep::checked_scaled_profile(&profile, path, radius, normal, sections, budget)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn variable_radius_matches_independent_cone_between_stations() {
        let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
        let law = crate::primitives::line([1., 0., 0.], [3., 0., 0.]).unwrap();
        let result = checked_variable(&path, &law, [1., 0., 0.], 8, 1e-8).unwrap();
        assert!(result.report.accepted);
        let surface = result.surface.unwrap();
        let lo = surface.knots_u[surface.degree_u];
        let hi = surface.knots_u[surface.control_points.len()];
        for i in 0..=100 {
            let v = i as f64 / 100.;
            for j in 0..=40 {
                let p = surface
                    .evaluate(lo + (hi - lo) * j as f64 / 40., v)
                    .unwrap()
                    .point;
                assert!((p[0].hypot(p[1]) - (1. + 2. * v)).abs() < 1e-11);
                assert!((p[2] - 10. * v).abs() < 1e-11);
            }
        }
    }
    #[test]
    fn curved_variable_pipe_refines_and_closed_radius_must_match() {
        let path = crate::primitives::circle_arc([0.; 3], [0., 0., 1.], 5., 0., 90.).unwrap();
        let law = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
        let coarse = checked_variable(&path, &law, [0., 0., 1.], 3, 0.02).unwrap();
        let fine = checked_variable(&path, &law, [0., 0., 1.], 32, 0.02).unwrap();
        assert!(!coarse.report.accepted && coarse.surface.is_none());
        assert!(fine.report.accepted);
        let surface = fine.surface.unwrap();
        let lo = surface.knots_u[surface.degree_u];
        let hi = surface.knots_u[surface.control_points.len()];
        for i in 0..32 {
            let v = i as f64 / 31.;
            let c = path.evaluate(v).unwrap().point;
            for j in 0..=40 {
                let p = surface
                    .evaluate(lo + (hi - lo) * j as f64 / 40., v)
                    .unwrap()
                    .point;
                assert!(
                    ((p[0] - c[0]).hypot(p[1] - c[1]).hypot(p[2] - c[2]) - (1. + v)).abs() < 1e-10
                );
            }
        }
        let closed = crate::primitives::circle([0.; 3], [0., 0., 1.], 5.).unwrap();
        assert!(checked_variable(&closed, &law, [0., 0., 1.], 32, 0.1).is_err());
        let bad = crate::primitives::line([-1., 0., 0.], [2., 0., 0.]).unwrap();
        assert!(checked_variable(&path, &bad, [0., 0., 1.], 32, 0.02).is_err());
    }
    #[test]
    fn straight_pipe_matches_independent_cylinder_equation_between_sections() {
        let path = crate::primitives::line([3., 4., 5.], [3., 4., 12.]).unwrap();
        let fit = checked(&path, 2., [1., 0., 0.], 8, 1e-5).unwrap();
        assert!(fit.report.accepted);
        let surface = fit.surface.unwrap();
        let lo = surface.knots_u[surface.degree_u];
        let hi = surface.knots_u[surface.control_points.len()];
        for i in 0..=100 {
            let u = lo + (hi - lo) * i as f64 / 100.;
            for v in [0., 0.13, 0.5, 0.87, 1.] {
                let p = surface.evaluate(u, v).unwrap().point;
                assert!(((p[0] - 3.).hypot(p[1] - 4.) - 2.).abs() < 1e-11);
                assert!((p[2] - 5. - 7. * v).abs() < 1e-11);
            }
        }
    }
    #[test]
    fn curved_pipe_has_normal_circular_sections_and_refuses_sampled_budget_failure() {
        let path = crate::primitives::circle_arc([0.; 3], [0., 0., 1.], 5., 0., 90.).unwrap();
        let coarse = checked(&path, 1., [0., 0., 1.], 3, 0.005).unwrap();
        assert!(!coarse.report.accepted);
        assert!(coarse.surface.is_none());
        let fit = checked(&path, 1., [0., 0., 1.], 32, 0.005).unwrap();
        assert!(fit.report.accepted);
        assert!(fit.report.sampled_control_deviation < coarse.report.sampled_control_deviation);
        let surface = fit.surface.unwrap();
        let lo = surface.knots_u[surface.degree_u];
        let hi = surface.knots_u[surface.control_points.len()];
        let domain = path.domain();
        for i in 0..32 {
            let v = i as f64 / 31.;
            let (center, tangent) =
                crate::framed_sweep::sample(&path, domain[0] + (domain[1] - domain[0]) * v)
                    .unwrap();
            for j in 0..=40 {
                let p = surface
                    .evaluate(lo + (hi - lo) * j as f64 / 40., v)
                    .unwrap()
                    .point;
                let q = [p[0] - center[0], p[1] - center[1], p[2] - center[2]];
                assert!((q[0].hypot(q[1]).hypot(q[2]) - 1.).abs() < 1e-10);
                assert!(q.iter().zip(tangent).map(|(a, b)| a * b).sum::<f64>().abs() < 1e-10);
            }
        }
    }
    #[test]
    fn closed_circle_pipe_preserves_cyclic_seam_and_reports_closed_path() {
        let path = crate::primitives::circle([0.; 3], [0., 0., 1.], 5.).unwrap();
        let fit = checked(&path, 1., [0., 0., 1.], 32, 0.1).unwrap();
        assert!(fit.report.accepted);
        assert!(fit.report.closed_path);
        let surface = fit.surface.unwrap();
        assert!(surface.periodic_v);
        let lo = surface.knots_u[surface.degree_u];
        let hi = surface.knots_u[surface.control_points.len()];
        for i in 0..=40 {
            let u = lo + (hi - lo) * i as f64 / 40.;
            let a = surface.evaluate(u, 0.).unwrap().point;
            let b = surface.evaluate(u, 1.).unwrap().point;
            assert!(a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-11));
        }
    }
    #[test]
    fn refuses_invalid_radius_frame_budget_and_dimension() {
        let mut path = crate::primitives::line([0.; 3], [0., 0., 1.]).unwrap();
        assert!(checked(&path, 0., [1., 0., 0.], 8, 1e-4).is_err());
        assert!(checked(&path, 1., [0., 0., 1.], 8, 1e-4).is_err());
        assert!(checked(&path, 1., [1., 0., 0.], 8, 0.).is_err());
        path.control_points.iter_mut().for_each(|p| {
            p.pop();
        });
        assert!(checked(&path, 1., [1., 0., 0.], 8, 1e-4).is_err());
    }
    #[test]
    fn nan_radius_and_normal_are_typed_non_finite_rejections() {
        let path = crate::primitives::line([0.; 3], [0., 0., 1.]).unwrap();
        let err = checked(&path, f64::NAN, [1., 0., 0.], 8, 1e-4).unwrap_err();
        assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
        assert!(err.contains("radius"), "{err}");
        let err = checked(&path, 1., [f64::NAN, 0., 0.], 8, 1e-4).unwrap_err();
        assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
        assert!(err.contains("initial_normal"), "{err}");
        let err = checked(&path, 1., [1., 0., 0.], 8, f64::NAN).unwrap_err();
        assert!(err.contains("max_deviation"), "{err}");
    }
}
