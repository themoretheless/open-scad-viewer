//! Exact rational seam maps recovered from a proved affine source coordinate.
//! All span pairs with nonempty mapped overlap are checked as polynomials.
use crate::bspline_internal_jets::{Rational as Q, Series};
use crate::{BezierIdentity, Expansion, PredicateContext, Reason, Sign};
pub(crate) type P = Vec<Q>;
pub(crate) type H = [P; 4];
fn one() -> Q {
    Q::scalar(Expansion::scalar(1.))
}
fn integer(n: usize) -> Q {
    Q::scalar(Expansion::scalar(n as f64))
}
fn trim(mut p: P) -> P {
    while p.len() > 1 && p.last().unwrap().n.sign() == Sign::Zero {
        p.pop();
    }
    p
}
pub(crate) fn add(a: &P, b: &P, ctx: &mut PredicateContext<'_>) -> Result<P, Reason> {
    let mut p = vec![Q::zero(); a.len().max(b.len())];
    for i in 0..p.len() {
        p[i] = a
            .get(i)
            .cloned()
            .unwrap_or_else(Q::zero)
            .add(&b.get(i).cloned().unwrap_or_else(Q::zero), ctx)?;
    }
    Ok(trim(p))
}
fn neg(a: &P, ctx: &mut PredicateContext<'_>) -> Result<P, Reason> {
    a.iter().map(|v| Q::zero().sub(v, ctx)).collect()
}
pub(crate) fn sub(a: &P, b: &P, ctx: &mut PredicateContext<'_>) -> Result<P, Reason> {
    add(a, &neg(b, ctx)?, ctx)
}
pub(crate) fn mul(a: &P, b: &P, ctx: &mut PredicateContext<'_>) -> Result<P, Reason> {
    let mut p = vec![Q::zero(); a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            p[i + j] = p[i + j].add(&x.mul(y, ctx)?, ctx)?;
        }
    }
    Ok(trim(p))
}
fn scaled(a: &P, b: &Q, ctx: &mut PredicateContext<'_>) -> Result<P, Reason> {
    Ok(trim(
        a.iter().map(|v| v.mul(b, ctx)).collect::<Result<_, _>>()?,
    ))
}
pub(crate) fn zero(a: &P) -> bool {
    a.iter().all(|v| v.n.sign() == Sign::Zero)
}
pub(crate) fn value(a: &P, t: &Q, ctx: &mut PredicateContext<'_>) -> Result<Q, Reason> {
    let mut v = Q::zero();
    for c in a.iter().rev() {
        v = v.mul(t, ctx)?.add(c, ctx)?;
    }
    Ok(v)
}
pub(crate) fn derivative(a: &P, ctx: &mut PredicateContext<'_>) -> Result<P, Reason> {
    if a.len() == 1 {
        return Ok(vec![Q::zero()]);
    }
    a.iter()
        .enumerate()
        .skip(1)
        .map(|(i, c)| c.mul(&integer(i), ctx))
        .collect()
}
pub(crate) fn compose(
    a: &P,
    n: &P,
    d: &P,
    degree: usize,
    ctx: &mut PredicateContext<'_>,
) -> Result<P, Reason> {
    let mut v = vec![Q::zero()];
    let mut power = vec![one()];
    for i in (0..=degree).rev() {
        v = mul(&v, n, ctx)?;
        if let Some(c) = a.get(i) {
            v = add(&v, &scaled(&power, c, ctx)?, ctx)?;
        }
        power = mul(&power, d, ctx)?;
    }
    Ok(v)
}
/// Strict positive Bernstein coefficients on the ENTIRE original interval.
pub(crate) fn positive(
    a: &P,
    lo: &Q,
    hi: &Q,
    ctx: &mut PredicateContext<'_>,
) -> Result<bool, Reason> {
    let width = hi.sub(lo, ctx)?;
    let local = compose(a, &vec![lo.clone(), width], &vec![one()], a.len() - 1, ctx)?;
    let n = local.len() - 1;
    for i in 0..=n {
        let mut b = Q::zero();
        let mut ratio = one();
        for k in 0..=i {
            if k > 0 {
                ratio = ratio
                    .mul(&integer(i + 1 - k), ctx)?
                    .div_positive(&integer(n + 1 - k), ctx)?;
            }
            b = b.add(&local[k].mul(&ratio, ctx)?, ctx)?;
        }
        if b.n.sign() != Sign::Positive {
            return Ok(false);
        }
    }
    Ok(true)
}
pub(crate) fn span(
    jets: &[Series],
    u: &[Q],
    p: usize,
    i: usize,
    ctx: &mut PredicateContext<'_>,
) -> Result<Vec<H>, Reason> {
    let order = jets[0].len() - 1;
    let mut out = Vec::new();
    for q in 0..=order {
        let mut rows = (i - p..=i)
            .map(|j| std::array::from_fn::<_, 4, _>(|k| vec![jets[j][q][k].clone()]))
            .collect::<Vec<H>>();
        for r in 1..=p {
            for j in (r..=p).rev() {
                let at = i - p + j;
                let den = u[i + 1 + j - r].sub(&u[at], ctx)?;
                let alpha = vec![
                    Q::zero().sub(&u[at], ctx)?.div_positive(&den, ctx)?,
                    one().div_positive(&den, ctx)?,
                ];
                let beta = sub(&vec![one()], &alpha, ctx)?;
                let mut h = std::array::from_fn(|_| vec![Q::zero()]);
                for k in 0..4 {
                    h[k] = add(
                        &mul(&rows[j - 1][k], &beta, ctx)?,
                        &mul(&rows[j][k], &alpha, ctx)?,
                        ctx,
                    )?;
                }
                rows[j] = h;
            }
        }
        out.push(rows.pop().unwrap());
    }
    Ok(out)
}
fn velocity(h: &[H], ctx: &mut PredicateContext<'_>) -> Result<[P; 3], Reason> {
    let mut v = std::array::from_fn(|_| vec![Q::zero()]);
    for k in 0..3 {
        v[k] = sub(
            &mul(&h[1][k], &h[0][3], ctx)?,
            &mul(&h[0][k], &h[1][3], ctx)?,
            ctx,
        )?;
    }
    Ok(v)
}
fn acceleration(h: &[H], ctx: &mut PredicateContext<'_>) -> Result<[P; 3], Reason> {
    let w2 = mul(&h[0][3], &h[0][3], ctx)?;
    let j2 = mul(&h[1][3], &h[1][3], ctx)?;
    let mut a = std::array::from_fn(|_| vec![Q::zero()]);
    for k in 0..3 {
        let first = sub(
            &mul(&h[2][k], &w2, ctx)?,
            &mul(&mul(&h[0][k], &h[2][3], ctx)?, &h[0][3], ctx)?,
            ctx,
        )?;
        let second = sub(
            &mul(&h[0][k], &j2, ctx)?,
            &mul(&mul(&h[1][k], &h[1][3], ctx)?, &h[0][3], ctx)?,
            ctx,
        )?;
        a[k] = add(&first, &scaled(&second, &integer(2), ctx)?, ctx)?;
    }
    Ok(a)
}
pub(crate) fn relation(
    a: &[H],
    b: &[H],
    scale: &Q,
    order: usize,
    ctx: &mut PredicateContext<'_>,
) -> Result<bool, Reason> {
    for k in 0..3 {
        if !zero(&sub(
            &mul(&a[0][k], &b[0][3], ctx)?,
            &mul(&b[0][k], &a[0][3], ctx)?,
            ctx,
        )?) {
            return Ok(false);
        }
    }
    let va = velocity(a, ctx)?;
    let vb = velocity(b, ctx)?;
    let aw2 = mul(&a[0][3], &a[0][3], ctx)?;
    let bw2 = mul(&b[0][3], &b[0][3], ctx)?;
    for k in 0..3 {
        if !zero(&add(
            &mul(&vb[k], &aw2, ctx)?,
            &scaled(&mul(&va[k], &bw2, ctx)?, scale, ctx)?,
            ctx,
        )?) {
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
    let scale2 = scale.mul(scale, ctx)?;
    let mut residual = std::array::from_fn::<_, 3, _>(|_| vec![Q::zero()]);
    for k in 0..3 {
        residual[k] = sub(
            &mul(&cb[k], &aw3, ctx)?,
            &scaled(&mul(&ca[k], &bw3, ctx)?, &scale2, ctx)?,
            ctx,
        )?;
    }
    // A nonzero transverse tangent is independently required by the normal cover.
    for k in 0..3 {
        let j = (k + 1) % 3;
        if !zero(&sub(
            &mul(&residual[k], &va[j], ctx)?,
            &mul(&residual[j], &va[k], ctx)?,
            ctx,
        )?) {
            return Ok(false);
        }
    }
    Ok(true)
}
pub(crate) fn compare(
    ctx: &mut PredicateContext<'_>,
    jets: &[Vec<Series>],
    u: &[Vec<Q>],
    degrees: [usize; 2],
    scale: &Q,
    order: usize,
) -> Result<BezierIdentity, Reason> {
    if order == 0 {
        return Err(Reason::MissingProof);
    }
    let mut spans = Vec::new();
    for side in 0..2 {
        let mut rows = Vec::new();
        for i in degrees[side]..jets[side].len() {
            if u[side][i + 1].sub(&u[side][i], ctx)?.n.sign() == Sign::Positive {
                rows.push((i, span(&jets[side], &u[side], degrees[side], i, ctx)?));
            }
        }
        if rows.is_empty() {
            return Err(Reason::MissingProof);
        }
        spans.push(rows);
    }
    for axis in 0..3 {
        let first = &spans[0][0].1[0];
        let last = &spans[0].last().unwrap().1[0];
        let lo = value(&first[axis], &Q::zero(), ctx)?
            .div_positive(&value(&first[3], &Q::zero(), ctx)?, ctx)?;
        let hi =
            value(&last[axis], &one(), ctx)?.div_positive(&value(&last[3], &one(), ctx)?, ctx)?;
        let width = hi.sub(&lo, ctx)?;
        if width.n.sign() == Sign::Zero {
            continue;
        }
        let mut anchor = true;
        for (_, a) in &spans[0] {
            let expected = add(
                &scaled(&a[0][3], &lo, ctx)?,
                &mul(&vec![Q::zero(), width.clone()], &a[0][3], ctx)?,
                ctx,
            )?;
            if !zero(&sub(&a[0][axis], &expected, ctx)?) {
                anchor = false;
                break;
            }
        }
        if !anchor {
            continue;
        }
        let sign = Q::scalar(Expansion::scalar(if width.n.sign() == Sign::Positive {
            1.
        } else {
            -1.
        }));
        let positive_width = width.mul(&sign, ctx)?;
        let mut previous = Q::zero();
        let mut success = true;
        for (bi, b) in &spans[1] {
            let n = scaled(
                &sub(&b[0][axis], &scaled(&b[0][3], &lo, ctx)?, ctx)?,
                &sign,
                ctx,
            )?;
            let d = scaled(&b[0][3], &positive_width, ctx)?;
            let derivative = sub(
                &mul(&derivative(&n, ctx)?, &d, ctx)?,
                &mul(&n, &derivative(&d, ctx)?, ctx)?,
                ctx,
            )?;
            if !positive(&derivative, &u[1][*bi], &u[1][bi + 1], ctx)? {
                success = false;
                break;
            }
            let start =
                value(&n, &u[1][*bi], ctx)?.div_positive(&value(&d, &u[1][*bi], ctx)?, ctx)?;
            let end = value(&n, &u[1][bi + 1], ctx)?
                .div_positive(&value(&d, &u[1][bi + 1], ctx)?, ctx)?;
            if !start.equal(&previous, ctx)? || one().sub(&end, ctx)?.n.sign() == Sign::Negative {
                success = false;
                break;
            }
            previous = end.clone();
            let mut covered = false;
            for (ai, a) in &spans[0] {
                if end.sub(&u[0][*ai], ctx)?.n.sign() != Sign::Positive
                    || u[0][ai + 1].sub(&start, ctx)?.n.sign() != Sign::Positive
                {
                    continue;
                }
                covered = true;
                let mut composed = Vec::new();
                for h in a {
                    let mut row = std::array::from_fn(|_| vec![Q::zero()]);
                    for k in 0..4 {
                        row[k] = compose(&h[k], &n, &d, degrees[0], ctx)?;
                    }
                    composed.push(row);
                }
                if !relation(&composed, b, scale, order, ctx)? {
                    success = false;
                    break;
                }
            }
            if !covered || !success {
                success = false;
                break;
            }
        }
        if success && previous.equal(&one(), ctx)? {
            ctx.charge(0)?;
            return Ok(BezierIdentity::Equal);
        }
    }
    local_charts(ctx, &spans, u, degrees, scale, order)
}

// Ordered local graph charts admit closed/periodic sources without assuming
// that one globally single-valued Cartesian coordinate parameterizes a loop.
// A B span must fit a single original A chart; no rounded preimage split occurs.
fn local_charts(
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
        while cursor < spans[0].len() && progress.equal(&u[0][spans[0][cursor].0 + 1], ctx)? {
            cursor += 1;
        }
        let Some((ai, a)) = spans[0].get(cursor) else {
            return Err(Reason::MissingProof);
        };
        let bounds = [&u[0][*ai], &u[0][ai + 1]];
        let mut accepted = None;
        for axis in 0..3 {
            let lo = value(&a[0][axis], bounds[0], ctx)?
                .div_positive(&value(&a[0][3], bounds[0], ctx)?, ctx)?;
            let hi = value(&a[0][axis], bounds[1], ctx)?
                .div_positive(&value(&a[0][3], bounds[1], ctx)?, ctx)?;
            let width = hi
                .sub(&lo, ctx)?
                .div_positive(&bounds[1].sub(bounds[0], ctx)?, ctx)?;
            if width.n.sign() == Sign::Zero {
                continue;
            }
            let offset = lo.sub(&width.mul(bounds[0], ctx)?, ctx)?;
            let expected = add(
                &scaled(&a[0][3], &offset, ctx)?,
                &mul(&vec![Q::zero(), width.clone()], &a[0][3], ctx)?,
                ctx,
            )?;
            if !zero(&sub(&a[0][axis], &expected, ctx)?) {
                continue;
            }
            let sign = Q::scalar(Expansion::scalar(if width.n.sign() == Sign::Positive {
                1.
            } else {
                -1.
            }));
            let d = scaled(&b[0][3], &width.mul(&sign, ctx)?, ctx)?;
            let n = scaled(
                &sub(&b[0][axis], &scaled(&b[0][3], &offset, ctx)?, ctx)?,
                &sign,
                ctx,
            )?;
            let slope = sub(
                &mul(&derivative(&n, ctx)?, &d, ctx)?,
                &mul(&n, &derivative(&d, ctx)?, ctx)?,
                ctx,
            )?;
            if !positive(&slope, &u[1][*bi], &u[1][bi + 1], ctx)? {
                continue;
            }
            let start =
                value(&n, &u[1][*bi], ctx)?.div_positive(&value(&d, &u[1][*bi], ctx)?, ctx)?;
            let end = value(&n, &u[1][bi + 1], ctx)?
                .div_positive(&value(&d, &u[1][bi + 1], ctx)?, ctx)?;
            if !start.equal(&progress, ctx)?
                || start.sub(bounds[0], ctx)?.n.sign() == Sign::Negative
                || bounds[1].sub(&end, ctx)?.n.sign() == Sign::Negative
            {
                continue;
            }
            let mut composed = Vec::new();
            for h in a {
                let mut row = std::array::from_fn(|_| vec![Q::zero()]);
                for k in 0..4 {
                    row[k] = compose(&h[k], &n, &d, degrees[0], ctx)?;
                }
                composed.push(row);
            }
            if relation(&composed, b, scale, order, ctx)? {
                accepted = Some(end);
                break;
            }
        }
        if accepted.is_none() {
            if let (Some(l), Some(m)) = (
                projective_coordinate(&a[0], 1, ctx)?,
                projective_coordinate(&a[0], 0, ctx)?,
            ) {
                let mut n = linear_form(&b[0], &l, ctx)?;
                let mut d = linear_form(&b[0], &m, ctx)?;
                let midpoint = u[1][*bi]
                    .add(&u[1][bi + 1], ctx)?
                    .div_positive(&integer(2), ctx)?;
                if value(&d, &midpoint, ctx)?.n.sign() == Sign::Negative {
                    n = neg(&n, ctx)?;
                    d = neg(&d, ctx)?;
                }
                if positive(&d, &u[1][*bi], &u[1][bi + 1], ctx)? {
                    let slope = sub(
                        &mul(&derivative(&n, ctx)?, &d, ctx)?,
                        &mul(&n, &derivative(&d, ctx)?, ctx)?,
                        ctx,
                    )?;
                    if positive(&slope, &u[1][*bi], &u[1][bi + 1], ctx)? {
                        let start = value(&n, &u[1][*bi], ctx)?
                            .div_positive(&value(&d, &u[1][*bi], ctx)?, ctx)?;
                        let end = value(&n, &u[1][bi + 1], ctx)?
                            .div_positive(&value(&d, &u[1][bi + 1], ctx)?, ctx)?;
                        if start.equal(&progress, ctx)?
                            && start.sub(bounds[0], ctx)?.n.sign() != Sign::Negative
                            && bounds[1].sub(&end, ctx)?.n.sign() != Sign::Negative
                        {
                            let mut composed = Vec::new();
                            for h in a {
                                let mut row = std::array::from_fn(|_| vec![Q::zero()]);
                                for k in 0..4 {
                                    row[k] = compose(&h[k], &n, &d, degrees[0], ctx)?;
                                }
                                composed.push(row);
                            }
                            if relation(&composed, b, scale, order, ctx)? {
                                accepted = Some(end);
                            }
                        }
                    }
                }
            }
        }
        let Some(end) = accepted else {
            return Err(Reason::MissingProof);
        };
        progress = end;
    }
    if progress.equal(&one(), ctx)? {
        ctx.charge(0)?;
        Ok(BezierIdentity::Equal)
    } else {
        Err(Reason::MissingProof)
    }
}

// An exact projective linear inverse proposes a chart for general conics and
// full-rank spatial cubics. A/D can be nonlinear in every Cartesian coordinate.
// Free variables are fixed to zero; all final map identities remain mandatory.
fn projective_coordinate(
    h: &H,
    power: usize,
    ctx: &mut PredicateContext<'_>,
) -> Result<Option<[Q; 4]>, Reason> {
    let n = h.iter().map(Vec::len).max().unwrap().max(power + 1);
    let mut rows = (0..n)
        .map(|i| {
            let mut r = (0..4)
                .map(|k| h[k].get(i).cloned().unwrap_or_else(Q::zero))
                .collect::<Vec<_>>();
            r.push(if i == power { one() } else { Q::zero() });
            r
        })
        .collect::<Vec<_>>();
    let mut pivots = Vec::new();
    let mut r = 0;
    for col in 0..4 {
        let Some(at) = (r..n).find(|&j| rows[j][col].n.sign() != Sign::Zero) else {
            continue;
        };
        rows.swap(r, at);
        if rows[r][col].n.sign() == Sign::Negative {
            for k in col..=4 {
                rows[r][k] = Q::zero().sub(&rows[r][k], ctx)?;
            }
        }
        let pivot = rows[r][col].clone();
        for k in col..=4 {
            rows[r][k] = rows[r][k].div_positive(&pivot, ctx)?;
        }
        for j in 0..n {
            if j == r {
                continue;
            }
            let factor = rows[j][col].clone();
            for k in col..=4 {
                rows[j][k] = rows[j][k].sub(&factor.mul(&rows[r][k], ctx)?, ctx)?;
            }
        }
        pivots.push(col);
        r += 1;
        if r == n {
            break;
        }
    }
    if rows
        .iter()
        .any(|r| r[..4].iter().all(|v| v.n.sign() == Sign::Zero) && r[4].n.sign() != Sign::Zero)
    {
        return Ok(None);
    }
    let mut answer = std::array::from_fn(|_| Q::zero());
    for (row, &col) in pivots.iter().enumerate() {
        answer[col] = rows[row][4].clone();
    }
    Ok(Some(answer))
}
fn linear_form(h: &H, c: &[Q; 4], ctx: &mut PredicateContext<'_>) -> Result<P, Reason> {
    let mut p = vec![Q::zero()];
    for k in 0..4 {
        p = add(&p, &scaled(&h[k], &c[k], ctx)?, ctx)?;
    }
    Ok(p)
}
