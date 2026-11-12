//! Piecewise B-spline curve/pcurve composition identity under a shared work budget.
use super::*;

fn affine_fraction(
    q: &[Expansion; 2],
    a: &[Expansion; 2],
    b: &[Expansion; 2],
    ctx: &mut PredicateContext<'_>,
) -> Result<[Expansion; 2], Reason> {
    let delta = b[0].mul(&a[1], ctx)?.sub(&a[0].mul(&b[1], ctx)?, ctx)?;
    Ok([
        a[0].mul(&b[1], ctx)?
            .mul(&q[1], ctx)?
            .add(&delta.mul(&q[0], ctx)?, ctx)?,
        a[1].mul(&b[1], ctx)?.mul(&q[1], ctx)?,
    ])
}

fn inverse_fraction(
    q: &[Expansion; 2],
    a: &[Expansion; 2],
    b: &[Expansion; 2],
    ctx: &mut PredicateContext<'_>,
) -> Result<[Expansion; 2], Reason> {
    let delta = b[0].mul(&a[1], ctx)?.sub(&a[0].mul(&b[1], ctx)?, ctx)?;
    let mut n = q[0]
        .mul(&a[1], ctx)?
        .sub(&a[0].mul(&q[1], ctx)?, ctx)?
        .mul(&b[1], ctx)?;
    let mut d = q[1].mul(&delta, ctx)?;
    if d.sign() == Sign::Negative {
        n = Expansion::scalar(0.).sub(&n, ctx)?;
        d = Expansion::scalar(0.).sub(&d, ctx)?;
    }
    Ok([n, d])
}

/// A B-spline given by source leaves: `N`-coordinate homogeneous controls, knots and degree.
#[derive(Clone, Copy)]
pub struct SplineLeafRefs<'a, const N: usize> {
    pub controls: &'a [[LeafRef; N]],
    pub knots: &'a [LeafRef],
    pub degree: usize,
}

