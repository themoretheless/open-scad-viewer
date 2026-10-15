//! Keep shared edge samples on the admitted side of a curved cusp. Uniform
//! parameter grids can produce coincident chords on distinct cubic carriers.
//! Only the subdivision schedule changes; every position remains on its source.
use super::{Result, input, is_affine_plane};
use cad_predicates::{
    AuthoredScalar, Limits, Outcome, PredicateContext, Sign, SourceArena, ToleranceContext,
    orient2d,
};
use nurbs_core::{
    curve::Curve,
    interval_eval::{Interval as I, evaluate_interval},
};
const MAX_DIVISIONS: usize = 256;
fn supported(c: &Curve) -> bool {
    !c.periodic
        && (1..=3).contains(&c.degree)
        && c.control_points.len() == c.degree + 1
        && c.control_points.iter().all(|p| p.len() == 2)
        && c.knots[..=c.degree].iter().all(|t| *t == c.domain()[0])
        && c.knots[c.degree + 1..].iter().all(|t| *t == c.domain()[1])
}
fn direction(c: &Curve) -> Option<[f64; 2]> {
    let p = &c.control_points[0];
    c.control_points.iter().skip(1).find_map(|q| {
        let d = [q[0] - p[0], q[1] - p[1]];
        (d != [0., 0.]).then_some(d)
    })
}
fn sign(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> Result<Sign> {
    let source = SourceArena::authored(
        "display-cusp-chord-order",
        1,
        [p, a, b]
            .into_iter()
            .flatten()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .map_err(|_| input("Invalid cusp sampling coordinates"))?;
    let refs = |i: usize| std::array::from_fn(|k| source.leaf(2 * i + k).unwrap());
    let tolerance = ToleranceContext::default_valid();
    let mut limits = Limits::default();
    limits.max_work = 10000;
    let mut ctx = PredicateContext::new(&source, &tolerance, limits, None);
    match orient2d(&mut ctx, refs(0), refs(1), refs(2))
        .map_err(|_| input("Invalid cusp orientation reference"))?
        .outcome
    {
        Outcome::Sign(s) => Ok(s),
        _ => Err(input("Cusp sampling orientation is unresolved")),
    }
}
fn same_ray(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> Result<bool> {
    let Some(axis) = (0..2).find(|&k| a[k] != p[k] && b[k] != p[k]) else {
        return Ok(false);
    };
    if (a[axis] > p[axis]) != (b[axis] > p[axis]) {
        return Ok(false);
    }
    Ok(sign(p, a, b)? == Sign::Zero)
}
fn image(c: &Curve, t: I, work: &mut usize) -> Result<Vec<I>> {
    *work += 1;
    if *work > 100000 {
        return Err(input("Cusp sampling order budget exceeded"));
    }
    evaluate_interval(c, t)
}
fn exit(c: &Curve, sides: [f64; 2], work: &mut usize) -> Result<Option<[I; 2]>> {
    let Some(d) = direction(c) else {
        return Ok(None);
    };
    let preferred = usize::from(d[1].abs() > d[0].abs());
    let p = &c.control_points[0];
    for axis in [preferred, 1 - preferred] {
        if !nurbs_core::intersection::planar_scalar::monotone(c, axis) {
            continue;
        }
        let other = 1 - axis;
        let end = c.control_points.last().unwrap()[axis];
        let positive = end > p[axis];
        let target = p[axis] + if positive { sides[axis] } else { -sides[axis] };
        if !(p[axis].min(end) < target && target < p[axis].max(end)) {
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
                let v = image(c, I::point(t), work)?[axis];
                if (positive && v.hi < target) || (!positive && v.lo > target) {
                    lo = t
                } else if (positive && v.lo > target) || (!positive && v.hi < target) {
                    hi = t
                }
            }
            if [lo, hi] == old {
                break;
            }
        }
        let q = image(c, I::new(lo, hi)?, work)?;
        let prefix = image(c, I::new(start, hi)?, work)?[other].sub(I::point(p[other]))?;
        let lower = I::point(p[other])
            .sub(I::point(p[other] - sides[other]))?
            .lo;
        let upper = I::point(p[other] + sides[other])
            .sub(I::point(p[other]))?
            .lo;
        if prefix.lo <= -lower || prefix.hi >= upper {
            continue;
        }
        return Ok(Some([q[0].sub(I::point(p[0]))?, q[1].sub(I::point(p[1]))?]));
    }
    Ok(None)
}
fn source_order(a: &Curve, b: &Curve, work: &mut usize) -> Result<Option<Sign>> {
    let p = &a.control_points[0];
    let radius = [a, b]
        .iter()
        .map(|c| {
            let q = c.control_points.last().unwrap();
            (q[0] - p[0]).abs().max((q[1] - p[1]).abs())
        })
        .fold(f64::INFINITY, f64::min);
    // Cap UV scaling can make a globally monotone coordinate nearly parallel
    // to a box side. Align additional box aspect ratios with the departure jet;
    // the interval first-exit proof still determines admission and order.
    let mut aspects = vec![1., 1.5, 0.5, 2.];
    for curve in [a, b] {
        if let Some(d) = direction(curve) {
            let ratio = (d[1] / d[0]).abs();
            for aspect in [ratio * 0.5, ratio * 2.] {
                if aspect.is_finite() && aspect > 0. {
                    aspects.push(aspect);
                }
            }
        }
    }
    for depth in 3..22 {
        let delta = radius * 2_f64.powi(-depth);
        if !delta.is_finite() || delta <= 0. {
            break;
        }
        for &aspect in &aspects {
            let sides = [delta, delta * aspect];
            let (Some(x), Some(y)) = (exit(a, sides, work)?, exit(b, sides, work)?) else {
                continue;
            };
            let cross = x[0].mul(y[1])?.sub(x[1].mul(y[0])?)?;
            let dot = x[0].mul(y[0])?.add(x[1].mul(y[1])?)?;
            if dot.lo <= 0. {
                continue;
            }
            if cross.lo > 0. {
                return Ok(Some(Sign::Positive));
            }
            if cross.hi < 0. {
                return Ok(Some(Sign::Negative));
            }
        }
    }
    Ok(None)
}
/// Use precisely the coedge parameter arithmetic used by sample_wire.
fn sample_parameter(c: &brep_core::Coedge, divisions: usize, index: usize) -> f64 {
    let canonical = if c.reversed { divisions - index } else { index };
    let q = canonical as f64 / divisions as f64;
    let local = if c.reversed { 1. - q } else { q };
    let [a, b] = c.pcurve.domain();
    a + (b - a) * local
}
fn sample(c: &brep_core::Coedge, divisions: usize, index: usize) -> Result<[f64; 2]> {
    let p = c
        .pcurve
        .evaluate(sample_parameter(c, divisions, index))?
        .point;
    Ok([p[0], p[1]])
}
/// A rounded positive determinant is insufficient: cap UV arithmetic may
/// separate two chords whose authoritative 3D samples are exactly collinear.
/// Require an outward interval proof before keeping the current schedule.
fn chord_order(
    previous: &brep_core::Coedge,
    next: &brep_core::Coedge,
    counts: [usize; 2],
    p: [f64; 2],
    work: &mut usize,
) -> Result<Option<Sign>> {
    let a = image(
        &previous.pcurve,
        I::point(sample_parameter(previous, counts[0], counts[0] - 1)),
        work,
    )?;
    let b = image(
        &next.pcurve,
        I::point(sample_parameter(next, counts[1], 1)),
        work,
    )?;
    let x = [a[0].sub(I::point(p[0]))?, a[1].sub(I::point(p[1]))?];
    let y = [b[0].sub(I::point(p[0]))?, b[1].sub(I::point(p[1]))?];
    let cross = x[0].mul(y[1])?.sub(x[1].mul(y[0])?)?;
    Ok(if cross.lo > 0. {
        Some(Sign::Positive)
    } else if cross.hi < 0. {
        Some(Sign::Negative)
    } else {
        None
    })
}

fn adjust_pair(
    previous: &brep_core::Coedge,
    next: &brep_core::Coedge,
    counts: [usize; 2],
    work: &mut usize,
) -> Result<Option<[usize; 2]>> {
    if (previous.pcurve.degree == 1 && next.pcurve.degree == 1)
        || !supported(&previous.pcurve)
        || !supported(&next.pcurve)
    {
        return Ok(None);
    }
    let a = previous.pcurve.reverse()?;
    let b = &next.pcurve;
    if a.control_points[0] != b.control_points[0] {
        return Ok(None);
    }
    let p = [a.control_points[0][0], a.control_points[0][1]];
    let (Some(da), Some(db)) = (direction(&a), direction(b)) else {
        return Ok(None);
    };
    let first = sample(previous, counts[0], counts[0] - 1)?;
    let second = sample(next, counts[1], 1)?;
    if !same_ray(
        p,
        [p[0] + da[0], p[1] + da[1]],
        [p[0] + db[0], p[1] + db[1]],
    )? && !same_ray(p, first, second)?
    {
        return Ok(None);
    }
    let wanted = source_order(&a, b, work)?.ok_or_else(|| {
        #[cfg(test)]
        eprintln!("unresolved cusp a={:?} b={:?}", a, b);
        input("Curved cusp departure order is unresolved")
    })?;
    if chord_order(previous, next, counts, p, work)? == Some(wanted) {
        return Ok(None);
    }
    for factor in [2, 4, 8, 16, 32, 64, 128, 256] {
        for axis in 0..2 {
            let curve = if axis == 0 {
                &previous.pcurve
            } else {
                &next.pcurve
            };
            if curve.degree == 1 {
                continue;
            }
            let mut candidate = counts;
            candidate[axis] = candidate[axis].saturating_mul(factor);
            if candidate[axis] > MAX_DIVISIONS {
                continue;
            }
            if chord_order(previous, next, candidate, p, work)? == Some(wanted) {
                return Ok(Some(candidate));
            }
        }
    }
    Err(input(
        "Curved cusp exceeds the boundary sampling precision budget",
    ))
}
pub(super) fn refine(
    model: &brep_core::Model,
    schedule: &mut [usize],
    parallel: &[[usize; 2]],
) -> Result<()> {
    let mut work = 0;
    for _ in 0..32 {
        let mut changed = false;
        for face in &model.faces {
            if !is_affine_plane(&face.surface, model.tolerance_mm) {
                continue;
            }
            for &wire in std::iter::once(&face.outer).chain(&face.holes) {
                let cs = &model.loops[wire].coedges;
                for i in 0..cs.len() {
                    let a = &cs[(i + cs.len() - 1) % cs.len()];
                    let b = &cs[i];
                    if model.edges[a.edge].degenerate || model.edges[b.edge].degenerate {
                        continue;
                    }
                    if let Some(counts) =
                        adjust_pair(a, b, [schedule[a.edge], schedule[b.edge]], &mut work)?
                    {
                        schedule[a.edge] = counts[0];
                        schedule[b.edge] = counts[1];
                        changed = true;
                    }
                }
            }
        }
        loop {
            let mut propagated = false;
            for &[a, b] in parallel {
                let n = schedule[a].max(schedule[b]);
                propagated |= schedule[a] != n || schedule[b] != n;
                schedule[a] = n;
                schedule[b] = n;
            }
            if !propagated {
                break;
            }
            changed = true;
        }
        if !changed {
            return Ok(());
        }
    }
    Err(input("Cusp boundary sampling propagation budget exceeded"))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn cubic(points: Vec<Vec<f64>>) -> Curve {
        Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: points,
            weights: vec![1.; 4],
            periodic: false,
        }
    }
    #[test]
    fn curved_cusp_refines_the_correct_carrier_without_moving_controls() {
        for rotated in [false, true] {
            for reversed in [false, true] {
                for flags in [false, true] {
                    let mut a = cubic(vec![vec![0., 0.], vec![2., 0.], vec![4., 2.], vec![6., 6.]]);
                    let mut b = cubic(vec![
                        vec![0., 0.],
                        vec![2., 0.],
                        vec![6., 2.],
                        vec![12., 12.],
                    ]);
                    if rotated {
                        for c in [&mut a, &mut b] {
                            for p in &mut c.control_points {
                                let (x, y) = (p[0], p[1]);
                                p[0] = 17. + x - y;
                                p[1] = -9. + x + y;
                            }
                        }
                    }
                    if reversed {
                        std::mem::swap(&mut a, &mut b)
                    }
                    let previous = brep_core::Coedge {
                        edge: 0,
                        reversed: flags,
                        pcurve: a.reverse().unwrap(),
                    };
                    let next = brep_core::Coedge {
                        edge: 1,
                        reversed: !flags,
                        pcurve: b.clone(),
                    };
                    let before = (
                        previous.pcurve.control_points.clone(),
                        next.pcurve.control_points.clone(),
                    );
                    for n in [1, 2, 3, 4, 8, 16, 32] {
                        let wanted = source_order(&a, &b, &mut 0).unwrap().unwrap();
                        let first = sample(&previous, n, n - 1).unwrap();
                        let second = sample(&next, n, 1).unwrap();
                        let origin = [a.control_points[0][0], a.control_points[0][1]];
                        let initial = sign(origin, first, second).unwrap();
                        let adjusted = adjust_pair(&previous, &next, [n, n], &mut 0)
                            .unwrap_or_else(|e| {
                                panic!(
                                    "rotated={rotated} reversed={reversed} flags={flags} n={n}: {e}"
                                )
                            });
                        let result = adjusted.unwrap_or([n, n]);
                        if initial != wanted {
                            assert_eq!(result, if reversed { [2 * n, n] } else { [n, 2 * n] });
                        }
                        assert_eq!(
                            sign(
                                origin,
                                sample(&previous, result[0], result[0] - 1).unwrap(),
                                sample(&next, result[1], 1).unwrap()
                            )
                            .unwrap(),
                            wanted
                        );
                        assert!(
                            adjust_pair(&previous, &next, result, &mut 0)
                                .unwrap()
                                .is_none()
                        );
                    }
                    assert_eq!(
                        (previous.pcurve.control_points, next.pcurve.control_points),
                        before
                    );
                }
            }
        }
    }
}
