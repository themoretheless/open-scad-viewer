//! Exact represented tensor-Bezier membership in one homogeneous quadric.
//! This predicate supplies no regularity, topology, smoothness or Solid credit.
use crate::{
    Algebra, AuthoredScalar, BezierIdentity, BezierIdentityDecision, Expansion, InputError,
    LeafRef, PredicateContext, Reason, Sign,
};
type Net = Vec<Vec<Expansion>>;
fn choose(n: usize, k: usize) -> u64 {
    (0..k).fold(1, |v, i| v * (n - i) as u64 / (i + 1) as u64)
}
fn power(a: &Net, ctx: &mut PredicateContext<'_>) -> Result<Net, Reason> {
    let (p, q) = (a.len() - 1, a[0].len() - 1);
    let mut out = vec![vec![Expansion::scalar(0.); q + 1]; p + 1];
    for i in 0..=p {
        for j in 0..=q {
            for u in i..=p {
                for v in j..=q {
                    let n =
                        choose(p, i) * choose(p - i, u - i) * choose(q, j) * choose(q - j, v - j);
                    let factor = Expansion::integer(n, ctx)?;
                    let term = a[i][j].mul(&factor, ctx)?;
                    out[u][v] = if (u - i + v - j) % 2 == 0 {
                        out[u][v].add(&term, ctx)?
                    } else {
                        out[u][v].sub(&term, ctx)?
                    };
                }
            }
        }
    }
    Ok(out)
}
/// Original Euclidean XYZ + positive weight per tensor control; degree 1..8
/// in each direction. Q is the literal matrix in H^T Q H=0, H=(w*x,w*y,w*z,w).
/// Every leaf MUST be Binary64Bits. No common normalization of coordinates and
/// Q is permitted: they have different polynomial units. RationalConstant
/// inputs therefore reject explicitly rather than silently change the locus.
pub fn binary_bezier_quadric_identity(
    ctx: &mut PredicateContext<'_>,
    surface: &[Vec<[LeafRef; 4]>],
    q: [[LeafRef; 4]; 4],
) -> Result<BezierIdentityDecision, InputError> {
    binary_bezier_quadric_with_constant_products_identity(ctx, surface, q, &[])
}
/// H^T Q H + sum(a_i*b_i)*W^2=0, retaining literal original products.
/// This admits radius squared and translated centers without rounding them
/// into a new binary64 coefficient. At most 16 constant products are accepted.
pub fn binary_bezier_quadric_with_constant_products_identity(
    ctx: &mut PredicateContext<'_>,
    surface: &[Vec<[LeafRef; 4]>],
    q: [[LeafRef; 4]; 4],
    constant_products: &[[LeafRef; 2]],
) -> Result<BezierIdentityDecision, InputError> {
    if !(2..=9).contains(&surface.len())
        || surface.first().is_none_or(|r| !(2..=9).contains(&r.len()))
        || surface.iter().any(|r| r.len() != surface[0].len())
        || constant_products.len() > 16
    {
        return Err(InputError::InvalidInput(
            "Quadric membership requires a rectangular degree 1..8 Bezier tensor",
        ));
    }
    let refs = surface
        .iter()
        .flatten()
        .flatten()
        .chain(q.iter().flatten())
        .chain(constant_products.iter().flatten())
        .copied()
        .collect::<Vec<_>>();
    let values = refs
        .iter()
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<_>, _>>()?;
    if values
        .iter()
        .any(|v| !matches!(v, AuthoredScalar::Binary64Bits(_)))
    {
        return Err(InputError::InvalidInput(
            "Quadric membership requires literal binary64 coefficients",
        ));
    }
    let result = (|| -> Result<BezierIdentity, Reason> {
        ctx.charge(values.len() as u64)?;
        let a = values
            .iter()
            .map(|v| match v {
                AuthoredScalar::Binary64Bits(b) => Expansion::scalar(f64::from_bits(*b)),
                _ => unreachable!(),
            })
            .collect::<Vec<_>>();
        let (nu, nv) = (surface.len(), surface[0].len());
        let len = nu * nv * 4;
        if a[..len]
            .chunks_exact(4)
            .any(|p| p[3].sign() != Sign::Positive)
        {
            return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));
        }
        let mut coefficients = a[len..len + 16].to_vec();
        for pair in a[len + 16..].chunks_exact(2) {
            coefficients[15] = coefficients[15].add(&pair[0].mul(&pair[1], ctx)?, ctx)?;
        }
        let mut h = Vec::new();
        for axis in 0..4 {
            let mut controls = vec![vec![Expansion::scalar(0.); nv]; nu];
            for i in 0..nu {
                for j in 0..nv {
                    let k = (i * nv + j) * 4;
                    controls[i][j] = if axis == 3 {
                        a[k + 3].clone()
                    } else {
                        a[k + axis].mul(&a[k + 3], ctx)?
                    };
                }
            }
            h.push(power(&controls, ctx)?);
        }
        let mut residual = vec![vec![Expansion::scalar(0.); 2 * nv - 1]; 2 * nu - 1];
        for i in 0..4 {
            for j in 0..4 {
                let coefficient = &coefficients[i * 4 + j];
                ctx.charge(1)?;
                if coefficient.sign() == Sign::Zero {
                    continue;
                }
                for u in 0..nu {
                    for v in 0..nv {
                        for r in 0..nu {
                            for s in 0..nv {
                                if h[i][u][v].sign() == Sign::Zero
                                    || h[j][r][s].sign() == Sign::Zero
                                {
                                    ctx.charge(1)?;
                                    continue;
                                }
                                let term =
                                    h[i][u][v].mul(&h[j][r][s], ctx)?.mul(coefficient, ctx)?;
                                residual[u + r][v + s] = residual[u + r][v + s].add(&term, ctx)?;
                            }
                        }
                    }
                }
            }
        }
        for c in residual.iter().flatten() {
            ctx.charge(1)?;
            if c.sign() != Sign::Zero {
                return Ok(BezierIdentity::Different);
            }
        }
        Ok(BezierIdentity::Equal)
    })();
    Ok(BezierIdentityDecision {
        outcome: result.unwrap_or_else(BezierIdentity::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Limits, SourceArena, ToleranceContext};
    use std::sync::atomic::AtomicBool;
    fn arena(x: f64) -> SourceArena {
        let mut data = Vec::new();
        for p in [
            [x, 0., 0., 1.],
            [x, 1., 0., 1.],
            [x + 1., 0., 0., 1.],
            [x + 1., 1., 0., 1.],
        ] {
            data.extend(p);
        }
        let mut q = [[0.; 4]; 4];
        q[2][3] = 0.5;
        q[3][2] = 0.5;
        data.extend(q.into_iter().flatten());
        SourceArena::authored(
            "quadric-predicate-test",
            1,
            data.into_iter()
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .unwrap()
    }
    fn refs(a: &SourceArena) -> (Vec<Vec<[LeafRef; 4]>>, [[LeafRef; 4]; 4]) {
        (
            (0..2)
                .map(|i| {
                    (0..2)
                        .map(|j| std::array::from_fn(|k| a.leaf((i * 2 + j) * 4 + k).unwrap()))
                        .collect()
                })
                .collect(),
            std::array::from_fn(|i| std::array::from_fn(|j| a.leaf(16 + i * 4 + j).unwrap())),
        )
    }
    #[test]
    fn exact_quadric_predicate_obeys_zero_short_and_cancelled_contexts() {
        let a = arena(0.);
        let (s, q) = refs(&a);
        let t = ToleranceContext::default_valid();
        let mut c = PredicateContext::new(&a, &t, Limits::default(), None);
        let full = binary_bezier_quadric_identity(&mut c, &s, q).unwrap();
        assert_eq!(full.outcome, BezierIdentity::Equal);
        for max in [0, full.work_used - 1] {
            let mut c = PredicateContext::new(
                &a,
                &t,
                Limits {
                    max_work: max,
                    ..Limits::default()
                },
                None,
            );
            let r = binary_bezier_quadric_identity(&mut c, &s, q).unwrap();
            assert_ne!(r.outcome, BezierIdentity::Equal);
            assert!(r.work_used <= max);
        }
        let cancel = AtomicBool::new(true);
        let mut c = PredicateContext::new(&a, &t, Limits::default(), Some(&cancel));
        assert_eq!(
            binary_bezier_quadric_identity(&mut c, &s, q)
                .unwrap()
                .outcome,
            BezierIdentity::Indeterminate(Reason::Cancelled)
        );
        let foreign = arena(0.);
        let mut c = PredicateContext::new(
            &foreign,
            &t,
            Limits {
                max_work: 0,
                ..Limits::default()
            },
            None,
        );
        assert!(binary_bezier_quadric_identity(&mut c, &s, q).is_err());
    }
    #[test]
    fn rational_coefficients_are_rejected_before_zero_budget_refusal() {
        let mut values = vec![AuthoredScalar::Binary64Bits(0_f64.to_bits()); 32];
        for k in [3, 7, 11, 15] {
            values[k] = AuthoredScalar::Binary64Bits(1_f64.to_bits());
        }
        values[16] = AuthoredScalar::RationalConstant {
            numerator: 1,
            denominator: 3,
        };
        let a = SourceArena::authored("rational-quadric-rejected", 1, values).unwrap();
        let (s, q) = refs(&a);
        let t = ToleranceContext::default_valid();
        let mut c = PredicateContext::new(
            &a,
            &t,
            Limits {
                max_work: 0,
                ..Limits::default()
            },
            None,
        );
        assert!(binary_bezier_quadric_identity(&mut c, &s, q).is_err());
        assert_eq!(c.work_used(), 0);
    }
    #[test]
    fn underflowed_nonzero_residual_is_not_an_exact_quadric_identity() {
        let x: f64 = 1e-300;
        let mut data = Vec::new();
        for _ in 0..4 {
            data.extend([x, 0., 0., 1.]);
        }
        let mut q = [[0.; 4]; 4];
        q[0][0] = 1.;
        data.extend(q.into_iter().flatten());
        let a = SourceArena::authored(
            "underflow-quadric",
            1,
            data.into_iter()
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .unwrap();
        let (s, q) = refs(&a);
        let t = ToleranceContext::default_valid();
        let mut c = PredicateContext::new(&a, &t, Limits::default(), None);
        assert_ne!(
            binary_bezier_quadric_identity(&mut c, &s, q)
                .unwrap()
                .outcome,
            BezierIdentity::Equal
        );
    }
}
