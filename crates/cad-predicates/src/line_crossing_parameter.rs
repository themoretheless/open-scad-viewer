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
    let refs: Vec<_> = (0..2)
        .flat_map(|i| {
            main[i]
                .iter()
                .flatten()
                .chain(cutter[i].iter().flatten())
                .copied()
        })
        .collect();
    let values = refs
        .iter()
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    let computed = (|| -> Result<ParameterIdentity, Reason> {
        let v = exact_inputs(&values, ctx)?;
        let cross = |a: &[Expansion; 2], b: &[Expansion; 2], ctx: &mut PredicateContext<'_>| {
            a[0].mul(&b[1], ctx)?.sub(&a[1].mul(&b[0], ctx)?, ctx)
        };
        let mut ratios = Vec::new();
        for i in 0..2 {
            let v = &v[10 * i..10 * i + 10];
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
            ratios.push((numerator, denominator));
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
