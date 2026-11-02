//! Two independent authored trim clocks, including reversed world traversal.
use super::*;
fn restrict<const N: usize>(
    points: &[[Expansion; N]],
    clock: &[Expansion],
    reverse: bool,
    ctx: &mut PredicateContext<'_>,
) -> Result<Option<Vec<[Expansion; N]>>, Reason> {
    let d = &clock[..2];
    let r = &clock[2..];
    if d[1].sub(&d[0], ctx)?.sign() != Sign::Positive || r[1].sub(&r[0], ctx)?.sign() == Sign::Zero
    {
        return Ok(None);
    }
    let mut clocks = Vec::new();
    for t in r {
        let a = d[1].sub(t, ctx)?;
        let b = t.sub(&d[0], ctx)?;
        if a.sign() == Sign::Negative || b.sign() == Sign::Negative {
            return Ok(None);
        }
        clocks.push(if reverse { [b, a] } else { [a, b] });
    }
    if points.iter().any(|p| p[N - 1].sign() != Sign::Positive) {
        return Ok(None);
    }
    let h = points
        .iter()
        .map(|p| {
            let mut row = p.clone();
            for k in 0..N - 1 {
                row[k] = p[k].mul(&p[N - 1], ctx)?;
            }
            Ok(row)
        })
        .collect::<Result<Vec<_>, Reason>>()?;
    let degree = h.len() - 1;
    let mut out = Vec::new();
    for i in 0..=degree {
        let mut net = h.clone();
        for step in 0..degree {
            let c = &clocks[usize::from(step >= degree - i)];
            for j in 0..net.len() - 1 {
                for k in 0..N {
                    net[j][k] = net[j][k]
                        .mul(&c[0], ctx)?
                        .add(&net[j + 1][k].mul(&c[1], ctx)?, ctx)?;
                }
            }
            net.pop();
        }
        out.push(net.remove(0));
    }
    Ok(Some(out))
}
/// Full C(reflect_if_reversed(curve_range(t))) = S(P(pcurve_range(t))).
/// Clock arrays are [original domain, authored interval], both physical.
/// Exact restricted homogeneous UV hulls also certify chart inclusion.
pub fn rational_bezier_trimmed_composition_identity(
    ctx: &mut PredicateContext<'_>,
    curve: &[[LeafRef; 4]],
    pcurve: &[[LeafRef; 3]],
    surface: &[Vec<[LeafRef; 4]>],
    domain: [[LeafRef; 2]; 2],
    clocks: [[[LeafRef; 2]; 2]; 2],
    reverse: bool,
) -> Result<BezierIdentityDecision, InputError> {
    if !(2..=33).contains(&curve.len())
        || !(2..=9).contains(&pcurve.len())
        || !(2..=9).contains(&surface.len())
        || surface.first().is_none_or(|r| !(2..=9).contains(&r.len()))
        || surface.iter().any(|r| r.len() != surface[0].len())
    {
        return Err(InputError::InvalidInput(
            "Trimmed composition degree bounds exceeded",
        ));
    }
    let values = curve
        .iter()
        .flatten()
        .chain(pcurve.iter().flatten())
        .chain(surface.iter().flatten().flatten())
        .chain(domain.iter().flatten())
        .chain(clocks.iter().flatten().flatten())
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    let result = (|| -> Result<BezierIdentity, Reason> {
        ctx.charge(values.len() as u64)?;
        let v = exact_inputs(&values, ctx)?;
        let nc = curve.len();
        let np = pcurve.len();
        let nu = surface.len();
        let nv = surface[0].len();
        let c = v[..4 * nc]
            .chunks_exact(4)
            .map(|p| std::array::from_fn(|k| p[k].clone()))
            .collect::<Vec<[Expansion; 4]>>();
        let p = v[4 * nc..4 * nc + 3 * np]
            .chunks_exact(3)
            .map(|p| std::array::from_fn(|k| p[k].clone()))
            .collect::<Vec<[Expansion; 3]>>();
        let end = 4 * nc + 3 * np + 4 * nu * nv;
        let s = v[4 * nc + 3 * np..end]
            .chunks_exact(4)
            .map(|p| std::array::from_fn(|k| p[k].clone()))
            .collect::<Vec<[Expansion; 4]>>();
        let d = &v[end..end + 4];
        let clocks = &v[end + 4..];
        let missing = || BezierIdentity::Indeterminate(Reason::MissingProof);
        if s.iter().any(|p| p[3].sign() != Sign::Positive)
            || d[1].sub(&d[0], ctx)?.sign() != Sign::Positive
            || d[3].sub(&d[2], ctx)?.sign() != Sign::Positive
        {
            return Ok(missing());
        }
        let Some(c) = restrict(&c, &clocks[..4], reverse, ctx)? else {
            return Ok(missing());
        };
        let Some(p) = restrict(&p, &clocks[4..], false, ctx)? else {
            return Ok(missing());
        };
        for point in &p {
            for k in 0..2 {
                if point[k].sub(&d[2 * k].mul(&point[2], ctx)?, ctx)?.sign() == Sign::Negative
                    || d[2 * k + 1]
                        .mul(&point[2], ctx)?
                        .sub(&point[k], ctx)?
                        .sign()
                        == Sign::Negative
                {
                    return Ok(missing());
                }
            }
        }
        let cp = (0..4)
            .map(|k| polynomial(&c.iter().map(|p| p[k].clone()).collect::<Vec<_>>(), ctx))
            .collect::<Result<Vec<_>, _>>()?;
        let pp = (0..3)
            .map(|k| polynomial(&p.iter().map(|p| p[k].clone()).collect::<Vec<_>>(), ctx))
            .collect::<Result<Vec<_>, _>>()?;
        compare_composition(&cp, &pp, &s, d, nu, nv, [false; 3], ctx)
    })()
    .and_then(|v| ctx.charge(0).map(|_| v));
    Ok(BezierIdentityDecision {
        outcome: result.unwrap_or_else(BezierIdentity::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}
