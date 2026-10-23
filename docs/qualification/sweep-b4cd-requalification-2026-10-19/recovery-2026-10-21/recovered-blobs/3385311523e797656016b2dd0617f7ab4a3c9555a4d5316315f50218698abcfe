//! Ruled affine-ellipse transition preserving authored angular correspondence.
use crate::{Result, surface::Surface};
#[derive(Clone, Debug)]
pub struct EllipseSection {
    pub center: [f64; 3],
    pub axis_u: [f64; 3],
    pub axis_v: [f64; 3],
}
/// Each section is center+axis_u*cos(theta)+axis_v*sin(theta).
/// Axis vectors have length units; independent skew vectors are permitted.
/// Both use the same rational angular parameterization on U=[0,1].
/// Linear V gives a G0 ruled transition, not an automatic G1/G2 blend.
pub fn ruled(start: &EllipseSection, end: &EllipseSection) -> Result<Surface> {
    let a = crate::primitives::ellipse_arc(start.center, start.axis_u, start.axis_v, 0., 360.)?;
    let b = crate::primitives::ellipse_arc(end.center, end.axis_u, end.axis_v, 0., 360.)?;
    super::ruled_loft_two(a, b, "Ellipse transition endpoint profiles must differ")
}
#[cfg(test)]
mod tests {
    use super::*;
    fn angular(u: f64) -> [f64; 2] {
        let t = u * 4.;
        let quadrant = (t.floor() as usize).min(3);
        let s = t - quadrant as f64;
        let a = (1. - s).powi(2);
        let b = 2. * std::f64::consts::FRAC_1_SQRT_2 * s * (1. - s);
        let c = s * s;
        let x = (a + b) / (a + b + c);
        let y = (b + c) / (a + b + c);
        match quadrant {
            0 => [x, y],
            1 => [-y, x],
            2 => [-x, -y],
            _ => [y, -x],
        }
    }
    #[test]
    fn unequal_rotated_and_skew_ellipses_match_independent_ruled_equation() {
        let a = EllipseSection {
            center: [3., 4., 5.],
            axis_u: [2., 0., 0.],
            axis_v: [0., 1., 0.],
        };
        let b = EllipseSection {
            center: [6., 7., 8.],
            axis_u: [0., 3., 0.],
            axis_v: [1., 1., 2.],
        };
        let s = ruled(&a, &b).unwrap();
        assert_eq!(s.degree_u, 2);
        assert_eq!(s.degree_v, 1);
        assert_eq!(s.control_points.len(), 9);
        assert!(s.control_points.iter().all(|row| row.len() == 2));
        assert_eq!(s.control_points.first(), s.control_points.last());
        for i in 0..=1000 {
            let u = i as f64 / 1000.;
            let [x, y] = angular(u);
            for v in [0., 0.13, 0.5, 0.87, 1.] {
                let p = s.evaluate(u, v).unwrap().point;
                for k in 0..3 {
                    let expected = (1. - v) * (a.center[k] + a.axis_u[k] * x + a.axis_v[k] * y)
                        + v * (b.center[k] + b.axis_u[k] * x + b.axis_v[k] * y);
                    assert!((p[k] - expected).abs() < 1e-11);
                }
                if v == 0. {
                    assert!((((p[0] - 3.) / 2.).powi(2) + (p[1] - 4.).powi(2) - 1.).abs() < 1e-11);
                    assert!((p[2] - 5.).abs() < 1e-11);
                }
            }
        }
    }
    #[test]
    fn refuses_degenerate_nonfinite_and_coincident_ellipse_profiles() {
        let a = EllipseSection {
            center: [0.; 3],
            axis_u: [2., 0., 0.],
            axis_v: [0., 1., 0.],
        };
        assert!(ruled(&a, &a).is_err());
        for b in [
            EllipseSection {
                center: [0., 0., 1.],
                axis_u: [0.; 3],
                axis_v: [0., 1., 0.],
            },
            EllipseSection {
                center: [0., 0., 1.],
                axis_u: [2., 0., 0.],
                axis_v: [1., 0., 0.],
            },
            EllipseSection {
                center: [f64::NAN, 0., 1.],
                axis_u: [2., 0., 0.],
                axis_v: [0., 1., 0.],
            },
        ] {
            assert!(ruled(&a, &b).is_err());
        }
    }
}
