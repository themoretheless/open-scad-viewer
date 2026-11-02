//! Piecewise power coefficients from original homogeneous B-spline jet rows.
//! All divisions and interval ordering are exact and debit the caller's ledger.
use crate::bspline_internal_jets::{Rational as Q, Series};
use crate::{
    Algebra, BezierIdentity, Expansion, LeafRef, PredicateContext, Reason, Sign, exact_inputs,
};

pub(crate) fn compare(
    ctx: &mut PredicateContext<'_>,
    jets: &[Vec<Series>],
    knots: [&[LeafRef]; 2],
    degrees: [usize; 2],
    scale: Q,
    order: usize,
    propose: bool,
) -> Result<BezierIdentity, Reason> {
    compare_search(ctx, jets, knots, degrees, scale, order, propose, false)
}

pub(crate) fn compare_search(
    ctx: &mut PredicateContext<'_>,
    jets: &[Vec<Series>],
    knots: [&[LeafRef]; 2],
    degrees: [usize; 2],
    scale: Q,
    order: usize,
    propose: bool,
    nonlinear_only: bool,
) -> Result<BezierIdentity, Reason> {
    let mut normalized = Vec::new();
    let mut breaks: Vec<Q> = Vec::new();
    for side in 0..2 {
        let values = knots[side]
            .iter()
            .map(|r| ctx.resolve(*r).cloned())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| Reason::MissingProof)?;
        ctx.charge(values.len() as u64)?;
        let x = exact_inputs(&values, ctx)?;
        let p = degrees[side];
        let n = jets[side].len();
        let width = Q::scalar(x[n].sub(&x[p], ctx)?);
        let mut u = Vec::new();
        for k in &x {
            u.push(Q::scalar(k.sub(&x[p], ctx)?).div_positive(&width, ctx)?);
        }
        for pair in u.windows(2) {
            if pair[1].sub(&pair[0], ctx)?.n.sign() == Sign::Negative {
                return Err(Reason::MissingProof);
            }
        }
        for k in &u[p..=n] {
            let mut at = 0;
            while at < breaks.len() && breaks[at].sub(k, ctx)?.n.sign() == Sign::Negative {
                at += 1;
            }
            if at == breaks.len() || !breaks[at].equal(k, ctx)? {
                breaks.insert(at, k.clone());
            }
        }
        normalized.push(u);
    }
    // Exact closed clamped traces have no globally affine Cartesian anchor.
    // Try their original ordered/overlapping algebraic charts before spending
    // the shared ledger on an unrelated fractional-linear knot proposal.
    let mut closed = order > 0 && !propose && !nonlinear_only;
    if closed {
        'sides: for side in 0..2 {
            let n = jets[side].len();
            let p = degrees[side];
            for k in 0..=p {
                if !normalized[side][k].equal(&Q::zero(), ctx)?
                    || !normalized[side][n + k].equal(&Q::scalar(Expansion::scalar(1.)), ctx)?
                {
                    closed = false;
                    break 'sides;
                }
            }
            let a = &jets[side][0][0];
            let b = &jets[side][n - 1][0];
            for k in 0..3 {
                if !a[k].mul(&b[3], ctx)?.equal(&b[k].mul(&a[3], ctx)?, ctx)? {
                    closed = false;
                    break 'sides;
                }
            }
        }
    }
    if closed && !nonlinear_only {
        match super::bspline_strip_nonlinear::compare(
            ctx,
            jets,
            &normalized,
            degrees,
            &scale,
            order,
        ) {
            Err(Reason::MissingProof) => {}
            result => return result,
        }
    }
    if nonlinear_only {
        return super::bspline_strip_nonlinear::compare(
            ctx,
            jets,
            &normalized,
            degrees,
            &scale,
            order,
        );
    }
    let affine = compare_spans(
        ctx,
        jets,
        &normalized,
        degrees,
        &breaks,
        &scale,
        order,
        propose,
        None,
    )?;
    if affine == BezierIdentity::Equal || order == 0 {
        return Ok(affine);
    }
    // An ORIGINAL interior knot pair proposes a positive fractional-linear
    // parameter map only. Every polynomial coefficient is still mandatory.
    // A trace with an authored linear pcurve (order 0) MUST remain affine.
    let mut middle = Vec::new();
    for side in 0..2 {
        let mut found = None;
        for k in &normalized[side][degrees[side] + 1..jets[side].len()] {
            if k.n.sign() == Sign::Positive
                && Q::scalar(Expansion::scalar(1.)).sub(k, ctx)?.n.sign() == Sign::Positive
            {
                found = Some(k.clone());
                break;
            }
        }
        let Some(k) = found else {
            return super::bspline_strip_nonlinear::compare(
                ctx,
                jets,
                &normalized,
                degrees,
                &scale,
                order,
            );
        };
        middle.push(k);
    }
    let one = Q::scalar(Expansion::scalar(1.));
    let ratio = middle[0]
        .mul(&one.sub(&middle[1], ctx)?, ctx)?
        .div_positive(&middle[1].mul(&one.sub(&middle[0], ctx)?, ctx)?, ctx)?;
    if ratio.n.sign() != Sign::Positive || ratio.equal(&one, ctx)? {
        return super::bspline_strip_nonlinear::compare(
            ctx,
            jets,
            &normalized,
            degrees,
            &scale,
            order,
        );
    }
    // The proposed Mobius map sends these original knots to one another.
    // Distinct exact Cartesian images rule it out BEFORE a costly polynomial
    // pullback. A plausible knot proposal still follows its established route.
    let a = knot_point(&jets[0], &normalized[0], degrees[0], &middle[0], ctx)?;
    let b = knot_point(&jets[1], &normalized[1], degrees[1], &middle[1], ctx)?;
    for k in 0..3 {
        if !a[k].mul(&b[3], ctx)?.equal(&b[k].mul(&a[3], ctx)?, ctx)? {
            return super::bspline_strip_nonlinear::compare(
                ctx,
                jets,
                &normalized,
                degrees,
                &scale,
                order,
            );
        }
    }
    let mut mapped = normalized[1][degrees[1]..=jets[1].len()].to_vec();
    for k in &normalized[0][degrees[0]..=jets[0].len()] {
        let denominator = ratio.add(&one.sub(&ratio, ctx)?.mul(k, ctx)?, ctx)?;
        let t = k.div_positive(&denominator, ctx)?;
        let mut at = 0;
        while at < mapped.len() && mapped[at].sub(&t, ctx)?.n.sign() == Sign::Negative {
            at += 1;
        }
        if at == mapped.len() || !mapped[at].equal(&t, ctx)? {
            mapped.insert(at, t);
        }
    }
    // The B knots were nondecreasing and may repeat: zero spans grant no credit.
    let mapped_proof = compare_spans(
        ctx,
        jets,
        &normalized,
        degrees,
        &mapped,
        &scale,
        order,
        propose,
        Some(&ratio),
    )?;
    if mapped_proof == BezierIdentity::Equal {
        Ok(mapped_proof)
    } else {
        super::bspline_strip_nonlinear::compare(ctx, jets, &normalized, degrees, &scale, order)
    }
}

