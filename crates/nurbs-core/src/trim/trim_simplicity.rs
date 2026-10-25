//! Sufficient simplicity evidence for a closed rational NURBS UV loop.
//! Failure of a sufficient test is unproven, never evidence of an intersection.
use crate::{Result, check, curve::Curve, curve_distance, curve_jets, trim_domain};
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

// Positive rational Bernstein coefficients prove a strict open-curve side
// even when additional endpoint controls lie on the supporting line.
fn adjacent_supported(a:&Curve,b:&Curve,join:&[f64],second_join:bool,cells:&mut usize,max_cells:usize)->bool {
    use cad_predicates::{AuthoredScalar,Limits,Outcome,PredicateContext,Sign,SourceArena,ToleranceContext};
    if [a,b].iter().any(|c|!bezier(c) || c.degree==0 || c.degree>32 || c.weights.iter().any(|w|*w<=0.)) {return false;}
    let side=|anchor:&[f64],point:&[f64],cells:&mut usize|->Option<Sign> {
        if point==join {return Some(Sign::Zero);}
        if *cells==max_cells {return None;}*cells+=1;
        let raw=[join[0],join[1],anchor[0],anchor[1],point[0],point[1]];
        let arena=SourceArena::authored("original-adjacent-support-line",1,raw.into_iter().map(|v|AuthoredScalar::Binary64Bits(v.to_bits())).collect()).ok()?;
        let tolerance=ToleranceContext::default_valid();
        // One charged cell permits at most 256 exact predicate work units.
        let mut ctx=PredicateContext::new(&arena,&tolerance,Limits{max_work:256,..Limits::default()},None);
        match cad_predicates::orient2d(&mut ctx,[arena.leaf(0).ok()?,arena.leaf(1).ok()?],
            [arena.leaf(2).ok()?,arena.leaf(3).ok()?],[arena.leaf(4).ok()?,arena.leaf(5).ok()?]).ok()?.outcome {
            Outcome::Sign(s)=>Some(s),Outcome::Indeterminate(_)=>None,
        }
    };
    for (strict,closed) in [(a,b),(b,a)] {
        let endpoint=if strict.control_points[0].as_slice()==join {strict.control_points.last().unwrap()}
            else if strict.control_points.last().unwrap().as_slice()==join {&strict.control_points[0]}else{return false;};
        for anchor in closed.control_points.iter().filter(|p|p.as_slice()!=join) {
            let Some(sa)=strict.control_points.iter().map(|p|side(anchor,p,cells)).collect::<Option<Vec<_>>>() else {return false;};
            let Some(sb)=closed.control_points.iter().map(|p|side(anchor,p,cells)).collect::<Option<Vec<_>>>() else {return false;};
            for sign in [Sign::Positive,Sign::Negative] {
                let opposite=if sign==Sign::Positive {Sign::Negative}else{Sign::Positive};
                if !sa.iter().all(|s|*s==Sign::Zero || *s==sign) || !sa.contains(&sign)
                    || !sb.iter().all(|s|*s==Sign::Zero || *s==opposite) {continue;}
                let index=if strict.control_points[0].as_slice()==join {sa.len()-1}else{0};
                let far_is_shared=second_join && [closed.control_points.first().unwrap(),closed.control_points.last().unwrap()].contains(&endpoint);
                let far_is_separated=(0..2).any(|axis|closed.control_points.iter().all(|p|endpoint[axis]<p[axis])
                    || closed.control_points.iter().all(|p|endpoint[axis]>p[axis]));
                if sa[index]==sign || far_is_shared || far_is_separated {return true;}
            }
        }
    }
    false
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
    let b_start = &b.control_points[0];
    let b_end = b.control_points.last().unwrap();
    let a_start = &a.control_points[0];
    let a_end = a.control_points.last().unwrap();
    let s0 = a_start == b_start || a_start == b_end;
    let s1 = a_end == b_start || a_end == b_end;
    Ok(match (s0, s1) {
        (true, false) => adjacent_separated(a, b, a_start),
        (false, true) => adjacent_separated(a, b, a_end),
        (true, true) => lens_separated(a, b),
        (false, false) => false,
    })
}

#[cfg(test)]
#[path = "trim_simplicity/tests/tangent_join_tests_integration.rs"]
mod tangent_join_tests;
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
    let mut cells = 0;
    let injective = curves.iter().map(|c| {
        if injective(c) { Ok(true) } else { monotone_spans(c, &mut cells, max_cells) }
    }).collect::<Result<Vec<_>>>()?;
    let n = curves.len();
    let total_pairs = n * (n - 1) / 2;
    let mut out = Report {
        proven_simple: false,
        exact_joins,
        injective,
        pairs: Vec::with_capacity(total_pairs.min(max_pairs)),
        total_pairs,
        cells,
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
                let join = curves[b].control_points[0].as_slice();
                adjacent_supported(&curves[a], &curves[b], join, true, &mut out.cells, max_cells)
                    || (out.cells < max_cells
                        && bezier(&curves[a])
                        && bezier(&curves[b])
                        && lens_separated(&curves[a], &curves[b])
                        && {
                            out.cells += 1;
                            true
                        })
            } else if adjacent {
                let join = if b == a + 1 {
                    curves[b].control_points[0].as_slice()
                } else {
                    curves[0].control_points[0].as_slice()
                };
                (n > 2 && adjacent_separated(&curves[a], &curves[b], join))
                    || adjacent_supported(&curves[a], &curves[b], join, false, &mut out.cells, max_cells)
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
#[path = "trim_simplicity/tests/tests_integration.rs"]
mod tests;

fn monotone_spans(c: &Curve, cells: &mut usize, max_cells: usize) -> Result<bool> {
    if c.periodic {
        return Ok(false);
    }
    let spans = (c.degree..c.control_points.len())
        .filter(|&i| c.knots[i] < c.knots[i + 1])
        .collect::<Vec<_>>();
    for axis in 0..2 {
        for positive in [true, false] {
            let mut pending = spans
                .iter()
                .map(|&i| (i, [c.knots[i], c.knots[i + 1]], 0))
                .collect::<Vec<_>>();
            let mut proven = true;
            while let Some((span, domain, depth)) = pending.pop() {
                if *cells == max_cells {
                    return Ok(false);
                }
                *cells += 1;
                let derivative = curve_jets::enclose(c, span, domain)?[1][axis];
                if if positive {
                    derivative.lo > 0.
                } else {
                    derivative.hi < 0.
                } {
                    continue;
                }
                if if positive {
                    derivative.hi <= 0.
                } else {
                    derivative.lo >= 0.
                } {
                    proven = false;
                    break;
                }
                let middle = domain[0] * 0.5 + domain[1] * 0.5;
                if depth == 16 || middle <= domain[0] || middle >= domain[1] {
                    proven = false;
                    break;
                }
                pending.push((span, [domain[0], middle], depth + 1));
                pending.push((span, [middle, domain[1]], depth + 1));
            }
            if proven {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

#[cfg(test)]
#[path = "trim_simplicity/tests/multispan_regressions_integration.rs"]
mod multispan_regressions;
