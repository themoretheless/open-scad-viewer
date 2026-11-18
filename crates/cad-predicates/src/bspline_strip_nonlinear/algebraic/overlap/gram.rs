//! A source-derived rational Gram candidate with exact PSD verification.
//! A candidate's failure grants no evidence; there is no numerical SDP fit.
use super::*;

pub(super) fn certificate(p: &B, ctx: &mut PredicateContext<'_>) -> Result<bool, Reason> {
    let Some((_, leading)) = p.last_key_value() else {
        return Ok(false);
    };
    ctx.charge(p.len() as u64)?;
    let sign = if leading.n.sign() == Sign::Negative {
        Q::zero().sub(&one(), ctx)?
    } else {
        one()
    };
    let p = combine(B::new(), p.clone(), &sign, ctx)?;
    if p.is_empty() {
        return Ok(false);
    }
    // Half-exponents of the ORIGINAL positive even monomials propose a basis.
    // This is a sufficient construction, not a complete Gram search.
    let basis = p
        .iter()
        .filter_map(|(&(i, j), q)| {
            (i % 2 == 0 && j % 2 == 0 && q.n.sign() == Sign::Positive).then_some((i / 2, j / 2))
        })
        .collect::<Vec<_>>();
    let n = basis.len();
    if n == 0 {
        return Ok(false);
    }
    ctx.charge(n.checked_mul(n).ok_or(Reason::ResourceLimit)? as u64)?;
    let mut matrix = vec![vec![Q::zero(); n]; n];
    for (&(i, j), coefficient) in &p {
        let mut pair = None;
        for (a, &(x, y)) in basis.iter().enumerate() {
            for (b, &(z, w)) in basis.iter().enumerate().skip(a) {
                ctx.charge(1)?;
                if x.checked_add(z) == Some(i) && y.checked_add(w) == Some(j) {
                    // Prefer a diagonal, otherwise the nearest exponent pair.
                    let distance = x
                        .abs_diff(z)
                        .checked_add(y.abs_diff(w))
                        .ok_or(Reason::MissingProof)?;
                    let key = (a != b, distance, a, b);
                    if pair.is_none_or(|old| key < old) {
                        pair = Some(key);
                    }
                }
            }
        }
        let Some((_, _, a, b)) = pair else {
            return Ok(false);
        };
        let q = if a == b {
            coefficient.clone()
        } else {
            coefficient.div_positive(&integer(2), ctx)?
        };
        matrix[a][b] = matrix[a][b].add(&q, ctx)?;
        matrix[b][a] = matrix[a][b].clone();
    }
    // Reconstruct v^T G v and compare every exact original coefficient.
    let mut reconstructed = B::new();
    for (a, &(i, j)) in basis.iter().enumerate() {
        for (b, &(k, l)) in basis.iter().enumerate() {
            ctx.charge(1)?;
            let key = (
                i.checked_add(k).ok_or(Reason::MissingProof)?,
                j.checked_add(l).ok_or(Reason::MissingProof)?,
            );
            reconstructed = combine(
                reconstructed,
                B::from([(key, matrix[a][b].clone())]),
                &one(),
                ctx,
            )?;
        }
    }
    let minus = Q::zero().sub(&one(), ctx)?;
    if !combine(reconstructed, p, &minus, ctx)?.is_empty() {
        return Ok(false);
    }
    // Rational symmetric elimination: each positive pivot is a weighted
    // square, and its Schur complement is checked in the SAME exact ledger.
    // A zero pivot requires the entire remaining row to vanish.
    for k in 0..n {
        ctx.charge(1)?;
        let pivot = matrix[k][k].clone();
        match pivot.n.sign() {
            Sign::Negative => return Ok(false),
            Sign::Zero => {
                for j in k + 1..n {
                    ctx.charge(1)?;
                    if matrix[k][j].n.sign() != Sign::Zero {
                        return Ok(false);
                    }
                }
            }
            Sign::Positive => {
                for i in k + 1..n {
                    for j in i..n {
                        let correction = matrix[i][k]
                            .mul(&matrix[j][k], ctx)?
                            .div_positive(&pivot, ctx)?;
                        matrix[i][j] = matrix[i][j].sub(&correction, ctx)?;
                        matrix[j][i] = matrix[i][j].clone();
                    }
                }
            }
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuthoredScalar, Limits, SourceArena, ToleranceContext};
    #[test]
    fn quintic_divided_difference_is_exact_psd_outside_direct_constructors() {
        let arena =
            SourceArena::authored("gram-cofactor", 1, vec![AuthoredScalar::Binary64Bits(0)])
                .unwrap();
        let tol = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&arena, &tol, Limits::default(), None);
        let p = B::from_iter((0..5).map(|i| ((i, 4 - i), one())));
        assert!(!semidefinite::certificate(&p, &mut ctx).unwrap());
        assert!(!circuits::certificate(&p, &mut ctx).unwrap());
        assert!(certificate(&p, &mut ctx).unwrap());
        let indefinite = B::from([
            ((2, 0), one()),
            ((1, 1), integer(2)),
            ((0, 2), Q::scalar(Expansion::scalar(1f64.next_down()))),
        ]);
        assert!(!certificate(&indefinite, &mut ctx).unwrap());
        // A zero diagonal cannot hide a nonzero off-diagonal entry.
        assert!(!certificate(&B::from([((1, 1), one())]), &mut ctx).unwrap());
        assert!(!certificate(&B::new(), &mut ctx).unwrap());
        let mut proof = PredicateContext::new(&arena, &tol, Limits::default(), None);
        assert!(certificate(&p, &mut proof).unwrap());
        let mut short = PredicateContext::new(
            &arena,
            &tol,
            Limits {
                max_work: proof.work_used() - 1,
                ..Limits::default()
            },
            None,
        );
        assert!(certificate(&p, &mut short).is_err());
    }

    #[test]
    fn regular_quintic_original_strips_continue_through_critical_projection() {
        let arena =
            SourceArena::authored("gram-strips", 1, vec![AuthoredScalar::Binary64Bits(0)]).unwrap();
        let tol = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&arena, &tol, Limits::default(), None);
        // Actual polynomial surface H(q,v)=(q^5,q^2+q,v,1).
        // X' vanishes at zero; Y'=1 there, so this is not a physical singularity.
        let h = vec![
            [
                vec![Q::zero(), Q::zero(), Q::zero(), Q::zero(), Q::zero(), one()],
                vec![Q::zero(), one(), one()],
                vec![Q::zero()],
                vec![one()],
            ],
            [
                vec![Q::zero()],
                vec![Q::zero()],
                vec![one()],
                vec![Q::zero()],
            ],
            std::array::from_fn(|_| vec![Q::zero()]),
        ];
        let spans = vec![vec![(0, h.clone())], vec![(0, h)]];
        let lo = Q::zero().sub(&one(), &mut ctx).unwrap();
        let u = vec![vec![lo.clone(), one()], vec![lo, one()]];
        let scale = Q::zero().sub(&one(), &mut ctx).unwrap();
        let pair = pair(&spans[0][0].1, &spans[1][0].1, &scale, 2, &mut ctx)
            .unwrap()
            .unwrap();
        assert!(!semidefinite::certificate(&pair.cofactors[0], &mut ctx).unwrap());
        assert!(!circuits::certificate(&pair.cofactors[0], &mut ctx).unwrap());
        assert!(certificate(&pair.cofactors[0], &mut ctx).unwrap());
        assert!(
            Bounds::default()
                .cofactor(
                    super::bounds::BoxKey { a: 0, b: 0, axis: 0, level: 0, pa: 0, pb: 0 },
                    &pair.cofactors[0],
                    super::bounds::StripKnots { spans: &spans, u: &u },
                    &mut ctx,
                )
                .unwrap()
        );
        assert!(matches!(
            super::super::compare(&mut ctx, &spans, &u, &scale, 2),
            Ok(BezierIdentity::Equal)
        ));
    }
}
