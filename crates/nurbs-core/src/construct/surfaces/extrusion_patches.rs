//! Extrusion of a dense curve through retained knot-span patches, without fitting.
use crate::{Result, check, curve::Curve, surface::Surface};

/// Preserve the original U subdomains; every patch has V=[0,1].
/// Small curves retain their single-surface extrusion. Dense nonperiodic
/// curves split at existing knots, with no tolerance relaxation or resampling.
/// Binary64 knot-insertion roundoff and sewn topology are not certified.
pub fn extrude(curve: &Curve, vector: [f64; 3]) -> Result<Vec<Surface>> {
    curve.validate()?;
    check(
        curve.control_points[0].len() == 3,
        "Patch extrusion requires a 3D curve",
    )?;
    if curve.control_points.len() <= 32 {
        return Ok(vec![crate::surface::extrude(curve, vector)?]);
    }
    check(
        !curve.periodic,
        "Dense periodic extrusion requires an explicit clamped representation",
    )?;
    let segments = curve.decompose()?;
    check(
        !segments.is_empty() && segments.len() <= 256,
        "Patch extrusion exceeds 256 active spans",
    )?;
    segments
        .iter()
        .map(|segment| crate::surface::extrude(segment.definition(), vector))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dense_full_knot_retains_tight_fit_and_original_parameter_domains() {
        let a = crate::toroidal_spiral::approximate_knot([3., 4., 5.], 3., 1., 2, 3, 0., 0., 1e-4)
            .unwrap();
        assert!(a.curve.control_points.len() > 32);
        let full = crate::surface::extrude(&a.curve, [0., 0., 1.]).unwrap();
        let patches = extrude(&a.curve, [0., 0., 1.]).unwrap();
        assert_eq!(patches.len(), a.spans);
        for (i, patch) in patches.iter().enumerate() {
            let lo = i as f64 / a.spans as f64;
            let hi = (i + 1) as f64 / a.spans as f64;
            assert_eq!(patch.knots_u[0], lo);
            assert_eq!(*patch.knots_u.last().unwrap(), hi);
            for s in [0., 0.17, 0.5, 0.83, 1.] {
                let u = lo + (hi - lo) * s;
                let p = a.curve.evaluate(u).unwrap().point;
                for v in [0., 0.3, 1.] {
                    let q = patch.evaluate(u, v).unwrap().point;
                    let whole = full.evaluate(u, v).unwrap().point;
                    assert!(whole.iter().zip(&q).all(|(x, y)| (x - y).abs() < 1e-11));
                    for d in 0..3 {
                        assert!((q[d] - p[d] - if d == 2 { v } else { 0. }).abs() < 1e-11);
                    }
                    let theta = std::f64::consts::TAU * 2. * u;
                    let phi = std::f64::consts::TAU * 3. * u;
                    let exact = [
                        3. + (3. + phi.cos()) * theta.cos(),
                        4. + (3. + phi.cos()) * theta.sin(),
                        5. + phi.sin() + v,
                    ];
                    let e = (q[0] - exact[0])
                        .hypot(q[1] - exact[1])
                        .hypot(q[2] - exact[2]);
                    assert!(e <= a.real_arithmetic_error_estimate + 1e-11);
                }
            }
            if i > 0 {
                for v in [0., 0.3, 1.] {
                    let p = patches[i - 1].evaluate(lo, v).unwrap().point;
                    let q = patch.evaluate(lo, v).unwrap().point;
                    assert!(p.iter().zip(q).all(|(x, y)| (x - y).abs() < 1e-11));
                }
            }
        }
    }
    #[test]
    fn dense_rational_weights_and_nonuniform_parameters_are_retained() {
        let points: Vec<_> = (0..34).map(|i| [i as f64, (i as f64).sin(), 0.]).collect();
        let mut curve = crate::primitives::polyline(&points, false).unwrap();
        curve.weights = (0..34).map(|i| 1. + (i % 3) as f64).collect();
        // A strictly increasing nonlinear reparameterization retains the shape
        // of each rational span and checks that no global [0,1] reset occurs.
        curve.knots.iter_mut().for_each(|k| *k = 2. + k.powi(2));
        let patches = extrude(&curve, [0., 1., 2.]).unwrap();
        assert_eq!(patches.len(), 33);
        for patch in patches {
            let lo = patch.knots_u[0];
            let hi = *patch.knots_u.last().unwrap();
            for t in [0.13, 0.5, 0.87] {
                let u = lo + (hi - lo) * t;
                let p = curve.evaluate(u).unwrap().point;
                let q = patch.evaluate(u, 0.3).unwrap().point;
                for d in 0..3 {
                    assert!((q[d] - p[d] - [0., 0.3, 0.6][d]).abs() < 1e-11);
                }
            }
        }
    }
    #[test]
    fn preserves_small_rational_profile_and_refuses_invalid_inputs() {
        let mut curve = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
        curve.weights = vec![1., 2.];
        let patches = extrude(&curve, [0., 0., 2.]).unwrap();
        assert_eq!(patches.len(), 1);
        let point = patches[0].evaluate(0.5, 0.5).unwrap().point;
        assert!((point[0] - 5. / 3.).abs() < 1e-12);
        assert!((point[2] - 1.).abs() < 1e-12);
        assert!(extrude(&curve, [0.; 3]).is_err());
        curve.control_points[0].pop();
        curve.control_points[1].pop();
        assert!(extrude(&curve, [0., 0., 2.]).is_err());
    }
}
