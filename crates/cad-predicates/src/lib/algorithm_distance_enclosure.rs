//! lib algorithm: distance_enclosure.
use super::*;
pub fn distance_enclosure(
    ctx: &mut PredicateContext<'_>,
    p: &[LeafRef],
    q: &[LeafRef],
) -> Result<ResidualOutcome, InputError> {
    let references = distance_references(ctx, p, q)?;
    let values = references
        .iter()
        .map(|r| ctx.resolve(*r).cloned())
        .collect::<Result<Vec<_>, _>>()?;
    let computed = (|| {
        ctx.charge(references.len() as u64)?;
        let values = values
            .iter()
            .map(|v| Interval::authored(v, ctx))
            .collect::<Result<Vec<_>, _>>()?;
        let mut sum = Interval::point(0.);
        for i in 0..p.len() {
            let difference = values[i].sub(&values[p.len() + i], ctx)?;
            sum = sum.add(&difference.mul(&difference, ctx)?, ctx)?;
        }
        // Intersect with the independently known nonnegativity of a sum of
        // squares. This is a mathematical enclosure refinement, not snapping.
        sum.lo = sum.lo.max(0.);
        ctx.charge(8)?;
        if sum.distance_width() > ctx.tolerance.spec.max_entity_error {
            return Err(Reason::PrecisionExhausted);
        }
        ctx.charge(0)?;
        Ok(sum)
    })();
    Ok(match computed {
        Ok(squared) => ResidualOutcome::Proven(ProvenResidual {
            squared,
            context: ctx.identity(),
        }),
        Err(reason) => ResidualOutcome::Indeterminate(reason),
    })
}
