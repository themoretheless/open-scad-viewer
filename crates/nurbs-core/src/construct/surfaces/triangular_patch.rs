//! Three-boundary rational Coons patch with a collapsed fourth boundary.
use crate::{Result, check, curve::Curve, surface::Surface};

/// Boundaries are A->B, A->C and B->C. Their endpoints must coincide exactly.
/// V=1 collapses to C and is an intentional parameter singularity.
pub fn patch(base: &Curve, side_a: &Curve, side_b: &Curve) -> Result<Surface> {
    let mut curves = Vec::with_capacity(3);
    for c in [base, side_a, side_b] {
        c.validate()?;
        check(
            c.control_points[0].len() == 3 && !c.periodic,
            "Triangular patch needs nonperiodic 3D boundaries",
        )?;
        let [a, b] = c.domain();
        curves.push(c.trim(a, b)?);
    }
    let endpoint = |c: &Curve| c.control_points.last().unwrap().clone();
    check(
        curves[0].control_points[0] == curves[1].control_points[0]
            && endpoint(&curves[0]) == curves[2].control_points[0]
            && endpoint(&curves[1]) == endpoint(&curves[2]),
        "Triangular patch endpoints must coincide exactly in A->B, A->C, B->C orientation",
    )?;
    // Match precisely the constant scales applied by Coons before constructing
    // the collapsed top. Its varying weights preserve the single apex point.
    let scale = |c: &mut Curve, target: f64| {
        let factor = target / c.weights[0];
        for w in &mut c.weights {
            *w *= factor;
        }
    };
    scale(&mut curves[0], 1.);
    let w_a = curves[0].weights[0];
    let w_b = *curves[0].weights.last().unwrap();
    scale(&mut curves[1], w_a);
    scale(&mut curves[2], w_b);
    for c in &curves {
        c.validate()?;
    }
    let apex = endpoint(&curves[1]);
    let top = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![apex.clone(), apex],
        weights: vec![
            *curves[1].weights.last().unwrap(),
            *curves[2].weights.last().unwrap(),
        ],
        periodic: false,
    };
    crate::coons::patch(&[curves[0].clone(), top, curves[1].clone(), curves[2].clone()])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn collapsed_boundary_adapts_to_different_apex_weights() {
        let base = line([0., 0., 0.], [2., 0., 0.]);
        let mut left = line([0., 0., 0.], [0.5, 2., 1.]);
        left.weights = vec![2., 2.];
        let mut right = line([2., 0., 0.], [0.5, 2., 1.]);
        right.weights = vec![3., 4.];
        let s = patch(&base, &left, &right).unwrap();
        for t in [0., 0.13, 0.37, 0.83, 1.] {
            assert!((s.evaluate(t, 1.).unwrap().point[2] - 1.).abs() < 1e-10);
            for (p, q) in [
                (
                    s.evaluate(0., t).unwrap().point,
                    left.evaluate(t).unwrap().point,
                ),
                (
                    s.evaluate(1., t).unwrap().point,
                    right.evaluate(t).unwrap().point,
                ),
            ] {
                for k in 0..3 {
                    assert!((p[k] - q[k]).abs() < 1e-10);
                }
            }
        }
    }
    #[cfg(feature = "transport")]
    #[test]
    fn json_constructor_preserves_the_collapsed_vertex() {
        let result=crate::transport::dispatch(value_codec::json!({"op":"surface_triangular_patch",
            "base":line([0.,0.,0.],[2.,0.,0.]),"side_a":line([0.,0.,0.],[0.5,2.,1.]),"side_b":line([2.,0.,0.],[0.5,2.,1.])})).unwrap();
        let s: Surface = value_codec::from_value(result).unwrap();
        assert_eq!(s.evaluate(0.37, 1.).unwrap().point, [0.5, 2., 1.]);
    }
    fn line(a: [f64; 3], b: [f64; 3]) -> Curve {
        crate::primitives::line(a, b).unwrap()
    }
    #[test]
    fn matches_independent_affine_triangle() {
        let a = [0., 0., 0.];
        let b = [2., 0., 0.];
        let c = [0.5, 2., 1.];
        let s = patch(&line(a, b), &line(a, c), &line(b, c)).unwrap();
        for u in [0., 0.13, 0.5, 0.87, 1.] {
            for v in [0., 0.17, 0.5, 0.83, 1.] {
                let p = s.evaluate(u, v).unwrap().point;
                for k in 0..3 {
                    assert!(
                        (p[k] - ((1. - v) * ((1. - u) * a[k] + u * b[k]) + v * c[k])).abs() < 1e-10
                    );
                }
            }
        }
    }
    #[test]
    fn rational_arc_and_collapsed_apex_form_an_independent_cone_sector() {
        let base = crate::paths::bezier(
            vec![vec![2., 0., 0.], vec![2., 2., 0.], vec![0., 2., 0.]],
            Some(vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.]),
        )
        .unwrap();
        let s = patch(
            &base,
            &line([2., 0., 0.], [0., 0., 2.]),
            &line([0., 2., 0.], [0., 0., 2.]),
        )
        .unwrap();
        for u in [0., 0.13, 0.5, 0.87, 1.] {
            for v in [0., 0.17, 0.5, 0.83, 1.] {
                let p = s.evaluate(u, v).unwrap().point;
                assert!((p[0] * p[0] + p[1] * p[1] - (2. - p[2]).powi(2)).abs() < 1e-10);
                if v == 1. {
                    assert_eq!(p, [0., 0., 2.]);
                    assert!(s.evaluate(u, v).unwrap().unit_normal().is_none());
                }
            }
        }
    }
    #[test]
    fn preserves_curved_sides_and_refuses_unmatched_endpoints() {
        let base = line([0., 0., 0.], [2., 0., 0.]);
        let left = crate::paths::bezier(
            vec![vec![0., 0., 0.], vec![-1., 1., 1.], vec![0.5, 2., 1.]],
            None,
        )
        .unwrap();
        let right = crate::paths::bezier(
            vec![vec![2., 0., 0.], vec![3., 1., -1.], vec![0.5, 2., 1.]],
            None,
        )
        .unwrap();
        let s = patch(&base, &left, &right).unwrap();
        for t in [0., 0.13, 0.37, 0.83, 1.] {
            for (p, q) in [
                (
                    s.evaluate(t, 0.).unwrap().point,
                    base.evaluate(t).unwrap().point,
                ),
                (
                    s.evaluate(0., t).unwrap().point,
                    left.evaluate(t).unwrap().point,
                ),
                (
                    s.evaluate(1., t).unwrap().point,
                    right.evaluate(t).unwrap().point,
                ),
            ] {
                for k in 0..3 {
                    assert!((p[k] - q[k]).abs() < 1e-10);
                }
            }
        }
        let wrong = line([2., 0., 0.], [0.5, 2., 1.001]);
        assert!(patch(&base, &left, &wrong).is_err());
    }
}