/// Exact composition of an affine canonical restriction and a piecewise UV
/// B-spline. Every intersected span pair must agree under one work budget.
pub fn rational_bspline_pair_composition_piecewise_identity(
    ctx: &mut PredicateContext<'_>,
    curve: SplineLeafRefs<'_, 4>,
    pcurve: SplineLeafRefs<'_, 3>,
    surface: &[Vec<[LeafRef; 4]>],
    domain: [[LeafRef; 2]; 2],
    range: [[LeafRef; 2]; 2],
) -> Result<BezierIdentityDecision, InputError> {
    let SplineLeafRefs { controls: curve, knots, degree } = curve;
    let SplineLeafRefs { controls: pcurve, knots: pknots, degree: pdegree } = pcurve;
    if !(1..=32).contains(&degree)
        || !(1..=8).contains(&pdegree)
        || curve.len() <= degree
        || curve.len() > 4096
        || knots.len() != curve.len() + degree + 1
        || pcurve.len() <= pdegree
        || pcurve.len() > 4096
        || pknots.len() != pcurve.len() + pdegree + 1
        || !(2..=9).contains(&surface.len())
        || surface.first().is_none_or(|row| !(2..=9).contains(&row.len()))
        || surface.iter().any(|row| row.len() != surface[0].len())
    {
        return Err(InputError::InvalidInput("Invalid piecewise spline composition layout"));
    }
    let refs: Vec<_> = curve
        .iter()
        .flatten()
        .chain(pcurve.iter().flatten())
        .chain(surface.iter().flatten().flatten())
        .chain(domain.iter().flatten())
        .chain(range.iter().flatten())
        .chain(knots)
        .chain(pknots)
        .copied()
        .collect();
    let values = refs
        .iter()
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    let result = (|| -> Result<BezierIdentity, Reason> {
        ctx.charge(values.len() as u64)?;
        let values = exact_inputs(&values, ctx)?;
        let (nc, np, nu, nv) = (curve.len(), pcurve.len(), surface.len(), surface[0].len());
        let c: Vec<[Expansion; 4]> = values[..4 * nc]
            .chunks_exact(4)
            .map(|x| std::array::from_fn(|i| x[i].clone()))
            .collect();
        let p: Vec<[Expansion; 3]> = values[4 * nc..4 * nc + 3 * np]
            .chunks_exact(3)
            .map(|x| std::array::from_fn(|i| x[i].clone()))
            .collect();
        let offset = 4 * nc + 3 * np;
        let s: Vec<[Expansion; 4]> = values[offset..offset + 4 * nu * nv]
            .chunks_exact(4)
            .map(|x| std::array::from_fn(|i| x[i].clone()))
            .collect();
        let offset = offset + 4 * nu * nv;
        let d = &values[offset..offset + 4];
        let a = [values[offset + 4].clone(), values[offset + 5].clone()];
        let b = [values[offset + 6].clone(), values[offset + 7].clone()];
        let ck = &values[offset + 8..offset + 8 + knots.len()];
        let pk = &values[offset + 8 + knots.len()..];
        let missing = BezierIdentity::Indeterminate(Reason::MissingProof);
        if c.iter().chain(&s).any(|x| x[3].sign() != Sign::Positive)
            || p.iter().any(|x| x[2].sign() != Sign::Positive)
            || d[1].sub(&d[0], ctx)?.sign() != Sign::Positive
            || d[3].sub(&d[2], ctx)?.sign() != Sign::Positive
        {
            return Ok(missing);
        }
        for k in [ck, pk] {
            for pair in k.windows(2) {
                if pair[1].sub(&pair[0], ctx)?.sign() == Sign::Negative {
                    return Ok(missing);
                }
            }
        }
        for q in [&a, &b] {
            if q[1].sign() != Sign::Positive
                || q[0].sign() == Sign::Negative
                || q[1].sub(&q[0], ctx)?.sign() == Sign::Negative
            {
                return Ok(missing);
            }
        }
        let direction = fraction_cmp(&b, &a, ctx)?;
        if direction == Sign::Zero {
            return Ok(missing);
        }
        let (lo, hi) = if direction == Sign::Positive { (&a, &b) } else { (&b, &a) };
        let cw = ck[nc].sub(&ck[degree], ctx)?;
        let pw = pk[np].sub(&pk[pdegree], ctx)?;
        if cw.sign() != Sign::Positive || pw.sign() != Sign::Positive {
            return Ok(missing);
        }
        let mut covered = false;
        for cs in degree..nc {
            ctx.charge(1)?;
            if ck[cs + 1].sub(&ck[cs], ctx)?.sign() != Sign::Positive {
                continue;
            }
            let l = [ck[cs].sub(&ck[degree], ctx)?, cw.clone()];
            let r = [ck[cs + 1].sub(&ck[degree], ctx)?, cw.clone()];
            let l = if fraction_cmp(&l, lo, ctx)? == Sign::Negative { lo.clone() } else { l };
            let r = if fraction_cmp(&r, hi, ctx)? == Sign::Positive { hi.clone() } else { r };
            if fraction_cmp(&r, &l, ctx)? != Sign::Positive {
                continue;
            }
            let ends = if direction == Sign::Positive { [l, r] } else { [r, l] };
            let low = inverse_fraction(&ends[0], &a, &b, ctx)?;
            let high = inverse_fraction(&ends[1], &a, &b, ctx)?;
            for ps in pdegree..np {
                ctx.charge(1)?;
                if pk[ps + 1].sub(&pk[ps], ctx)?.sign() != Sign::Positive {
                    continue;
                }
                let l = [pk[ps].sub(&pk[pdegree], ctx)?, pw.clone()];
                let r = [pk[ps + 1].sub(&pk[pdegree], ctx)?, pw.clone()];
                let l = if fraction_cmp(&l, &low, ctx)? == Sign::Negative { low.clone() } else { l };
                let r = if fraction_cmp(&r, &high, ctx)? == Sign::Positive { high.clone() } else { r };
                if fraction_cmp(&r, &l, ctx)? != Sign::Positive {
                    continue;
                }
                covered = true;
                let canonical = [affine_fraction(&l, &a, &b, ctx)?, affine_fraction(&r, &a, &b, ctx)?];
                let cr = canonical.into_iter().flatten().collect::<Vec<_>>();
                let pr = [l, r].into_iter().flatten().collect::<Vec<_>>();
                let Some(cc) = spline_homogeneous(&c, ck, degree, cs, &cr, ctx)? else {
                    return Ok(missing);
                };
                let Some(pp) = spline_homogeneous(&p, pk, pdegree, ps, &pr, ctx)? else {
                    return Ok(missing);
                };
                let patch = SurfacePatch { s: &s, d, nu, nv };
                match compare_composition(&cc, &pp, patch, [false; 3], ctx)? {
                    BezierIdentity::Equal => {}
                    other => return Ok(other),
                }
            }
        }
        Ok(if covered { BezierIdentity::Equal } else { missing })
    })()
    .and_then(|value| ctx.charge(0).map(|_| value));
    Ok(BezierIdentityDecision {
        outcome: result.unwrap_or_else(BezierIdentity::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}
