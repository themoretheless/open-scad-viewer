//! Exact original symmetric-quartic reduction via s=t(1-t), 0<=s<=1/4.
//! No fitted conic, sampled maximum or rounded coordinate surrogate.
use super::*;
fn scaled(p: &P, n: u64, ctx: &mut PredicateContext<'_>) -> Result<P, Reason> {
    mul(p, &vec![E::integer(n, ctx)?], ctx)
}
fn terms(
    e: &E,
    y: &E,
    r: &E,
    b: &E,
    m: &E,
    ctx: &mut PredicateContext<'_>,
) -> Result<(P, P, P), Reason> {
    let a = e.mul(y, ctx)?;
    let br = b.mul(r, ctx)?.mul(&E::scalar(4.), ctx)?;
    let w = vec![
        e.clone(),
        b.sub(e, ctx)?.mul(&E::scalar(4.), ctx)?,
        e.mul(&E::scalar(2.), ctx)?
            .sub(&b.mul(&E::scalar(8.), ctx)?, ctx)?
            .add(&m.mul(&E::scalar(6.), ctx)?, ctx)?,
    ];
    let f = vec![
        a.clone(),
        br.sub(&a.mul(&E::scalar(4.), ctx)?, ctx)?,
        a.sub(&br, ctx)?.mul(&E::scalar(2.), ctx)?,
    ];
    let g = vec![a.clone(), br.sub(&a.mul(&E::scalar(2.), ctx)?, ctx)?];
    let wd = derivative(&w, ctx)?;
    let l = add(
        &mul(&derivative(&f, ctx)?, &w, ctx)?,
        &mul(&f, &wd, ctx)?,
        true,
        ctx,
    )?;
    let h = add(
        &mul(&derivative(&g, ctx)?, &w, ctx)?,
        &mul(&g, &wd, ctx)?,
        true,
        ctx,
    )?;
    let ld = add(
        &mul(&derivative(&l, ctx)?, &w, ctx)?,
        &scaled(&mul(&l, &wd, ctx)?, 2, ctx)?,
        true,
        ctx,
    )?;
    let hd = add(
        &mul(&derivative(&h, ctx)?, &w, ctx)?,
        &scaled(&mul(&h, &wd, ctx)?, 2, ctx)?,
        true,
        ctx,
    )?;
    let d2 = vec![E::scalar(1.), E::scalar(-4.)];
    let v = add(
        &scaled(&mul(&g, &w, ctx)?, 2, ctx)?,
        &mul(&d2, &h, ctx)?,
        true,
        ctx,
    )?;
    let z = add(
        &mul(&d2, &ld, ctx)?,
        &scaled(&mul(&l, &w, ctx)?, 2, ctx)?,
        true,
        ctx,
    )?;
    let t = add(
        &mul(&d2, &hd, ctx)?,
        &scaled(&mul(&h, &w, ctx)?, 6, ctx)?,
        true,
        ctx,
    )?;
    // In rotated coordinates A=x+y=F/W, B=x-y=(2t-1)G/W:
    // speed^2 = V/(2 W^4), cross(A',B'')-cross(B',A'') = D/W^5.
    // Therefore kappa^2 = 2 W^2 D^2 / V^3.
    let speed = add(
        &mul(&d2, &square(&l, ctx)?, ctx)?,
        &square(&v, ctx)?,
        false,
        ctx,
    )?;
    let det = add(
        &mul(&mul(&d2, &l, ctx)?, &t, ctx)?,
        &mul(&v, &z, ctx)?,
        false,
        ctx,
    )?;
    Ok((w, speed, det))
}
fn inequality(
    w: &P,
    speed: &P,
    det: &P,
    k: &E,
    ctx: &mut PredicateContext<'_>,
) -> Result<P, Reason> {
    let lhs = mul(
        &vec![k.mul(k, ctx)?],
        &mul(&square(&speed, ctx)?, &speed, ctx)?,
        ctx,
    )?;
    let rhs = scaled(&mul(&square(&w, ctx)?, &square(&det, ctx)?, ctx)?, 2, ctx)?;
    add(&lhs, &rhs, true, ctx)
}
#[cfg(test)]
fn formula(
    e: &E,
    y: &E,
    r: &E,
    b: &E,
    m: &E,
    k: &E,
    ctx: &mut PredicateContext<'_>,
) -> Result<P, Reason> {
    let (w, speed, det) = terms(e, y, r, b, m, ctx)?;
    inequality(&w, &speed, &det, k, ctx)
}

