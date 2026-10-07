//! Sampled rotation-minimizing transport of arbitrary profiles with a scale law.
use crate::{Result, curve::Curve, foundation::guards::require_finite_point, framed_sweep::CheckedSweep};
/// Profile is authored in world coordinates relative to the path start.
/// Scale is a positive dimensionless rational law encoded as [scale,0,0].
/// All three curve domains normalize independently. Initial normal selects the
/// frame but does not rotate the first profile. Closed paths correct holonomy
/// and require equal scale endpoints. Report is sampled, not continuous.
pub fn checked(
    profile: &Curve,
    path: &Curve,
    scale: &Curve,
    normal: [f64; 3],
    sections: usize,
    budget: f64,
) -> Result<CheckedSweep> {
    profile.validate()?;
    scale.validate()?;
    require_finite_point(&normal, "normal")?;
    crate::framed_sweep::checked_scaled_profile(profile, path, scale, normal, sections, budget)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn scale(a: f64, b: f64) -> Curve {
        Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![a, 0., 0.], vec![b, 0., 0.]],
            weights: vec![1., 1.],
            periodic: false,
        }
    }
    #[test]
    fn independent_weighted_profile_and_straight_varying_scale_equation() {
        let profile = crate::paths::bezier(
            vec![vec![2., 0., 0.], vec![2., 2., 1.], vec![0., 2., 2.]],
            Some(vec![1., 2., 1.]),
        )
        .unwrap();
        let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
        let fit = checked(&profile, &path, &scale(1., 2.), [1., 0., 0.], 8, 1e-8).unwrap();
        assert!(fit.report.accepted);
        let s = fit.surface.unwrap();
        assert_eq!(
            s.weights.iter().map(|r| r[0]).collect::<Vec<_>>(),
            profile.weights
        );
        for i in 0..=100 {
            let u = i as f64 / 100.;
            let a = (1. - u).powi(2);
            let b = 4. * u * (1. - u);
            let c = u * u;
            let q = [
                (2. * a + 2. * b) / (a + b + c),
                (2. * b + 2. * c) / (a + b + c),
                (b + 2. * c) / (a + b + c),
            ];
            for v in [0., 0.13, 0.5, 0.87, 1.] {
                let p = s.evaluate(u, v).unwrap().point;
                for k in 0..3 {
                    let e = (1. + v) * q[k] + if k == 2 { 10. * v } else { 0. };
                    assert!((p[k] - e).abs() < 1e-11);
                }
            }
        }
    }
    #[test]
    fn curved_profile_sections_keep_scaled_orientation_and_reject_coarse_budget() {
        let profile = crate::paths::bezier(
            vec![vec![5., 0., -1.], vec![6., 0., 0.], vec![5., 0., 1.]],
            None,
        )
        .unwrap();
        let path = crate::primitives::circle_arc([0.; 3], [0., 0., 1.], 5., 0., 90.).unwrap();
        let coarse = checked(&profile, &path, &scale(1., 2.), [0., 0., 1.], 3, 0.01).unwrap();
        assert!(!coarse.report.accepted && coarse.surface.is_none());
        let fine = checked(&profile, &path, &scale(1., 2.), [0., 0., 1.], 32, 0.01).unwrap();
        assert!(fine.report.accepted);
        let s = fine.surface.unwrap();
        for i in 0..32 {
            let v = i as f64 / 31.;
            let center = path.evaluate(v).unwrap().point;
            for u in [0., 0.13, 0.5, 0.87, 1.] {
                let radial = 2. * u * (1. - u);
                let height = 2. * u - 1.;
                let p = s.evaluate(u, v).unwrap().point;
                assert!((p[0] - center[0] - (1. + v) * radial * center[0] / 5.).abs() < 1e-10);
                assert!((p[1] - center[1] - (1. + v) * radial * center[1] / 5.).abs() < 1e-10);
                assert!((p[2] - (1. + v) * height).abs() < 1e-10);
            }
        }
        assert!(checked(&profile, &path, &scale(0., 2.), [0., 0., 1.], 32, 0.01).is_err());
    }
    #[test]
    fn closed_profile_sweep_retains_cyclic_seam_and_requires_equal_scale() {
        let path = crate::primitives::circle([0.; 3], [0., 0., 1.], 5.).unwrap();
        let profile = crate::primitives::line([5., 0., -1.], [5., 0., 1.]).unwrap();
        let fit = checked(&profile, &path, &scale(1., 1.), [0., 0., 1.], 32, 0.1).unwrap();
        assert!(fit.report.accepted && fit.report.closed_path);
        let s = fit.surface.unwrap();
        for u in [0., 0.13, 0.5, 0.87, 1.] {
            let a = s.evaluate(u, 0.).unwrap().point;
            let b = s.evaluate(u, 1.).unwrap().point;
            assert!(a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-11));
        }
        assert!(checked(&profile, &path, &scale(1., 2.), [0., 0., 1.], 32, 0.1).is_err());
    }
}
