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
    line_parameter_impl(ctx, main, cutter, reversed, None, None)
}
/// Compare original linear crossing parameters after exact affine carrier maps.
pub fn line_crossing_parameter_identity_affine(
    ctx: &mut PredicateContext<'_>,
    main: [[[LeafRef; 3]; 2]; 2],
    cutter: [[[LeafRef; 2]; 2]; 2],
    ranges: [[[LeafRef; 2]; 2]; 2],
) -> Result<ParameterDecision, InputError> {
    line_parameter_impl(ctx, main, cutter, [false, false], Some(ranges), None)
}
/// Compare original transverse linear roots after positive Mobius maps.
/// Rational root fractions are never converted to binary64 parameters.
pub fn line_crossing_parameter_identity_projective(
    ctx: &mut PredicateContext<'_>,
    main: [[[LeafRef; 3]; 2]; 2],
    cutter: [[[LeafRef; 2]; 2]; 2],
    ranges: [[[LeafRef; 2]; 2]; 2],
    weights: [[LeafRef; 2]; 2],
) -> Result<ParameterDecision, InputError> {
    line_parameter_impl(ctx, main, cutter, [false;2], Some(ranges), Some(weights))
}
fn line_parameter_impl(
    ctx: &mut PredicateContext<'_>,
    main: [[[LeafRef; 3]; 2]; 2],
    cutter: [[[LeafRef; 2]; 2]; 2],
    reversed: [bool; 2],
    ranges: Option<[[[LeafRef; 2]; 2]; 2]>,
    weights: Option<[[LeafRef;2];2]>,
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
        .chain(weights.iter().flatten().flatten().copied())
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
            let (numerator,denominator)=if weights.is_some() {
                let w=&all[28+2*i..30+2*i];
                if w.iter().any(|v|v.sign()!=Sign::Positive) {
                    return Ok(ParameterIdentity::Indeterminate(Reason::MissingProof));
                }
                let n=numerator.mul(&w[1],ctx)?;
                let d=denominator.sub(&numerator,ctx)?.mul(&w[0],ctx)?.add(&n,ctx)?;
                if d.sign()==Sign::Zero {return Ok(ParameterIdentity::Indeterminate(Reason::MissingProof));}
                (n,d)
            }else{(numerator,denominator)};
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
        // Structural cancellation is exact for finite represented binary64
        // pairs, including subnormals. Do not manufacture a huge common scale
        // for (-a)+a-(-t)-t, or round any addition to claim an identity.
        let binary = values.iter().map(|v|match v {
            AuthoredScalar::Binary64Bits(bits)=>Some(f64::from_bits(*bits)),
            _=>None,
        }).collect::<Option<Vec<_>>>();
        if let Some(v)=binary.filter(|v|v.iter().all(|x|x.is_finite())) {
            if (v[0]==v[2]&&v[1]==v[3])||(v[0]==v[3]&&v[1]==v[2])
                ||(v[0]==-v[1]&&v[2]==-v[3]) {
                ctx.charge(4)?;
                return Ok(ParameterIdentity::Equal);
            }
        }
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
    #[test]
    fn reflected_extreme_pairs_are_exact_and_still_charge_the_shared_budget() {
        let inspect=|values:[f64;4],budget| {
            let source=SourceArena::authored("reflected-cancellation",1,
                values.iter().map(|v|AuthoredScalar::Binary64Bits(v.to_bits())).collect()).unwrap();
            let tolerance=ToleranceContext::default_valid();
            let mut ctx=PredicateContext::new(&source,&tolerance,Limits{max_work:budget,..Limits::default()},None);
            reflected_parameter_identity(&mut ctx,std::array::from_fn(|i|source.leaf(i).unwrap())).unwrap()
        };
        let values=[-2.,2.,f64::from_bits(1),-f64::from_bits(1)];
        let proof=inspect(values,1000000);assert_eq!(proof.outcome,ParameterIdentity::Equal);assert!(proof.work_used>0);
        assert_ne!(inspect(values,proof.work_used-1).outcome,ParameterIdentity::Equal);
        assert_ne!(inspect(values,0).outcome,ParameterIdentity::Equal);
        assert_eq!(inspect([1e16,1.,1e16,0.],1000000).outcome,ParameterIdentity::Different);
    }
    #[test]
    fn projective_linear_crossings_keep_one_third_exact() {
        let evaluate=|weight:f64,budget:u64| {
            let values=[1.,0.,1.,1.,1.,1.,-1.,0.5,2.,0.5,
                0.,0.,1.,0.,1.,1.,-1.,0.,2.,1.,
                0.,1.,1.,1.,0.,1.,1.,1.,1.,1.,1.,weight];
            let arena=SourceArena::authored("projective-line-roots",1,values.into_iter().map(|v|AuthoredScalar::Binary64Bits(v.to_bits())).collect()).unwrap();
            let main=std::array::from_fn(|i|std::array::from_fn(|j|std::array::from_fn(|k|arena.leaf(10*i+3*j+k).unwrap())));
            let cutter=std::array::from_fn(|i|std::array::from_fn(|j|std::array::from_fn(|k|arena.leaf(10*i+6+2*j+k).unwrap())));
            let ranges=std::array::from_fn(|i|std::array::from_fn(|j|std::array::from_fn(|k|arena.leaf(20+4*i+2*j+k).unwrap())));
            let weights=std::array::from_fn(|i|std::array::from_fn(|j|arena.leaf(28+2*i+j).unwrap()));
            let tolerance=ToleranceContext::default_valid();
            let mut ctx=PredicateContext::new(&arena,&tolerance,Limits{max_work:budget,..Limits::default()},None);
            line_crossing_parameter_identity_projective(&mut ctx,main,cutter,ranges,weights).unwrap()
        };
        let proof=evaluate(2.,1000000);assert_eq!(proof.outcome,ParameterIdentity::Equal);
        assert_eq!(evaluate(1.,1000000).outcome,ParameterIdentity::Different);
        assert!(matches!(evaluate(0.,1000000).outcome,ParameterIdentity::Indeterminate(_)));
        assert_eq!(evaluate(2.,proof.work_used-1).outcome,ParameterIdentity::Indeterminate(Reason::ResourceLimit));
    }
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

/// Compare source parameters after positive rational-linear carrier maps.
/// The normalized source fractions and mapped values remain unreduced exact
/// numerator/denominator expressions throughout the comparison.
pub fn projective_parameter_identity(
    ctx: &mut PredicateContext<'_>,
    parameters: [[LeafRef; 3]; 2],
    ranges: [[[LeafRef; 2]; 2]; 2],
    weights: [[LeafRef; 2]; 2],
) -> Result<ParameterDecision, InputError> {
    let values=parameters.iter().flatten().chain(ranges.iter().flatten().flatten())
        .chain(weights.iter().flatten()).map(|&r|ctx.resolve(r).cloned())
        .collect::<Result<Vec<AuthoredScalar>,_>>()?;
    let computed=(||->Result<ParameterIdentity,Reason>{
        let v=exact_inputs(&values,ctx)?;
        let mut ratios=Vec::new();
        for i in 0..2 {
            let row=&v[3*i..3*i+3];
            let d=row[2].sub(&row[1],ctx)?;
            let n=row[0].sub(&row[1],ctx)?;
            let high=d.sub(&n,ctx)?;
            let w=&v[14+2*i..16+2*i];
            if d.sign()!=Sign::Positive || n.sign()==Sign::Negative || high.sign()==Sign::Negative
                || w.iter().any(|x|x.sign()!=Sign::Positive) {
                return Ok(ParameterIdentity::Indeterminate(Reason::MissingProof));
            }
            let numerator=n.mul(&w[1],ctx)?;
            let denominator=high.mul(&w[0],ctx)?.add(&numerator,ctx)?;
            let Some(r)=affine_fraction(&numerator,&denominator,&v[6+4*i..10+4*i],ctx)? else {
                return Ok(ParameterIdentity::Indeterminate(Reason::MissingProof));
            };
            ratios.push(r);
        }
        let delta=ratios[0].0.mul(&ratios[1].1,ctx)?.sub(&ratios[1].0.mul(&ratios[0].1,ctx)?,ctx)?;
        ctx.charge(0)?;
        Ok(if delta.sign()==Sign::Zero {ParameterIdentity::Equal}else{ParameterIdentity::Different})
    })();
    Ok(ParameterDecision{outcome:computed.unwrap_or_else(ParameterIdentity::Indeterminate),work_used:ctx.work_used(),context:ctx.identity()})
}
/// Source parameter rows: numerator, positive denominator, domain low, domain high.
pub fn projective_parameter_fraction_identity(
    ctx: &mut PredicateContext<'_>,
    parameters: [[LeafRef; 4]; 2],
    ranges: [[[LeafRef; 2]; 2]; 2],
    weights: [[LeafRef; 2]; 2],
) -> Result<ParameterDecision, InputError> {
    let mut values=parameters.iter().flatten().chain(ranges.iter().flatten().flatten())
        .chain(weights.iter().flatten()).map(|&r|ctx.resolve(r).cloned())
        .collect::<Result<Vec<AuthoredScalar>,_>>()?;
    values.push(AuthoredScalar::Binary64Bits(1_f64.to_bits()));
    let computed=(||->Result<ParameterIdentity,Reason>{
        let v=exact_inputs(&values,ctx)?;
        let mut ratios=Vec::new();
        for i in 0..2 {
            let row=&v[4*i..4*i+4];
            if row[1].sign()!=Sign::Positive {return Ok(ParameterIdentity::Indeterminate(Reason::MissingProof));}
            let d=row[3].sub(&row[2],ctx)?.mul(&row[1],ctx)?;
            let n=row[0].mul(v.last().unwrap(),ctx)?.sub(&row[2].mul(&row[1],ctx)?,ctx)?;
            let high=d.sub(&n,ctx)?;
            let w=&v[16+2*i..18+2*i];
            if d.sign()!=Sign::Positive || n.sign()==Sign::Negative || high.sign()==Sign::Negative
                || w.iter().any(|x|x.sign()!=Sign::Positive) {
                return Ok(ParameterIdentity::Indeterminate(Reason::MissingProof));
            }
            let numerator=n.mul(&w[1],ctx)?;
            let denominator=high.mul(&w[0],ctx)?.add(&numerator,ctx)?;
            let Some(r)=affine_fraction(&numerator,&denominator,&v[8+4*i..12+4*i],ctx)? else {
                return Ok(ParameterIdentity::Indeterminate(Reason::MissingProof));
            };
            ratios.push(r);
        }
        let delta=ratios[0].0.mul(&ratios[1].1,ctx)?.sub(&ratios[1].0.mul(&ratios[0].1,ctx)?,ctx)?;
        ctx.charge(0)?;
        Ok(if delta.sign()==Sign::Zero {ParameterIdentity::Equal}else{ParameterIdentity::Different})
    })();
    Ok(ParameterDecision{outcome:computed.unwrap_or_else(ParameterIdentity::Indeterminate),work_used:ctx.work_used(),context:ctx.identity()})
}
#[cfg(test)]
mod projective_parameter_tests {
    use super::*;
    use crate::{Limits,SourceArena,ToleranceContext};
    #[test]
    fn fraction_parameters_keep_authored_domains_and_exact_map_identity() {
        let evaluate=|second:f64,denominator:f64,budget:u64| {
            let parameters=[[38.,3.,10.,18.],[second,denominator,10.,18.]];
            let ranges=[[[0.,1.],[1.,1.]];2];let weights=[[1.,1.],[1.,2.]];
            let values:Vec<f64>=parameters.iter().flatten().chain(ranges.iter().flatten().flatten()).chain(weights.iter().flatten()).copied().collect();
            let arena=SourceArena::authored("fraction-parameter-domains",1,values.into_iter().map(|v|AuthoredScalar::Binary64Bits(v.to_bits())).collect()).unwrap();
            let p=std::array::from_fn(|i|std::array::from_fn(|j|arena.leaf(4*i+j).unwrap()));
            let r=std::array::from_fn(|i|std::array::from_fn(|j|std::array::from_fn(|k|arena.leaf(8+4*i+2*j+k).unwrap())));
            let w=std::array::from_fn(|i|std::array::from_fn(|j|arena.leaf(16+2*i+j).unwrap()));
            let tolerance=ToleranceContext::default_valid();
            let mut ctx=PredicateContext::new(&arena,&tolerance,Limits{max_work:budget,..Limits::default()},None);
            projective_parameter_fraction_identity(&mut ctx,p,r,w).unwrap()
        };
        let proof=evaluate(58.,5.,1000000);assert_eq!(proof.outcome,ParameterIdentity::Equal);
        assert_eq!(evaluate(59.,5.,1000000).outcome,ParameterIdentity::Different);
        assert!(matches!(evaluate(58.,0.,1000000).outcome,ParameterIdentity::Indeterminate(_)));
        assert_eq!(evaluate(58.,5.,proof.work_used-1).outcome,ParameterIdentity::Indeterminate(Reason::ResourceLimit));
    }
    fn check(parameters:[[f64;3];2],ranges:[[[f64;2];2];2],weights:[[f64;2];2],budget:u64)->ParameterDecision {
        let source=SourceArena::authored("projective-parameters",1,
            parameters.iter().flatten().chain(ranges.iter().flatten().flatten()).chain(weights.iter().flatten())
            .map(|v|AuthoredScalar::Binary64Bits(v.to_bits())).collect()).unwrap();
        let p=std::array::from_fn(|i|std::array::from_fn(|j|source.leaf(3*i+j).unwrap()));
        let r=std::array::from_fn(|i|std::array::from_fn(|j|std::array::from_fn(|k|source.leaf(6+4*i+2*j+k).unwrap())));
        let w=std::array::from_fn(|i|std::array::from_fn(|j|source.leaf(14+2*i+j).unwrap()));
        let tolerance=ToleranceContext::default_valid();
        let mut ctx=PredicateContext::new(&source,&tolerance,Limits{max_work:budget,..Limits::default()},None);
        projective_parameter_identity(&mut ctx,p,r,w).unwrap()
    }
    #[test]
    fn rational_parameters_are_compared_without_rounded_map_values() {
        let parameters=[[1.,0.,3.],[0.5,0.,1.]];
        let ranges=[[[0.,1.],[1.,1.]];2];let weights=[[1.,2.],[1.,1.]];
        let full=check(parameters,ranges,weights,1000000);
        assert_eq!(full.outcome,ParameterIdentity::Equal);
        assert_eq!(check([[1./3.,0.,1.],parameters[1]],ranges,weights,1000000).outcome,ParameterIdentity::Different);
        assert_eq!(check(parameters,[[[1.,1.],[0.,1.]],ranges[1]],weights,1000000).outcome,ParameterIdentity::Equal);
        assert_eq!(check(parameters,[[[1.,4.],[3.,4.]],ranges[1]],weights,1000000).outcome,ParameterIdentity::Equal);
        for budget in [0,full.work_used-1] {
            let r=check(parameters,ranges,weights,budget);
            assert_eq!(r.outcome,ParameterIdentity::Indeterminate(Reason::ResourceLimit));assert!(r.work_used<=budget);
        }
        for weights in [[[0.,2.],[1.,1.]],[[-1.,2.],[1.,1.]]] {
            assert_eq!(check(parameters,ranges,weights,1000000).outcome,ParameterIdentity::Indeterminate(Reason::MissingProof));
        }
        assert_eq!(check([[-1.,0.,3.],parameters[1]],ranges,weights,1000000).outcome,ParameterIdentity::Indeterminate(Reason::MissingProof));
    }
}