fn compare_spans(
    ctx: &mut PredicateContext<'_>,
    jets: &[Vec<Series>],
    normalized: &[Vec<Q>],
    degrees: [usize; 2],
    breaks: &[Q],
    scale: &Q,
    order: usize,
    propose: bool,
    map: Option<&Q>,
) -> Result<BezierIdentity, Reason> {
    let one = Q::scalar(Expansion::scalar(1.));
    let d1 = if let Some(r) = map {
        r.sub(&one, ctx)?
    } else {
        Q::zero()
    };
    for window in breaks.windows(2) {
        if window[0].equal(&window[1], ctx)? {
            continue;
        }
        let d0 = one.add(&d1.mul(&window[0], ctx)?, ctx)?;
        let image = |t: &Q, ctx: &mut PredicateContext<'_>| -> Result<Q, Reason> {
            if let Some(r) = map {
                r.mul(t, ctx)?
                    .div_positive(&one.add(&d1.mul(t, ctx)?, ctx)?, ctx)
            } else {
                Ok(t.clone())
            }
        };
        let mut polys = Vec::new();
        let degree = degrees[0].max(degrees[1]);
        for side in 0..2 {
            let p = degrees[side];
            let u = &normalized[side];
            let interval = if side == 0 {
                [image(&window[0], ctx)?, image(&window[1], ctx)?]
            } else {
                [window[0].clone(), window[1].clone()]
            };
            let mut found = None;
            for i in p..jets[side].len() {
                if interval[0].sub(&u[i], ctx)?.n.sign() != Sign::Negative
                    && u[i + 1].sub(&interval[1], ctx)?.n.sign() != Sign::Negative
                    && u[i + 1].sub(&u[i], ctx)?.n.sign() == Sign::Positive
                {
                    found = Some(i);
                    break;
                }
            }
            let span = found.ok_or(Reason::MissingProof)?;
            let zero = || {
                (0..=degree)
                    .map(|_| {
                        (0..=order)
                            .map(|_| std::array::from_fn(|_| Q::zero()))
                            .collect::<Series>()
                    })
                    .collect::<Vec<_>>()
            };
            let mut rows = Vec::new();
            for i in span - p..=span {
                let mut row = zero();
                row[0] = jets[side][i].clone();
                rows.push(row);
            }
            for r in 1..=p {
                for j in (r..=p).rev() {
                    let at = span - p + j;
                    let den = u[span + 1 + j - r].sub(&u[at], ctx)?;
                    let (alpha, beta, linear, beta_linear) = if side == 0 && map.is_some() {
                        let ratio = map.unwrap();
                        let alpha = ratio
                            .mul(&window[0], ctx)?
                            .sub(&u[at].mul(&d0, ctx)?, ctx)?
                            .div_positive(&den, ctx)?;
                        let linear = ratio
                            .sub(&u[at].mul(&d1, ctx)?, ctx)?
                            .div_positive(&den, ctx)?;
                        let beta = d0.sub(&alpha, ctx)?;
                        let beta_linear = d1.sub(&linear, ctx)?;
                        (alpha, beta, linear, beta_linear)
                    } else {
                        let alpha = window[0].sub(&u[at], ctx)?.div_positive(&den, ctx)?;
                        let beta = one.sub(&alpha, ctx)?;
                        let linear = one.div_positive(&den, ctx)?;
                        let negative = Q::zero().sub(&linear, ctx)?;
                        (alpha, beta, linear, negative)
                    };
                    let mut out = zero();
                    for q in 0..=r {
                        for d in 0..=order {
                            for k in 0..4 {
                                out[q][d][k] = rows[j - 1][q][d][k]
                                    .mul(&beta, ctx)?
                                    .add(&rows[j][q][d][k].mul(&alpha, ctx)?, ctx)?;
                                if q > 0 {
                                    out[q][d][k] = out[q][d][k]
                                        .add(&rows[j][q - 1][d][k].mul(&linear, ctx)?, ctx)?
                                        .add(
                                            &rows[j - 1][q - 1][d][k].mul(&beta_linear, ctx)?,
                                            ctx,
                                        )?;
                                }
                            }
                        }
                    }
                    rows[j] = out;
                }
            }
            // For mapped A, de Boor numerators represent D(t)^p_A H_A(phi(t)).
            // D is strictly positive on [0,1] and independent of the transverse
            // parameter, so it cancels from ALL Cartesian cross jets. B keeps
            // its original polynomial gauge; adding D to B would be incorrect.
            let out = rows.pop().unwrap();
            polys.push(out);
        }
        let proof =
            super::bspline_strip_jets::verify_jets(ctx, polys, scale.clone(), order, propose)?;
        if proof != BezierIdentity::Equal {
            return Ok(proof);
        }
    }
    ctx.charge(0)?;
    Ok(BezierIdentity::Equal)
}

