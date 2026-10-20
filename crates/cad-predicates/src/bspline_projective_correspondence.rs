//! Exact global projective correspondence of original clamped B-spline bases.
use crate::{
    Algebra, AuthoredScalar, BezierIdentity, BezierIdentityDecision, Expansion, InputError,
    LeafRef, PredicateContext, Reason, Sign, exact_inputs,
};

/// Sufficient ORIGINAL-basis proof of B(t)=A(phi(t)) with positive fractional
/// linear phi on complete normalized domains; reverse traverses A backwards.
/// Knot multiplicities and all original poles must correspond. Other equivalent
/// representations remain unresolved, never falsely classified as different.
///
/// For D(t)=beta*(Bhi-t)+alpha*(t-Blo), the Cox-de Boor basis transforms as
/// N^A_i,p(phi(t))=N^B_i,p(t)*product(D(k_{i+1}),...,D(k_{i+p}))/D(t)^p.
/// Thus the common positive denominator cancels in the rational curve when the
/// original B weights are proportional to these exact transformed A weights.
/// No knot insertion, rounded mapped knots, fitted poles or sample is authority.
pub fn rational_bspline_projective_correspondence(
    ctx: &mut PredicateContext<'_>,
    degree: usize,
    a: &[[LeafRef; 4]],
    ak: &[LeafRef],
    b: &[[LeafRef; 4]],
    bk: &[LeafRef],
    alpha_beta: [LeafRef; 2],
    reverse: bool,
) -> Result<BezierIdentityDecision, InputError> {
    if !(1..=32).contains(&degree)
        || a.len() <= degree
        || a.len() > 4096
        || a.len() != b.len()
        || ak.len() != a.len() + degree + 1
        || ak.len() != bk.len()
    {
        return Err(InputError::InvalidInput(
            "Invalid projective B-spline basis census",
        ));
    }
    let values = a
        .iter()
        .chain(b)
        .flatten()
        .chain(ak)
        .chain(bk)
        .chain(alpha_beta.iter())
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    let result = (|| -> Result<BezierIdentity, Reason> {
        ctx.charge(values.len() as u64)?;
        let values = exact_inputs(&values, ctx)?;
        let n = a.len();
        let m = ak.len();
        let ka = &values[8 * n..8 * n + m];
        let kb = &values[8 * n + m..8 * n + 2 * m];
        let alpha = &values[8 * n + 2 * m];
        let beta = &values[8 * n + 2 * m + 1];
        let missing = || BezierIdentity::Indeterminate(Reason::MissingProof);
        if alpha.sign() != Sign::Positive
            || beta.sign() != Sign::Positive
            || values[..8 * n]
                .chunks_exact(4)
                .any(|p| p[3].sign() != Sign::Positive)
        {
            return Ok(missing());
        }
        for k in [ka, kb] {
            if k[n].sub(&k[degree], ctx)?.sign() != Sign::Positive {
                return Ok(missing());
            }
            for pair in k.windows(2) {
                if pair[1].sub(&pair[0], ctx)?.sign() == Sign::Negative {
                    return Ok(missing());
                }
            }
            for i in 0..=degree {
                if k[i].sub(&k[degree], ctx)?.sign() != Sign::Zero
                    || k[n + i].sub(&k[n], ctx)?.sign() != Sign::Zero
                {
                    return Ok(missing());
                }
            }
            // Internal multiplicity above p would split the curve into
            // disconnected charts. This owner admits continuous bases only.
            for i in degree + 1..n {
                if i + degree < n && k[i + degree].sub(&k[i], ctx)?.sign() == Sign::Zero {
                    return Ok(missing());
                }
            }
        }
        // Independent exact positive gauges cancel from the normalized knot
        // equations. Remove whole-domain units before products so tiny source
        // domains do not underflow an otherwise ordinary phase proof.
        let mut ac = ka
            .iter()
            .map(|k| k.sub(&ka[degree], ctx))
            .collect::<Result<Vec<_>, _>>()?;
        let mut bc = kb
            .iter()
            .map(|k| k.sub(&kb[degree], ctx))
            .collect::<Result<Vec<_>, _>>()?;
        Expansion::normalize_common(&mut ac, ctx)?;
        Expansion::normalize_common(&mut bc, ctx)?;
        let mut factors = vec![alpha.clone(), beta.clone()];
        Expansion::normalize_common(&mut factors, ctx)?;
        let (alpha, beta) = (&factors[0], &factors[1]);
        let da = &ac[n];
        let mut denominators = Vec::with_capacity(m);
        for i in 0..m {
            let t = bc[i].clone();
            let u = bc[n].sub(&bc[i], ctx)?;
            let at = alpha.mul(&t, ctx)?;
            let bu = beta.mul(&u, ctx)?;
            let denominator = at.add(&bu, ctx)?;
            if denominator.sign() != Sign::Positive {
                return Ok(missing());
            }
            let ai = if reverse { m - 1 - i } else { i };
            let coordinate = &ac[ai];
            let expected = if reverse { &bu } else { &at };
            if coordinate
                .mul(&denominator, ctx)?
                .sub(&da.mul(expected, ctx)?, ctx)?
                .sign()
                != Sign::Zero
            {
                return Ok(missing());
            }
            denominators.push(denominator);
        }
        // A common denominator gauge contributes the SAME p-th power to
        // every transformed weight, so it cancels from the final cross ratios.
        Expansion::normalize_common(&mut denominators, ctx)?;
        let mut transformed = Vec::with_capacity(n);
        for i in 0..n {
            let ai = if reverse { n - 1 - i } else { i };
            for axis in 0..3 {
                if values[4 * ai + axis]
                    .sub(&values[4 * (n + i) + axis], ctx)?
                    .sign()
                    != Sign::Zero
                {
                    return Ok(missing());
                }
            }
            let mut weight = values[4 * ai + 3].clone();
            for d in &denominators[i + 1..i + degree + 1] {
                weight = weight.mul(d, ctx)?;
            }
            transformed.push(weight);
        }
        let wb0 = &values[4 * n + 3];
        for i in 0..n {
            if values[4 * (n + i) + 3]
                .mul(&transformed[0], ctx)?
                .sub(&wb0.mul(&transformed[i], ctx)?, ctx)?
                .sign()
                != Sign::Zero
            {
                return Ok(missing());
            }
        }
        Ok(BezierIdentity::Equal)
    })()
    .and_then(|v| ctx.charge(0).map(|_| v));
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
    #[test]
    fn projective_bspline_admission_preserves_exact_work_cancellation_and_provenance() {
        use std::{sync::atomic::AtomicBool, time::Instant};
        let a = [
            [0., 0., 0., 1.],
            [1., 1., 0., 1.],
            [2., -1., 0., 1.],
            [3., 1., 0., 1.],
            [4., 0., 0., 1.],
        ];
        let mut b = a;
        for (p, w) in b.iter_mut().zip([1., 1.5, 3., 6., 9.]) {
            p[3] = w;
        }
        let ak = [0., 0., 0., 0.5, 0.75, 1., 1., 1.];
        let bk = [0., 0., 0., 0.25, 0.5, 1., 1., 1.];
        let values = a
            .iter()
            .chain(&b)
            .flatten()
            .chain(&ak)
            .chain(&bk)
            .chain([3., 1.].iter())
            .map(|v| AuthoredScalar::Binary64Bits(f64::to_bits(*v)))
            .collect();
        let arena = SourceArena::authored("projective-bspline-admission", 1, values).unwrap();
        let rows = |start: usize| {
            (start..start + 5)
                .map(|i| std::array::from_fn(|k| arena.leaf(4 * i + k).unwrap()))
                .collect::<Vec<_>>()
        };
        let aa = rows(0);
        let bb = rows(5);
        let refs = |start: usize| {
            (start..start + 8)
                .map(|i| arena.leaf(i).unwrap())
                .collect::<Vec<_>>()
        };
        let ka = refs(40);
        let kb = refs(48);
        let factors = [arena.leaf(56).unwrap(), arena.leaf(57).unwrap()];
        let tolerance = ToleranceContext::default_valid();
        let run = |limits, cancel| {
            let mut ctx = PredicateContext::new(&arena, &tolerance, limits, cancel);
            rational_bspline_projective_correspondence(
                &mut ctx, 2, &aa, &ka, &bb, &kb, factors, false,
            )
            .unwrap()
            .outcome
        };
        assert_eq!(run(Limits::default(), None), BezierIdentity::Equal);
        assert_eq!(
            run(
                Limits {
                    max_work: 0,
                    ..Default::default()
                },
                None
            ),
            BezierIdentity::Indeterminate(Reason::ResourceLimit)
        );
        let cancel = AtomicBool::new(true);
        assert_eq!(
            run(Limits::default(), Some(&cancel)),
            BezierIdentity::Indeterminate(Reason::Cancelled)
        );
        assert_eq!(
            run(
                Limits {
                    deadline: Some(Instant::now()),
                    ..Default::default()
                },
                None
            ),
            BezierIdentity::Indeterminate(Reason::DeadlineExceeded)
        );
        let other = SourceArena::authored(
            "foreign-projective-knot",
            1,
            vec![AuthoredScalar::Binary64Bits(0.0_f64.to_bits())],
        )
        .unwrap();
        let mut foreign = ka.clone();
        foreign[0] = other.leaf(0).unwrap();
        let mut ctx = PredicateContext::new(
            &arena,
            &tolerance,
            Limits {
                max_work: 0,
                ..Default::default()
            },
            None,
        );
        assert_eq!(
            rational_bspline_projective_correspondence(
                &mut ctx, 2, &aa, &foreign, &bb, &kb, factors, false
            )
            .unwrap_err(),
            InputError::ProvenanceMismatch
        );
    }
}
