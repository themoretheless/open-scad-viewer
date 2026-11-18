//! Outward polynomial filter before exact high-degree expansion products.
use super::*;
use crate::Interval as I;
type IP = Vec<I>;
fn product(a: &IP, b: &IP, c: &mut PredicateContext<'_>) -> Result<IP, Reason> {
    let mut p = vec![I::point(0.); a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            if (x.lo == 0. && x.hi == 0.) || (y.lo == 0. && y.hi == 0.) {
                c.charge(1)?;
                continue;
            }
            p[i + j] = p[i + j].add(&x.mul(y, c)?, c)?;
        }
    }
    Ok(p)
}
fn sum(a: &IP, b: &IP, minus: bool, c: &mut PredicateContext<'_>) -> Result<IP, Reason> {
    (0..a.len().max(b.len()))
        .map(|i| {
            let z = I::point(0.);
            let x = a.get(i).unwrap_or(&z);
            let y = b.get(i).unwrap_or(&z);
            if minus { x.sub(y, c) } else { x.add(y, c) }
        })
        .collect()
}
fn derivative(a: &IP, c: &mut PredicateContext<'_>) -> Result<IP, Reason> {
    if a.len() == 1 {
        return Ok(vec![I::point(0.)]);
    }
    a.iter()
        .enumerate()
        .skip(1)
        .map(|(i, x)| x.mul(&I::point(i as f64), c))
        .collect()
}
/// Returns true only when EVERY Bernstein cell has a nonnegative lower bound.
/// Failure of this sufficient filter is never a geometric rejection.
pub(super) fn within(
    k: &E,
    w: &P,
    n: &[P; 3],
    max_cells: usize,
    cells: &mut usize,
    ctx: &mut PredicateContext<'_>,
) -> Result<bool, Reason> {
    let enclose = |p: &P, c: &mut PredicateContext<'_>| {
        p.iter()
            .map(|x| x.enclosure(c))
            .collect::<Result<IP, Reason>>()
    };
    let w = enclose(w, ctx)?;
    let n = [
        enclose(&n[0], ctx)?,
        enclose(&n[1], ctx)?,
        enclose(&n[2], ctx)?,
    ];
    let mut speed = vec![I::point(0.)];
    for p in &n {
        speed = sum(&speed, &product(p, p, ctx)?, false, ctx)?;
    }
    let nd = [
        derivative(&n[0], ctx)?,
        derivative(&n[1], ctx)?,
        derivative(&n[2], ctx)?,
    ];
    let mut cross2 = vec![I::point(0.)];
    for axis in 0..3 {
        let a = (axis + 1) % 3;
        let b = (axis + 2) % 3;
        let x = sum(
            &product(&n[a], &nd[b], ctx)?,
            &product(&n[b], &nd[a], ctx)?,
            true,
            ctx,
        )?;
        cross2 = sum(&cross2, &product(&x, &x, ctx)?, false, ctx)?;
    }
    let k = k.enclosure(ctx)?;
    let lhs = product(
        &vec![k.mul(&k, ctx)?],
        &product(&product(&speed, &speed, ctx)?, &speed, ctx)?,
        ctx,
    )?;
    let w2 = product(&w, &w, ctx)?;
    let rhs = product(&product(&w2, &w2, ctx)?, &cross2, ctx)?;
    let q = sum(&lhs, &rhs, true, ctx)?;
    nonnegative(&q, max_cells, cells, ctx)
}
pub(super) enum SymmetricResult {
    Within,
    MidpointExceeds,
    Unresolved,
}
pub(super) fn symmetric_within(
    k: &E,
    w: &P,
    speed: &P,
    det: &P,
    max_cells: usize,
    cells: &mut usize,
    ctx: &mut PredicateContext<'_>,
) -> Result<SymmetricResult, Reason> {
    let enclose = |p: &P, c: &mut PredicateContext<'_>| {
        p.iter()
            .map(|v| v.enclosure(c))
            .collect::<Result<IP, Reason>>()
    };
    let w = enclose(w, ctx)?;
    let speed = enclose(speed, ctx)?;
    let det = enclose(det, ctx)?;
    let k = k.enclosure(ctx)?;
    let lhs = product(
        &vec![k.mul(&k, ctx)?],
        &product(&product(&speed, &speed, ctx)?, &speed, ctx)?,
        ctx,
    )?;
    let rhs = product(
        &vec![I::point(2.)],
        &product(&product(&w, &w, ctx)?, &product(&det, &det, ctx)?, ctx)?,
        ctx,
    )?;
    let mut q = sum(&lhs, &rhs, true, ctx)?;
    for (i, coefficient) in q.iter_mut().enumerate() {
        *coefficient = coefficient.mul(&I::point(0.25f64.powi(i as i32)), ctx)?;
    }
    // This is the boundary z=1 of the SAME root cell, with exact preimage
    // t=1/2. A negative outward upper bound is an original point rejection.
    let mut midpoint = I::point(0.);
    for coefficient in &q {
        midpoint = midpoint.add(coefficient, ctx)?;
    }
    if midpoint.hi < 0. {
        if *cells >= max_cells {
            return Err(Reason::ResourceLimit);
        }
        *cells += 1;
        return Ok(SymmetricResult::MidpointExceeds);
    }
    Ok(if nonnegative(&q, max_cells, cells, ctx)? {
        SymmetricResult::Within
    } else {
        SymmetricResult::Unresolved
    })
}
fn nonnegative(
    q: &IP,
    max_cells: usize,
    cells: &mut usize,
    ctx: &mut PredicateContext<'_>,
) -> Result<bool, Reason> {
    let degree = q.len() - 1;
    if degree > 54 {
        return Ok(false);
    }
    let mut bernstein = Vec::new();
    for j in 0..=degree {
        let mut b = I::point(0.);
        for (i, a) in q.iter().enumerate().take(j + 1) {
            // All choose(54,i) integers are exactly representable binary64.
            let factor = I::point(choose(j, i) as f64)
                .div_nonzero(&I::point(choose(degree, i) as f64), ctx)?;
            b = b.add(&a.mul(&factor, ctx)?, ctx)?;
        }
        bernstein.push(b);
    }
    // The SAME cell allowance is charged even when this sufficient filter
    // fails and the caller proceeds to exact arithmetic. No point sample or
    // unfinished subdivision can grant acceptance of the closed interval.
    let mut stack = vec![bernstein];
    let started = *cells;
    while let Some(mut row) = stack.pop() {
        if *cells >= max_cells || *cells - started >= 128 {
            return Ok(false);
        }
        *cells += 1;
        ctx.charge(row.len() as u64)?;
        if row.iter().all(|x| x.lo >= 0. && x.hi.is_finite()) {
            continue;
        }
        if row[0].hi < 0. || row.last().unwrap().hi < 0. {
            return Ok(false);
        }
        let mut left = vec![row[0]];
        let mut right = vec![*row.last().unwrap()];
        for n in (1..row.len()).rev() {
            for i in 0..n {
                row[i] = row[i].add(&row[i + 1], ctx)?.mul(&I::point(0.5), ctx)?;
            }
            left.push(row[0]);
            right.push(row[n - 1]);
        }
        right.reverse();
        stack.push(right);
        stack.push(left);
    }
    Ok(true)
}