// Exact C0 value at an ORIGINAL interior knot. Fully repeated Bezier knots
// use their literal original pole; other bases use exact homogeneous de Boor.
fn knot_point(
    jets: &[Series],
    u: &[Q],
    degree: usize,
    t: &Q,
    ctx: &mut PredicateContext<'_>,
) -> Result<[Q; 4], Reason> {
    let mut span = None;
    for i in degree..jets.len() {
        if u[i].equal(t, ctx)? && u[i + 1].sub(t, ctx)?.n.sign() == Sign::Positive {
            span = Some(i);
            break;
        }
    }
    let i = span.ok_or(Reason::MissingProof)?;
    let mut bezier = true;
    for j in 0..degree {
        bezier &= u[i - j].equal(t, ctx)?;
    }
    if bezier {
        return Ok(jets[i - degree][0].clone());
    }
    let mut rows = (i - degree..=i)
        .map(|j| jets[j][0].clone())
        .collect::<Vec<_>>();
    for r in 1..=degree {
        for j in (r..=degree).rev() {
            let at = i - degree + j;
            let den = u[i + 1 + j - r].sub(&u[at], ctx)?;
            let alpha = t.sub(&u[at], ctx)?.div_positive(&den, ctx)?;
            let beta = Q::scalar(Expansion::scalar(1.)).sub(&alpha, ctx)?;
            let mut out = std::array::from_fn(|_| Q::zero());
            for k in 0..4 {
                out[k] = rows[j - 1][k]
                    .mul(&beta, ctx)?
                    .add(&rows[j][k].mul(&alpha, ctx)?, ctx)?;
            }
            rows[j] = out;
        }
    }
    Ok(rows.pop().unwrap())
}
