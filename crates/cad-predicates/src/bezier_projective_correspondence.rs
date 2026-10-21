//! Exact positive fractional-linear parameter correspondence of rational curves.
use crate::{
    Algebra, AuthoredScalar, BezierIdentity, BezierIdentityDecision, Expansion, InputError,
    LeafRef, PredicateContext, Reason, Sign, exact_inputs,
};

/// Prove B(t)=A(phi(t)), optionally with A's control sequence reversed, where
/// phi(t)=alpha*t/(beta*(1-t)+alpha*t). Positive alpha,beta make phi an
/// orientation-preserving bijection of the COMPLETE normalized interval.
/// Degrees may differ. Original coefficients are transformed in exact arithmetic;
/// no sampled image, rounded reweighted curve, or rounded alpha/beta is authority.
pub fn rational_bezier_projective_correspondence(
    ctx: &mut PredicateContext<'_>,
    a: &[[LeafRef; 4]],
    b: &[[LeafRef; 4]],
    alpha_beta: [LeafRef; 2],
    reverse: bool,
) -> Result<BezierIdentityDecision, InputError> {
    if !(2..=33).contains(&a.len()) || !(2..=33).contains(&b.len()) {
        return Err(InputError::InvalidInput(
            "Projective Bezier degrees must be 1..32",
        ));
    }
    let values = a
        .iter()
        .chain(b)
        .flatten()
        .chain(alpha_beta.iter())
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    let computed = (|| -> Result<BezierIdentity, Reason> {
        ctx.charge(values.len() as u64)?;
        let values = exact_inputs(&values, ctx)?;
        let (alpha, beta) = (
            &values[4 * (a.len() + b.len())],
            &values[4 * (a.len() + b.len()) + 1],
        );
        if alpha.sign() != Sign::Positive
            || beta.sign() != Sign::Positive
            || values[..4 * (a.len() + b.len())]
                .chunks_exact(4)
                .any(|p| p[3].sign() != Sign::Positive)
        {
            return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));
        }
        correspondence(
            ctx,
            &values[..4 * (a.len() + b.len())],
            a.len(),
            b.len(),
            alpha,
            beta,
            reverse,
        )
    })()
    .and_then(|value| ctx.charge(0).map(|_| value));
    Ok(BezierIdentityDecision {
        outcome: computed.unwrap_or_else(BezierIdentity::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}

fn correspondence(
    ctx: &mut PredicateContext<'_>,
    values: &[Expansion],
    na: usize,
    nb: usize,
    alpha: &Expansion,
    beta: &Expansion,
    reverse: bool,
) -> Result<BezierIdentity, Reason> {
    let (p, q) = (na - 1, nb - 1);
    // The map depends only on alpha/beta; this exact positive common gauge
    // cancels from every transformed degree-p coefficient. Keep high-degree
    // source weights from forcing these factors to underflow together.
    let mut factors = vec![alpha.clone(), beta.clone()];
    Expansion::normalize_common(&mut factors, ctx)?;
    let (alpha, beta) = (&factors[0], &factors[1]);
    let powers =
        |x: &Expansion, ctx: &mut PredicateContext<'_>| -> Result<Vec<Expansion>, Reason> {
            let mut out = vec![Expansion::scalar(1.)];
            for i in 0..p {
                out.push(out[i].mul(x, ctx)?);
            }
            Ok(out)
        };
    let ap = powers(alpha, ctx)?;
    let bp = powers(beta, ctx)?;
    let mut weights = Vec::with_capacity(na);
    for i in 0..=p {
        let source_i = if reverse { p - i } else { i };
        weights.push(
            values[4 * source_i + 3]
                .mul(&ap[i], ctx)?
                .mul(&bp[p - i], ctx)?,
        );
    }
    let choose = crate::binomial_u64;
    // Each Bernstein coefficient of Na*Wb-Nb*Wa must be exactly zero.
    // The omitted common denominator C(p+q,k) is strictly positive.
    for k in 0..=p + q {
        for axis in 0..3 {
            let mut coefficient = Expansion::scalar(0.);
            for i in k.saturating_sub(q)..=p.min(k) {
                let j = k - i;
                let source_i = if reverse { p - i } else { i };
                let aa = &values[4 * source_i..4 * source_i + 4];
                let bb = &values[4 * (na + j)..4 * (na + j) + 4];
                let factor = Expansion::integer(choose(p, i) * choose(q, j), ctx)?;
                let term = aa[axis]
                    .sub(&bb[axis], ctx)?
                    .mul(&weights[i], ctx)?
                    .mul(&bb[3], ctx)?
                    .mul(&factor, ctx)?;
                coefficient = coefficient.add(&term, ctx)?;
            }
            if coefficient.sign() != Sign::Zero {
                return Ok(BezierIdentity::Different);
            }
        }
    }
    Ok(BezierIdentity::Equal)
}

/// Qualify the restriction of one GLOBAL projective map to two original
/// subintervals, then prove the candidate Bezier functions correspond under
/// that exact restricted map. Bounds/factors are immutable authored leaves.
/// No rounded inverse, local ratio or transformed knot is introduced.
pub fn rational_bezier_projective_subinterval_correspondence(
    ctx: &mut PredicateContext<'_>,
    a: &[[LeafRef; 4]],
    b: &[[LeafRef; 4]],
    alpha_beta: [LeafRef; 2],
    domains: [[LeafRef; 2]; 2],
    ranges: [[LeafRef; 2]; 2],
    reverse: bool,
) -> Result<BezierIdentityDecision, InputError> {
    if !(2..=33).contains(&a.len()) || !(2..=33).contains(&b.len()) {
        return Err(InputError::InvalidInput(
            "Projective candidate degrees must be 1..32",
        ));
    }
    let values = a
        .iter()
        .chain(b)
        .flatten()
        .chain(alpha_beta.iter())
        .chain(domains.iter().flatten())
        .chain(ranges.iter().flatten())
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    let result = (|| -> Result<BezierIdentity, Reason> {
        ctx.charge(values.len() as u64)?;
        let values = exact_inputs(&values, ctx)?;
        let start = 4 * (a.len() + b.len());
        let (alpha, beta) = (&values[start], &values[start + 1]);
        let d = &values[start + 2..start + 6];
        let r = &values[start + 6..];
        if alpha.sign() != Sign::Positive
            || beta.sign() != Sign::Positive
            || values[..start]
                .chunks_exact(4)
                .any(|p| p[3].sign() != Sign::Positive)
        {
            return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));
        }
        for k in 0..2 {
            if d[2 * k + 1].sub(&d[2 * k], ctx)?.sign() != Sign::Positive
                || r[2 * k + 1].sub(&r[2 * k], ctx)?.sign() != Sign::Positive
                || r[2 * k].sub(&d[2 * k], ctx)?.sign() == Sign::Negative
                || d[2 * k + 1].sub(&r[2 * k + 1], ctx)?.sign() == Sign::Negative
            {
                return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));
            }
        }
        let da = d[1].sub(&d[0], ctx)?;
        let mut denominator = Vec::new();
        for k in 0..2 {
            let t = r[2 + k].sub(&d[2], ctx)?;
            let u = d[3].sub(&r[2 + k], ctx)?;
            let at = alpha.mul(&t, ctx)?;
            let bu = beta.mul(&u, ctx)?;
            let den = at.add(&bu, ctx)?;
            let coordinate = r[if reverse { 1 - k } else { k }].sub(&d[0], ctx)?;
            let expected = if reverse { &bu } else { &at };
            if coordinate
                .mul(&den, ctx)?
                .sub(&da.mul(expected, ctx)?, ctx)?
                .sign()
                != Sign::Zero
            {
                return Ok(BezierIdentity::Different);
            }
            denominator.push(den);
        }
        correspondence(
            ctx,
            &values[..start],
            a.len(),
            b.len(),
            &denominator[1],
            &denominator[0],
            reverse,
        )
    })()
    .and_then(|v| ctx.charge(0).map(|_| v));
    Ok(BezierIdentityDecision {
        outcome: result.unwrap_or_else(BezierIdentity::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Limits, SourceArena, ToleranceContext};
    #[test]
    fn exact_projective_identity_preserves_cancellation_deadline_and_resource_admission() {
        use std::{sync::atomic::AtomicBool, time::Instant};
        let data = [
            0., 0., 0., 1., 1., 2., 0., 1., 0., 0., 0., 1., 1., 2., 0., 2., 2., 1.,
        ];
        let arena = SourceArena::authored(
            "projective-admission",
            1,
            data.iter()
                .map(|v| AuthoredScalar::Binary64Bits(f64::to_bits(*v)))
                .collect(),
        )
        .unwrap();
        let a = (0..2)
            .map(|i| std::array::from_fn(|k| arena.leaf(4 * i + k).unwrap()))
            .collect::<Vec<_>>();
        let b = (2..4)
            .map(|i| std::array::from_fn(|k| arena.leaf(4 * i + k).unwrap()))
            .collect::<Vec<_>>();
        let factors = [arena.leaf(16).unwrap(), arena.leaf(17).unwrap()];
        let tolerance = ToleranceContext::default_valid();
        let run = |limits, cancel| {
            let mut ctx = PredicateContext::new(&arena, &tolerance, limits, cancel);
            rational_bezier_projective_correspondence(&mut ctx, &a, &b, factors, false)
                .unwrap()
                .outcome
        };
        assert_eq!(run(Limits::default(), None), BezierIdentity::Equal);
        assert_eq!(
            run(
                Limits {
                    max_work: 0,
                    ..Default::default()
                },
                None
            ),
            BezierIdentity::Indeterminate(Reason::ResourceLimit)
        );
        let cancel = AtomicBool::new(true);
        assert_eq!(
            run(Limits::default(), Some(&cancel)),
            BezierIdentity::Indeterminate(Reason::Cancelled)
        );
        assert_eq!(
            run(
                Limits {
                    deadline: Some(Instant::now()),
                    ..Default::default()
                },
                None
            ),
            BezierIdentity::Indeterminate(Reason::DeadlineExceeded)
        );
    }
}
