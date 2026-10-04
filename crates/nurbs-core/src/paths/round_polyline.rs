use crate::{Result, check, curve::Curve};
use math_core::{cross, dot, sub};

type Point = [f64; 3];
fn length(v: Point) -> f64 {
    v[0].hypot(v[1]).hypot(v[2])
}

/// Replaces each nonstraight corner of an open spatial polyline by a rational
/// circular arc of the requested radius. Lines and arcs meet geometrically G1;
/// their composed parameterization is only C0. No global intersection certificate.
pub fn round_polyline(points: &[Point], radius: f64) -> Result<Curve> {
    corner_polyline(points, radius, false, false)
}

/// Quintic transitions with tangent and zero-curvature joins to retained lines.
/// Setback is the distance from a corner along either incident edge.
/// C2 in real arithmetic after speed scaling; binary64 jets are not certified.
pub fn transition_polyline(points: &[Point], setback: f64) -> Result<Curve> {
    corner_polyline(points, setback, true, false)
}

/// Closed path through 3..16 cyclic sites, encoded as clamped NURBS.
/// Seam starts after the final corner; no repeated endpoint is supplied.
pub fn closed_round_polyline(points: &[Point], radius: f64) -> Result<Curve> {
    corner_polyline(points, radius, false, true)
}
pub fn closed_transition_polyline(points: &[Point], setback: f64) -> Result<Curve> {
    corner_polyline(points, setback, true, true)
}

