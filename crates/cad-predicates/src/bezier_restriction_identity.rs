//! Exact rational root restriction without a rounded normalized cut parameter.
use crate::{
    Algebra, AuthoredScalar, BezierIdentity, BezierIdentityDecision, Expansion, InputError,
    LeafRef, PredicateContext, Reason, Sign, exact_inputs,
};

/// Candidate B(t) equals original A(lo+(hi-lo)t), optionally reversing B.
/// Original controls are Euclidean XYZ and positive weight. The source domain
/// and restricted range are authored leaves, not a rounded local quotient.
/// No endpoint-only or sampled agreement is used.
pub fn rational_bezier_restriction_identity(
    ctx: &mut PredicateContext<'_>,
    original: &[[LeafRef; 4]],
    candidate: &[[LeafRef; 4]],
    domain: [LeafRef; 2],
    range: [LeafRef; 2],
    reverse: bool,
) -> Result<BezierIdentityDecision, InputError> {
    if !(2..=33).contains(&original.len()) || !(2..=33).contains(&candidate.len()) {
        return Err(InputError::InvalidInput(
            "Restriction identity degrees must be 1..32",
        ));
    }
    let values = original
        .iter()
        .chain(candidate)
        .flatten()
        .chain(domain.iter())
        .chain(range.iter())
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    let result = (|| -> Result<BezierIdentity, Reason> {
        ctx.charge(values.len() as u64)?;
        let v = exact_inputs(&values, ctx)?;
        let end = 4 * (original.len() + candidate.len());
        let d = &v[end..end + 2];
        let r = &v[end + 2..];
        if d[1].sub(&d[0], ctx)?.sign() != Sign::Positive
            || r[1].sub(&r[0], ctx)?.sign() != Sign::Positive
            || r[0].sub(&d[0], ctx)?.sign() == Sign::Negative
            || d[1].sub(&r[1], ctx)?.sign() == Sign::Negative
            || v[..end]
                .chunks_exact(4)
                .any(|p| p[3].sign() != Sign::Positive)
        {
            return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));
        }
        let homogeneous = |points: &[Expansion],
                           ctx: &mut PredicateContext<'_>|
         -> Result<Vec<[Expansion; 4]>, Reason> {
            points
                .chunks_exact(4)
                .map(|p| {
                    Ok([
                        p[0].mul(&p[3], ctx)?,
                        p[1].mul(&p[3], ctx)?,
                        p[2].mul(&p[3], ctx)?,
                        p[3].clone(),
                    ])
                })
                .collect()
        };
        let a = homogeneous(&v[..4 * original.len()], ctx)?;
        let mut b = homogeneous(&v[4 * original.len()..end], ctx)?;
        if reverse {
            b.reverse();
        }
        let clocks = [
            [d[1].sub(&r[0], ctx)?, r[0].sub(&d[0], ctx)?],
            [d[1].sub(&r[1], ctx)?, r[1].sub(&d[0], ctx)?],
        ];
        let p = a.len() - 1;
        let q = b.len() - 1;
        let mut restricted = Vec::with_capacity(a.len());
        // Blossom control i has p-i lower and i upper arguments. Omit the
        // common positive domain-span^p denominator from every control.
        for i in 0..=p {
            let mut net = a.clone();
            for step in 0..p {
                let c = &clocks[usize::from(step >= p - i)];
                for j in 0..net.len() - 1 {
                    for axis in 0..4 {
                        net[j][axis] = net[j][axis]
                            .mul(&c[0], ctx)?
                            .add(&net[j + 1][axis].mul(&c[1], ctx)?, ctx)?;
                    }
                }
                net.pop();
            }
            restricted.push(net.remove(0));
        }
        for k in 0..=p + q {
            for axis in 0..3 {
                let mut coefficient = Expansion::scalar(0.);
                for i in k.saturating_sub(q)..=p.min(k) {
                    let j = k - i;
                    let factor = Expansion::integer(
                        crate::binomial_u64(p, i) * crate::binomial_u64(q, j),
                        ctx,
                    )?;
                    let term = restricted[i][axis]
                        .mul(&b[j][3], ctx)?
                        .sub(&b[j][axis].mul(&restricted[i][3], ctx)?, ctx)?
                        .mul(&factor, ctx)?;
                    coefficient = coefficient.add(&term, ctx)?;
                }
                if coefficient.sign() != Sign::Zero {
                    return Ok(BezierIdentity::Different);
                }
            }
        }
        ctx.charge(0)?;
        Ok(BezierIdentity::Equal)
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
    use std::sync::atomic::AtomicBool;
    #[test]
    fn cancellation_and_missing_domain_proof_never_become_equality() {
        let values: [f64; 20] = [
            0., 0., 0., 1., 2., 0., 0., 3., 0., 0., 0., 2., 1.5, 0., 0., 4., 0., 1., 0., 0.5,
        ];
        let arena = SourceArena::authored(
            "restriction-cancel",
            1,
            values
                .iter()
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .unwrap();
        let refs = (0..values.len())
            .map(|i| arena.leaf(i).unwrap())
            .collect::<Vec<_>>();
        let a = [
            refs[0..4].try_into().unwrap(),
            refs[4..8].try_into().unwrap(),
        ];
        let b = [
            refs[8..12].try_into().unwrap(),
            refs[12..16].try_into().unwrap(),
        ];
        let tolerance = ToleranceContext::default_valid();
        let cancelled = AtomicBool::new(true);
        let mut ctx =
            PredicateContext::new(&arena, &tolerance, Limits::default(), Some(&cancelled));
        let r = rational_bezier_restriction_identity(
            &mut ctx,
            &a,
            &b,
            [refs[16], refs[17]],
            [refs[18], refs[19]],
            false,
        )
        .unwrap();
        assert_eq!(r.outcome, BezierIdentity::Indeterminate(Reason::Cancelled));
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        let r = rational_bezier_restriction_identity(
            &mut ctx,
            &a,
            &b,
            [refs[16], refs[17]],
            [refs[19], refs[18]],
            false,
        )
        .unwrap();
        assert_eq!(
            r.outcome,
            BezierIdentity::Indeterminate(Reason::MissingProof)
        );
    }
}
