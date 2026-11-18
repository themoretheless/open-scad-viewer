//! Resolve tied tangent continuations by certified exits from one common box.
//! Pair intersections have already split every branch. Distinct exits on a
//! common box therefore preserve their local circular order at the junction.
use super::{Curve, Result, refused};
use nurbs_core::interval_eval::{Interval as I, evaluate_interval};
pub(super) fn direction(c: &Curve) -> Option<[f64; 2]> {
    let p = &c.control_points[0];
    c.control_points.iter().skip(1).find_map(|q| {
        let d = [q[0] - p[0], q[1] - p[1]];
        (d[0] != 0. || d[1] != 0.).then_some(d)
    })
}
fn evaluate(c: &Curve, t: I, work: &mut usize) -> Result<Vec<I>> {
    *work += 1;
    if *work > 100000 {
        return Err(refused("Boundary departure ordering budget exceeded"));
    }
    evaluate_interval(c, t)
}
fn exit(c: &Curve, sides: [f64; 2], work: &mut usize) -> Result<Option<[I; 2]>> {
    let Some(d) = direction(c) else {
        return Ok(None);
    };
    let preferred = usize::from(d[1].abs() > d[0].abs());
    let origin = &c.control_points[0];
    for axis in [preferred, 1 - preferred] {
        let other = 1 - axis;
        if !nurbs_core::intersection::planar_scalar::monotone(c, axis) {
            continue;
        }
        let end = c.control_points.last().unwrap()[axis];
        let positive = end > origin[axis];
        let target = origin[axis] + if positive { sides[axis] } else { -sides[axis] };
        if !(origin[axis].min(end) < target && target < origin[axis].max(end)) {
            continue;
        }
        let [start, mut hi] = c.domain();
        let mut lo = start;
        for _ in 0..90 {
            let old = [lo, hi];
            for fraction in [1. / 3., 2. / 3.] {
                let t = lo + (hi - lo) * fraction;
                if t <= lo || t >= hi {
                    continue;
                }
                let value = evaluate(c, I::point(t), work)?[axis];
                if (positive && value.hi < target) || (!positive && value.lo > target) {
                    lo = t;
                } else if (positive && value.lo > target) || (!positive && value.hi < target) {
                    hi = t;
                }
            }
            if old == [lo, hi] {
                break;
            }
        }
        let point = evaluate(c, I::new(lo, hi)?, work)?;
        let prefix = evaluate(c, I::new(start, hi)?, work)?[other].sub(I::point(origin[other]))?;
        let lower = I::point(origin[other])
            .sub(I::point(origin[other] - sides[other]))?
            .lo;
        let upper = I::point(origin[other] + sides[other])
            .sub(I::point(origin[other]))?
            .lo;
        // Both rectangle sides are shared by every incident branch. Strict
        // prefix bounds prove this is the first boundary exit, including when
        // the initial tangent points toward a square corner.
        if prefix.lo <= -lower || prefix.hi >= upper {
            continue;
        }
        return Ok(Some([
            point[0].sub(I::point(origin[0]))?,
            point[1].sub(I::point(origin[1]))?,
        ]));
    }
    Ok(None)
}