fn corner_polyline(points: &[Point], radius: f64, transition: bool, closed: bool) -> Result<Curve> {
    check(
        if closed {
            (3..=16).contains(&points.len())
        } else {
            (2..=17).contains(&points.len())
        },
        "Corner path needs 2..17 open or 3..16 closed points",
    )?;
    check(
        radius.is_finite() && radius > 0.,
        "Corner path radius must be positive and finite",
    )?;
    check(
        points.iter().flatten().all(|x| x.is_finite()),
        "Corner path points must be finite",
    )?;
    check(
        points.first() != points.last(),
        "Corner path requires distinct open endpoints",
    )?;
    let edges = (0..points.len() - usize::from(!closed))
        .map(|i| {
            let delta = sub(points[(i + 1) % points.len()], points[i]);
            let size = length(delta);
            check(
                size.is_finite() && size > 0.,
                "Corner path segments must have finite positive length",
            )?;
            Ok((delta.map(|x| x / size), size))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut cuts = vec![0.; points.len()];
    let mut weights = vec![1.; points.len()];
    for i in usize::from(!closed)..points.len() - usize::from(!closed) {
        let cosine = dot(edges[(i + edges.len() - 1) % edges.len()].0, edges[i].0).clamp(-1., 1.);
        check(
            cosine > -1. + 1e-12,
            "Corner path cannot resolve a reversal",
        )?;
        let angle = length(cross(
            edges[(i + edges.len() - 1) % edges.len()].0,
            edges[i].0,
        ))
        .atan2(cosine);
        if angle > 1e-12 {
            cuts[i] = if transition {
                radius
            } else {
                radius * (angle / 2.).tan()
            };
            weights[i] = (angle / 2.).cos();
            check(
                cuts[i].is_finite() && cuts[i] > 0.,
                "Corner path corner offset is not representable",
            )?;
        }
    }
    for (i, edge) in edges.iter().enumerate() {
        check(
            edge.1 - cuts[i] - cuts[(i + 1) % points.len()] > 64. * f64::EPSILON * edge.1,
            "Corner path neighboring fillets overlap or consume a segment",
        )?;
    }
    let mut pieces = Vec::new();
    let mut cursor = if closed {
        let last = points.len() - 1;
        std::array::from_fn(|k| points[last][k] + cuts[last] * edges[last].0[k])
    } else {
        points[0]
    };
    for i in usize::from(!closed)..points.len() - usize::from(!closed) {
        let entry = std::array::from_fn(|k| {
            points[i][k] - cuts[i] * edges[(i + edges.len() - 1) % edges.len()].0[k]
        });
        let exit = std::array::from_fn(|k| points[i][k] + cuts[i] * edges[i].0[k]);
        pieces.push(crate::primitives::line(cursor, entry)?);
        if cuts[i] > 0. {
            if transition {
                let incoming = edges[(i + edges.len() - 1) % edges.len()].0;
                let outgoing = edges[i].0;
                let step = 2. * cuts[i] / 5.;
                let from_entry =
                    |n: f64| (0..3).map(|k| entry[k] + n * step * incoming[k]).collect();
                let from_exit = |n: f64| (0..3).map(|k| exit[k] - n * step * outgoing[k]).collect();
                pieces.push(super::bezier(
                    vec![
                        entry.to_vec(),
                        from_entry(1.),
                        from_entry(2.),
                        from_exit(2.),
                        from_exit(1.),
                        exit.to_vec(),
                    ],
                    None,
                )?);
            } else {
                pieces.push(super::bezier(
                    vec![entry.to_vec(), points[i].to_vec(), exit.to_vec()],
                    Some(vec![1., weights[i], 1.]),
                )?);
            }
        }
        cursor = exit;
    }
    if !closed {
        pieces.push(crate::primitives::line(cursor, *points.last().unwrap())?);
    }
    if pieces.len() == 1 {
        return Ok(pieces.remove(0));
    }
    let mut curve = super::compose(&pieces)?;
    // Equal piece intervals introduce avoidable speed jumps at joins.
    // Each corner piece has equal endpoint speeds; scale its interval by that
    // speed, just as a line interval scales by its length. The image is unchanged.
    let speeds = pieces
        .iter()
        .map(|piece| {
            let derivative = piece.evaluate(piece.domain()[0])?.d1.unwrap();
            Ok(length([derivative[0], derivative[1], derivative[2]]))
        })
        .collect::<Result<Vec<_>>>()?;
    let total = speeds.iter().sum::<f64>();
    check(
        total.is_finite() && total > 0.,
        "Corner path parameter speed is not representable",
    )?;
    let mut boundaries = vec![0.];
    let mut cumulative = 0.;
    for speed in speeds {
        cumulative += speed;
        boundaries.push(cumulative / total);
    }
    *boundaries.last_mut().unwrap() = 1.;
    check(
        boundaries.windows(2).all(|pair| pair[0] < pair[1]),
        "Corner path parameter interval collapsed",
    )?;
    for knot in &mut curve.knots {
        *knot = boundaries[(*knot * pieces.len() as f64).round() as usize];
    }
    curve.validate()?;
    Ok(curve)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cyclic_corner_paths_refine_with_closed_rmf_sections() {
        let planar = [[0., 0., 0.], [10., 0., 0.], [10., 10., 0.], [0., 10., 0.]];
        let spatial = [[0., 0., 0.], [10., 0., 0.], [10., 10., 5.], [0., 10., 0.]];
        let profile = crate::primitives::line([0., 8., 0.25], [0., 8., 0.5]).unwrap();
        let scale = super::super::bezier(vec![vec![1., 0., 0.]; 2], None).unwrap();
        let twist = super::super::bezier(vec![vec![0.; 3]; 2], None).unwrap();
        for (constructor, points) in [closed_round_polyline, closed_transition_polyline]
            .into_iter()
            .flat_map(|constructor| {
                [planar, spatial]
                    .into_iter()
                    .map(move |points| (constructor, points))
            })
        {
            let path = constructor(&points, 2.).unwrap();
            let options = crate::progressive_sweep::Options {
                normal: [0., 0., 1.],
                orientation: crate::progressive_sweep::Orientation::RotationMinimizing,
                spacing: crate::progressive_sweep::Spacing::Parameter,
                initial_sections: 5,
                max_sections: 257,
                max_deviation: 0.01,
            };
            let result =
                crate::progressive_sweep::approximate(&profile, &path, &scale, &twist, options)
                    .unwrap();
            assert!(
                result.levels.last().unwrap().accepted,
                "{:?}",
                result.levels
            );
            assert!(result.levels.last().unwrap().closed_path);
            let sections =
                crate::progressive_sweep::Sweep::new(&profile, &path, &scale, &twist, options)
                    .unwrap()
                    .sections_at(17)
                    .unwrap();
            assert_eq!(
                sections.first().unwrap().control_points,
                sections.last().unwrap().control_points
            );
        }
    }
    #[test]
    fn closed_corners_match_seam_and_preserve_all_cyclic_fillets() {
        let points = [[0., 0., 0.], [10., 0., 0.], [10., 10., 0.], [0., 10., 0.]];
        for transition in [false, true] {
            let path = corner_polyline(&points, 2., transition, true).unwrap();
            assert_eq!(path.control_points.first(), path.control_points.last());
            let a = path.evaluate(0.).unwrap();
            let b = path.evaluate(1.).unwrap();
            for k in 0..3 {
                assert!((a.d1.as_ref().unwrap()[k] - b.d1.as_ref().unwrap()[k]).abs() < 1e-10);
                if transition {
                    assert!(a.d2.as_ref().unwrap()[k].abs() < 1e-8);
                    assert!(b.d2.as_ref().unwrap()[k].abs() < 1e-8);
                }
            }
            assert!(corner_polyline(&points, 5., transition, true).is_err());
        }
        let path = closed_round_polyline(&points, 2.).unwrap();
        let mut knots = path.knots.clone();
        knots.dedup();
        let centers = [[2., 2.], [8., 2.], [8., 8.], [2., 8.]];
        // Starts at exit of corner 3, then alternates retained line/corner.
        for (corner, center) in centers.iter().enumerate() {
            let a = knots[2 * corner + 1];
            let b = knots[2 * corner + 2];
            for i in 0..=10 {
                let p = path.evaluate(a + (b - a) * i as f64 / 10.).unwrap().point;
                assert!(((p[0] - center[0]).hypot(p[1] - center[1]) - 2.).abs() < 1e-12);
            }
        }
    }
    #[test]
    fn transition_matches_position_tangent_and_zero_curvature_at_spatial_joins() {
        let path = transition_polyline(
            &[[0., 0., 0.], [0., 0., 10.], [10., 0., 10.], [10., 10., 10.]],
            2.,
        )
        .unwrap();
        let mut seams = path.knots.clone();
        seams.dedup();
        for t in seams.into_iter().filter(|t| *t > 0. && *t < 1.) {
            let left = path.trim(0., t).unwrap().evaluate(t).unwrap();
            let right = path.trim(t, 1.).unwrap().evaluate(t).unwrap();
            for k in 0..3 {
                assert!((left.point[k] - right.point[k]).abs() < 1e-12);
                assert!(
                    (left.d1.as_ref().unwrap()[k] - right.d1.as_ref().unwrap()[k]).abs() < 1e-10
                );
                assert!(left.d2.as_ref().unwrap()[k].abs() < 1e-8);
                assert!(right.d2.as_ref().unwrap()[k].abs() < 1e-8);
            }
        }
        assert!(transition_polyline(&[[0., 0., 0.], [1., 0., 0.], [1., 1., 0.]], 2.).is_err());
    }

    #[test]
    fn transition_midpoint_and_derivative_follow_independent_quintic_formula() {
        let path = transition_polyline(&[[0., 0., 0.], [10., 0., 0.], [10., 10., 0.]], 2.).unwrap();
        let mut spans = path.knots.clone();
        spans.dedup();
        let a = spans[1];
        let b = spans[2];
        let e = path.evaluate((a + b) / 2.).unwrap();
        // Bernstein degree-five values at 1/2: [1,5,10,10,5,1]/32.
        let x = (8. + 5. * 8.8 + 10. * 9.6 + 10. * 10. + 5. * 10. + 10.) / 32.;
        let y = (10. * 0.4 + 5. * 1.2 + 2.) / 32.;
        assert!((e.point[0] - x).abs() < 1e-12 && (e.point[1] - y).abs() < 1e-12);
        assert!(e.d1.unwrap().iter().take(2).all(|v| *v > 0.));
        for i in 0..=100 {
            let e = path.evaluate(a + (b - a) * i as f64 / 100.).unwrap();
            assert!((8. - 1e-12..=10. + 1e-12).contains(&e.point[0]));
            assert!((-1e-12..=2. + 1e-12).contains(&e.point[1]));
        }
    }
    #[test]
    fn right_angle_has_exact_circle_radius_and_tangent_line_joins() {
        let path = round_polyline(&[[0., 0., 0.], [10., 0., 0.], [10., 10., 0.]], 2.).unwrap();
        let start = path.knots[3];
        let end = path.knots[5];
        for i in 0..=20 {
            let point = path
                .evaluate(start + (end - start) * i as f64 / 20.)
                .unwrap()
                .point;
            assert!(((point[0] - 8.).hypot(point[1] - 2.) - 2.).abs() < 1e-12);
        }
        for t in [start, end] {
            let left = path.trim(0., t).unwrap().evaluate(t).unwrap().d1.unwrap();
            let right = path.trim(t, 1.).unwrap().evaluate(t).unwrap().d1.unwrap();
            let a = length([left[0], left[1], left[2]]);
            let b = length([right[0], right[1], right[2]]);
            assert!(length(std::array::from_fn(|k| left[k] / a - right[k] / b)) < 1e-12);
        }
        assert_eq!(path.control_points.first().unwrap(), &vec![0., 0., 0.]);
        assert_eq!(path.control_points.last().unwrap(), &vec![10., 10., 0.]);
    }
    #[test]
    fn spatial_round_path_is_accepted_by_progressive_rmf() {
        let path = round_polyline(
            &[[0., 0., 0.], [0., 0., 10.], [10., 0., 10.], [10., 10., 10.]],
            2.,
        )
        .unwrap();
        let profile = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
        let law = super::super::bezier(vec![vec![1., 0., 0.]; 2], None).unwrap();
        let twist = super::super::bezier(vec![vec![0.; 3]; 2], None).unwrap();
        let result = crate::progressive_sweep::approximate(
            &profile,
            &path,
            &law,
            &twist,
            crate::progressive_sweep::Options {
                normal: [1., 0., 0.],
                orientation: crate::progressive_sweep::Orientation::RotationMinimizing,
                spacing: crate::progressive_sweep::Spacing::Parameter,
                initial_sections: 5,
                max_sections: 1025,
                max_deviation: 0.01,
            },
        )
        .unwrap();
        assert!(
            result.levels.last().unwrap().accepted,
            "{:?}",
            result.levels
        );
    }
    #[test]
    fn refuses_overlapping_fillets_reversals_and_degenerate_segments() {
        assert!(round_polyline(&[[0., 0., 0.], [1., 0., 0.], [1., 1., 0.]], 2.).is_err());
        assert!(round_polyline(&[[0., 0., 0.], [1., 0., 0.], [0., 0., 0.]], 0.1).is_err());
        assert!(round_polyline(&[[0.; 3], [0.; 3]], 1.).is_err());
        assert!(round_polyline(&[[0.; 3], [1., 0., 0.]], 0.).is_err());
    }
}
