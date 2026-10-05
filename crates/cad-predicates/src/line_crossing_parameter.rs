//! Exact normalized parameter identity for transverse rational linear crossings.
use crate::{
    exact_inputs, Algebra, AuthoredScalar, ContextIdentity, Expansion, InputError, LeafRef,
    PredicateContext, Reason, Sign,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterIdentity {
    Equal,
    Different,
    Indeterminate(Reason),
}
pub struct ParameterDecision {
    pub outcome: ParameterIdentity,
    pub work_used: u64,
    pub context: ContextIdentity,
}
/// Main endpoint rows contain authored UV and positive rational weight.
/// Cutter rows contain authored UV; positive cutter weights do not change its line.
pub fn line_crossing_parameter_identity(
    ctx: &mut PredicateContext<'_>,
    main: [[[LeafRef; 3]; 2]; 2],
    cutter: [[[LeafRef; 2]; 2]; 2],
) -> Result<ParameterDecision, InputError> {
    line_crossing_parameter_identity_oriented(ctx, main, cutter, [false, false])
}
pub fn line_crossing_parameter_identity_oriented(
    ctx: &mut PredicateContext<'_>,
    main: [[[LeafRef; 3]; 2]; 2],
    cutter: [[[LeafRef; 2]; 2]; 2],
    reversed: [bool; 2],
) -> Result<ParameterDecision, InputError> {
    line_parameter_impl(ctx, main, cutter, reversed, None)
}
/// Compare original linear crossing parameters after exact affine carrier maps.
pub fn line_crossing_parameter_identity_affine(
    ctx: &mut PredicateContext<'_>,
    main: [[[LeafRef; 3]; 2]; 2],
    cutter: [[[LeafRef; 2]; 2]; 2],
    ranges: [[[LeafRef; 2]; 2]; 2],
) -> Result<ParameterDecision, InputError> {
    line_parameter_impl(ctx, main, cutter, [false, false], Some(ranges))
}
fn line_parameter_impl(
    ctx: &mut PredicateContext<'_>,
    main: [[[LeafRef; 3]; 2]; 2],
    cutter: [[[LeafRef; 2]; 2]; 2],
    reversed: [bool; 2],
    ranges: Option<[[[LeafRef; 2]; 2]; 2]>,
) -> Result<ParameterDecision, InputError> {
    let refs: Vec<_> = (0..2)
        .flat_map(|i| {
            main[i]
                .iter()
                .flatten()
                .chain(cutter[i].iter().flatten())
                .copied()
        })
        .chain(ranges.iter().flatten().flatten().flatten().copied())
        .collect();
    let values = refs
        .iter()
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    let computed = (|| -> Result<ParameterIdentity, Reason> {
        let all = exact_inputs(&values, ctx)?;
        let cross = |a: &[Expansion; 2], b: &[Expansion; 2], ctx: &mut PredicateContext<'_>| {
            a[0].mul(&b[1], ctx)?.sub(&a[1].mul(&b[0], ctx)?, ctx)
        };
        let mut ratios = Vec::new();
        for i in 0..2 {
            let v = &all[10 * i..10 * i + 10];
            if v[2].sign() != Sign::Positive || v[5].sign() != Sign::Positive {
                return Ok(ParameterIdentity::Indeterminate(Reason::MissingProof));
            }
            let r = [v[3].sub(&v[0], ctx)?, v[4].sub(&v[1], ctx)?];
            let s = [v[8].sub(&v[6], ctx)?, v[9].sub(&v[7], ctx)?];
            let q = [v[6].sub(&v[0], ctx)?, v[7].sub(&v[1], ctx)?];
            let d = cross(&r, &s, ctx)?;
            let n = cross(&q, &s, ctx)?;
            if d.sign() == Sign::Zero {
                return Ok(ParameterIdentity::Indeterminate(Reason::MissingProof));
            }
            // alpha = n/d is the Cartesian chord fraction. Original rational
            // Bezier parameter x satisfies alpha=x*w1/((1-x)*w0+x*w1).
            let numerator = n.mul(&v[2], ctx)?;
            let denominator = d
                .mul(&v[5], ctx)?
                .add(&v[2].sub(&v[5], ctx)?.mul(&n, ctx)?, ctx)?;
            if denominator.sign() == Sign::Zero {
                return Ok(ParameterIdentity::Indeterminate(Reason::MissingProof));
            }
            let numerator = if reversed[i] {
                denominator.sub(&numerator, ctx)?
            } else {
                numerator
            };
            let ratio = if ranges.is_some() {
                let Some(r) =
                    affine_fraction(&numerator, &denominator, &all[20 + 4 * i..24 + 4 * i], ctx)?
                else {
                    return Ok(ParameterIdentity::Indeterminate(Reason::MissingProof));
                };
                r
            } else {
                (numerator, denominator)
            };
            ratios.push(ratio);
        }
        let difference = ratios[0]
            .0
            .mul(&ratios[1].1, ctx)?
            .sub(&ratios[1].0.mul(&ratios[0].1, ctx)?, ctx)?;
        ctx.charge(0)?;
        Ok(if difference.sign() == Sign::Zero {
            ParameterIdentity::Equal
        } else {
            ParameterIdentity::Different
        })
    })();
    Ok(ParameterDecision {
        outcome: computed.unwrap_or_else(ParameterIdentity::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}
/// Compare a reflected pair without forming rounded (lo + hi - t).
pub fn reflected_parameter_identity(
    ctx: &mut PredicateContext<'_>,
    values: [LeafRef; 4],
) -> Result<ParameterDecision, InputError> {
    let values = values
        .iter()
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    let computed = (|| -> Result<ParameterIdentity, Reason> {
        let v = exact_inputs(&values, ctx)?;
        let diff = v[0].add(&v[1], ctx)?.sub(&v[2], ctx)?.sub(&v[3], ctx)?;
        ctx.charge(0)?;
        Ok(if diff.sign() == Sign::Zero {
            ParameterIdentity::Equal
        } else {
            ParameterIdentity::Different
        })
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
    fn check(second_x: f64, second_weight: f64, budget: u64) -> ParameterIdentity {
        let values = vec![
            0.,
            0.,
            1.,
            1.,
            0.,
            1.,
            0.5,
            -1.,
            0.5,
            1.,
            0.,
            0.,
            1.,
            0.,
            1.,
            second_weight,
            -1.,
            second_x,
            1.,
            second_x,
        ];
        let arena = SourceArena::authored(
            "root-param",
            1,
            values
                .iter()
                .map(|v: &f64| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .unwrap();
        let main = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                std::array::from_fn(|k| arena.leaf(10 * i + 3 * j + k).unwrap())
            })
        });
        let cutter = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                std::array::from_fn(|k| arena.leaf(10 * i + 6 + 2 * j + k).unwrap())
            })
        });
        let tol = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(
            &arena,
            &tol,
            Limits {
                max_work: budget,
                ..Limits::default()
            },
            None,
        );
        line_crossing_parameter_identity(&mut ctx, main, cutter)
            .unwrap()
            .outcome
    }
    #[test]
    fn reflected_parameters_are_compared_without_rounding_the_sum() {
        let check = |a: f64, b: f64| {
            let arena = SourceArena::authored(
                "reflect",
                1,
                [a, b, 0., 1.]
                    .iter()
                    .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                    .collect(),
            )
            .unwrap();
            let tol = ToleranceContext::default_valid();
            let mut ctx = PredicateContext::new(&arena, &tol, Limits::default(), None);
            reflected_parameter_identity(&mut ctx, std::array::from_fn(|i| arena.leaf(i).unwrap()))
                .unwrap()
                .outcome
        };
        assert_eq!(check(0.25, 0.75), ParameterIdentity::Equal);
        assert_eq!(check(0.25, 0.75 + 1e-12), ParameterIdentity::Different);
        assert_eq!(check(0.1, 0.9), ParameterIdentity::Different);
    }
    #[test]
    fn root_identity_is_exact_and_tracks_rational_parameter_not_chord_fraction() {
        assert_eq!(check(0.5, 1., 1_000_000), ParameterIdentity::Equal);
        assert_eq!(
            check(0.5 + 1e-12, 1., 1_000_000),
            ParameterIdentity::Different
        );
        assert_eq!(check(0.5, 2., 1_000_000), ParameterIdentity::Different);
        assert_eq!(check(0.75, 3., 1_000_000), ParameterIdentity::Equal);
        assert!(matches!(
            check(0.5, 0., 1_000_000),
            ParameterIdentity::Indeterminate(_)
        ));
        // alpha=2/3 with weights 1,2 has original parameter 1/2.
        // Binary64 2/3 is not exact 2/3: equality must not be invented.
        assert_eq!(check(2. / 3., 2., 1_000_000), ParameterIdentity::Different);
        assert!(matches!(
            check(0.5, 1., 0),
            ParameterIdentity::Indeterminate(_)
        ));
    }
}

