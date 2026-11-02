//! Image equality under a freshly derived positive projective clock. Endpoint
//! jets propose the clock; only a COMPLETE coefficient identity grants credit.
use super::*;
use crate::ContextIdentity;
type E = Expansion;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectiveImageOutcome {
    SameOrientedProjectiveImage,
    Unproved(Reason),
}
pub struct ProjectiveImageDecision {
    pub outcome: ProjectiveImageOutcome,
    pub work_used: u64,
    pub context: ContextIdentity,
}
fn first_jet(h: &[Poly], axis: usize, ctx: &mut PredicateContext<'_>) -> Result<(E, E), Reason> {
    let zero = E::scalar(0.);
    let n0 = h[axis].first().unwrap_or(&zero);
    let n1 = h[axis].get(1).unwrap_or(&zero);
    let d0 = h[3].first().unwrap_or(&zero);
    let d1 = h[3].get(1).unwrap_or(&zero);
    if d0.sign() != Sign::Positive {
        return Err(Reason::MissingProof);
    }
    Ok((
        n1.mul(d0, ctx)?.sub(&n0.mul(d1, ctx)?, ctx)?,
        d0.mul(d0, ctx)?,
    ))
}
/// Checks C(M(t))=S(P(t)) for the entire original Bezier definitions, with
/// M(t)=alpha*t/(beta*(1-t)+alpha*t), alpha,beta freshly derived and positive.
/// This grants complete oriented image agreement, NOT normalized map identity,
/// independent regularity, injectivity or material authority. All source weights
/// and the complete UV control hull in the original chart are mandatory.
pub fn rational_bezier_projective_image(
    ctx: &mut PredicateContext<'_>,
    curve: &[[LeafRef; 4]],
    pcurve: &[[LeafRef; 3]],
    surface: &[Vec<[LeafRef; 4]>],
    domain: [[LeafRef; 2]; 2],
) -> Result<ProjectiveImageDecision, InputError> {
    if !(2..=33).contains(&curve.len())
        || !(2..=9).contains(&pcurve.len())
        || !(2..=9).contains(&surface.len())
        || surface.first().is_none_or(|r| !(2..=9).contains(&r.len()))
        || surface.iter().any(|r| r.len() != surface[0].len())
    {
        return Err(InputError::InvalidInput(
            "Projective image dimensions exceed Bezier bounds",
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
        ctx.charge(values.len() as u64)?;
        let v = exact_inputs(&values, ctx)?;
        let (nc, np, nu, nv) = (curve.len(), pcurve.len(), surface.len(), surface[0].len());
        let c: Vec<[E; 4]> = v[..4 * nc]
            .chunks_exact(4)
            .map(|r| std::array::from_fn(|k| r[k].clone()))
            .collect();
        let p: Vec<[E; 3]> = v[4 * nc..4 * nc + 3 * np]
            .chunks_exact(3)
            .map(|r| std::array::from_fn(|k| r[k].clone()))
            .collect();
        let o = 4 * nc + 3 * np;
        let s: Vec<[E; 4]> = v[o..o + 4 * nu * nv]
            .chunks_exact(4)
            .map(|r| std::array::from_fn(|k| r[k].clone()))
            .collect();
        let d = &v[o + 4 * nu * nv..];
        if c.iter().chain(&s).any(|r| r[3].sign() != Sign::Positive)
            || p.iter().any(|r| r[2].sign() != Sign::Positive)
        {
            return Err(Reason::MissingProof);
        }
        for k in 0..2 {
            if d[2 * k + 1].sub(&d[2 * k], ctx)?.sign() != Sign::Positive {
                return Err(Reason::MissingProof);
            }
            for r in &p {
                if r[k].sub(&d[2 * k], ctx)?.sign() == Sign::Negative
                    || d[2 * k + 1].sub(&r[k], ctx)?.sign() == Sign::Negative
                {
                    return Err(Reason::MissingProof);
                }
            }
        }
        let canonical = homogeneous(&c, ctx)?;
        let uv = homogeneous(&p, ctx)?;
        let composed = composition_polynomials(&uv, &s, d, nu, nv, [false; 3], ctx)?;
        let mut weights = None;
        for axis in 0..3 {
            let (cn, cd) = first_jet(&canonical, axis, ctx)?;
            if cn.sign() == Sign::Zero {
                continue;
            }
            let (fn_, fd) = first_jet(&composed, axis, ctx)?;
            let mut alpha = fn_.mul(&cd, ctx)?;
            let mut beta = cn.mul(&fd, ctx)?;
            if alpha.sign() == Sign::Negative && beta.sign() == Sign::Negative {
                alpha = E::scalar(0.).sub(&alpha, ctx)?;
                beta = E::scalar(0.).sub(&beta, ctx)?;
            }
            if alpha.sign() != Sign::Positive || beta.sign() != Sign::Positive {
                return Err(Reason::MissingProof);
            }
            weights = Some([beta, alpha]);
            break;
        }
        let weights = weights.ok_or(Reason::MissingProof)?;
        let range = [E::scalar(0.), E::scalar(1.), E::scalar(1.), E::scalar(1.)];
        let mapped =
            projective_homogeneous(&c, &range, &weights, ctx)?.ok_or(Reason::MissingProof)?;
        if compare_polynomial_rows(&mapped, &composed, [false; 3], ctx)? != BezierIdentity::Equal {
            return Err(Reason::MissingProof);
        }
        ctx.charge(0)
    })();
    Ok(ProjectiveImageDecision {
        outcome: match computed {
            Ok(()) => ProjectiveImageOutcome::SameOrientedProjectiveImage,
            Err(r) => ProjectiveImageOutcome::Unproved(r),
        },
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}
#[cfg(test)]
mod tests;
