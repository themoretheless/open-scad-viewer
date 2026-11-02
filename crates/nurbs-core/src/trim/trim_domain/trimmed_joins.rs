//! Exact endpoint closure of the declared trimmed UV intervals.
use super::*;
use cad_predicates::{
    AuthoredScalar, BezierIdentity, Limits, PredicateContext, SourceArena, ToleranceContext,
};
pub struct TrimmedLoopJoins {
    pub closed: Option<bool>,
    pub joins: Vec<Option<bool>>,
    pub work_used: u64,
}
/// Checks each declared end against the next declared start. Endpoint controls
/// remain a free structural proof; interior parameters use exact homogeneous
/// evaluation under ONE shared algebraic budget. Multi-span roots use exact
/// de Boor evaluation; both sides of an interior knot must agree. Unknown joins
/// stay visible; periodic roots remain unsupported.
/// This proves closure only, not simplicity, winding or a face/solid domain.
pub fn exact_trimmed_loop_joins(
    curves: &[Curve],
    ranges: &[[f64; 2]],
    max_work: u64,
) -> Result<TrimmedLoopJoins> {
    check(
        (1..=256).contains(&curves.len())
            && curves.len() == ranges.len()
            && max_work <= cad_predicates::MAX_WORK,
        "Invalid trimmed UV closure dimensions or work allowance",
    )?;
    for (c, r) in curves.iter().zip(ranges) {
        c.validate()?;
        let d = c.domain();
        check(
            c.control_points[0].len() == 2
                && r[0] != r[1]
                && r.iter().all(|t| t.is_finite() && *t >= d[0] && *t <= d[1]),
            "Invalid original UV trim endpoint",
        )?;
    }
    let literal = |c: &Curve, t: f64| -> Option<[f64; 2]> {
        if c.periodic {
            return None;
        }
        let d = c.domain();
        let n = c.control_points.len();
        if t == d[0] && c.knots[..=c.degree].iter().all(|k| *k == d[0]) {
            Some([c.control_points[0][0], c.control_points[0][1]])
        } else if t == d[1] && c.knots[n..].iter().all(|k| *k == d[1]) {
            Some([c.control_points[n - 1][0], c.control_points[n - 1][1]])
        } else {
            None
        }
    };
    let mut joins = (0..curves.len())
        .map(|i| {
            let j = (i + 1) % curves.len();
            match (
                literal(&curves[i], ranges[i][1]),
                literal(&curves[j], ranges[j][0]),
            ) {
                (Some(a), Some(b)) => Some(a == b),
                _ => None,
            }
        })
        .collect::<Vec<_>>();
    if joins.iter().all(Option::is_some) {
        return Ok(TrimmedLoopJoins {
            closed: Some(joins.iter().all(|j| *j == Some(true))),
            joins,
            work_used: 0,
        });
    }
    let mut values = Vec::new();
    let mut layouts = Vec::new();
    let mut controls = 0usize;
    for (c, r) in curves.iter().zip(ranges) {
        let d = c.domain();
        let n = c.control_points.len();
        if c.periodic || !(1..=32).contains(&c.degree) || n > 4096 {
            layouts.push(None);
            continue;
        }
        controls += n;
        if controls > 32768 {
            return Err(crate::resource(
                "Trimmed UV join source census exceeds limit",
            ));
        }
        let start = values.len();
        for (p, w) in c.control_points.iter().zip(&c.weights) {
            values.extend([p[0], p[1], *w]);
        }
        let clock = values.len();
        values.extend(d);
        values.extend(r);
        let knots = values.len();
        values.extend(&c.knots);
        layouts.push(Some((start, n, clock, knots)));
    }
    let mut work = 0;
    if joins.iter().any(Option::is_none) && !values.is_empty() && max_work > 0 {
        let arena = SourceArena::authored(
            "original-trimmed-uv-joins",
            1,
            values
                .iter()
                .map(|v: &f64| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .map_err(|_| crate::input("Invalid trimmed UV join source"))?;
        let rows = |start: usize, n: usize| {
            (0..n)
                .map(|i| std::array::from_fn(|k| arena.leaf(start + 3 * i + k).unwrap()))
                .collect::<Vec<_>>()
        };
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(
            &arena,
            &tolerance,
            Limits {
                max_work,
                ..Limits::default()
            },
            None,
        );
        for i in 0..curves.len() {
            if joins[i].is_some() || ctx.work_used() >= max_work {
                continue;
            }
            let j = (i + 1) % curves.len();
            let (Some(a), Some(b)) = (layouts[i], layouts[j]) else {
                continue;
            };
            let aa = rows(a.0, a.1);
            let bb = rows(b.0, b.1);
            let clocks = [
                [
                    arena.leaf(a.2).unwrap(),
                    arena.leaf(a.2 + 1).unwrap(),
                    arena.leaf(a.2 + 3).unwrap(),
                ],
                [
                    arena.leaf(b.2).unwrap(),
                    arena.leaf(b.2 + 1).unwrap(),
                    arena.leaf(b.2 + 2).unwrap(),
                ],
            ];
            let bezier = |c: &Curve| {
                let n = c.control_points.len();
                let d = c.domain();
                n == c.degree + 1
                    && c.knots[..=c.degree].iter().all(|k| *k == d[0])
                    && c.knots[n..].iter().all(|k| *k == d[1])
            };
            let r = if bezier(&curves[i]) && bezier(&curves[j]) {
                cad_predicates::rational_bezier_uv_trim_join_identity(&mut ctx, [&aa, &bb], clocks)
            } else {
                let knots = |start: usize, n: usize| {
                    (start..start + n)
                        .map(|k| arena.leaf(k).unwrap())
                        .collect::<Vec<_>>()
                };
                let ak = knots(a.3, curves[i].knots.len());
                let bk = knots(b.3, curves[j].knots.len());
                cad_predicates::rational_bspline_uv_point_identity(
                    &mut ctx,
                    [&aa, &bb],
                    [&ak, &bk],
                    [curves[i].degree, curves[j].degree],
                    [clocks[0][2], clocks[1][2]],
                )
            }
            .map_err(|_| crate::input("Invalid trimmed UV join predicate"))?;
            joins[i] = match r.outcome {
                BezierIdentity::Equal => Some(true),
                BezierIdentity::Different => Some(false),
                _ => None,
            };
        }
        work = ctx.work_used();
    }
    let closed = if joins.contains(&Some(false)) {
        Some(false)
    } else if joins.iter().all(|j| *j == Some(true)) {
        Some(true)
    } else {
        None
    };
    Ok(TrimmedLoopJoins {
        closed,
        joins,
        work_used: work,
    })
}
