//! Ordered rational chart switches at exact, potentially irrational cuts.
use super::*;
struct Map {
    n: P,
    d: P,
}
fn nonnegative(p: &P, lo: &Q, hi: &Q, ctx: &mut PredicateContext<'_>) -> Result<bool, Reason> {
    let local = compose(
        p,
        &vec![lo.clone(), hi.sub(lo, ctx)?],
        &vec![one()],
        p.len() - 1,
        ctx,
    )?;
    let degree = local.len() - 1;
    let mut some = false;
    for i in 0..=degree {
        let mut coefficient = Q::zero();
        let mut ratio = one();
        for k in 0..=i {
            if k > 0 {
                ratio = ratio
                    .mul(&integer(i + 1 - k), ctx)?
                    .div_positive(&integer(degree + 1 - k), ctx)?;
            }
            coefficient = coefficient.add(&local[k].mul(&ratio, ctx)?, ctx)?;
        }
        if coefficient.n.sign() == Sign::Negative {
            return Ok(false);
        }
        some |= coefficient.n.sign() == Sign::Positive;
    }
    Ok(some)
}
fn boundary(m: &Map, k: &Q, ctx: &mut PredicateContext<'_>) -> Result<P, Reason> {
    sub(&m.n, &scaled(&m.d, k, ctx)?, ctx)
}
fn recover(
    a: &[H],
    b: &[H],
    interval: [&Q; 2],
    source_bounds: [&Q; 2],
    scale: &Q,
    degree: usize,
    order: usize,
    ctx: &mut PredicateContext<'_>,
) -> Result<Option<Map>, Reason> {
    let Some((l, m, inverse_degree, scales)) = coupled_coordinate(&a[0], ctx)? else {
        return Ok(None);
    };
    let target = normalize_inverse_coordinates(&b[0], &scales, ctx)?;
    let (n, d) = inverse_forms(&target, &l, &m, inverse_degree, ctx)?;
    let (mut n, mut d) = reduce_ratio(n, d, ctx)?;
    let mid = interval[0]
        .add(interval[1], ctx)?
        .div_positive(&integer(2), ctx)?;
    if value(&d, &mid, ctx)?.n.sign() == Sign::Negative {
        n = neg(&n, ctx)?;
        d = neg(&d, ctx)?;
    }
    // Nonnegative Bernstein coefficients with at least one positive coefficient
    // prove D>0 in the open B interval. Endpoint poles may only be OUTSIDE the
    // active branch; finite first/last map values are checked separately.
    if !nonnegative(&d, interval[0], interval[1], ctx)? {
        // A pole can lie INSIDE B but outside this chart's active branch.
        // Prove both source-boundary polynomials strictly increasing instead.
        // At a cut, S=D*F' >0 proves D>0. Between the ordered cuts,
        // F_lo>0 and F_hi<0 imply (hi-lo)D>0, so no pole is admitted there.
        let mut lower = derivative(&sub(&n, &scaled(&d, source_bounds[0], ctx)?, ctx)?, ctx)?;
        let mut upper = derivative(&sub(&n, &scaled(&d, source_bounds[1], ctx)?, ctx)?, ctx)?;
        if positive(&neg(&lower, ctx)?, interval[0], interval[1], ctx)?
            && positive(&neg(&upper, ctx)?, interval[0], interval[1], ctx)?
        {
            n = neg(&n, ctx)?;
            d = neg(&d, ctx)?;
            lower = neg(&lower, ctx)?;
            upper = neg(&upper, ctx)?;
        }
        if !positive(&lower, interval[0], interval[1], ctx)?
            || !positive(&upper, interval[0], interval[1], ctx)?
        {
            return Ok(None);
        }
    }
    let slope = sub(
        &mul(&derivative(&n, ctx)?, &d, ctx)?,
        &mul(&n, &derivative(&d, ctx)?, ctx)?,
        ctx,
    )?;
    if !positive(&slope, interval[0], interval[1], ctx)? {
        return Ok(None);
    }
    let mut composed = Vec::new();
    for jet in a {
        let mut h = std::array::from_fn(|_| vec![Q::zero()]);
        for k in 0..4 {
            h[k] = compose(&jet[k], &n, &d, degree, ctx)?;
        }
        composed.push(h);
    }
    if !relation(&composed, b, scale, order, ctx)? {
        return Ok(None);
    }
    Ok(Some(Map { n, d }))
}
fn shared_cut(
    mut a: P,
    mut b: P,
    interval: [&Q; 2],
    ctx: &mut PredicateContext<'_>,
) -> Result<bool, Reason> {
    // Euclidean gcd over exact Q. A sign-changing common factor has a root
    // inside B. Each certified map's monotone boundary gives exactly one root,
    // so the two charts meet at the very same cut; no root approximation occurs.
    while !zero(&b) {
        let mut r = a;
        while !zero(&r) && r.len() >= b.len() {
            let at = r.len() - b.len();
            let lead = b.last().unwrap();
            let sign = if lead.n.sign() == Sign::Negative {
                Q::zero().sub(&one(), ctx)?
            } else {
                one()
            };
            let q = r
                .last()
                .unwrap()
                .mul(&sign, ctx)?
                .div_positive(&lead.mul(&sign, ctx)?, ctx)?;
            for j in 0..b.len() {
                r[at + j] = r[at + j].sub(&q.mul(&b[j], ctx)?, ctx)?;
            }
            r = trim(r);
        }
        a = b;
        b = r;
    }
    if zero(&a) {
        return Ok(false);
    }
    let left = value(&a, interval[0], ctx)?.n.sign();
    let right = value(&a, interval[1], ctx)?.n.sign();
    Ok(matches!(
        (left, right),
        (Sign::Positive, Sign::Negative) | (Sign::Negative, Sign::Positive)
    ))
}
pub(super) fn compare(
    ctx: &mut PredicateContext<'_>,
    spans: &[Vec<(usize, Vec<H>)>],
    u: &[Vec<Q>],
    degrees: [usize; 2],
    scale: &Q,
    order: usize,
) -> Result<BezierIdentity, Reason> {
    let mut cursor = 0;
    let mut progress = Q::zero();
    for (bi, b) in &spans[1] {
        while cursor < spans[0].len()
            && progress.sub(&u[0][spans[0][cursor].0 + 1], ctx)?.n.sign() != Sign::Negative
        {
            cursor += 1;
        }
        let Some((ai, a)) = spans[0].get(cursor) else {
            return Err(Reason::MissingProof);
        };
        let interval = [&u[1][*bi], &u[1][bi + 1]];
        let Some(mut map) = recover(
            a,
            b,
            interval,
            [&u[0][*ai], &u[0][ai + 1]],
            scale,
            degrees[0],
            order,
            ctx,
        )?
        else {
            return Err(Reason::MissingProof);
        };
        let start_den = value(&map.d, interval[0], ctx)?;
        if start_den.n.sign() != Sign::Positive {
            return Err(Reason::MissingProof);
        }
        let start = value(&map.n, interval[0], ctx)?.div_positive(&start_den, ctx)?;
        if !start.equal(&progress, ctx)? || start.sub(&u[0][*ai], ctx)?.n.sign() == Sign::Negative {
            return Err(Reason::MissingProof);
        }
        loop {
            let ai = spans[0][cursor].0;
            let cut = &u[0][ai + 1];
            let end_boundary = boundary(&map, cut, ctx)?;
            if value(&end_boundary, interval[1], ctx)?.n.sign() != Sign::Positive {
                let denominator = value(&map.d, interval[1], ctx)?;
                if denominator.n.sign() != Sign::Positive {
                    return Err(Reason::MissingProof);
                }
                let end = value(&map.n, interval[1], ctx)?.div_positive(&denominator, ctx)?;
                if end.sub(&u[0][ai], ctx)?.n.sign() == Sign::Negative {
                    return Err(Reason::MissingProof);
                }
                progress = end;
                break;
            }
            if value(&end_boundary, interval[0], ctx)?.n.sign() != Sign::Negative {
                return Err(Reason::MissingProof);
            }
            let Some((next_i, next_a)) = spans[0].get(cursor + 1) else {
                return Err(Reason::MissingProof);
            };
            if !cut.equal(&u[0][*next_i], ctx)? {
                return Err(Reason::MissingProof);
            }
            let Some(next) = recover(
                next_a,
                b,
                interval,
                [&u[0][*next_i], &u[0][next_i + 1]],
                scale,
                degrees[0],
                order,
                ctx,
            )?
            else {
                return Err(Reason::MissingProof);
            };
            if !shared_cut(end_boundary, boundary(&next, cut, ctx)?, interval, ctx)? {
                return Err(Reason::MissingProof);
            }
            map = next;
            cursor += 1;
        }
    }
    if !progress.equal(&one(), ctx)? {
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
    fn shared_cut_requires_exact_common_root_not_overlapping_numeric_brackets() {
        let src = SourceArena::authored(
            "chart-cut-fixture",
            1,
            vec![AuthoredScalar::Binary64Bits(0)],
        )
        .unwrap();
        let tol = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&src, &tol, Limits::default(), None);
        let p = |x: &[f64]| {
            x.iter()
                .map(|&v| Q::scalar(Expansion::scalar(v)))
                .collect::<P>()
        };
        let cut = p(&[-1., 1., 1.]);
        let multiple = mul(&cut, &p(&[2., 0., 2.]), &mut ctx).unwrap();
        assert!(shared_cut(cut.clone(), multiple, [&Q::zero(), &one()], &mut ctx).unwrap());
        assert!(
            !shared_cut(
                cut.clone(),
                p(&[-0.9999999999999999, 1., 1.]),
                [&Q::zero(), &one()],
                &mut ctx
            )
            .unwrap()
        );
    }
}