pub(super) fn inspect(
    source: &[E],
    max_cells: usize,
    cells: &mut usize,
    witness: &mut Option<f64>,
    ctx: &mut PredicateContext<'_>,
) -> Result<Option<CurvatureBoundOutcome>, Reason> {
    if source.len() != 21 {
        return Ok(None);
    }
    let p = |i: usize, k: usize| &source[4 * i + k];
    if p(0, 3).sub(p(4, 3), ctx)?.sign() != Sign::Zero
        || p(1, 3).sub(p(3, 3), ctx)?.sign() != Sign::Zero
    {
        return Ok(None);
    }
    let mut matched = None;
    for (x, y) in [(0, 1), (1, 2), (2, 0)] {
        let mut same = true;
        for i in 0..5 {
            for axis in 0..3 {
                if axis != x && axis != y || (i < 2 && axis == x) || (i > 2 && axis == y) {
                    same &= p(i, axis).sub(p(2, axis), ctx)?.sign() == Sign::Zero;
                }
            }
        }
        if !same {
            continue;
        }
        let yy = p(0, y).sub(p(2, y), ctx)?;
        let rr = p(1, y).sub(p(2, y), ctx)?;
        if yy.sub(&p(4, x).sub(p(2, x), ctx)?, ctx)?.sign() == Sign::Zero
            && rr.sub(&p(3, x).sub(p(2, x), ctx)?, ctx)?.sign() == Sign::Zero
            && yy.sign() == Sign::Positive
            && rr.sign() == Sign::Positive
        {
            matched = Some((yy, rr));
            break;
        }
    }
    let Some((y, r)) = matched else {
        return Ok(None);
    };
    let mut weights = vec![p(0, 3).clone(), p(1, 3).clone(), p(2, 3).clone()];
    E::normalize_common(&mut weights, ctx)?;
    let (w, speed, det) = terms(&weights[0], &y, &r, &weights[1], &weights[2], ctx)?;
    match filter::symmetric_within(
        source.last().unwrap(),
        &w,
        &speed,
        &det,
        max_cells,
        cells,
        ctx,
    )? {
        filter::SymmetricResult::Within => return Ok(Some(CurvatureBoundOutcome::Within)),
        filter::SymmetricResult::MidpointExceeds => {
            *witness = Some(0.5);
            return Ok(Some(CurvatureBoundOutcome::Exceeds));
        }
        filter::SymmetricResult::Unresolved => {}
    }
    if *cells >= max_cells {
        return Err(Reason::ResourceLimit);
    }
    let mut q = inequality(&w, &speed, &det, source.last().unwrap(), ctx)?;
    // Exact dyadic substitution s=z/4. z covers [0,1], hence ALL original t.
    let mut factor = E::scalar(1.);
    for coefficient in &mut q {
        *coefficient = coefficient.mul(&factor, ctx)?;
        factor = factor.mul(&E::scalar(0.25), ctx)?;
    }
    let coefficients = bernstein(&q, ctx)?;
    if coefficients[0].sign() == Sign::Negative {
        *witness = Some(0.);
        *cells += 1;
        return Ok(Some(CurvatureBoundOutcome::Exceeds));
    }
    if coefficients.last().unwrap().sign() == Sign::Negative {
        *witness = Some(0.5);
        *cells += 1;
        return Ok(Some(CurvatureBoundOutcome::Exceeds));
    }
    let mut stack = vec![(coefficients, 0usize)];
    while let Some((cell, depth)) = stack.pop() {
        if *cells >= max_cells {
            return Err(Reason::ResourceLimit);
        }
        *cells += 1;
        ctx.charge(cell.len() as u64)?;
        if cell.iter().all(|v| v.sign() != Sign::Negative) {
            continue;
        }
        if cell[0].sign() == Sign::Negative
            || cell.last().unwrap().sign() == Sign::Negative
            || depth == 32
        {
            // A reduced-variable point generally has an irrational preimage.
            // Do not fabricate a binary64 original-parameter witness.
            return Err(Reason::MissingProof);
        }
        let (a, b) = split(&cell, ctx)?;
        stack.push((b, depth + 1));
        stack.push((a, depth + 1));
    }
    Ok(Some(CurvatureBoundOutcome::Within))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reduced_polynomial_is_exactly_eight_times_the_original_inequality() {
        let arena = crate::SourceArena::authored(
            "quartic-identity",
            1,
            vec![AuthoredScalar::Binary64Bits(1f64.to_bits())],
        )
        .unwrap();
        let tolerance = crate::ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&arena, &tolerance, crate::Limits::default(), None);
        let c = &mut ctx;
        let e = E::scalar(0.125);
        let y = E::scalar(2.);
        let r = E::scalar(1.);
        let b = E::scalar(0.25);
        let m = E::scalar(0.5);
        let k = E::scalar(3.);
        let reduced = formula(&e, &y, &r, &b, &m, &k, c).unwrap();
        let mut composed = vec![E::scalar(0.)];
        let s = vec![E::scalar(0.), E::scalar(1.), E::scalar(-1.)];
        for coefficient in reduced.iter().rev() {
            composed = add(
                &mul(&composed, &s, c).unwrap(),
                &vec![coefficient.clone()],
                false,
                c,
            )
            .unwrap();
        }
        let x = power(&[0., 0., 0., 0.25, 0.25].map(E::scalar).to_vec(), c).unwrap();
        let y = power(&[0.25, 0.25, 0., 0., 0.].map(E::scalar).to_vec(), c).unwrap();
        let w = power(&[0.125, 0.25, 0.5, 0.25, 0.125].map(E::scalar).to_vec(), c).unwrap();
        let wd = derivative(&w, c).unwrap();
        let nx = add(
            &mul(&derivative(&x, c).unwrap(), &w, c).unwrap(),
            &mul(&x, &wd, c).unwrap(),
            true,
            c,
        )
        .unwrap();
        let ny = add(
            &mul(&derivative(&y, c).unwrap(), &w, c).unwrap(),
            &mul(&y, &wd, c).unwrap(),
            true,
            c,
        )
        .unwrap();
        let speed = add(&square(&nx, c).unwrap(), &square(&ny, c).unwrap(), false, c).unwrap();
        let cross = add(
            &mul(&nx, &derivative(&ny, c).unwrap(), c).unwrap(),
            &mul(&ny, &derivative(&nx, c).unwrap(), c).unwrap(),
            true,
            c,
        )
        .unwrap();
        let lhs = mul(
            &vec![k.mul(&k, c).unwrap()],
            &mul(&square(&speed, c).unwrap(), &speed, c).unwrap(),
            c,
        )
        .unwrap();
        let rhs = mul(
            &square(&square(&w, c).unwrap(), c).unwrap(),
            &square(&cross, c).unwrap(),
            c,
        )
        .unwrap();
        let original = scaled(&add(&lhs, &rhs, true, c).unwrap(), 8, c).unwrap();
        let difference = add(&composed, &original, true, c).unwrap();
        assert!(difference.iter().all(|v| v.sign() == Sign::Zero));
    }
}
