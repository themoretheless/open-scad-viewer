//! Exact supporting-plane traces and complete trim-loop boundary coverage.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolynomialPlaneTrace {
    Plane,
    Boundary {
        parameter_axis: usize,
        at_end: bool,
        side: i8,
    },
}
pub fn polynomial_supporting_trace(
    s: &Surface,
    axis: usize,
    plane: &Q,
    b: &mut Budget,
) -> Result<Option<PolynomialPlaneTrace>> {
    s.validate()?;
    check(axis < 3, "Invalid supporting-plane axis")?;
    let (n, m) = (s.degree_u, s.degree_v);
    check(
        !s.periodic_u
            && !s.periodic_v
            && n <= 32
            && m <= 32
            && s.control_points.len() == n + 1
            && s.control_points[0].len() == m + 1
            && s.control_points.iter().flatten().all(|p| p.len() == 3)
            && s.weights.iter().flatten().all(|w| *w == s.weights[0][0]),
        "Supporting plane requires polynomial Bezier support",
    )?;
    for (knots, d) in [(&s.knots_u, n), (&s.knots_v, m)] {
        check(
            knots[..=d].iter().all(|x| *x == knots[d])
                && knots[d + 1..].iter().all(|x| *x == knots[d + 1]),
            "Supporting plane requires original clamped knots",
        )?;
    }
    let plane = b.keep(plane.clone())?;
    let net = s
        .control_points
        .iter()
        .map(|r| {
            r.iter()
                .map(|p| {
                    let x = b.authored(p[axis])?;
                    b.keep(x - &plane)
                })
                .collect()
        })
        .collect::<Result<Vec<Vec<Q>>>>()?;
    if net.iter().flatten().all(Zero::is_zero) {
        return Ok(Some(PolynomialPlaneTrace::Plane));
    }
    for parameter_axis in 0..2 {
        for at_end in [false, true] {
            let limit = if parameter_axis == 0 { n } else { m };
            let end = if at_end { limit } else { 0 };
            let mut sign = None;
            let mut valid = true;
            for (i, row) in net.iter().enumerate() {
                for (j, x) in row.iter().enumerate() {
                    if (if parameter_axis == 0 { i } else { j }) == end {
                        if !x.is_zero() {
                            valid = false;
                        }
                    } else if x.is_zero() {
                        valid = false;
                    } else {
                        let s = if x.is_positive() { 1 } else { -1 };
                        if sign.is_some_and(|v| v != s) {
                            valid = false;
                        }
                        sign = Some(s);
                    }
                }
            }
            if valid {
                if let Some(side) = sign {
                    return Ok(Some(PolynomialPlaneTrace::Boundary {
                        parameter_axis,
                        at_end,
                        side,
                    }));
                }
            }
        }
    }
    Ok(None)
}
/// Every point where a simple trim domain reaches a natural chart boundary is
/// on a designated covered edge. Domain simplicity/containment is a prerequisite.
/// Isolated endpoint touches are checked too; endpoint tests alone are unsound.
pub fn polynomial_trim_trace_covered(
    curves: &[Vec<[Q; 2]>],
    covered: &[bool],
    axis: usize,
    at_end: bool,
    value: &Q,
    b: &mut Budget,
) -> Result<bool> {
    check(
        axis < 2 && curves.len() == covered.len() && (3..=4096).contains(&curves.len()),
        "Invalid supporting-plane trim loop",
    )?;
    let value = b.keep(value.clone())?;
    let mut ranges = Vec::new();
    let mut touches = Vec::new();
    for (i, c) in curves.iter().enumerate() {
        check(
            (2..=33).contains(&c.len()) && c.last() == curves[(i + 1) % curves.len()].first(),
            "Supporting-plane trim loop is not exactly closed",
        )?;
        for p in c {
            for x in p {
                b.keep(x.clone())?;
            }
            if if at_end {
                p[axis] > value
            } else {
                p[axis] < value
            } {
                return Ok(false);
            }
        }
        let on = c.iter().all(|p| p[axis] == value);
        if on {
            if !covered[i] {
                return Ok(false);
            }
            let lo = c[0][1 - axis].clone();
            let hi = c.last().unwrap()[1 - axis].clone();
            let span = b.keep(&hi - &lo)?;
            if span.is_zero() {
                return Ok(false);
            }
            for (j, p) in c.iter().enumerate() {
                let expected = b.keep(&lo + &span * ratio(j as u64, (c.len() - 1) as u64))?;
                if p[1 - axis] != expected {
                    return Ok(false);
                }
            }
            ranges.push([lo.clone().min(hi.clone()), lo.max(hi)]);
        } else {
            if covered[i] {
                return Ok(false);
            }
            for p in [&c[0], c.last().unwrap()] {
                if p[axis] == value {
                    touches.push(p[1 - axis].clone());
                }
            }
        }
    }
    if ranges.is_empty() {
        return Ok(false);
    }
    Ok(touches
        .iter()
        .all(|x| ranges.iter().any(|r| r[0] <= *x && *x <= r[1])))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn source() -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 2.]],
                vec![vec![1., 0., 1.], vec![1., 1., 3.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn supporting_trace_is_whole_boundary_with_strict_off_boundary_sign() {
        let s = source();
        let mut b = Budget::new(100_000).unwrap();
        assert_eq!(
            polynomial_supporting_trace(&s, 0, &Q::zero(), &mut b).unwrap(),
            Some(PolynomialPlaneTrace::Boundary {
                parameter_axis: 0,
                at_end: false,
                side: 1
            })
        );
        assert_eq!(
            polynomial_supporting_trace(&s, 1, &Q::one(), &mut b).unwrap(),
            Some(PolynomialPlaneTrace::Boundary {
                parameter_axis: 1,
                at_end: true,
                side: -1
            })
        );
        assert!(
            polynomial_supporting_trace(&s, 2, &Q::zero(), &mut b)
                .unwrap()
                .is_none()
        );
        let mut plane = s.clone();
        for row in &mut plane.control_points {
            for p in row {
                p[0] = 3.;
            }
        }
        assert_eq!(
            polynomial_supporting_trace(&plane, 0, &ratio(3, 1), &mut b).unwrap(),
            Some(PolynomialPlaneTrace::Plane)
        );
        let mut mixed = s.clone();
        mixed.control_points[1][1][0] = -2f64.powi(-100);
        assert!(
            polynomial_supporting_trace(&mixed, 0, &Q::zero(), &mut b)
                .unwrap()
                .is_none()
        );
        assert!(
            polynomial_supporting_trace(&s, 0, &Q::zero(), &mut Budget::new(0).unwrap()).is_err()
        );
    }
    fn notch() -> Vec<Vec<[Q; 2]>> {
        let points = [
            [-1., 0.],
            [0., 0.],
            [0., 0.25],
            [2., 0.25],
            [2., 0.],
            [3., 0.],
            [3., 1.],
            [-1., 1.],
        ];
        (0..points.len())
            .map(|i| {
                [points[i], points[(i + 1) % points.len()]]
                    .map(|p| p.map(|x| Q::from_float(x).unwrap()))
                    .to_vec()
            })
            .collect()
    }
    #[test]
    fn retained_crease_trace_has_two_covered_intervals_including_end_touches() {
        let curves = notch();
        let mut covered = vec![false; 8];
        covered[0] = true;
        covered[4] = true;
        let mut b = Budget::new(100_000).unwrap();
        assert!(
            polynomial_trim_trace_covered(&curves, &covered, 1, false, &Q::zero(), &mut b).unwrap()
        );
        covered[4] = false;
        assert!(
            !polynomial_trim_trace_covered(&curves, &covered, 1, false, &Q::zero(), &mut b)
                .unwrap()
        );
        covered[4] = true;
        covered[1] = true;
        assert!(
            !polynomial_trim_trace_covered(&curves, &covered, 1, false, &Q::zero(), &mut b)
                .unwrap()
        );
        assert!(
            polynomial_trim_trace_covered(
                &curves,
                &covered,
                1,
                false,
                &Q::zero(),
                &mut Budget::new(0).unwrap()
            )
            .is_err()
        );
    }
    #[test]
    fn isolated_boundary_touch_without_a_covered_edge_is_not_admitted() {
        let points = [
            [Q::zero(), Q::zero()],
            [Q::one(), Q::one()],
            [-Q::one(), Q::one()],
        ];
        let curves = (0..3)
            .map(|i| vec![points[i].clone(), points[(i + 1) % 3].clone()])
            .collect::<Vec<_>>();
        assert!(
            !polynomial_trim_trace_covered(
                &curves,
                &[false; 3],
                1,
                false,
                &Q::zero(),
                &mut Budget::new(100_000).unwrap()
            )
            .unwrap()
        );
    }
}
