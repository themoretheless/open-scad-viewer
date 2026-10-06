//! Sufficient exact identity of rational Bezier equations in normalized chord charts.
//! This proves equation identity, never root existence or unique root selection.
use crate::{
    Algebra, AuthoredScalar, Expansion, InputError, LeafRef, ParameterDecision, ParameterIdentity,
    PredicateContext, Reason, Sign, exact_inputs,
};
pub fn line_chart_cutter_identity(
    ctx: &mut PredicateContext<'_>,
    main: [[[LeafRef; 3]; 2]; 2],
    cutters: [&[[LeafRef; 3]]; 2],
    reversed: [bool; 2],
) -> Result<ParameterDecision, InputError> {
    normalized_bezier_chart_identity(ctx, [&main[0], &main[1]], cutters, reversed)
}
/// Sufficient exact identity of both rational Bezier equations under chord charts.
/// Reversal applies to the main parameter; cutter traversal stays unchanged.
pub fn normalized_bezier_chart_identity(
    ctx: &mut PredicateContext<'_>,
    main: [&[[LeafRef; 3]]; 2],
    cutters: [&[[LeafRef; 3]]; 2],
    reversed: [bool; 2],
) -> Result<ParameterDecision, InputError> {
    if cutters[0].len() != cutters[1].len()
        || !(2..=33).contains(&cutters[0].len())
        || main[0].len() != main[1].len()
        || !(2..=33).contains(&main[0].len())
    {
        return Err(InputError::InvalidInput(
            "Use equal Bezier degrees in 1..32",
        ));
    }
    let values = main
        .iter()
        .flat_map(|c| c.iter().flatten())
        .chain(cutters.iter().flat_map(|c| c.iter().flatten()))
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    let computed = (|| -> Result<ParameterIdentity, Reason> {
        let v = exact_inputs(&values, ctx)?;
        let main_count = main[0].len();
        let count = main_count + cutters[0].len();
        let mut charts = Vec::new();
        for side in 0..2 {
            let m = &v[3 * main_count * side..3 * main_count * (side + 1)];
            let last = 3 * (main_count - 1);
            if m[2].sign() != Sign::Positive {
                return Ok(ParameterIdentity::Indeterminate(Reason::MissingProof));
            }
            let dx = m[last].sub(&m[0], ctx)?;
            let dy = m[last + 1].sub(&m[1], ctx)?;
            let d2 = dx.mul(&dx, ctx)?.add(&dy.mul(&dy, ctx)?, ctx)?;
            if d2.sign() != Sign::Positive {
                return Ok(ParameterIdentity::Indeterminate(Reason::MissingProof));
            }
            let mut controls = Vec::new();
            for i in 0..count {
                let row = if i < main_count {
                    let index = if reversed[side] {
                        main_count - 1 - i
                    } else {
                        i
                    };
                    &m[3 * index..3 * index + 3]
                } else {
                    let offset = 6 * main_count + 3 * (side * cutters[0].len() + i - main_count);
                    &v[offset..offset + 3]
                };
                if row[2].sign() != Sign::Positive {
                    return Ok(ParameterIdentity::Indeterminate(Reason::MissingProof));
                }
                let x = row[0].sub(&m[0], ctx)?;
                let y = row[1].sub(&m[1], ctx)?;
                let mut along = x.mul(&dx, ctx)?.add(&y.mul(&dy, ctx)?, ctx)?;
                if reversed[side] {
                    along = d2.sub(&along, ctx)?;
                }
                let across = dx.mul(&y, ctx)?.sub(&dy.mul(&x, ctx)?, ctx)?;
                controls.push([along, across, row[2].clone()]);
            }
            charts.push((d2, controls));
        }
        let (a, b) = (&charts[0], &charts[1]);
        for i in 0..count {
            for axis in 0..2 {
                if a.1[i][axis]
                    .mul(&b.0, ctx)?
                    .sub(&b.1[i][axis].mul(&a.0, ctx)?, ctx)?
                    .sign()
                    != Sign::Zero
                {
                    return Ok(ParameterIdentity::Different);
                }
            }
            if a.1[i][2]
                .mul(&b.1[if i < main_count { 0 } else { main_count }][2], ctx)?
                .sub(
                    &b.1[i][2].mul(&a.1[if i < main_count { 0 } else { main_count }][2], ctx)?,
                    ctx,
                )?
                .sign()
                != Sign::Zero
            {
                return Ok(ParameterIdentity::Different);
            }
        }
        Ok(ParameterIdentity::Equal)
    })();
    Ok(ParameterDecision {
        outcome: computed.unwrap_or_else(ParameterIdentity::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Limits, SourceArena, ToleranceContext};
    fn check(change: f64, reverse: bool, work: u64) -> ParameterIdentity {
        let values = [
            0., 0., 1., 1., 0., 1., 2., 3., 2., 2., 5., 2., 0., -0.5, 1., 0.5, -0.5, 1., 1., 0.5,
            1., 3., 5., 1., 3., 4., 1., change, 3., 1.,
        ];
        let arena = SourceArena::authored(
            "line-chart-test",
            1,
            values
                .iter()
                .map(|v: &f64| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .unwrap();
        let main = std::array::from_fn(|side| {
            std::array::from_fn(|i| {
                std::array::from_fn(|axis| arena.leaf(6 * side + 3 * i + axis).unwrap())
            })
        });
        let cutters: [Vec<_>; 2] = std::array::from_fn(|side| {
            (0..3)
                .map(|i| {
                    std::array::from_fn(|axis| arena.leaf(12 + 9 * side + 3 * i + axis).unwrap())
                })
                .collect()
        });
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(
            &arena,
            &tolerance,
            Limits {
                max_work: work,
                ..Limits::default()
            },
            None,
        );
        line_chart_cutter_identity(&mut ctx, main, [&cutters[0], &cutters[1]], [false, reverse])
            .unwrap()
            .outcome
    }
    #[test]
    fn nonlinear_rational_main_requires_all_original_controls() {
        let evaluate = |middle: f64, work: u64| {
            // A curved quadratic main; both equations in the second chart are
            // translated and scaled. The main has nonuniform rational weights.
            let values: [f64; 30] = [
                0., 0., 1., 0.5, 0.25, 2., 1., 0., 1., 2., 3., 3., 3., middle, 6., 4., 3., 3., 0.5,
                -1., 1., 0.5, 1., 1., 3., 1., 2., 3., 5., 2.,
            ];
            let arena = SourceArena::authored(
                "nonlinear-chart",
                1,
                values
                    .iter()
                    .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                    .collect(),
            )
            .unwrap();
            let refs = |offset: usize, count: usize| {
                (0..count)
                    .map(|i| std::array::from_fn(|axis| arena.leaf(offset + 3 * i + axis).unwrap()))
                    .collect::<Vec<_>>()
            };
            let mains = [refs(0, 3), refs(9, 3)];
            let cutters = [refs(18, 2), refs(24, 2)];
            let tolerance = ToleranceContext::default_valid();
            let mut ctx = PredicateContext::new(
                &arena,
                &tolerance,
                Limits {
                    max_work: work,
                    ..Limits::default()
                },
                None,
            );
            normalized_bezier_chart_identity(
                &mut ctx,
                [&mains[0], &mains[1]],
                [&cutters[0], &cutters[1]],
                [false, false],
            )
            .unwrap()
            .outcome
        };
        assert_eq!(evaluate(3.5, 100000), ParameterIdentity::Equal);
        assert_eq!(evaluate(3.501, 100000), ParameterIdentity::Different);
        assert!(matches!(
            evaluate(3.5, 1),
            ParameterIdentity::Indeterminate(_)
        ));
    }
    #[test]
    fn exact_chart_identity_needs_controls_orientation_and_work() {
        assert_eq!(check(1., true, 100000), ParameterIdentity::Equal);
        assert_eq!(check(1.001, true, 100000), ParameterIdentity::Different);
        assert_eq!(check(1., false, 100000), ParameterIdentity::Different);
        assert!(matches!(
            check(1., true, 1),
            ParameterIdentity::Indeterminate(_)
        ));
    }
}
