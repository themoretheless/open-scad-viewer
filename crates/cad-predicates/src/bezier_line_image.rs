//! Oriented image agreement, not equality of normalized parameter maps.
//! Positive rational Bezier curves with strictly ordered collinear controls are
//! analytic regular bijections of their endpoint segment. A bilinear rational
//! patch restricted to an axis line has the same property when nonconstant.
use crate::{
    Algebra, AuthoredScalar, ContextIdentity, Expansion, InputError, LeafRef, PredicateContext,
    Reason, Sign, exact_inputs,
};
type E = Expansion;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LineImageOutcome {
    SameOrientedSegment,
    Unproved(Reason),
}
pub struct LineImageDecision {
    pub outcome: LineImageOutcome,
    pub work_used: u64,
    pub context: ContextIdentity,
}
/// Euclidean XYZ/UV controls followed by positive weights. All source values
/// enter through the context. No rounded surface endpoint or caller report is
/// accepted. The result grants image agreement only, never a pointwise map.
pub fn rational_bezier_axis_line_image(
    ctx: &mut PredicateContext<'_>,
    curve: &[[LeafRef; 4]],
    pcurve: &[[LeafRef; 3]],
    surface: [[[LeafRef; 4]; 2]; 2],
    domain: [[LeafRef; 2]; 2],
) -> Result<LineImageDecision, InputError> {
    if !(2..=33).contains(&curve.len()) || !(2..=33).contains(&pcurve.len()) {
        return Err(InputError::InvalidInput(
            "Line image needs Bezier degrees in 1..32",
        ));
    }
    let values = curve
        .iter()
        .flatten()
        .chain(pcurve.iter().flatten())
        .chain(surface.iter().flatten().flatten())
        .chain(domain.iter().flatten())
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    let computed = (|| -> Result<(), Reason> {
        let v = exact_inputs(&values, ctx)?;
        let c: Vec<[E; 4]> = v[..curve.len() * 4]
            .chunks_exact(4)
            .map(|r| std::array::from_fn(|k| r[k].clone()))
            .collect();
        let offset = curve.len() * 4;
        let p: Vec<[E; 3]> = v[offset..offset + pcurve.len() * 3]
            .chunks_exact(3)
            .map(|r| std::array::from_fn(|k| r[k].clone()))
            .collect();
        let offset = offset + pcurve.len() * 3;
        let net: [[E; 4]; 4] =
            std::array::from_fn(|i| std::array::from_fn(|k| v[offset + 4 * i + k].clone()));
        let d = &v[offset + 16..];
        if c.iter().any(|r| r[3].sign() != Sign::Positive)
            || p.iter().any(|r| r[2].sign() != Sign::Positive)
            || net.iter().any(|r| r[3].sign() != Sign::Positive)
        {
            return Err(Reason::MissingProof);
        }
        for k in 0..2 {
            if d[2 * k + 1].sub(&d[2 * k], ctx)?.sign() != Sign::Positive {
                return Err(Reason::MissingProof);
            }
            for row in &p {
                if row[k].sub(&d[2 * k], ctx)?.sign() == Sign::Negative
                    || d[2 * k + 1].sub(&row[k], ctx)?.sign() == Sign::Negative
                {
                    return Err(Reason::MissingProof);
                }
            }
        }
        let mut fixed = None;
        for k in 0..2 {
            let mut constant = true;
            for row in &p {
                constant &= row[k].sub(&p[0][k], ctx)?.sign() == Sign::Zero;
            }
            if constant {
                fixed = Some(k);
                break;
            }
        }
        let free = 1 - fixed.ok_or(Reason::MissingProof)?;
        let delta = p.last().unwrap()[free].sub(&p[0][free], ctx)?;
        if delta.sign() == Sign::Zero {
            return Err(Reason::MissingProof);
        }
        for pair in p.windows(2) {
            if pair[1][free]
                .sub(&pair[0][free], ctx)?
                .mul(&delta, ctx)?
                .sign()
                != Sign::Positive
            {
                return Err(Reason::MissingProof);
            }
        }
        // Homogeneous original surface endpoints without division/refitting.
        let endpoint = |uv: &[E; 3], ctx: &mut PredicateContext<'_>| -> Result<[E; 4], Reason> {
            let u = [d[1].sub(&uv[0], ctx)?, uv[0].sub(&d[0], ctx)?];
            let v = [d[3].sub(&uv[1], ctx)?, uv[1].sub(&d[2], ctx)?];
            let mut h = std::array::from_fn(|_| E::scalar(0.));
            for i in 0..2 {
                for j in 0..2 {
                    let w = u[i].mul(&v[j], ctx)?.mul(&net[2 * i + j][3], ctx)?;
                    for k in 0..4 {
                        let term = if k == 3 {
                            w.clone()
                        } else {
                            w.mul(&net[2 * i + j][k], ctx)?
                        };
                        h[k] = h[k].add(&term, ctx)?;
                    }
                }
            }
            if h[3].sign() != Sign::Positive {
                return Err(Reason::MissingProof);
            }
            Ok(h)
        };
        let a = endpoint(&p[0], ctx)?;
        let b = endpoint(p.last().unwrap(), ctx)?;
        let mut direction = std::array::from_fn::<_, 3, _>(|_| E::scalar(0.));
        let mut norm = E::scalar(0.);
        for k in 0..3 {
            direction[k] = b[k].mul(&a[3], ctx)?.sub(&a[k].mul(&b[3], ctx)?, ctx)?;
            norm = norm.add(&direction[k].mul(&direction[k], ctx)?, ctx)?;
            if c[0][k].mul(&a[3], ctx)?.sub(&a[k], ctx)?.sign() != Sign::Zero
                || c.last().unwrap()[k]
                    .mul(&b[3], ctx)?
                    .sub(&b[k], ctx)?
                    .sign()
                    != Sign::Zero
            {
                return Err(Reason::MissingProof);
            }
        }
        if norm.sign() != Sign::Positive {
            return Err(Reason::MissingProof);
        }
        for row in &c {
            let mut x = std::array::from_fn::<_, 3, _>(|_| E::scalar(0.));
            for k in 0..3 {
                x[k] = row[k].mul(&a[3], ctx)?.sub(&a[k], ctx)?;
            }
            for (i, j) in [(0, 1), (1, 2), (2, 0)] {
                if x[i]
                    .mul(&direction[j], ctx)?
                    .sub(&x[j].mul(&direction[i], ctx)?, ctx)?
                    .sign()
                    != Sign::Zero
                {
                    return Err(Reason::MissingProof);
                }
            }
        }
        for pair in c.windows(2) {
            let mut along = E::scalar(0.);
            for k in 0..3 {
                along = along.add(
                    &pair[1][k].sub(&pair[0][k], ctx)?.mul(&direction[k], ctx)?,
                    ctx,
                )?;
            }
            if along.sign() != Sign::Positive {
                return Err(Reason::MissingProof);
            }
        }
        Ok(())
    })();
    Ok(LineImageDecision {
        outcome: match computed {
            Ok(()) => LineImageOutcome::SameOrientedSegment,
            Err(r) => LineImageOutcome::Unproved(r),
        },
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}
#[cfg(test)]
mod tests;
