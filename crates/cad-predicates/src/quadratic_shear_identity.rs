//! Exact coefficient identity for a biquadratic polynomial chart under inverse shear.
//! Caller must separately establish clamped degree-two layout and constant weights.
//! This is equation identity only; it does not authorize contacts or a Body.
use crate::{
    Algebra, AuthoredScalar, Expansion, InputError, LeafRef, ParameterDecision, ParameterIdentity,
    PredicateContext, Reason, Sign, exact_inputs,
};
pub fn quadratic_shear_chart_identity(
    ctx: &mut PredicateContext<'_>,
    controls: [[[LeafRef; 3]; 3]; 3],
    coefficient: LeafRef,
    driver: usize,
    height: usize,
) -> Result<ParameterDecision, InputError> {
    if driver > 2 || height > 2 || driver == height {
        return Err(InputError::InvalidInput(
            "Choose distinct shear coordinate axes",
        ));
    }
    let mut values = controls
        .iter()
        .flatten()
        .flatten()
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    values.push(ctx.resolve(coefficient)?.clone());
    let calculated = (|| -> Result<ParameterIdentity, Reason> {
        let v = exact_inputs(&values, ctx)?;
        let p = |i: usize, j: usize, k: usize| &v[9 * i + 3 * j + k];
        let k = &v[27];
        let a = p(0, 0, driver);
        let b = p(2, 0, driver).sub(a, ctx)?;
        let c = p(0, 2, driver).sub(a, ctx)?;
        let preimage = |i: usize, j: usize, ctx: &mut PredicateContext<'_>| {
            p(i, j, height).sub(
                &k.mul(&p(i, j, driver).mul(p(i, j, driver), ctx)?, ctx)?,
                ctx,
            )
        };
        let h = preimage(0, 0, ctx)?;
        let hu = preimage(2, 0, ctx)?.sub(&h, ctx)?;
        let hv = preimage(0, 2, ctx)?.sub(&h, ctx)?;
        for i in 0..3 {
            for j in 0..3 {
                for axis in 0..3 {
                    let expected = if axis == height {
                        let mut square = a.mul(a, ctx)?.mul(&Expansion::scalar(4.), ctx)?;
                        square = square.add(
                            &a.mul(&b, ctx)?
                                .mul(&Expansion::scalar(4. * i as f64), ctx)?,
                            ctx,
                        )?;
                        square = square.add(
                            &a.mul(&c, ctx)?
                                .mul(&Expansion::scalar(4. * j as f64), ctx)?,
                            ctx,
                        )?;
                        if i == 2 {
                            square = square
                                .add(&b.mul(&b, ctx)?.mul(&Expansion::scalar(4.), ctx)?, ctx)?;
                        }
                        if j == 2 {
                            square = square
                                .add(&c.mul(&c, ctx)?.mul(&Expansion::scalar(4.), ctx)?, ctx)?;
                        }
                        square = square.add(
                            &b.mul(&c, ctx)?
                                .mul(&Expansion::scalar(2. * (i * j) as f64), ctx)?,
                            ctx,
                        )?;
                        h.mul(&Expansion::scalar(4.), ctx)?
                            .add(&hu.mul(&Expansion::scalar(2. * i as f64), ctx)?, ctx)?
                            .add(&hv.mul(&Expansion::scalar(2. * j as f64), ctx)?, ctx)?
                            .add(&k.mul(&square, ctx)?, ctx)?
                    } else {
                        let start = p(0, 0, axis);
                        start
                            .mul(&Expansion::scalar(4.), ctx)?
                            .add(
                                &p(2, 0, axis)
                                    .sub(start, ctx)?
                                    .mul(&Expansion::scalar(2. * i as f64), ctx)?,
                                ctx,
                            )?
                            .add(
                                &p(0, 2, axis)
                                    .sub(start, ctx)?
                                    .mul(&Expansion::scalar(2. * j as f64), ctx)?,
                                ctx,
                            )?
                    };
                    if p(i, j, axis)
                        .mul(&Expansion::scalar(4.), ctx)?
                        .sub(&expected, ctx)?
                        .sign()
                        != Sign::Zero
                    {
                        return Ok(ParameterIdentity::Different);
                    }
                }
            }
        }
        Ok(ParameterIdentity::Equal)
    })();
    Ok(ParameterDecision {
        outcome: calculated.unwrap_or_else(ParameterIdentity::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Limits, SourceArena, ToleranceContext};
    #[test]
    fn inverse_shear_identity_checks_every_coefficient_and_work() {
        let check = |damage: f64, budget: u64| {
            let mut values = Vec::new();
            for i in 0..3 {
                for j in 0..3 {
                    values.extend([i as f64 / 2., j as f64 / 2., if i == 2 { 0.25 } else { 0. }]);
                }
            }
            values[14] += damage;
            values.push(0.25);
            let arena = SourceArena::authored(
                "shear-test",
                1,
                values
                    .iter()
                    .map(|v: &f64| AuthoredScalar::Binary64Bits(v.to_bits()))
                    .collect(),
            )
            .unwrap();
            let controls = std::array::from_fn(|i| {
                std::array::from_fn(|j| {
                    std::array::from_fn(|k| arena.leaf(9 * i + 3 * j + k).unwrap())
                })
            });
            let tolerance = ToleranceContext::default_valid();
            let mut ctx = PredicateContext::new(
                &arena,
                &tolerance,
                Limits {
                    max_work: budget,
                    ..Limits::default()
                },
                None,
            );
            quadratic_shear_chart_identity(&mut ctx, controls, arena.leaf(27).unwrap(), 0, 2)
                .unwrap()
                .outcome
        };
        assert_eq!(check(0., 100000), ParameterIdentity::Equal);
        assert_eq!(check(1e-12, 100000), ParameterIdentity::Different);
        assert!(matches!(check(0., 1), ParameterIdentity::Indeterminate(_)));
    }
}
