//! Sufficient simplicity evidence for a closed rational NURBS UV loop.
//! Failure of a sufficient test is unproven, never evidence of an intersection.
use crate::{Result, check, curve::Curve, curve_distance, trim_domain};
pub struct Pair {
    pub curves: [usize; 2],
    pub proven: bool,
}
pub struct Report {
    pub proven_simple: bool,
    pub exact_joins: Option<bool>,
    pub injective: Vec<bool>,
    pub pairs: Vec<Pair>,
    pub total_pairs: usize,
    pub cells: usize,
}
/// Failure of a sufficient proof is not proof of self-intersection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    ProvenSimple,
    ClosureMismatch,
    ClosureUnproven,
    InjectivityUnproven,
    PairBudgetExhausted,
    SeparationUnproven,
}
impl Report {
    pub fn outcome(&self) -> Outcome {
        if self.proven_simple {
            Outcome::ProvenSimple
        } else if self.exact_joins == Some(false) {
            Outcome::ClosureMismatch
        } else if self.exact_joins.is_none() {
            Outcome::ClosureUnproven
        } else if self.injective.iter().any(|v| !v) {
            Outcome::InjectivityUnproven
        } else if self.pairs.len() < self.total_pairs {
            Outcome::PairBudgetExhausted
        } else {
            Outcome::SeparationUnproven
        }
    }
}

