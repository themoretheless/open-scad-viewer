//! Four retained rational patches from a circle to a rectangular wire.
use crate::{Result, check, circle_transition::CircleSection, surface::Surface};
#[derive(Clone, Debug)]
pub struct RectangleSection {
    pub center: [f64; 3],
    /// Orthogonal half-edge vectors, in length units.
    pub axis_u: [f64; 3],
    pub axis_v: [f64; 3],
}
/// Circle quarter i maps to rectangle edge i, starting at center+axis_u+axis_v.
/// Shared Bernstein weight multiplication preserves linear Cartesian V.
/// U keeps the original circle quarter domain; V=[0,1]. G0 seams, no sewing.
pub fn ruled(circle: &CircleSection, rectangle: &RectangleSection) -> Result<Vec<Surface>> {
    check(
        rectangle
            .center
            .iter()
            .chain(&rectangle.axis_u)
            .chain(&rectangle.axis_v)
            .all(|x| x.is_finite()),
        "Rectangle frame must be finite",
    )?;
    let norm = |p: [f64; 3]| p[0].hypot(p[1]).hypot(p[2]);
    let a = norm(rectangle.axis_u);
    let b = norm(rectangle.axis_v);
    check(
        a.is_finite() && b.is_finite() && a > 1e-12 && b > 1e-12,
        "Rectangle half-edge vectors must be finite and nonzero",
    )?;
    let dot = rectangle
        .axis_u
        .iter()
        .zip(rectangle.axis_v)
        .map(|(x, y)| (x / a) * (y / b))
        .sum::<f64>();
    check(
        dot.abs() <= 1e-12,
        "Rectangle half-edge vectors must be perpendicular",
    )?;
    let corners: Vec<[f64; 3]> = [(1., 1.), (-1., 1.), (-1., -1.), (1., -1.)]
        .into_iter()
        .map(|(a, b)| {
            std::array::from_fn(|k| {
                rectangle.center[k] + a * rectangle.axis_u[k] + b * rectangle.axis_v[k]
            })
        })
        .collect();
    let curve = crate::circle_transition::profile(circle)?;
    let spans = curve.decompose()?;
    check(
        spans.len() == 4,
        "Circle transition requires four quadrants",
    )?;
    let mut patches = Vec::new();
    for (i, span) in spans.iter().enumerate() {
        let c = span.definition();
        let w = &c.weights;
        check(
            c.control_points.len() == 3,
            "Circle quadrant must have three controls",
        )?;
        let weights = [w[0], (w[0] + 2. * w[1]) / 3., (2. * w[1] + w[2]) / 3., w[2]];
        let h: Vec<[f64; 3]> = c
            .control_points
            .iter()
            .zip(w)
            .map(|(p, w)| std::array::from_fn(|k| p[k] * w))
            .collect();
        let mut controls = Vec::new();
        for j in 0..4 {
            let circle_h: [f64; 3] = std::array::from_fn(|k| match j {
                0 => h[0][k],
                1 => (h[0][k] + 2. * h[1][k]) / 3.,
                2 => (2. * h[1][k] + h[2][k]) / 3.,
                _ => h[2][k],
            });
            let p = corners[i];
            let q = corners[(i + 1) % 4];
            let rectangle_h: [f64; 3] = std::array::from_fn(|k| match j {
                0 => p[k] * w[0],
                1 => (q[k] * w[0] + 2. * p[k] * w[1]) / 3.,
                2 => (2. * q[k] * w[1] + p[k] * w[2]) / 3.,
                _ => q[k] * w[2],
            });
            controls.push(vec![
                circle_h.map(|x| x / weights[j]).to_vec(),
                rectangle_h.map(|x| x / weights[j]).to_vec(),
            ]);
        }
        let [lo, hi] = c.domain();
        let patch = Surface {
            degree_u: 3,
            degree_v: 1,
            knots_u: vec![lo, lo, lo, lo, hi, hi, hi, hi],
            knots_v: vec![0., 0., 1., 1.],
            control_points: controls,
            weights: weights.into_iter().map(|w| vec![w, w]).collect(),
            periodic_u: false,
            periodic_v: false,
        };
        patch.validate()?;
        patches.push(patch);
    }
    Ok(patches)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn circle() -> CircleSection {
        CircleSection {
            center: [0.; 3],
            normal: [0., 0., 1.],
            seam: [1., 0., 0.],
            radius: 2.,
        }
    }
    fn rectangle() -> RectangleSection {
        RectangleSection {
            center: [1., 2., 5.],
            axis_u: [3., 0., 0.],
            axis_v: [0., 1., 0.],
        }
    }
    #[test]
    fn independent_quarter_circle_rectangle_edges_and_linear_interior() {
        let patches = ruled(&circle(), &rectangle()).unwrap();
        assert_eq!(patches.len(), 4);
        let corners = [[4., 3., 5.], [-2., 3., 5.], [-2., 1., 5.], [4., 1., 5.]];
        for (i, s) in patches.iter().enumerate() {
            assert_eq!(s.degree_u, 3);
            assert_eq!(s.control_points.len(), 4);
            for j in 0..=200 {
                let t = j as f64 / 200.;
                let u = (i as f64 + t) / 4.;
                let a = (1. - t).powi(2);
                let b = 2. * std::f64::consts::FRAC_1_SQRT_2 * t * (1. - t);
                let c = t * t;
                let x = (a + b) / (a + b + c);
                let y = (b + c) / (a + b + c);
                let xy = match i {
                    0 => [x, y],
                    1 => [-y, x],
                    2 => [-x, -y],
                    _ => [y, -x],
                };
                let ca = [2. * xy[0], 2. * xy[1], 0.];
                let rb: [f64; 3] =
                    std::array::from_fn(|k| (1. - t) * corners[i][k] + t * corners[(i + 1) % 4][k]);
                for v in [0., 0.13, 0.5, 0.87, 1.] {
                    let p = s.evaluate(u, v).unwrap().point;
                    assert!(
                        p.iter()
                            .enumerate()
                            .all(|(k, x)| (x - ((1. - v) * ca[k] + v * rb[k])).abs() < 1e-11)
                    );
                }
            }
            for v in [0., 0.13, 0.5, 0.87, 1.] {
                let u = (i + 1) as f64 / 4.;
                let a = s.evaluate(u, v).unwrap().point;
                let next = (i + 1) % 4;
                let b = patches[next].evaluate(next as f64 / 4., v).unwrap().point;
                assert!(a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-11));
            }
        }
    }
    #[test]
    fn refuses_skew_zero_or_nonfinite_rectangle_frames() {
        let r = rectangle();
        for bad in [
            RectangleSection {
                axis_v: [1., 1., 0.],
                ..r.clone()
            },
            RectangleSection {
                axis_u: [0.; 3],
                ..r.clone()
            },
            RectangleSection {
                center: [f64::NAN, 0., 0.],
                ..r
            },
        ] {
            assert!(ruled(&circle(), &bad).is_err());
        }
    }
}
