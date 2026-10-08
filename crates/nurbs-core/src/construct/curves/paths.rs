//! Rational Bezier paths and C0 composition without geometric fitting.
use crate::{Result, check, curve::Curve};

/// A single clamped rational Bezier span on [0,1], degree 1..25.
pub fn bezier(points: Vec<Vec<f64>>, weights: Option<Vec<f64>>) -> Result<Curve> {
    check(
        (2..=26).contains(&points.len()),
        "Bezier requires 2..26 control points",
    )?;
    let degree = points.len() - 1;
    let mut knots = vec![0.; degree + 1];
    knots.extend(vec![1.; degree + 1]);
    let curve = Curve {
        degree,
        knots,
        weights: weights.unwrap_or_else(|| vec![1.; points.len()]),
        control_points: points,
        periodic: false,
    };
    curve.validate()?;
    Ok(curve)
}

/// Joins clamped curves whose endpoint controls match exactly, elevating their
/// degrees to a common degree without fitting.
/// Each input receives an equal subinterval of [0,1]; seams are C0. There is
/// no snapping, refitting, reversal or G1/G2 enforcement.
pub fn compose(curves: &[Curve]) -> Result<Curve> {
    check(
        (2..=32).contains(&curves.len()),
        "Composition requires 2..32 curves",
    )?;
    let degree = curves.iter().map(|c| c.degree).max().unwrap();
    for curve in curves {
        curve.validate()?;
        let [start, end] = curve.domain();
        check(
            (end - start).is_finite(),
            "Composite source domain span must be representable",
        )?;
        check(!curve.periodic, "Composition requires nonperiodic curves")?;
        check(
            curve.knots[..=curve.degree].iter().all(|&k| k == start)
                && curve.knots[curve.control_points.len()..]
                    .iter()
                    .all(|&k| k == end),
            "Composition requires clamped endpoints",
        )?;
    }
    for pair in curves.windows(2) {
        check(
            pair[0].control_points.last() == pair[1].control_points.first(),
            "Composite endpoints must match exactly",
        )?;
    }
    let prepared = curves
        .iter()
        .map(|source| {
            let mut curve = source.elevate(degree)?;
            // Homogeneous materialization may round an endpoint after degree
            // elevation. Retain the authored common endpoint controls explicitly.
            curve.control_points[0] = source.control_points[0].clone();
            let last = curve.control_points.len() - 1;
            curve.control_points[last] = source.control_points.last().unwrap().clone();
            curve.validate()?;
            Ok(curve)
        })
        .collect::<Result<Vec<_>>>()?;
    let curves = prepared.as_slice();
    let total = 1 + curves
        .iter()
        .map(|c| c.control_points.len() - 1)
        .sum::<usize>();
    check(total <= 256, "Composite path exceeds 256 control points")?;
    let count = curves.len() as f64;
    let mut points = Vec::with_capacity(total);
    let mut weights = Vec::<f64>::with_capacity(total);
    let mut knots = vec![0.; degree + 1];
    for (i, curve) in curves.iter().enumerate() {
        let [start, end] = curve.domain();
        let factor: f64 = if i == 0 {
            1.
        } else {
            weights[weights.len() - 1] / curve.weights[0]
        };
        check(
            factor.is_finite() && factor > 0.,
            "Composite weight scaling is not representable",
        )?;
        for j in usize::from(i != 0)..curve.control_points.len() {
            points.push(curve.control_points[j].clone());
            weights.push(curve.weights[j] * factor);
        }
        let source = &curve.knots[degree + 1..curve.knots.len() - degree - 1];
        let mapped = source
            .iter()
            .map(|k| (i as f64 + (*k - start) / (end - start)) / count)
            .collect::<Vec<_>>();
        check(
            source
                .windows(2)
                .zip(mapped.windows(2))
                .all(|(a, b)| a[0] == a[1] || b[0] < b[1]),
            "Composite parameter mapping collapsed distinct knots",
        )?;
        check(
            mapped
                .iter()
                .all(|&k| k > i as f64 / count && k < (i + 1) as f64 / count),
            "Composite internal knot collapsed onto a seam",
        )?;
        knots.extend(mapped);
        knots.extend(vec![
            (i + 1) as f64 / count;
            if i + 1 == curves.len() {
                degree + 1
            } else {
                degree
            }
        ]);
    }
    let curve = Curve {
        degree,
        knots,
        control_points: points,
        weights,
        periodic: false,
    };
    curve.validate()?;
    Ok(curve)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "transport")]
    #[test]
    fn json_paths_validate_weights_and_compose_references_as_curve_values() {
        let a = bezier(vec![vec![0., 0., 0.], vec![1., 1., 0.]], None).unwrap();
        let b = bezier(vec![vec![1., 1., 0.], vec![2., 0., 0.]], None).unwrap();
        let request = value_codec::json!({"op":"curve_compose","curves":[a,b]});
        let joined: Curve = value_codec::from_value(crate::dispatch(request).unwrap()).unwrap();
        assert_eq!(joined.control_points.len(), 3);
        let c: Curve = value_codec::from_value(
            crate::dispatch(value_codec::json!({"op":"curve_bezier","points":[[0,0,0],[1,2,3]]}))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(c.weights, vec![1., 1.]);
        assert!(
            crate::dispatch(
                value_codec::json!({"op":"curve_bezier","points":[[0,0,0],[1,2,3]],"weights":[1]})
            )
            .is_err()
        );
    }
    #[test]
    fn rational_bezier_matches_independent_homogeneous_casteljau() {
        let points = vec![
            vec![1., 2., 3.],
            vec![-2., 5., 1.],
            vec![4., -1., 2.],
            vec![8., 3., -2.],
        ];
        let weights = vec![2., 0.5, 3., 1.];
        let curve = bezier(points.clone(), Some(weights.clone())).unwrap();
        for t in [0., 0.13, 0.4, 0.91, 1.] {
            let mut h = points
                .iter()
                .zip(&weights)
                .map(|(p, w)| [p[0] * w, p[1] * w, p[2] * w, *w])
                .collect::<Vec<_>>();
            for remaining in (1..h.len()).rev() {
                for i in 0..remaining {
                    for a in 0..4 {
                        h[i][a] = (1. - t) * h[i][a] + t * h[i + 1][a]
                    }
                }
            }
            let actual = curve.evaluate(t).unwrap().point;
            for a in 0..3 {
                assert!((actual[a] - h[0][a] / h[0][3]).abs() < 1e-12)
            }
        }
        assert!(bezier(points.clone(), Some(vec![1.; 2])).is_err());
        assert!(bezier(points, Some(vec![1., 0., 1., 1.])).is_err());
    }
    #[test]
    fn composition_preserves_rational_pieces_with_different_weight_scales_and_domains() {
        let a = bezier(
            vec![vec![0., 0.], vec![1., 3.], vec![2., 1.]],
            Some(vec![2., 1., 4.]),
        )
        .unwrap();
        let mut b = bezier(
            vec![vec![2., 1.], vec![3., -4.], vec![5., 2.]],
            Some(vec![0.5, 3., 1.]),
        )
        .unwrap();
        for k in &mut b.knots {
            *k = 7. + 5. * *k
        }
        let c = compose(&[a.clone(), b.clone()]).unwrap();
        assert_eq!(c.control_points.len(), 5);
        for t in [0., 0.1, 0.43, 0.9, 1.] {
            for (source, u) in [(&a, t / 2.), (&b, (1. + t) / 2.)] {
                let [min, max] = source.domain();
                let p = source.evaluate(min + t * (max - min)).unwrap().point;
                let q = c.evaluate(u).unwrap().point;
                for i in 0..2 {
                    assert!((p[i] - q[i]).abs() < 1e-12)
                }
            }
        }
        b.control_points[0][0] += 0.001;
        assert!(compose(&[a.clone(), b]).is_err());
        let line = bezier(vec![vec![2., 1.], vec![3., 1.]], None).unwrap();
        let mixed = compose(&[a, line]).unwrap();
        assert_eq!(mixed.degree, 2);
        for t in [0., 0.3, 0.9, 1.] {
            let p = mixed.evaluate((1. + t) / 2.).unwrap().point;
            assert!((p[0] - (2. + t)).abs() < 1e-12);
            assert!((p[1] - 1.).abs() < 1e-12);
        }
    }
}

mod round_polyline;
pub use round_polyline::{round_polyline, transition_polyline, closed_round_polyline, closed_transition_polyline};

mod miter_sections;
pub use miter_sections::{miter_sections, closed_miter_sections};