fn bezier(c: &Curve) -> bool {
    let n = c.control_points.len();
    !c.periodic
        && n == c.degree + 1
        && c.knots[..=c.degree].iter().all(|k| *k == c.knots[c.degree])
        && c.knots[n..].iter().all(|k| *k == c.knots[n])
}
// Positive-weight rational B-splines stay in the convex hull of their actual
// control points. Strict coordinate ordering proves separation without a
// distance subdivision, including multi-span curves. No rounded subtraction.
fn control_boxes_separated(a: &Curve, b: &Curve) -> bool {
    (0..2).any(|k| {
        let amin = a
            .control_points
            .iter()
            .map(|p| p[k])
            .fold(f64::INFINITY, f64::min);
        let amax = a
            .control_points
            .iter()
            .map(|p| p[k])
            .fold(f64::NEG_INFINITY, f64::max);
        let bmin = b
            .control_points
            .iter()
            .map(|p| p[k])
            .fold(f64::INFINITY, f64::min);
        let bmax = b
            .control_points
            .iter()
            .map(|p| p[k])
            .fold(f64::NEG_INFINITY, f64::max);
        amax < bmin || bmax < amin
    })
}
fn span_count(c: &Curve) -> usize {
    c.knots[c.degree..=c.control_points.len()]
        .windows(2)
        .filter(|w| w[0] < w[1])
        .count()
}
fn projection_delta(
    a: &[f64],
    b: &[f64],
    direction: [f64; 2],
) -> Option<crate::distance_bounds::Interval> {
    use crate::distance_bounds::Interval;
    if a == b {
        return Some(Interval::point(0.));
    }
    let mut result = Interval::point(0.);
    for k in 0..2 {
        result = result
            .add(
                Interval::point(a[k])
                    .sub(Interval::point(b[k]))
                    .ok()?
                    .mul(Interval::point(direction[k]))
                    .ok()?,
            )
            .ok()?;
    }
    Some(result)
}
fn monotone(c: &Curve) -> bool {
    // For rational Bernstein coordinates, the derivative numerator is a sum
    // over i<j of (j-i)(x_j-x_i) w_i w_j B_i B_j / (t(1-t)). All factors
    // except the coordinate differences are positive inside (0,1). Ordered
    // controls and distinct endpoints therefore imply strict monotonicity.
    (0..2).any(|k| {
        let p = &c.control_points;
        (p[0][k] < p[p.len() - 1][k] && p.windows(2).all(|w| w[0][k] <= w[1][k]))
            || (p[0][k] > p[p.len() - 1][k] && p.windows(2).all(|w| w[0][k] >= w[1][k]))
    }) || {
        let a = &c.control_points[0];
        let b = c.control_points.last().unwrap();
        let direction = [b[0] - a[0], b[1] - a[1]];
        projection_delta(b, a, direction).is_some_and(|r| r.lo > 0.)
            && c.control_points
                .windows(2)
                .all(|w| projection_delta(&w[1], &w[0], direction).is_some_and(|r| r.lo >= 0.))
    }
}
// Two-arc lens: both endpoints are shared. A separating chord proves that
// the open interior of one positive-weight Bezier arc stays strictly on one
// side, while the other arc stays in the opposite closed half-plane. Together
// with the independent injectivity proof this excludes all extra contacts.
fn lens_separated(a: &Curve, b: &Curve) -> bool {
    use crate::distance_bounds::Interval as I;
    let start = &a.control_points[0];
    let end = a.control_points.last().unwrap();
    let side = |p: &Vec<f64>| -> Result<I> {
        if p == start || p == end {
            return Ok(I::point(0.));
        }
        let dx = I::point(end[0]).sub(I::point(start[0]))?;
        let dy = I::point(end[1]).sub(I::point(start[1]))?;
        let x = I::point(p[0]).sub(I::point(start[0]))?;
        let y = I::point(p[1]).sub(I::point(start[1]))?;
        dx.mul(y)?.sub(dy.mul(x)?)
    };
    let Ok(sa) = a
        .control_points
        .iter()
        .map(side)
        .collect::<Result<Vec<_>>>()
    else {
        return false;
    };
    let Ok(sb) = b
        .control_points
        .iter()
        .map(side)
        .collect::<Result<Vec<_>>>()
    else {
        return false;
    };
    let positive = |v: &[I]| v.iter().all(|x| x.lo >= 0.);
    let negative = |v: &[I]| v.iter().all(|x| x.hi <= 0.);
    (positive(&sa) && sa.iter().any(|x| x.lo > 0.) && negative(&sb))
        || (negative(&sa) && sa.iter().any(|x| x.hi < 0.) && positive(&sb))
        || (positive(&sb) && sb.iter().any(|x| x.lo > 0.) && negative(&sa))
        || (negative(&sb) && sb.iter().any(|x| x.hi < 0.) && positive(&sa))
}
// Positive rational B-spline bases preserve the order of scalar poles.
// Strict progress on every nonempty local knot block excludes flat spans;
// interior multiplicity <= degree preserves continuity between those spans.
// These checks concern ORIGINAL coefficients, not rounded decomposition pieces.
fn injective(c: &Curve) -> bool {
    if bezier(c) {
        return monotone(c);
    }
    let n = c.control_points.len();
    let d = c.domain();
    if c.periodic
        || !c.knots[..=c.degree].iter().all(|&t| t == d[0])
        || !c.knots[n..].iter().all(|&t| t == d[1])
    {
        return false;
    }
    let mut at = 0;
    while at < c.knots.len() {
        let value = c.knots[at];
        let mut end = at + 1;
        while end < c.knots.len() && c.knots[end] == value {
            end += 1;
        }
        if d[0] < value && value < d[1] && end - at > c.degree {
            return false;
        }
        at = end;
    }
    (0..2).any(|k| {
        let increasing = c.control_points[0][k] < c.control_points[n - 1][k];
        let decreasing = c.control_points[0][k] > c.control_points[n - 1][k];
        (increasing || decreasing)
            && c.control_points.windows(2).all(|p| {
                if increasing {
                    p[0][k] <= p[1][k]
                } else {
                    p[0][k] >= p[1][k]
                }
            })
            && (c.degree..n)
                .filter(|&i| c.knots[i] < c.knots[i + 1])
                .all(|i| {
                    if increasing {
                        c.control_points[i - c.degree][k] < c.control_points[i][k]
                    } else {
                        c.control_points[i - c.degree][k] > c.control_points[i][k]
                    }
                })
    })
}
fn control_cones_separated(a: &Curve, b: &Curve, join: &[f64]) -> bool {
    // Directions are proposals only. Admit a separator only when outward
    // interval signs separate EVERY non-join pole of both original curves.
    // Positive rational Bernstein weights then separate their open images.
    // Normalizing proposals avoids dependence on tangent control lengths.
    for pa in a
        .control_points
        .iter()
        .filter(|p| p.as_slice() != join)
        .take(16)
    {
        for pb in b
            .control_points
            .iter()
            .filter(|p| p.as_slice() != join)
            .take(16)
        {
            let va = [pa[0] - join[0], pa[1] - join[1]];
            let vb = [pb[0] - join[0], pb[1] - join[1]];
            let la = va[0].hypot(va[1]);
            let lb = vb[0].hypot(vb[1]);
            if !(la > 0. && lb > 0. && la.is_finite() && lb.is_finite()) {
                continue;
            }
            let direction = [va[0] / la - vb[0] / lb, va[1] / la - vb[1] / lb];
            let sides = |c: &Curve, positive: bool| {
                c.control_points
                    .iter()
                    .filter(|p| p.as_slice() != join)
                    .all(|p| {
                        projection_delta(p, join, direction)
                            .is_some_and(|r| if positive { r.lo > 0. } else { r.hi < 0. })
                    })
            };
            if sides(a, true) && sides(b, false) {
                return true;
            }
        }
    }
    false
}
fn adjacent_separated(a: &Curve, b: &Curve, join: &[f64]) -> bool {
    // One open curve lies strictly on one side of a coordinate line through
    // the common endpoint; the other lies on the opposite closed half-plane.
    // Strict positive Bernstein factors exclude every interior return to it.
    (0..2).any(|k| {
        [1., -1.].iter().any(|&sign| {
            let side = |p: &Vec<f64>| {
                if sign > 0. {
                    p[k] > join[k]
                } else {
                    p[k] < join[k]
                }
            };
            let opposite = |p: &Vec<f64>| {
                if sign > 0. {
                    p[k] <= join[k]
                } else {
                    p[k] >= join[k]
                }
            };
            let strict = |c: &Curve| {
                let strict_poles = c
                    .control_points
                    .iter()
                    .filter(|p| p.as_slice() != join)
                    .all(side)
                    && c.control_points.iter().any(side);
                // Every positive-weight Bernstein basis is strictly positive
                // in the open Bezier interval. Weak one-sided poles and one
                // strict pole therefore exclude an interior return, including
                // tangential joins whose adjacent handle lies on the line.
                // A general B-spline does not have this global positivity.
                strict_poles
                    || (bezier(c)
                        && c.control_points.iter().all(|p| {
                            if sign > 0. {
                                p[k] >= join[k]
                            } else {
                                p[k] <= join[k]
                            }
                        })
                        && c.control_points.iter().any(side))
            };
            (strict(a) && b.control_points.iter().all(opposite))
                || (strict(b) && a.control_points.iter().all(opposite))
        })
    }) || control_cones_separated(a, b, join)
        || {
            // Arbitrary separating direction, admitted only by outward interval
            // signs. Handles rotated rational arcs without snapping coefficients.
            let (a_control, b_control) = if a.control_points.last().unwrap().as_slice() == join {
                (
                    &a.control_points[a.control_points.len() - 2],
                    &b.control_points[1],
                )
            } else {
                (
                    &b.control_points[b.control_points.len() - 2],
                    &a.control_points[1],
                )
            };
            let direction = [b_control[0] - a_control[0], b_control[1] - a_control[1]];
            let sides = |c: &Curve, positive: bool| {
                c.control_points
                    .iter()
                    .filter(|p| p.as_slice() != join)
                    .all(|p| {
                        projection_delta(p, join, direction)
                            .is_some_and(|r| if positive { r.lo > 0. } else { r.hi < 0. })
                    })
            };
            (sides(a, true) && sides(b, false)) || (sides(a, false) && sides(b, true))
        }
}

