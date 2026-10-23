//! An open strip transported along a regular 3D path, with a positive width law.
use crate::{Result, check, curve::Curve, framed_sweep::CheckedSweep};
/// The initial direction is projected perpendicular to the path tangent.
/// Width uses positive [width,0,0] rational controls and normalized parameter.
/// Section width is authored exactly in real arithmetic; intervening loft
/// and fourfold refinement are sampled, not a continuous RMF certificate.
pub fn checked(
    path: &Curve,
    width: &Curve,
    normal: [f64; 3],
    sections: usize,
    budget: f64,
) -> Result<CheckedSweep> {
    path.validate()?;
    check(path.control_points[0].len() == 3, "Ribbon path must be 3D")?;
    let (center, tangent) = crate::framed_sweep::sample(path, path.domain()[0])?;
    let norm = normal[0].hypot(normal[1]).hypot(normal[2]);
    check(
        norm.is_finite() && norm > 1e-12,
        "Ribbon initial direction must be finite and nonzero",
    )?;
    let n = normal.map(|x| x / norm);
    let dot = n.iter().zip(tangent).map(|(x, y)| x * y).sum::<f64>();
    let perpendicular: [f64; 3] = std::array::from_fn(|k| n[k] - dot * tangent[k]);
    let length = perpendicular[0]
        .hypot(perpendicular[1])
        .hypot(perpendicular[2]);
    check(
        length.is_finite() && length > 1e-12,
        "Ribbon initial direction is parallel to the path",
    )?;
    let r = perpendicular.map(|x| x / length);
    let a = std::array::from_fn(|k| center[k] - 0.5 * r[k]);
    let b = std::array::from_fn(|k| center[k] + 0.5 * r[k]);
    let profile = crate::primitives::line(a, b)?;
    crate::framed_sweep::checked_scaled_profile(&profile, path, width, normal, sections, budget)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn straight_ribbon_matches_independent_signed_boundaries_and_orientation() {
        let path = crate::primitives::line([3., 4., 5.], [3., 4., 15.]).unwrap();
        let width = crate::primitives::line([2., 0., 0.], [4., 0., 0.]).unwrap();
        for normal in [[1., 0., 2.], [-1., 0., 2.]] {
            let fit = checked(&path, &width, normal, 8, 1e-8).unwrap();
            assert!(fit.report.accepted);
            let s = fit.surface.unwrap();
            for i in 0..=100 {
                let v = i as f64 / 100.;
                for u in [0., 0.13, 0.5, 0.87, 1.] {
                    let p = s.evaluate(u, v).unwrap().point;
                    assert!((p[0] - 3. - normal[0] * (u - 0.5) * (2. + 2. * v)).abs() < 1e-11);
                    assert!((p[1] - 4.).abs() < 1e-11);
                    assert!((p[2] - 5. - 10. * v).abs() < 1e-11);
                }
            }
        }
    }
    #[test]
    fn curved_ribbon_keeps_authored_width_and_refuses_coarse_budget() {
        let path = crate::primitives::circle_arc([0.; 3], [0., 0., 1.], 5., 0., 90.).unwrap();
        let width = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
        let coarse = checked(&path, &width, [0., 0., 1.], 3, 0.01).unwrap();
        assert!(!coarse.report.accepted && coarse.surface.is_none());
        let fine = checked(&path, &width, [0., 0., 1.], 32, 0.01).unwrap();
        assert!(fine.report.accepted);
        let s = fine.surface.unwrap();
        for i in 0..32 {
            let v = i as f64 / 31.;
            let c = path.evaluate(v).unwrap().point;
            let a = s.evaluate(0., v).unwrap().point;
            let b = s.evaluate(1., v).unwrap().point;
            assert!((a[0] - c[0]).abs() < 1e-11 && (b[1] - c[1]).abs() < 1e-11);
            assert!((b[2] - a[2] - 1. - v).abs() < 1e-11);
        }
        assert!(checked(&path, &width, [0., 1., 0.], 32, 0.01).is_err());
        let bad = crate::primitives::line([0., 0., 0.], [1., 0., 0.]).unwrap();
        assert!(checked(&path, &bad, [0., 0., 1.], 32, 0.01).is_err());
    }
    #[test]
    fn closed_ribbon_preserves_cyclic_edges_and_requires_width_match() {
        let path = crate::primitives::circle([0.; 3], [0., 0., 1.], 5.).unwrap();
        let width = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![1., 0., 0.], vec![1., 0., 0.]],
            weights: vec![1., 1.],
            periodic: false,
        };
        let fit = checked(&path, &width, [0., 0., 1.], 32, 0.1).unwrap();
        assert!(fit.report.accepted && fit.report.closed_path);
        let s = fit.surface.unwrap();
        for u in [0., 0.13, 0.5, 0.87, 1.] {
            let a = s.evaluate(u, 0.).unwrap().point;
            let b = s.evaluate(u, 1.).unwrap().point;
            assert!(a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-11));
        }
        let mismatch = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
        assert!(checked(&path, &mismatch, [0., 0., 1.], 32, 0.1).is_err());
    }
}