/// Compare normalized traversal across different original source domains.
/// Each row is [parameter, lower, upper]. Positive widths and containment are
/// checked with exact expansions; no division or rounded affine remap is used.
pub fn normalized_parameter_identity(
    ctx: &mut PredicateContext<'_>,
    values: [[LeafRef; 3]; 2],
    reversed: [bool; 2],
) -> Result<ParameterDecision, InputError> {
    let values = values
        .iter()
        .flatten()
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    let computed = (|| -> Result<ParameterIdentity, Reason> {
        let v = exact_inputs(&values, ctx)?;
        let mut ratios = Vec::new();
        for i in 0..2 {
            let row = &v[3 * i..3 * i + 3];
            let width = row[2].sub(&row[1], ctx)?;
            let low = row[0].sub(&row[1], ctx)?;
            let high = row[2].sub(&row[0], ctx)?;
            if width.sign() != Sign::Positive
                || low.sign() == Sign::Negative
                || high.sign() == Sign::Negative
            {
                return Ok(ParameterIdentity::Indeterminate(Reason::MissingProof));
            }
            ratios.push((if reversed[i] { high } else { low }, width));
        }
        let difference = ratios[0]
            .0
            .mul(&ratios[1].1, ctx)?
            .sub(&ratios[1].0.mul(&ratios[0].1, ctx)?, ctx)?;
        ctx.charge(0)?;
        Ok(if difference.sign() == Sign::Zero {
            ParameterIdentity::Equal
        } else {
            ParameterIdentity::Different
        })
    })();
    Ok(ParameterDecision {
        outcome: computed.unwrap_or_else(ParameterIdentity::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}
#[cfg(test)]
mod normalized_tests {
    use super::*;
    use crate::{Limits, SourceArena, ToleranceContext};
    fn run(v: [[f64; 3]; 2], reverse: [bool; 2], budget: u64) -> ParameterIdentity {
        let source = SourceArena::authored(
            "normalized-source-domains",
            1,
            v.iter()
                .flatten()
                .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
                .collect(),
        )
        .unwrap();
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(
            &source,
            &tolerance,
            Limits {
                max_work: budget,
                ..Limits::default()
            },
            None,
        );
        normalized_parameter_identity(
            &mut ctx,
            std::array::from_fn(|i| std::array::from_fn(|k| source.leaf(3 * i + k).unwrap())),
            reverse,
        )
        .unwrap()
        .outcome
    }
    #[test]
    fn domains_orientation_and_false_rounded_ratios() {
        assert_eq!(
            run([[2.5, 2., 4.], [12., 10., 18.]], [false, false], 1_000_000),
            ParameterIdentity::Equal
        );
        assert_eq!(
            run([[2.5, 2., 4.], [16., 10., 18.]], [false, true], 1_000_000),
            ParameterIdentity::Equal
        );
        assert_eq!(
            run([[2.5, 2., 4.], [12., 10., 18.]], [false, true], 1_000_000),
            ParameterIdentity::Different
        );
        assert_eq!(
            run([[0.1, 0., 1.], [0.9, 0., 1.]], [false, true], 1_000_000),
            ParameterIdentity::Different
        );
        assert_eq!(
            run(
                [[2.5, 2., 4.], [12. + 1e-12, 10., 18.]],
                [false, false],
                1_000_000
            ),
            ParameterIdentity::Different
        );
        assert_eq!(
            run([[2.5, 2., 4.], [2.5, 2., 6.]], [false, false], 1_000_000),
            ParameterIdentity::Different
        );
        assert!(matches!(
            run([[2.5, 2., 4.], [12., 10., 18.]], [false, false], 0),
            ParameterIdentity::Indeterminate(_)
        ));
        assert!(matches!(
            run([[2.5, 2., 2.], [12., 10., 18.]], [false, false], 1_000_000),
            ParameterIdentity::Indeterminate(_)
        ));
        assert!(matches!(
            run([[1.5, 2., 4.], [12., 10., 18.]], [false, false], 1_000_000),
            ParameterIdentity::Indeterminate(_)
        ));
    }
}

// Exact normalized fraction composed with an affine rational-endpoint map.
pub(crate) fn affine_fraction(
    n: &Expansion,
    d: &Expansion,
    range: &[Expansion],
    ctx: &mut PredicateContext<'_>,
) -> Result<Option<(Expansion, Expansion)>, Reason> {
    if d.sign() == Sign::Zero {
        return Ok(None);
    }
    let (n, d) = if d.sign() == Sign::Negative {
        (
            Expansion::scalar(0.).sub(n, ctx)?,
            Expansion::scalar(0.).sub(d, ctx)?,
        )
    } else {
        (n.clone(), d.clone())
    };
    if n.sign() == Sign::Negative || d.sub(&n, ctx)?.sign() == Sign::Negative {
        return Ok(None);
    }
    for i in 0..2 {
        if range[2 * i + 1].sign() != Sign::Positive
            || range[2 * i].sign() == Sign::Negative
            || range[2 * i + 1].sub(&range[2 * i], ctx)?.sign() == Sign::Negative
        {
            return Ok(None);
        }
    }
    let a = range[0].mul(&range[3], ctx)?;
    let b = range[2].mul(&range[1], ctx)?;
    if b.sub(&a, ctx)?.sign() == Sign::Zero {
        return Ok(None);
    }
    let numerator = a.mul(&d.sub(&n, ctx)?, ctx)?.add(&b.mul(&n, ctx)?, ctx)?;
    let denominator = range[1].mul(&range[3], ctx)?.mul(&d, ctx)?;
    Ok(Some((numerator, denominator)))
}
/// Fixed parameters from two original domains, each with its own exact map.
pub fn affine_parameter_identity(
    ctx: &mut PredicateContext<'_>,
    parameters: [[LeafRef; 3]; 2],
    ranges: [[[LeafRef; 2]; 2]; 2],
) -> Result<ParameterDecision, InputError> {
    let values = parameters
        .iter()
        .flatten()
        .chain(ranges.iter().flatten().flatten())
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    let computed = (|| -> Result<ParameterIdentity, Reason> {
        let v = exact_inputs(&values, ctx)?;
        let mut ratios = Vec::new();
        for i in 0..2 {
            let row = &v[3 * i..3 * i + 3];
            let width = row[2].sub(&row[1], ctx)?;
            if width.sign() != Sign::Positive {
                return Ok(ParameterIdentity::Indeterminate(Reason::MissingProof));
            }
            let Some(r) = affine_fraction(
                &row[0].sub(&row[1], ctx)?,
                &width,
                &v[6 + 4 * i..10 + 4 * i],
                ctx,
            )?
            else {
                return Ok(ParameterIdentity::Indeterminate(Reason::MissingProof));
            };
            ratios.push(r);
        }
        let delta = ratios[0]
            .0
            .mul(&ratios[1].1, ctx)?
            .sub(&ratios[1].0.mul(&ratios[0].1, ctx)?, ctx)?;
        ctx.charge(0)?;
        Ok(if delta.sign() == Sign::Zero {
            ParameterIdentity::Equal
        } else {
            ParameterIdentity::Different
        })
    })();
    Ok(ParameterDecision {
        outcome: computed.unwrap_or_else(ParameterIdentity::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}