/// Sufficient whole-curve proof that two authored planar Bezier curves meet
/// only at their exactly shared clamped endpoints. False means unproven.
pub fn prove_endpoint_contacts_only(a: &Curve, b: &Curve) -> Result<bool> {
    a.validate()?;
    b.validate()?;
    if !bezier(a)
        || !bezier(b)
        || a.control_points[0].len() != 2
        || b.control_points[0].len() != 2
        || !injective(a)
        || !injective(b)
    {
        return Ok(false);
    }
    let shared = [&a.control_points[0], a.control_points.last().unwrap()]
        .into_iter()
        .filter(|p| *p == &b.control_points[0] || *p == b.control_points.last().unwrap())
        .collect::<Vec<_>>();
    Ok(match shared.as_slice() {
        [point] => adjacent_separated(a, b, point),
        [_, _] => lens_separated(a, b),
        _ => false,
    })
}

#[cfg(test)]
mod tangent_join_tests {
    use super::*;
    #[test]
    fn positive_bezier_handles_admit_tangency_but_not_crossings_or_flat_spline_returns() {
        let line = Curve::from_polyline(vec![vec![0., 0.], vec![10., 0.]]).unwrap();
        let arc = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![3., 3.], vec![3., 0.], vec![0., 0.]],
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
            periodic: false,
        };
        assert!(adjacent_separated(&line, &arc, &[0., 0.]));
        assert!(prove_endpoint_contacts_only(&line, &arc).unwrap());
        let mut crossing = arc.clone();
        crossing.control_points[1][1] = -3.;
        assert!(!adjacent_separated(&line, &crossing, &[0., 0.]));
        assert!(!prove_endpoint_contacts_only(&line, &crossing).unwrap());
        let spline = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
            control_points: vec![
                vec![3., 3.],
                vec![3., 0.],
                vec![2., 0.],
                vec![1., 0.],
                vec![0., 0.],
            ],
            weights: vec![1.; 5],
            periodic: false,
        };
        spline.validate().unwrap();
        assert!(!adjacent_separated(&line, &spline, &[0., 0.]));
        assert!(!prove_endpoint_contacts_only(&line, &spline).unwrap());
    }
}
pub fn inspect(
    curves: &[Curve],
    tolerance_uv: f64,
    max_pairs: usize,
    max_cells: usize,
) -> Result<Report> {
    check(
        (2..=256).contains(&curves.len())
            && (1..=100000).contains(&max_pairs)
            && (1..=100000).contains(&max_cells)
            && tolerance_uv.is_finite()
            && tolerance_uv > 0.,
        "Trim simplicity requires 2..256 curves, positive tolerance and bounded work",
    )?;
    let exact_joins = trim_domain::exact_loop_joins(curves)?;
    let injective = curves.iter().map(injective).collect::<Vec<_>>();
    let n = curves.len();
    let mut out = Report {
        proven_simple: false,
        exact_joins,
        injective,
        pairs: Vec::new(),
        total_pairs: n * (n - 1) / 2,
        cells: 0,
    };
    if exact_joins != Some(true) || out.injective.iter().any(|v| !*v) {
        return Ok(out);
    }
    for a in 0..n {
        for b in a + 1..n {
            if out.pairs.len() == max_pairs {
                return Ok(out);
            }
            let adjacent = b == a + 1 || (a == 0 && b == n - 1);
            let proven = if n == 2 {
                bezier(&curves[a]) && bezier(&curves[b]) && lens_separated(&curves[a], &curves[b])
            } else if adjacent {
                let join = if b == a + 1 {
                    curves[b].control_points[0].as_slice()
                } else {
                    curves[0].control_points[0].as_slice()
                };
                n > 2 && adjacent_separated(&curves[a], &curves[b], join)
            } else if out.cells < max_cells && control_boxes_separated(&curves[a], &curves[b]) {
                out.cells += 1;
                true
            } else if out.cells < max_cells {
                let remaining = max_cells - out.cells;
                let initial = span_count(&curves[a]) * span_count(&curves[b]);
                let budget = (remaining / (out.total_pairs - out.pairs.len()))
                    .max(initial)
                    .min(remaining);
                if budget < initial {
                    false
                } else {
                    let r = curve_distance::distance(&curves[a], &curves[b], tolerance_uv, budget)?;
                    out.cells += r.cells;
                    r.distance_interval_mm[0] > 0.
                }
            } else {
                false
            };
            out.pairs.push(Pair {
                curves: [a, b],
                proven,
            });
        }
    }
    out.proven_simple = out.pairs.iter().all(|p| p.proven);
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn multispan_hull_separation_does_not_require_impossible_fair_share_cells() {
        let points = [[0., 0.], [2., 0.], [2., 2.], [0., 2.]];
        let curves = (0..4)
            .map(|i| {
                let a = points[i];
                let b = points[(i + 1) % 4];
                Curve {
                    degree: 1,
                    knots: vec![0., 0., 0.5, 1., 1.],
                    control_points: vec![
                        a.to_vec(),
                        vec![(a[0] + b[0]) / 2., (a[1] + b[1]) / 2.],
                        b.to_vec(),
                    ],
                    weights: vec![1., 1., 1.],
                    periodic: false,
                }
            })
            .collect::<Vec<_>>();
        let r = inspect(&curves, 1e-6, 6, 2).unwrap();
        assert!(r.proven_simple);
        assert_eq!(r.cells, 2);
        let mut rotated = curves.clone();
        for c in &mut rotated {
            for p in &mut c.control_points {
                let (x, y) = (p[0], p[1]);
                p[0] = x - y;
                p[1] = x + y;
            }
        }
        assert!(!inspect(&rotated, 1e-6, 6, 1).unwrap().proven_simple);
        assert!(inspect(&rotated, 1e-6, 6, 100).unwrap().proven_simple);
        let mut crossing = curves.clone();
        crossing[2].control_points[1] = vec![1., -1.];
        assert!(!inspect(&crossing, 1e-6, 6, 1).unwrap().proven_simple);
    }
    #[test]
    fn normalized_separators_check_whole_rational_control_cones() {
        let a = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![0.25, 0.5], vec![0.5, 0.25], vec![1., 1.]],
            weights: vec![1., 0.5, 0.5, 1.],
            periodic: false,
        };
        let mut b = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![0.05, 0.5], vec![0., 1.]],
            weights: vec![1., 0.7, 1.],
            periodic: false,
        };
        assert!(control_cones_separated(&a, &b, &[0., 0.]));
        assert!(control_cones_separated(&b, &a, &[0., 0.]));
        // Same endpoint and similar tangent are insufficient: a later pole
        // overlaps the other control cone and must prevent this certificate.
        b.control_points[1] = vec![0.9, 0.5];
        assert!(!control_cones_separated(&a, &b, &[0., 0.]));
        assert!(!control_cones_separated(&a, &a, &[0., 0.]));
    }
    fn polygon(p: &[[f64; 2]]) -> Vec<Curve> {
        (0..p.len())
            .map(|i| {
                Curve::from_polyline(vec![p[i].to_vec(), p[(i + 1) % p.len()].to_vec()]).unwrap()
            })
            .collect()
    }
    #[test]
    fn square_bowtie_and_incomplete_work_are_distinct() {
        let square = polygon(&[[0., 0.], [1., 0.], [1., 1.], [0., 1.]]);
        assert!(inspect(&square, 1e-8, 100, 1000).unwrap().proven_simple);
        let crossed = polygon(&[[0., 0.], [1., 1.], [0., 1.], [1., 0.]]);
        assert!(!inspect(&crossed, 1e-8, 100, 1000).unwrap().proven_simple);
        let r = inspect(&square, 1e-8, 1, 1000).unwrap();
        assert!(!r.proven_simple);
        assert_eq!(r.pairs.len(), 1);
        assert_eq!(r.total_pairs, 6);
    }
    #[test]
    fn coincident_edges_and_small_work_never_bypass_pair_checks() {
        let retraced = polygon(&[[0., 0.], [1., 0.], [0., 0.], [0., 1.]]);
        assert!(!inspect(&retraced, 1e-8, 100, 1000).unwrap().proven_simple);
        let square = polygon(&[[0., 0.], [1., 0.], [1., 1.], [0., 1.]]);
        let r = inspect(&square, 1e-8, 100, 1).unwrap();
        assert!(!r.proven_simple);
        assert!(r.cells <= 1);
        assert_eq!(r.pairs.len(), 6);
        let mut gap = square.clone();
        gap[1].control_points[0][1] = 1e-12;
        let r = inspect(&gap, 1e-6, 100, 1000).unwrap();
        assert!(!r.proven_simple);
        assert_eq!(r.exact_joins, Some(false));
    }
    #[test]
    fn rational_quarter_arcs_form_a_simple_loop() {
        let points = [
            [[1., 0.], [1., 1.], [0., 1.]],
            [[0., 1.], [-1., 1.], [-1., 0.]],
            [[-1., 0.], [-1., -1.], [0., -1.]],
            [[0., -1.], [1., -1.], [1., 0.]],
        ];
        let curves = points
            .iter()
            .map(|p| Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: p.iter().map(|v| v.to_vec()).collect(),
                weights: vec![1., 0.5_f64.sqrt(), 1.],
                periodic: false,
            })
            .collect::<Vec<_>>();
        let before = format!("{curves:?}");
        let r = inspect(&curves, 1e-8, 100, 10000).unwrap();
        assert!(r.proven_simple);
        assert!(r.cells <= 10000);
        assert_eq!(format!("{curves:?}"), before);
    }
    #[test]
    fn rotated_rational_arcs_use_interval_separating_directions() {
        let points = [
            [[1., 0.], [1., 1.], [0., 1.]],
            [[0., 1.], [-1., 1.], [-1., 0.]],
            [[-1., 0.], [-1., -1.], [0., -1.]],
            [[0., -1.], [1., -1.], [1., 0.]],
        ];
        for angle in [0.13_f64, 0.61, 1.17, 2.31] {
            let (sin, cos) = angle.sin_cos();
            let curves = points
                .iter()
                .map(|points| Curve {
                    degree: 2,
                    knots: vec![0., 0., 0., 1., 1., 1.],
                    control_points: points
                        .iter()
                        .map(|p| vec![cos * p[0] - sin * p[1], sin * p[0] + cos * p[1]])
                        .collect(),
                    weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
                    periodic: false,
                })
                .collect::<Vec<_>>();
            assert!(inspect(&curves, 1e-10, 100, 10000).unwrap().proven_simple);
        }
    }
}
