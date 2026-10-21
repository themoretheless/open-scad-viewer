//! Exact original B-spline restriction identity, including translations.
use crate::{
    Algebra, AuthoredScalar, BezierIdentity, BezierIdentityDecision, Expansion, InputError,
    LeafRef, PredicateContext, Reason, Sign, exact_inputs,
};
type Net = Vec<[Expansion; 4]>;
fn normalize(values: &mut [Expansion], ctx: &mut PredicateContext<'_>) -> Result<(), Reason> {
    Expansion::normalize_common(values, ctx)
}
// All blossoms retain the SAME positive homogeneous gauge. Clearing a row's
// de Boor denominators and normalizing ALL active blossoms together is crucial:
// independently rounded/extracted Cartesian poles would not prove identity.
fn extract(
    points: &[Expansion],
    knots: &[Expansion],
    p: usize,
    span: usize,
    range: [&Expansion; 2],
    ctx: &mut PredicateContext<'_>,
) -> Result<Net, Reason> {
    let base = (span - p..=span)
        .map(|i| {
            let w = &points[4 * i + 3];
            Ok([
                points[4 * i].mul(w, ctx)?,
                points[4 * i + 1].mul(w, ctx)?,
                points[4 * i + 2].mul(w, ctx)?,
                w.clone(),
            ])
        })
        .collect::<Result<Net, Reason>>()?;
    let mut rows = vec![base; p + 1];
    for r in 1..=p {
        let m = p - r + 1;
        let mut coefficients = Vec::new();
        for j in r..=p {
            let i = span - p + j;
            let upper = &knots[i + p - r + 1];
            let lower = &knots[i];
            coefficients.extend([
                upper.sub(lower, ctx)?,
                upper.sub(range[0], ctx)?,
                range[0].sub(lower, ctx)?,
                upper.sub(range[1], ctx)?,
                range[1].sub(lower, ctx)?,
            ]);
        }
        normalize(&mut coefficients, ctx)?;
        let mut prefix = vec![Expansion::scalar(1.)];
        for j in 0..m {
            if coefficients[5 * j].sign() != Sign::Positive {
                return Err(Reason::MissingProof);
            }
            prefix.push(prefix[j].mul(&coefficients[5 * j], ctx)?);
        }
        let mut suffix = vec![Expansion::scalar(1.); m + 1];
        for j in (0..m).rev() {
            suffix[j] = suffix[j + 1].mul(&coefficients[5 * j], ctx)?;
        }
        for (blossom, row) in rows.iter_mut().enumerate() {
            let end = usize::from(r > p - blossom);
            for j in (r..=p).rev() {
                let c = &coefficients[5 * (j - r)..5 * (j - r) + 5];
                let omit = prefix[j - r].mul(&suffix[j - r + 1], ctx)?;
                for axis in 0..4 {
                    row[j][axis] = row[j - 1][axis]
                        .mul(&c[1 + 2 * end], ctx)?
                        .add(&row[j][axis].mul(&c[2 + 2 * end], ctx)?, ctx)?
                        .mul(&omit, ctx)?;
                }
            }
        }
        let mut active = rows
            .iter()
            .flat_map(|row| row[r..].iter().flatten().cloned())
            .collect::<Vec<_>>();
        normalize(&mut active, ctx)?;
        let mut values = active.into_iter();
        for row in &mut rows {
            for pole in &mut row[r..] {
                for x in pole {
                    *x = values.next().unwrap();
                }
            }
        }
    }
    Ok(rows.into_iter().map(|row| row[p].clone()).collect())
}
fn basis(
    points: &[Expansion],
    knots: &[Expansion],
    p: usize,
    ctx: &mut PredicateContext<'_>,
) -> Result<bool, Reason> {
    let n = points.len() / 4;
    if points
        .chunks_exact(4)
        .any(|v| v[3].sign() != Sign::Positive)
        || knots[n].sub(&knots[p], ctx)?.sign() != Sign::Positive
        || knots[p + 1].sub(&knots[p], ctx)?.sign() != Sign::Positive
        || knots[n].sub(&knots[n - 1], ctx)?.sign() != Sign::Positive
    {
        return Ok(false);
    }
    for pair in knots.windows(2) {
        if pair[1].sub(&pair[0], ctx)?.sign() == Sign::Negative {
            return Ok(false);
        }
    }
    for i in p + 1..n {
        if i + p < n && knots[i + p].sub(&knots[i], ctx)?.sign() == Sign::Zero {
            return Ok(false);
        }
    }
    Ok(true)
}
fn compare(
    a: &Net,
    b: &Net,
    translation: &[Expansion],
    ctx: &mut PredicateContext<'_>,
) -> Result<bool, Reason> {
    let (p, q) = (a.len() - 1, b.len() - 1);
    let choose = crate::binomial_u64;
    for k in 0..=p + q {
        for axis in 0..3 {
            let mut coefficient = Expansion::scalar(0.);
            for i in k.saturating_sub(q)..=p.min(k) {
                let j = k - i;
                let shifted = b[j][axis].add(&translation[axis].mul(&b[j][3], ctx)?, ctx)?;
                let difference = a[i][axis]
                    .mul(&b[j][3], ctx)?
                    .sub(&shifted.mul(&a[i][3], ctx)?, ctx)?;
                let factor = Expansion::integer(choose(p, i) * choose(q, j), ctx)?;
                coefficient = coefficient.add(&difference.mul(&factor, ctx)?, ctx)?;
            }
            if coefficient.sign() != Sign::Zero {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
/// Prove A(u)=B(u)+translation on B's ENTIRE original natural interval.
/// B may have different degree/knots and occupies a subinterval of A. Both
/// definitions must be continuous on their natural intervals. Every common
/// original knot cell is compared with exact homogeneous Bernstein arithmetic.
pub fn rational_bspline_restriction_identity(
    ctx: &mut PredicateContext<'_>,
    ap: usize,
    a: &[[LeafRef; 4]],
    ak: &[LeafRef],
    bp: usize,
    b: &[[LeafRef; 4]],
    bk: &[LeafRef],
    translation: [LeafRef; 3],
) -> Result<BezierIdentityDecision, InputError> {
    if !(1..=32).contains(&ap)
        || !(1..=32).contains(&bp)
        || a.len() <= ap
        || b.len() <= bp
        || a.len() > 4096
        || b.len() > 4096
        || ak.len() != a.len() + ap + 1
        || bk.len() != b.len() + bp + 1
    {
        return Err(InputError::InvalidInput(
            "Invalid original restriction basis census",
        ));
    }
    let values = a
        .iter()
        .chain(b)
        .flatten()
        .chain(ak)
        .chain(bk)
        .chain(translation.iter())
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    let computed = (|| -> Result<BezierIdentity, Reason> {
        ctx.charge(values.len() as u64)?;
        let values = exact_inputs(&values, ctx)?;
        let (na, nb) = (a.len(), b.len());
        let aa = &values[..4 * na];
        let bb = &values[4 * na..4 * (na + nb)];
        let start = 4 * (na + nb);
        let ka = &values[start..start + ak.len()];
        let kb = &values[start + ak.len()..start + ak.len() + bk.len()];
        let tr = &values[start + ak.len() + bk.len()..];
        let missing = || BezierIdentity::Indeterminate(Reason::MissingProof);
        if !basis(aa, ka, ap, ctx)?
            || !basis(bb, kb, bp, ctx)?
            || kb[bp].sub(&ka[ap], ctx)?.sign() == Sign::Negative
            || ka[na].sub(&kb[nb], ctx)?.sign() == Sign::Negative
        {
            return Ok(missing());
        }
        let mut ai = ap;
        let mut bi = bp;
        let mut cursor = kb[bp].clone();
        let mut cells = 0;
        while ai < na && bi < nb {
            ctx.charge(1)?;
            let lo = if ka[ai].sub(&kb[bi], ctx)?.sign() == Sign::Positive {
                &ka[ai]
            } else {
                &kb[bi]
            };
            let hi = if ka[ai + 1].sub(&kb[bi + 1], ctx)?.sign() == Sign::Negative {
                &ka[ai + 1]
            } else {
                &kb[bi + 1]
            };
            if hi.sub(lo, ctx)?.sign() == Sign::Positive {
                if lo.sub(&cursor, ctx)?.sign() != Sign::Zero {
                    return Ok(missing());
                }
                let an = extract(aa, ka, ap, ai, [lo, hi], ctx)?;
                let bn = extract(bb, kb, bp, bi, [lo, hi], ctx)?;
                if !compare(&an, &bn, tr, ctx)? {
                    return Ok(BezierIdentity::Different);
                }
                cursor = hi.clone();
                cells += 1;
            }
            let order = ka[ai + 1].sub(&kb[bi + 1], ctx)?.sign();
            if order != Sign::Positive {
                ai += 1;
            }
            if order != Sign::Negative {
                bi += 1;
            }
        }
        Ok(
            if cells > 0 && cursor.sub(&kb[nb], ctx)?.sign() == Sign::Zero {
                BezierIdentity::Equal
            } else {
                missing()
            },
        )
    })()
    .and_then(|v| ctx.charge(0).map(|_| v));
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
    #[test]
    fn restriction_identity_keeps_admission_work_cancellation_and_deadline() {
        use std::{sync::atomic::AtomicBool, time::Instant};
        let values = [
            0., 0., 0., 1., 4., 4., 0., 1., 0., 0., 0., 1., 1., 1., 0., 1., 0., 0., 1., 1., 0., 0.,
            0.25, 0.25, 0., 0., 0.,
        ];
        let arena = SourceArena::authored(
            "original-restriction-budget",
            1,
            values
                .iter()
                .map(|v| AuthoredScalar::Binary64Bits(f64::to_bits(*v)))
                .collect(),
        )
        .unwrap();
        let rows = |start: usize| {
            (start..start + 2)
                .map(|i| std::array::from_fn(|k| arena.leaf(4 * i + k).unwrap()))
                .collect::<Vec<_>>()
        };
        let ka = (16..20).map(|i| arena.leaf(i).unwrap()).collect::<Vec<_>>();
        let kb = (20..24).map(|i| arena.leaf(i).unwrap()).collect::<Vec<_>>();
        let shift = std::array::from_fn(|i| arena.leaf(24 + i).unwrap());
        let tolerance = ToleranceContext::default_valid();
        let run = |limits, cancel| {
            let mut ctx = PredicateContext::new(&arena, &tolerance, limits, cancel);
            rational_bspline_restriction_identity(
                &mut ctx,
                1,
                &rows(0),
                &ka,
                1,
                &rows(2),
                &kb,
                shift,
            )
            .unwrap()
        };
        let proof = run(Limits::default(), None);
        assert_eq!(proof.outcome, BezierIdentity::Equal);
        assert_eq!(
            run(
                Limits {
                    max_work: proof.work_used - 1,
                    ..Default::default()
                },
                None
            )
            .outcome,
            BezierIdentity::Indeterminate(Reason::ResourceLimit)
        );
        let cancel = AtomicBool::new(true);
        assert_eq!(
            run(Limits::default(), Some(&cancel)).outcome,
            BezierIdentity::Indeterminate(Reason::Cancelled)
        );
        assert_eq!(
            run(
                Limits {
                    deadline: Some(Instant::now()),
                    ..Default::default()
                },
                None
            )
            .outcome,
            BezierIdentity::Indeterminate(Reason::DeadlineExceeded)
        );
    }
}
