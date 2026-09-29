//! Exact identity of positive-weight rational Bezier curves under the same
//! normalized parameter. Polynomial equality is stronger than sampled proximity.
use crate::{
    Algebra, AuthoredScalar, ContextIdentity, Expansion, InputError, LeafRef, PredicateContext,
    Reason, Sign, exact_inputs,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BezierIdentity {
    Equal,
    Different,
    Indeterminate(Reason),
}
#[derive(Clone, Debug)]
pub struct BezierIdentityDecision {
    pub outcome: BezierIdentity,
    pub work_used: u64,
    pub context: ContextIdentity,
}
/// Each row contains authored Euclidean XYZ followed by its positive weight.
/// Curves may have different degrees (1..32). Reversal is represented by the
/// caller reversing one control sequence, never by a rounded reconstruction.
pub fn rational_bezier_identity(
    ctx: &mut PredicateContext<'_>,
    a: &[[LeafRef; 4]],
    b: &[[LeafRef; 4]],
) -> Result<BezierIdentityDecision, InputError> {
    if !(2..=33).contains(&a.len()) || !(2..=33).contains(&b.len()) {
        return Err(InputError::InvalidInput(
            "Bezier identity degrees must be in 1..32",
        ));
    }
    let values = a
        .iter()
        .chain(b)
        .flatten()
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    let computed = (|| -> Result<BezierIdentity, Reason> {
        ctx.charge(values.len() as u64)?;
        let values = exact_inputs(&values, ctx)?;
        if values
            .chunks_exact(4)
            .any(|p| p[3].sign() != Sign::Positive)
        {
            return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));
        }
        let binomial = |n: usize, k: usize| -> u64 {
            (0..k).fold(1_u64, |v, i| v * (n - i) as u64 / (i + 1) as u64)
        };
        let (p, q) = (a.len() - 1, b.len() - 1);
        // The common positive factor C(p+q,k) is omitted. Every coefficient
        // of Na*Wb-Nb*Wa must vanish identically, for all three coordinates.
        for k in 0..=p + q {
            for axis in 0..3 {
                let mut coefficient = Expansion::scalar(0.);
                for i in k.saturating_sub(q)..=p.min(k) {
                    let j = k - i;
                    let aa = &values[4 * i..4 * i + 4];
                    let start = 4 * (a.len() + j);
                    let bb = &values[start..start + 4];
                    let factor = Expansion::integer(binomial(p, i) * binomial(q, j), ctx)?;
                    let term = aa[axis]
                        .sub(&bb[axis], ctx)?
                        .mul(&aa[3], ctx)?
                        .mul(&bb[3], ctx)?
                        .mul(&factor, ctx)?;
                    coefficient = coefficient.add(&term, ctx)?;
                }
                if coefficient.sign() != Sign::Zero {
                    return Ok(BezierIdentity::Different);
                }
            }
        }
        ctx.charge(0)?;
        Ok(BezierIdentity::Equal)
    })();
    let computed = computed.and_then(|value| ctx.charge(0).map(|_| value));
    Ok(BezierIdentityDecision {
        outcome: computed.unwrap_or_else(BezierIdentity::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Limits, SourceArena, ToleranceContext};
    fn check(a: &[[f64; 4]], b: &[[f64; 4]], limits: Limits) -> BezierIdentity {
        let source = SourceArena::authored(
            "bezier-test",
            1,
            a.iter()
                .chain(b)
                .flatten()
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .unwrap();
        let refs = |start: usize, count: usize| {
            (start..start + count)
                .map(|i| std::array::from_fn(|k| source.leaf(4 * i + k).unwrap()))
                .collect::<Vec<_>>()
        };
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&source, &tolerance, limits, None);
        rational_bezier_identity(&mut ctx, &refs(0, a.len()), &refs(a.len(), b.len()))
            .unwrap()
            .outcome
    }
    #[test]
    fn degree_elevation_and_weight_scaling_are_exact() {
        let a = [[0., 0., 0., 1.], [1., 2., 3., 3.]];
        let b = [[0., 0., 0., 2.], [0.75, 1.5, 2.25, 4.], [1., 2., 3., 6.]];
        assert_eq!(check(&a, &b, Limits::default()), BezierIdentity::Equal);
        let mut bad = b;
        bad[1][0] = bad[1][0].next_up();
        assert_eq!(
            check(&a, &bad, Limits::default()),
            BezierIdentity::Different
        );
        let reversed = [b[2], b[1], b[0]];
        assert_eq!(
            check(&a, &reversed, Limits::default()),
            BezierIdentity::Different
        );
    }
    #[test]
    fn exhausted_budget_and_nonpositive_weights_never_certify() {
        let a = [[0., 0., 0., 1.], [1., 0., 0., 1.]];
        assert!(matches!(
            check(
                &a,
                &a,
                Limits {
                    max_work: 0,
                    ..Limits::default()
                }
            ),
            BezierIdentity::Indeterminate(_)
        ));
        let mut bad = a;
        bad[0][3] = 0.;
        assert!(matches!(
            check(&a, &bad, Limits::default()),
            BezierIdentity::Indeterminate(_)
        ));
    }
    #[test]
    fn rational_quadratic_and_cubic_identity_uses_all_coordinates() {
        let a = [[0., 0., 0., 3.], [5., 10., 2.5, 6.], [10., 0., 5., 3.]];
        let b = [
            [0., 0., 0., 3.],
            [4., 8., 2., 5.],
            [6., 8., 3., 5.],
            [10., 0., 5., 3.],
        ];
        assert_eq!(check(&a, &b, Limits::default()), BezierIdentity::Equal);
        let mut bad = b;
        bad[2][2] = bad[2][2].next_down();
        assert_eq!(
            check(&a, &bad, Limits::default()),
            BezierIdentity::Different
        );
    }
}
