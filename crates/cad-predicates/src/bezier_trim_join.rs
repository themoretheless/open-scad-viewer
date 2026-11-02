//! Exact UV endpoint equality on independently trimmed rational Bezier roots.
use crate::{
    Algebra, AuthoredScalar, BezierIdentity, BezierIdentityDecision, Expansion, InputError,
    LeafRef, PredicateContext, Reason, Sign, exact_inputs,
};
fn point(
    p: &[[Expansion; 3]],
    clock: &[Expansion],
    ctx: &mut PredicateContext<'_>,
) -> Result<Option<[Expansion; 3]>, Reason> {
    let span = clock[1].sub(&clock[0], ctx)?;
    let a = clock[1].sub(&clock[2], ctx)?;
    let b = clock[2].sub(&clock[0], ctx)?;
    if span.sign() != Sign::Positive
        || a.sign() == Sign::Negative
        || b.sign() == Sign::Negative
        || p.iter().any(|p| p[2].sign() != Sign::Positive)
    {
        return Ok(None);
    }
    let mut net = p
        .iter()
        .map(|p| Ok([p[0].mul(&p[2], ctx)?, p[1].mul(&p[2], ctx)?, p[2].clone()]))
        .collect::<Result<Vec<_>, Reason>>()?;
    while net.len() > 1 {
        for i in 0..net.len() - 1 {
            for k in 0..3 {
                net[i][k] = net[i][k]
                    .mul(&a, ctx)?
                    .add(&net[i + 1][k].mul(&b, ctx)?, ctx)?;
            }
        }
        net.pop();
    }
    Ok(Some(net.remove(0)))
}
/// Compare two original UV curves at authored physical parameters. Clock rows
/// are [domain start, domain end, endpoint]. No rounded point becomes a leaf.
pub fn rational_bezier_uv_trim_join_identity(
    ctx: &mut PredicateContext<'_>,
    curves: [&[[LeafRef; 3]]; 2],
    clocks: [[LeafRef; 3]; 2],
) -> Result<BezierIdentityDecision, InputError> {
    if curves.iter().any(|c| !(2..=33).contains(&c.len())) {
        return Err(InputError::InvalidInput(
            "UV trim join degrees must be 1..32",
        ));
    }
    let values = curves
        .iter()
        .flat_map(|c| c.iter().flatten())
        .chain(clocks.iter().flatten())
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    let result = (|| -> Result<BezierIdentity, Reason> {
        ctx.charge(values.len() as u64)?;
        let v = exact_inputs(&values, ctx)?;
        let n = 3 * curves[0].len();
        let end = n + 3 * curves[1].len();
        let a = v[..n]
            .chunks_exact(3)
            .map(|p| std::array::from_fn(|k| p[k].clone()))
            .collect::<Vec<_>>();
        let b = v[n..end]
            .chunks_exact(3)
            .map(|p| std::array::from_fn(|k| p[k].clone()))
            .collect::<Vec<_>>();
        let Some(a) = point(&a, &v[end..end + 3], ctx)? else {
            return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));
        };
        let Some(b) = point(&b, &v[end + 3..], ctx)? else {
            return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));
        };
        for k in 0..2 {
            if a[k]
                .mul(&b[2], ctx)?
                .sub(&b[k].mul(&a[2], ctx)?, ctx)?
                .sign()
                != Sign::Zero
            {
                return Ok(BezierIdentity::Different);
            }
        }
        Ok(BezierIdentity::Equal)
    })()
    .and_then(|v| ctx.charge(0).map(|_| v));
    Ok(BezierIdentityDecision {
        outcome: result.unwrap_or_else(BezierIdentity::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}
