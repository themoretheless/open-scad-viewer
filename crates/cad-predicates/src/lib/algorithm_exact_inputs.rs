//! lib algorithm: exact_inputs.
use super::*;
pub fn exact_inputs(
    values: &[AuthoredScalar],
    ctx: &mut PredicateContext<'_>,
) -> Result<Vec<Expansion>, Reason> {
    // All coordinates share a positive denominator product. No division or
    // rounded rational center enters the exact expression. Products may refuse
    // if the finite expansion envelope cannot contain this particular matrix.
    let mut numerators = Vec::with_capacity(values.len());
    let mut denominators = Vec::with_capacity(values.len());
    for value in values {
        ctx.charge(1)?;
        match *value {
            AuthoredScalar::Binary64Bits(bits) => {
                numerators.push(Expansion::scalar(f64::from_bits(bits)));
                denominators.push(Expansion::scalar(1.));
            }
            AuthoredScalar::RationalConstant {
                numerator,
                denominator,
            } => {
                let n = Expansion::integer(numerator.unsigned_abs(), ctx)?;
                numerators.push(if numerator < 0 { n.negated() } else { n });
                denominators.push(Expansion::integer(denominator, ctx)?);
            }
        }
    }
    for (i, numerator) in numerators.iter_mut().enumerate() {
        for (j, denominator) in denominators.iter().enumerate() {
            if i != j && !denominator.is_one() {
                *numerator = numerator.mul(denominator, ctx)?;
            }
        }
    }
    Expansion::normalize_common(&mut numerators, ctx)?;
    Ok(numerators)
}
