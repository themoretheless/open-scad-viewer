//! lib algorithm: evaluate.
use super::*;
pub fn evaluate(
    ctx: &mut PredicateContext<'_>,
    kind: Predicate,
    references: &[LeafRef],
) -> Result<Decision, InputError> {
    // Validate every reference before either a shortcut or resource refusal.
    let values = references
        .iter()
        .map(|r| ctx.resolve(*r).cloned())
        .collect::<Result<Vec<_>, _>>()?;
    let mut stage = Stage::Admission;
    let computed = (|| {
        ctx.charge(references.len() as u64)?;
        stage = Stage::Binary64;
        let fast = values
            .iter()
            .map(|v| match v {
                AuthoredScalar::Binary64Bits(bits) => Some(Approx::leaf(f64::from_bits(*bits))),
                AuthoredScalar::RationalConstant { .. } => None,
            })
            .collect::<Option<Vec<_>>>();
        if let Some(fast) = fast
            && let Some(sign) = expression(kind, &fast, ctx)?.sign()
        {
            return Ok(sign);
        }
        stage = Stage::Interval;
        let intervals = values
            .iter()
            .map(|v| Interval::authored(v, ctx))
            .collect::<Result<Vec<_>, _>>()?;
        if let Some(sign) = expression(kind, &intervals, ctx)?.sign() {
            return Ok(sign);
        }
        stage = Stage::ExactExpansion;
        let expanded = exact_inputs(&values, ctx)?;
        let sign = expression(kind, &expanded, ctx)?.sign();
        ctx.charge(0)?;
        Ok(sign)
    })();
    let computed = computed.and_then(|sign| ctx.charge(0).map(|_| sign));
    Ok(Decision {
        outcome: match computed {
            Ok(sign) => Outcome::Sign(sign),
            Err(reason) => Outcome::Indeterminate(reason),
        },
        stage,
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}
