//! Implicit algebraic seam correspondence, proved without solving for roots.
//! A common strictly monotone Cartesian coordinate owns a unique one-cover.
//! Original position/normal jets must vanish modulo its exact bivariate relation.
use super::*;
pub(super) mod overlap;
use std::collections::BTreeMap;
type B = BTreeMap<(usize, usize), Q>;
fn tensor(a: &P, b: &P, ctx: &mut PredicateContext<'_>) -> Result<B, Reason> {
    let mut out = B::new();
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            ctx.charge(1)?;
            if x.n.sign() != Sign::Zero && y.n.sign() != Sign::Zero {
                out.insert((i, j), x.mul(y, ctx)?);
            }
        }
    }
    Ok(out)
}
fn combine(mut a: B, b: B, factor: &Q, ctx: &mut PredicateContext<'_>) -> Result<B, Reason> {
    for (key, v) in b {
        let v = a
            .remove(&key)
            .unwrap_or_else(Q::zero)
            .add(&v.mul(factor, ctx)?, ctx)?;
        if v.n.sign() != Sign::Zero {
            a.insert(key, v);
        }
    }
    Ok(a)
}
fn times_source(a: &B, b: &P, ctx: &mut PredicateContext<'_>) -> Result<B, Reason> {
    let mut out = B::new();
    for (&(i, j), x) in a {
        for (k, y) in b.iter().enumerate() {
            ctx.charge(1)?;
            if y.n.sign() == Sign::Zero {
                continue;
            }
            let key = (i + k, j);
            let v = out
                .remove(&key)
                .unwrap_or_else(Q::zero)
                .add(&x.mul(y, ctx)?, ctx)?;
            if v.n.sign() != Sign::Zero {
                out.insert(key, v);
            }
        }
    }
    Ok(out)
}
fn quotient(mut p: B, f: &B, ctx: &mut PredicateContext<'_>) -> Result<Option<B>, Reason> {
    let mut result = B::new();
    let Some((&(fi, fj), fc)) = f.last_key_value() else {
        return Ok(None);
    };
    let sign = if fc.n.sign() == Sign::Negative {
        neg(&vec![one()], ctx)?[0].clone()
    } else {
        one()
    };
    let den = fc.mul(&sign, ctx)?;
    while let Some((&(i, j), c)) = p.last_key_value() {
        ctx.charge(1)?;
        if i < fi || j < fj {
            return Ok(None);
        }
        let factor = c.mul(&sign, ctx)?.div_positive(&den, ctx)?;
        result.insert((i - fi, j - fj), factor.clone());
        for (&(a, b), v) in f {
            let key = (a + i - fi, b + j - fj);
            let r = p
                .remove(&key)
                .unwrap_or_else(Q::zero)
                .sub(&v.mul(&factor, ctx)?, ctx)?;
            if r.n.sign() != Sign::Zero {
                p.insert(key, r);
            }
        }
    }
    Ok(Some(result))
}
fn divisible(p: B, f: &B, ctx: &mut PredicateContext<'_>) -> Result<bool, Reason> {
    Ok(quotient(p, f, ctx)?.is_some())
}
fn difference(a: &P, b: &P, c: &P, d: &P, ctx: &mut PredicateContext<'_>) -> Result<B, Reason> {
    let left = tensor(a, b, ctx)?;
    let right = tensor(c, d, ctx)?;
    combine(left, right, &Q::zero().sub(&one(), ctx)?, ctx)
}
fn relation_mod(
    a: &[H],
    b: &[H],
    axis: usize,
    scale: &Q,
    order: usize,
    ctx: &mut PredicateContext<'_>,
) -> Result<bool, Reason> {
    let f = difference(&a[0][axis], &b[0][3], &a[0][3], &b[0][axis], ctx)?;
    if f.is_empty() {
        return Ok(false);
    }
    jets_mod(a, b, &f, scale, order, ctx)
}
fn jets_mod(
    a: &[H],
    b: &[H],
    f: &B,
    scale: &Q,
    order: usize,
    ctx: &mut PredicateContext<'_>,
) -> Result<bool, Reason> {
    for k in 0..3 {
        if !divisible(
            difference(&a[0][k], &b[0][3], &a[0][3], &b[0][k], ctx)?,
            &f,
            ctx,
        )? {
            return Ok(false);
        }
    }
    let va = velocity(a, ctx)?;
    let vb = velocity(b, ctx)?;
    let aw2 = mul(&a[0][3], &a[0][3], ctx)?;
    let bw2 = mul(&b[0][3], &b[0][3], ctx)?;
    for k in 0..3 {
        let p = combine(
            tensor(&aw2, &vb[k], ctx)?,
            tensor(&va[k], &bw2, ctx)?,
            scale,
            ctx,
        )?;
        if !divisible(p, &f, ctx)? {
            return Ok(false);
        }
    }
    if order == 1 {
        return Ok(true);
    }
    let ca = acceleration(a, ctx)?;
    let cb = acceleration(b, ctx)?;
    let aw3 = mul(&aw2, &a[0][3], ctx)?;
    let bw3 = mul(&bw2, &b[0][3], ctx)?;
    let factor = Q::zero().sub(&scale.mul(scale, ctx)?, ctx)?;
    let mut residual = Vec::new();
    for k in 0..3 {
        residual.push(combine(
            tensor(&aw3, &cb[k], ctx)?,
            tensor(&ca[k], &bw3, ctx)?,
            &factor,
            ctx,
        )?);
    }
    for k in 0..3 {
        let j = (k + 1) % 3;
        let p = combine(
            times_source(&residual[k], &va[j], ctx)?,
            times_source(&residual[j], &va[k], ctx)?,
            &Q::zero().sub(&one(), ctx)?,
            ctx,
        )?;
        if !divisible(p, &f, ctx)? {
            return Ok(false);
        }
    }
    Ok(true)
}
// Repeated original cuts may encode the very same polynomial jet. Equality
// of ALL exact coefficients licenses reusing only the divisibility result;
// every original interval still owns its separate coverage/monotonicity proof.
fn jet_representatives(
    spans: &[(usize, Vec<H>)],
    ctx: &mut PredicateContext<'_>,
) -> Result<Vec<usize>, Reason> {
    let mut reps = Vec::new();
    for (i, (_, a)) in spans.iter().enumerate() {
        let mut representative = i;
        'candidate: for j in 0..i {
            if reps[j] != j {
                continue;
            }
            let b = &spans[j].1;
            if a.len() != b.len() {
                continue;
            }
            for (x, y) in a.iter().zip(b) {
                for k in 0..4 {
                    for r in 0..x[k].len().max(y[k].len()) {
                        if !x[k]
                            .get(r)
                            .unwrap_or(&Q::zero())
                            .equal(y[k].get(r).unwrap_or(&Q::zero()), ctx)?
                        {
                            continue 'candidate;
                        }
                    }
                }
            }
            representative = j;
            break;
        }
        reps.push(representative);
    }
    Ok(reps)
}
pub(super) fn compare(
    ctx: &mut PredicateContext<'_>,
    spans: &[Vec<(usize, Vec<H>)>],
    u: &[Vec<Q>],
    scale: &Q,
    order: usize,
) -> Result<BezierIdentity, Reason> {
    for axis in 0..3 {
        let mut ranges = Vec::new();
        let mut direction = None;
        let mut valid = true;
        for side in 0..2 {
            let mut rows = Vec::new();
            let mut previous: Option<Q> = None;
            for (i, h) in &spans[side] {
                let n = &h[0][axis];
                let d = &h[0][3];
                let lo = &u[side][*i];
                let hi = &u[side][i + 1];
                let first = value(n, lo, ctx)?.div_positive(&value(d, lo, ctx)?, ctx)?;
                let last = value(n, hi, ctx)?.div_positive(&value(d, hi, ctx)?, ctx)?;
                let slope = sub(
                    &mul(&derivative(n, ctx)?, d, ctx)?,
                    &mul(n, &derivative(d, ctx)?, ctx)?,
                    ctx,
                )?;
                let sign = last.sub(&first, ctx)?.n.sign();
                if sign == Sign::Zero || direction.is_some_and(|s| s != sign) {
                    valid = false;
                    break;
                }
                if let Some(p) = previous {
                    if !p.equal(&first, ctx)? {
                        valid = false;
                        break;
                    }
                }
                let slope = if sign == Sign::Negative {
                    neg(&slope, ctx)?
                } else {
                    slope
                };
                if !positive(&slope, lo, hi, ctx)? {
                    valid = false;
                    break;
                }
                direction = Some(sign);
                previous = Some(last.clone());
                rows.push([first, last]);
            }
            if !valid {
                break;
            }
            ranges.push(rows);
        }
        if !valid {
            continue;
        }
        if !ranges[0][0][0].equal(&ranges[1][0][0], ctx)?
            || !ranges[0].last().unwrap()[1].equal(&ranges[1].last().unwrap()[1], ctx)?
        {
            continue;
        }
        let reverse = direction == Some(Sign::Negative);
        let representatives = [
            jet_representatives(&spans[0], ctx)?,
            jet_representatives(&spans[1], ctx)?,
        ];
        let mut proved = std::collections::BTreeSet::new();
        'pairs: for (i, (_, a)) in spans[0].iter().enumerate() {
            for (j, (_, b)) in spans[1].iter().enumerate() {
                let [al, ah] = if reverse {
                    [&ranges[0][i][1], &ranges[0][i][0]]
                } else {
                    [&ranges[0][i][0], &ranges[0][i][1]]
                };
                let [bl, bh] = if reverse {
                    [&ranges[1][j][1], &ranges[1][j][0]]
                } else {
                    [&ranges[1][j][0], &ranges[1][j][1]]
                };
                if ah.sub(bl, ctx)?.n.sign() != Sign::Positive
                    || bh.sub(al, ctx)?.n.sign() != Sign::Positive
                {
                    continue;
                }
                let key = (representatives[0][i], representatives[1][j]);
                if proved.contains(&key) {
                    continue;
                }
                if !relation_mod(a, b, axis, scale, order, ctx)? {
                    valid = false;
                    break 'pairs;
                }
                proved.insert(key);
            }
        }
        if valid {
            ctx.charge(0)?;
            return Ok(BezierIdentity::Equal);
        }
    }
    Err(Reason::MissingProof)
}
/// Local branches retain ORIGINAL span order. Each entire pair has a proved
/// scalar bijection, but the coordinate and direction may change at a join.
/// Matching three-dimensional endpoints and source adjacency sew those maps
/// into one increasing parameter cover. No nearest-point branch choice occurs.
pub(super) fn ordered_local_charts(
    ctx: &mut PredicateContext<'_>,
    spans: &[Vec<(usize, Vec<H>)>],
    u: &[Vec<Q>],
    scale: &Q,
    order: usize,
) -> Result<BezierIdentity, Reason> {
    if spans[0].len() != spans[1].len() {
        return Err(Reason::MissingProof);
    }
    let mut previous: Option<[Q; 3]> = None;
    for ((ai, a), (bi, b)) in spans[0].iter().zip(&spans[1]) {
        let intervals = [[&u[0][*ai], &u[0][ai + 1]], [&u[1][*bi], &u[1][bi + 1]]];
        let mut endpoints = Vec::new();
        for (side, h) in [a, b].into_iter().enumerate() {
            let mut ends = Vec::new();
            for at in intervals[side] {
                let den = value(&h[0][3], at, ctx)?;
                let mut point = std::array::from_fn(|_| Q::zero());
                for k in 0..3 {
                    point[k] = value(&h[0][k], at, ctx)?.div_positive(&den, ctx)?;
                }
                ends.push(point);
            }
            endpoints.push(ends);
        }
        for k in 0..3 {
            if !endpoints[0][0][k].equal(&endpoints[1][0][k], ctx)?
                || !endpoints[0][1][k].equal(&endpoints[1][1][k], ctx)?
            {
                return Err(Reason::MissingProof);
            }
            if let Some(p) = &previous {
                if !p[k].equal(&endpoints[0][0][k], ctx)? {
                    return Err(Reason::MissingProof);
                }
            }
        }
        // Normalize each ORIGINAL interval exactly before bivariate division.
        // Large global knot offsets must not inflate rational coefficients.
        let mut local = Vec::new();
        for (side, jets) in [a, b].into_iter().enumerate() {
            let linear = vec![
                intervals[side][0].clone(),
                intervals[side][1].sub(intervals[side][0], ctx)?,
            ];
            let mut rows = Vec::new();
            for h in jets {
                let mut row = std::array::from_fn(|_| vec![Q::zero()]);
                for k in 0..4 {
                    row[k] = compose(&h[k], &linear, &vec![one()], h[k].len() - 1, ctx)?;
                }
                rows.push(row);
            }
            local.push(rows);
        }
        let mut matched = false;
        for axis in 0..3 {
            let sign = endpoints[0][1][axis]
                .sub(&endpoints[0][0][axis], ctx)?
                .n
                .sign();
            if sign == Sign::Zero {
                continue;
            }
            let mut monotone = true;
            for h in &local {
                let n = &h[0][axis];
                let d = &h[0][3];
                let mut slope = sub(
                    &mul(&derivative(n, ctx)?, d, ctx)?,
                    &mul(n, &derivative(d, ctx)?, ctx)?,
                    ctx,
                )?;
                if sign == Sign::Negative {
                    slope = neg(&slope, ctx)?;
                }
                if !positive(&slope, &Q::zero(), &one(), ctx)? {
                    monotone = false;
                    break;
                }
            }
            if monotone && relation_mod(&local[0], &local[1], axis, scale, order, ctx)? {
                matched = true;
                break;
            }
        }
        if !matched {
            return Err(Reason::MissingProof);
        }
        previous = Some(endpoints[0][1].clone());
    }
    ctx.charge(0)?;
    Ok(BezierIdentity::Equal)
}
/// Ordered chart groups allow unrelated ORIGINAL interior knots. A group
/// endpoint must be an exact common point, but interior endpoints need not have
/// rational inverse parameters on the other side. `compare` checks every pair
/// of overlapping scalar ranges by implicit divisibility, without solving or
/// rounding those inverse roots. Strict monotonicity owns the unique cover.
pub(super) fn ordered_span_groups(
    ctx: &mut PredicateContext<'_>,
    spans: &[Vec<(usize, Vec<H>)>],
    u: &[Vec<Q>],
    scale: &Q,
    order: usize,
) -> Result<BezierIdentity, Reason> {
    fn endpoint(h: &[H], t: &Q, ctx: &mut PredicateContext<'_>) -> Result<[Q; 3], Reason> {
        let den = value(&h[0][3], t, ctx)?;
        let mut point = std::array::from_fn(|_| Q::zero());
        for k in 0..3 {
            point[k] = value(&h[0][k], t, ctx)?.div_positive(&den, ctx)?;
        }
        Ok(point)
    }
    fn same(a: &[Q; 3], b: &[Q; 3], ctx: &mut PredicateContext<'_>) -> Result<bool, Reason> {
        for k in 0..3 {
            if !a[k].equal(&b[k], ctx)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
    let mut points = Vec::new();
    for side in 0..2 {
        let mut ends = Vec::new();
        for (i, h) in &spans[side] {
            let first = endpoint(h, &u[side][*i], ctx)?;
            if let Some(last) = ends.last() {
                if !same(last, &first, ctx)? {
                    return Err(Reason::MissingProof);
                }
            } else {
                ends.push(first);
            }
            ends.push(endpoint(h, &u[side][i + 1], ctx)?);
        }
        points.push(ends);
    }
    if spans[0].is_empty() || spans[1].is_empty() || !same(&points[0][0], &points[1][0], ctx)? {
        return Err(Reason::MissingProof);
    }
    let mut start = [0, 0];
    while start[0] < spans[0].len() && start[1] < spans[1].len() {
        let mut next = None;
        // Deterministic earliest common endpoints; never skip an unproved group
        // or reset the caller's exact-work allowance after a failed proposal.
        'search: for a in start[0] + 1..=spans[0].len() {
            for b in start[1] + 1..=spans[1].len() {
                if !same(&points[0][a], &points[1][b], ctx)? {
                    continue;
                }
                // A strictly monotone coordinate cannot return to the same
                // Cartesian point. Avoid expanding an impossible closed group.
                if same(&points[0][start[0]], &points[0][a], ctx)? {
                    continue;
                }
                let groups = vec![
                    spans[0][start[0]..a].to_vec(),
                    spans[1][start[1]..b].to_vec(),
                ];
                // Normalize the complete group, retaining each ORIGINAL knot
                // and polynomial restriction, to avoid global-offset growth.
                let mut local_spans = Vec::new();
                let mut local_knots = Vec::new();
                for side in 0..2 {
                    let group = &groups[side];
                    let lo = &u[side][group[0].0];
                    let hi = &u[side][group.last().unwrap().0 + 1];
                    let width = hi.sub(lo, ctx)?;
                    let linear = vec![lo.clone(), width.clone()];
                    let mut knots = Vec::new();
                    let mut rows = Vec::new();
                    for (j, (i, jets)) in group.iter().enumerate() {
                        knots.push(u[side][*i].sub(lo, ctx)?.div_positive(&width, ctx)?);
                        let mut transformed = Vec::new();
                        for h in jets {
                            let mut row = std::array::from_fn(|_| vec![Q::zero()]);
                            for k in 0..4 {
                                row[k] =
                                    compose(&h[k], &linear, &vec![one()], h[k].len() - 1, ctx)?;
                            }
                            transformed.push(row);
                        }
                        rows.push((j, transformed));
                    }
                    knots.push(one());
                    local_spans.push(rows);
                    local_knots.push(knots);
                }
                match compare(ctx, &local_spans, &local_knots, scale, order) {
                    Ok(BezierIdentity::Equal) => {
                        next = Some([a, b]);
                        break 'search;
                    }
                    Err(Reason::MissingProof) => {}
                    Err(reason) => return Err(reason),
                    _ => {}
                }
            }
        }
        start = next.ok_or(Reason::MissingProof)?;
    }
    if start != [spans[0].len(), spans[1].len()] {
        return Err(Reason::MissingProof);
    }
    ctx.charge(0)?;
    Ok(BezierIdentity::Equal)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuthoredScalar, Limits, SourceArena, ToleranceContext};
    #[test]
    fn unmatched_interior_cuts_use_implicit_roots_and_one_ordered_cover() {
        let arena =
            SourceArena::authored("cuts", 1, vec![AuthoredScalar::Binary64Bits(0)]).unwrap();
        let tol = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&arena, &tol, Limits::default(), None);
        let integer = |n: usize| {
            let mut q = Q::zero();
            for _ in 0..n {
                q = q.add(&one(), &mut ctx).unwrap();
            }
            q
        };
        // Construct exact rational constants without going through rounded roots.
        let mut integer = integer;
        let two = integer(2);
        let sixty = integer(60);
        let half = one().div_positive(&two, &mut ctx).unwrap();
        let mut spans = Vec::new();
        let mut knots = Vec::new();
        for side in 0..2 {
            let mut rows = Vec::new();
            let mut us = Vec::new();
            for branch in 0..4 {
                let offset = Q::scalar(Expansion::scalar(branch as f64));
                let local = vec![Q::zero().sub(&offset, &mut ctx).unwrap(), one()];
                let clock = if side == 0 {
                    vec![Q::zero(), half.clone(), half.clone()]
                } else {
                    vec![Q::zero(), half.clone(), Q::zero(), half.clone()]
                };
                let q = compose(&clock, &local, &vec![one()], clock.len() - 1, &mut ctx).unwrap();
                let c = vec![sixty.clone()];
                let twice_c = add(&c, &c, &mut ctx).unwrap();
                let mut x = sub(&c, &mul(&twice_c, &q, &mut ctx).unwrap(), &mut ctx).unwrap();
                let q2 = mul(&q, &q, &mut ctx).unwrap();
                let mut y = add(
                    &x,
                    &mul(&twice_c, &sub(&q, &q2, &mut ctx).unwrap(), &mut ctx).unwrap(),
                    &mut ctx,
                )
                .unwrap();
                // y=60+120q-120q²: both ends join the rotated next branch.
                y = add(&y, &mul(&twice_c, &q, &mut ctx).unwrap(), &mut ctx).unwrap();
                for _ in 0..branch {
                    (x, y) = (neg(&y, &mut ctx).unwrap(), x);
                }
                let h = vec![
                    [x, y, vec![Q::zero()], vec![one()]],
                    [
                        vec![Q::zero()],
                        vec![Q::zero()],
                        vec![one()],
                        vec![Q::zero()],
                    ],
                    std::array::from_fn(|_| vec![Q::zero()]),
                ];
                // Only the target has an extra half-span cut. Its source
                // preimage solves s+s²=5/8 and is irrational.
                us.push(offset.clone());
                rows.push((us.len() - 1, h.clone()));
                if side == 1 {
                    us.push(offset.add(&half, &mut ctx).unwrap());
                    rows.push((us.len() - 1, h));
                }
            }
            us.push(Q::scalar(Expansion::scalar(4.)));
            spans.push(rows);
            knots.push(us);
        }
        let scale = Q::zero().sub(&one(), &mut ctx).unwrap();
        assert!(matches!(
            ordered_span_groups(&mut ctx, &spans, &knots, &scale, 2),
            Ok(BezierIdentity::Equal)
        ));
        let mut changed_normal = spans.clone();
        changed_normal[1][1].1[1][2] = vec![two.clone()];
        let mut fresh = PredicateContext::new(&arena, &tol, Limits::default(), None);
        assert!(ordered_span_groups(&mut fresh, &changed_normal, &knots, &scale, 1).is_err());
        let mut doubled = spans.clone();
        // A missing interval must not be silently bridged by a common endpoint.
        doubled[1].remove(2);
        assert!(ordered_span_groups(&mut ctx, &doubled, &knots, &scale, 2).is_err());
    }
    #[test]
    fn bivariate_division_requires_exact_identity_not_near_zero_resultant() {
        let arena =
            SourceArena::authored("implicit", 1, vec![AuthoredScalar::Binary64Bits(0)]).unwrap();
        let tol = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&arena, &tol, Limits::default(), None);
        let f = B::from([
            ((2, 0), one()),
            ((0, 3), Q::zero().sub(&one(), &mut ctx).unwrap()),
        ]);
        let p = times_source(&f, &vec![one(), one()], &mut ctx).unwrap();
        assert!(divisible(p.clone(), &f, &mut ctx).unwrap());
        let mut changed = p;
        changed.insert((0, 0), one());
        assert!(!divisible(changed, &f, &mut ctx).unwrap());
    }
}