fn cross(a: [I; 2], b: [I; 2]) -> Result<I> {
    a[0].mul(b[1])?.sub(a[1].mul(b[0])?)
}
fn dot(a: [I; 2], b: [I; 2]) -> Result<I> {
    a[0].mul(b[0])?.add(a[1].mul(b[1])?)
}
/// Strict clockwise precedence, without transcendental angle rounding. Rear
/// half-plane comparisons also handle an exactly opposite incoming ray.
fn clockwise_before(back: [I; 2], a: [I; 2], b: [I; 2]) -> Result<Option<bool>> {
    let ca = cross(back, a)?;
    let cb = cross(back, b)?;
    if ca.hi < 0. && cb.lo > 0. {
        return Ok(Some(true));
    }
    if ca.lo > 0. && cb.hi < 0. {
        return Ok(Some(false));
    }
    let da = dot(back, a)?;
    let db = dot(back, b)?;
    if (ca.hi < 0. && cb.hi < 0.) || (ca.lo > 0. && cb.lo > 0.) || (da.hi < 0. && db.hi < 0.) {
        let order = cross(a, b)?;
        return Ok(if order.hi < 0. {
            Some(true)
        } else if order.lo > 0. {
            Some(false)
        } else {
            None
        });
    }
    // Overlapping open half-planes give sufficient one-sided comparisons
    // even when one ray is exactly normal to the incoming ray.
    let order = cross(a, b)?;
    if order.hi < 0. && ((ca.hi < 0. && db.hi < 0.) || (da.hi < 0. && cb.lo > 0.)) {
        return Ok(Some(true));
    }
    if order.lo > 0. && ((da.hi < 0. && cb.hi < 0.) || (ca.lo > 0. && db.hi < 0.)) {
        return Ok(Some(false));
    }
    if da.hi < 0. && db.lo > 0. {
        return Ok(if cb.hi < 0. {
            Some(false)
        } else if cb.lo > 0. {
            Some(true)
        } else {
            None
        });
    }
    if da.lo > 0. && db.hi < 0. {
        return Ok(if ca.hi < 0. {
            Some(true)
        } else if ca.lo > 0. {
            Some(false)
        } else {
            None
        });
    }
    Ok(None)
}

pub(super) fn continuation(
    incoming: &Curve,
    outgoing: &[(usize, &Curve)],
    work: &mut usize,
) -> Result<Option<usize>> {
    let back = incoming.reverse()?;
    let mut radius = f64::INFINITY;
    for c in std::iter::once(&back).chain(outgoing.iter().map(|(_, c)| *c)) {
        let end = c.control_points.last().unwrap();
        let origin = &c.control_points[0];
        radius = radius.min((end[0] - origin[0]).abs().max((end[1] - origin[1]).abs()));
    }
    for exponent in 3..22 {
        let delta = radius * 2_f64.powi(-exponent);
        if !delta.is_finite() || delta <= 0. {
            break;
        }
        // Unequal aspect ratios avoid mistaking diagonal rays for an
        // unresolved corner hit. Each attempt uses one common rectangle.
        for aspect in [1., 1.5, 0.5, 2.] {
            let sides = [delta, delta * aspect];
            let Some(back_point) = exit(&back, sides, work)? else {
                continue;
            };
            let mut exits = Vec::new();
            for (id, c) in outgoing {
                let Some(point) = exit(c, sides, work)? else {
                    exits.clear();
                    break;
                };
                exits.push((*id, point));
            }
            if exits.len() != outgoing.len() {
                continue;
            }
            for (i, (id, point)) in exits.iter().enumerate() {
                let mut first = true;
                for (j, (_, other)) in exits.iter().enumerate() {
                    if i != j && clockwise_before(back_point, *point, *other)? != Some(true) {
                        first = false;
                        break;
                    }
                }
                if first {
                    return Ok(Some(*id));
                }
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clockwise_precedence_includes_cardinal_and_opposite_rays_in_each_frame() {
        let rays = [
            [1., -1.],
            [0., -1.],
            [-1., -1.],
            [-1., 0.],
            [-1., 1.],
            [0., 1.],
            [1., 1.],
        ];
        for rotation in 0..4 {
            let rotate = |mut p: [f64; 2]| {
                for _ in 0..rotation {
                    p = [-p[1], p[0]];
                }
                p.map(I::point)
            };
            for i in 0..rays.len() {
                for j in 0..rays.len() {
                    if i != j {
                        assert_eq!(
                            clockwise_before(rotate([1., 0.]), rotate(rays[i]), rotate(rays[j]))
                                .unwrap(),
                            Some(i < j),
                            "rotation {rotation}, rays {i},{j}"
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn indistinguishable_departures_are_refused_with_bounded_work() {
        let incoming = Curve::from_polyline(vec![vec![-1., 0.], vec![0., 0.]]).unwrap();
        let outgoing = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
        let mut work = 0;
        assert_eq!(
            continuation(&incoming, &[(0, &outgoing), (1, &outgoing)], &mut work).unwrap(),
            None
        );
        assert!(work < 100000);
    }
}
