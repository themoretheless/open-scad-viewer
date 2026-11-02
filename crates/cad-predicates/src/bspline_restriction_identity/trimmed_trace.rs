//! Original tensor boundary trace under independent authored trim clocks.
use super::*;
/// Compare the original world curve with an original clamped surface boundary
/// row. uv_line is the affine free-coordinate definition [u0,u1]; clocks are
/// [pcurve domain start,end,trim start,end] and world trim start/end.
/// The caller proves the actual surface-row ownership and constant UV axis.
pub fn rational_bspline_trimmed_boundary_identity(
    ctx: &mut PredicateContext<'_>,
    curves: [&[[LeafRef; 4]]; 2],
    knots: [&[LeafRef]; 2],
    degrees: [usize; 2],
    uv_line: [LeafRef; 2],
    uv_clock: [LeafRef; 4],
    world_range: [LeafRef; 2],
    reverse_world: bool,
) -> Result<BezierIdentityDecision, InputError> {
    if (0..2).any(|i| {
        !(1..=32).contains(&degrees[i])
            || curves[i].len() <= degrees[i]
            || curves[i].len() > 4096
            || knots[i].len() != curves[i].len() + degrees[i] + 1
    }) {
        return Err(InputError::InvalidInput("Invalid clocked boundary basis"));
    }
    let raw = curves
        .iter()
        .flat_map(|c| c.iter().flatten())
        .chain(knots.iter().flat_map(|k| k.iter()))
        .chain(uv_line.iter())
        .chain(uv_clock.iter())
        .chain(world_range.iter())
        .map(|&r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    let result = (|| -> Result<BezierIdentity, Reason> {
        ctx.charge(raw.len() as u64)?;
        let v = exact_inputs(&raw, ctx)?;
        let nc = 4 * curves[0].len();
        let nb = 4 * curves[1].len();
        let k = nc + nb;
        let tail = k + knots[0].len() + knots[1].len();
        let mut points = [v[..nc].to_vec(), v[nc..k].to_vec()];
        let mut ks = [
            v[k..k + knots[0].len()].to_vec(),
            v[k + knots[0].len()..tail].to_vec(),
        ];
        let uv = &v[tail..tail + 2];
        let clock = &v[tail + 2..tail + 6];
        let range = &v[tail + 6..];
        let missing = || BezierIdentity::Indeterminate(Reason::MissingProof);
        let span = clock[1].sub(&clock[0], ctx)?;
        if span.sign() != Sign::Positive || clock[3].sub(&clock[2], ctx)?.sign() == Sign::Zero {
            return Ok(missing());
        }
        for t in &clock[2..] {
            if t.sub(&clock[0], ctx)?.sign() == Sign::Negative
                || clock[1].sub(t, ctx)?.sign() == Sign::Negative
            {
                return Ok(missing());
            }
        }
        let dc = [ks[0][degrees[0]].clone(), ks[0][curves[0].len()].clone()];
        let mut a = Vec::new();
        for t in range {
            a.push(if reverse_world {
                dc[0].add(&dc[1], ctx)?.sub(t, ctx)?
            } else {
                t.clone()
            });
        }
        let delta_uv = uv[1].sub(&uv[0], ctx)?;
        let b = clock[2..]
            .iter()
            .map(|t| {
                uv[0]
                    .mul(&span, ctx)?
                    .add(&delta_uv.mul(&t.sub(&clock[0], ctx)?, ctx)?, ctx)
            })
            .collect::<Result<Vec<_>, Reason>>()?;
        let starts = [a[0].clone(), b[0].clone()];
        let ends = [a[1].clone(), b[1].clone()];
        let den = [Expansion::scalar(1.), span];
        let delta = [ends[0].sub(&starts[0], ctx)?, ends[1].sub(&starts[1], ctx)?];
        if delta.iter().any(|d| d.sign() == Sign::Zero) {
            return Ok(missing());
        }
        let abs = delta
            .iter()
            .map(|d| {
                if d.sign() == Sign::Negative {
                    Expansion::scalar(0.).sub(d, ctx)
                } else {
                    Ok(d.clone())
                }
            })
            .collect::<Result<Vec<_>, Reason>>()?;
        let width = abs[0].mul(&abs[1], ctx)?;
        let zero = Expansion::scalar(0.);
        for side in 0..2 {
            let reverse = delta[side].sign() == Sign::Negative;
            let transformed = ks[side]
                .iter()
                .map(|t| {
                    let x = den[side]
                        .mul(t, ctx)?
                        .sub(&starts[side], ctx)?
                        .mul(&abs[1 - side], ctx)?;
                    if reverse { zero.sub(&x, ctx) } else { Ok(x) }
                })
                .collect::<Result<Vec<_>, Reason>>()?;
            ks[side] = transformed;
            if reverse {
                ks[side].reverse();
                let mut rows = points[side]
                    .chunks_exact(4)
                    .map(|p| p.to_vec())
                    .collect::<Vec<_>>();
                rows.reverse();
                points[side] = rows.into_iter().flatten().collect();
            }
            if !basis(&points[side], &ks[side], degrees[side], ctx)?
                || zero.sub(&ks[side][degrees[side]], ctx)?.sign() == Sign::Negative
                || ks[side][curves[side].len()].sub(&width, ctx)?.sign() == Sign::Negative
            {
                return Ok(missing());
            }
        }
        let mut ai = degrees[0];
        let mut bi = degrees[1];
        let mut cursor = zero.clone();
        let mut cells = 0;
        while ai < curves[0].len() && bi < curves[1].len() {
            ctx.charge(1)?;
            let mut lo = if ks[0][ai].sub(&ks[1][bi], ctx)?.sign() == Sign::Positive {
                ks[0][ai].clone()
            } else {
                ks[1][bi].clone()
            };
            let mut hi = if ks[0][ai + 1].sub(&ks[1][bi + 1], ctx)?.sign() == Sign::Negative {
                ks[0][ai + 1].clone()
            } else {
                ks[1][bi + 1].clone()
            };
            if lo.sign() == Sign::Negative {
                lo = zero.clone();
            }
            if hi.sub(&width, ctx)?.sign() == Sign::Positive {
                hi = width.clone();
            }
            if hi.sub(&lo, ctx)?.sign() == Sign::Positive {
                if lo.sub(&cursor, ctx)?.sign() != Sign::Zero {
                    return Ok(missing());
                }
                let a = extract(&points[0], &ks[0], degrees[0], ai, [&lo, &hi], ctx)?;
                let b = extract(&points[1], &ks[1], degrees[1], bi, [&lo, &hi], ctx)?;
                if !compare(&a, &b, &[zero.clone(), zero.clone(), zero.clone()], ctx)? {
                    return Ok(BezierIdentity::Different);
                }
                cursor = hi;
                cells += 1;
                if cursor.sub(&width, ctx)?.sign() == Sign::Zero {
                    break;
                }
            }
            let order = ks[0][ai + 1].sub(&ks[1][bi + 1], ctx)?.sign();
            if order != Sign::Positive {
                ai += 1;
            }
            if order != Sign::Negative {
                bi += 1;
            }
        }
        Ok(
            if cells > 0 && cursor.sub(&width, ctx)?.sign() == Sign::Zero {
                BezierIdentity::Equal
            } else {
                missing()
            },
        )
    })()
    .and_then(|v| ctx.charge(0).map(|_| v));
    Ok(BezierIdentityDecision {
        outcome: result.unwrap_or_else(BezierIdentity::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}
