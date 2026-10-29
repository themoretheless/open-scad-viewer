//! Entire original knot-line Cartesian C1/C2, independent of regularity.
use super::*;
use crate::natural_meridian_g2::algebra::{Fraction, P, add, mul, power, times};
fn pieces(
    s: &mut Tensor,
    axis: usize,
    ctx: &mut PredicateContext<'_>,
) -> Result<Vec<Piece>, Reason> {
    let n = if axis == 0 {
        s.points.len()
    } else {
        s.points[0].len()
    };
    let p = s.degrees[axis];
    let origin = s.knots[axis][p].clone();
    s.knots[axis] = s.knots[axis]
        .iter()
        .map(|x| x.sub(&origin, ctx))
        .collect::<Result<Vec<_>, _>>()?;
    Expansion::normalize_common(&mut s.knots[axis], ctx)?;
    let k = &s.knots[axis];
    let mut out = Vec::new();
    for i in p..n {
        ctx.charge(1)?;
        if k[i + 1].sub(&k[i], ctx)?.sign() == Sign::Positive {
            out.push(Piece {
                spans: [i, i],
                range: [k[i].clone(), k[i + 1].clone()],
            });
        }
    }
    if out.is_empty() {
        return Err(Reason::MissingProof);
    }
    Ok(out)
}
fn jet(
    h: &Net,
    axis: usize,
    end: bool,
    k: usize,
    order: usize,
    ctx: &mut PredicateContext<'_>,
) -> Result<Vec<P>, Reason> {
    let p = if axis == 0 {
        h.len() - 1
    } else {
        h[0].len() - 1
    };
    let n = if axis == 0 { h[0].len() } else { h.len() };
    let value = |i: usize, j: usize| if axis == 0 { &h[j][i][k] } else { &h[i][j][k] };
    let mut layers = vec![Vec::new(); order + 1];
    for i in 0..n {
        let at = if end { p } else { 0 };
        let q0 = value(i, at).clone();
        layers[0].push(q0.clone());
        let first = if end {
            q0.sub(value(i, p - 1), ctx)?
        } else {
            value(i, 1).sub(&q0, ctx)?
        };
        layers[1].push(first.mul(&Expansion::integer(p as u64, ctx)?, ctx)?);
        if order == 2 {
            let second = if p < 2 {
                Expansion::scalar(0.)
            } else {
                let (a, b, c) = if end {
                    (value(i, p), value(i, p - 1), value(i, p - 2))
                } else {
                    (value(i, 2), value(i, 1), value(i, 0))
                };
                a.sub(&b.mul(&Expansion::scalar(2.), ctx)?, ctx)?
                    .add(c, ctx)?
                    .mul(&Expansion::integer((p * (p - 1)) as u64, ctx)?, ctx)?
            };
            layers[2].push(second);
        }
    }
    layers.iter().map(|p| power(p, ctx)).collect()
}
fn cartesian(
    h: &Net,
    axis: usize,
    end: bool,
    width: &Expansion,
    k: usize,
    order: usize,
    ctx: &mut PredicateContext<'_>,
) -> Result<Vec<Fraction>, Reason> {
    let w = jet(h, axis, end, 3, order, ctx)?;
    let x = jet(h, axis, end, k, order, ctx)?;
    let square = mul(&w[0], &w[0], ctx)?;
    let mut out = vec![Fraction::new(x[0].clone(), w[0].clone(), ctx)?];
    let first = add(
        &mul(&x[1], &w[0], ctx)?,
        &mul(&x[0], &w[1], ctx)?,
        true,
        ctx,
    )?;
    out.push(Fraction::new(
        first,
        mul(&square, &vec![width.clone()], ctx)?,
        ctx,
    )?);
    if order == 2 {
        let sum = add(
            &mul(&x[0], &w[2], ctx)?,
            &times(&mul(&x[1], &w[1], ctx)?, 2, ctx)?,
            false,
            ctx,
        )?;
        let second = add(
            &add(
                &mul(&x[2], &square, ctx)?,
                &mul(&sum, &w[0], ctx)?,
                true,
                ctx,
            )?,
            &times(&mul(&x[0], &mul(&w[1], &w[1], ctx)?, ctx)?, 2, ctx)?,
            false,
            ctx,
        )?;
        let cube = mul(&square, &w[0], ctx)?;
        let width2 = width.mul(width, ctx)?;
        out.push(Fraction::new(second, mul(&cube, &vec![width2], ctx)?, ctx)?);
    }
    Ok(out)
}
fn seam(
    a: &Net,
    b: &Net,
    axis: usize,
    widths: [Expansion; 2],
    order: usize,
    ctx: &mut PredicateContext<'_>,
) -> Result<bool, Reason> {
    for k in 0..3 {
        let a = cartesian(a, axis, true, &widths[0], k, order, ctx)?;
        let b = cartesian(b, axis, false, &widths[1], k, order, ctx)?;
        for (a, b) in a.iter().zip(b) {
            if !a.equal(&b, ctx)? {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
/// Exact Cartesian Ck across ALL original interior knot lines, including
/// mixed partials by differentiating the identical entire along-line jets.
/// Positive weights and complete original C0 basis coverage are mandatory.
/// Different jets refuse Ck; they do not classify all geometric Gk relations.
/// This predicate supplies no regularity, global embedding or boundary credit.
pub fn rational_bspline_tensor_knot_continuity(
    ctx: &mut PredicateContext<'_>,
    source: RationalTensorSource<'_>,
    order: usize,
) -> Result<BezierIdentityDecision, InputError> {
    if !matches!(order, 1 | 2) {
        return Err(InputError::InvalidInput("Choose original tensor C1 or C2"));
    }
    let n = source.shape()?;
    let raw = source.values(ctx)?;
    if raw
        .iter()
        .any(|v| !matches!(v, AuthoredScalar::Binary64Bits(_)))
    {
        return Err(InputError::InvalidInput(
            "Tensor continuity requires original binary64 leaves",
        ));
    }
    let result = (|| -> Result<BezierIdentity, Reason> {
        ctx.charge(raw.len() as u64)?;
        let mut s = decode(&raw, n, source.degrees, ctx)?;
        let axes = [pieces(&mut s, 0, ctx)?, pieces(&mut s, 1, ctx)?];
        for axis in 0..2 {
            for pair in axes[axis].windows(2) {
                for other in &axes[1 - axis] {
                    ctx.charge(1)?;
                    let query = |p: &Piece, ctx: &mut PredicateContext<'_>| {
                        if axis == 0 {
                            restrict(&s, p, other, 0, ctx)
                        } else {
                            restrict(&s, other, p, 0, ctx)
                        }
                    };
                    let a = query(&pair[0], ctx)?;
                    let b = query(&pair[1], ctx)?;
                    let widths = [
                        pair[0].range[1].sub(&pair[0].range[0], ctx)?,
                        pair[1].range[1].sub(&pair[1].range[0], ctx)?,
                    ];
                    if !seam(&a, &b, axis, widths, order, ctx)? {
                        return Ok(BezierIdentity::Different);
                    }
                }
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
mod tests;
