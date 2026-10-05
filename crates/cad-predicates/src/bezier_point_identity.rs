//! Exact equality of two original rational Bezier points at authored parameters.
use crate::{
    exact_inputs, Algebra, AuthoredScalar, BezierIdentity, BezierIdentityDecision, Expansion,
    InputError, LeafRef, PredicateContext, Reason, Sign,
};
fn point(
    ctx: &mut PredicateContext<'_>,
    poles: &[Expansion],
    parameter: &[Expansion],
) -> Result<Option<Vec<Expansion>>, Reason> {
    let low = parameter[0].sub(&parameter[1], ctx)?;
    let high = parameter[2].sub(&parameter[0], ctx)?;
    if parameter[2].sub(&parameter[1], ctx)?.sign() != Sign::Positive
        || low.sign() == Sign::Negative
        || high.sign() == Sign::Negative
        || poles.chunks_exact(3).any(|p| p[2].sign() != Sign::Positive)
    {
        return Ok(None);
    }
    let mut values = poles
        .chunks_exact(3)
        .map(|p| {
            Ok(vec![
                p[0].mul(&p[2], ctx)?,
                p[1].mul(&p[2], ctx)?,
                p[2].clone(),
            ])
        })
        .collect::<Result<Vec<_>, Reason>>()?;
    // Homogeneous de Casteljau with a common un-divided parameter denominator.
    for count in (1..values.len()).rev() {
        for i in 0..count {
            for k in 0..3 {
                values[i][k] = values[i][k]
                    .mul(&high, ctx)?
                    .add(&values[i + 1][k].mul(&low, ctx)?, ctx)?;
            }
        }
    }
    Ok(Some(values.remove(0)))
}
pub fn rational_bezier_uv_point_identity(
    ctx: &mut PredicateContext<'_>,
    curves: [&[[LeafRef; 3]]; 2],
    parameters: [[LeafRef; 3]; 2],
) -> Result<BezierIdentityDecision, InputError> {
    if curves.iter().any(|p| !(2..=33).contains(&p.len())) {
        return Err(InputError::InvalidInput(
            "UV point identity supports degree 1..32",
        ));
    }
    let refs: Vec<_> = curves
        .iter()
        .flat_map(|p| p.iter().flatten())
        .chain(parameters.iter().flatten())
        .copied()
        .collect();
    let values = refs
        .iter()
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    let result = (|| -> Result<BezierIdentity, Reason> {
        ctx.charge(values.len() as u64)?;
        let v = exact_inputs(&values, ctx)?;
        let n = curves[0].len() * 3;
        let m = curves[1].len() * 3;
        let Some(a) = point(ctx, &v[..n], &v[n + m..n + m + 3])? else {
            return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));
        };
        let Some(b) = point(ctx, &v[n..n + m], &v[n + m + 3..])? else {
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
