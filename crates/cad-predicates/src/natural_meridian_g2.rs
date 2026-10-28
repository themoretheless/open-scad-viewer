//! Exact Cartesian graph gradient/Hessian identities on original meridians.
//! Positional ownership, nonzero normals and finite pole limits are separate.
use crate::{
    Algebra, AuthoredScalar, BezierIdentity, BezierIdentityDecision, Expansion, InputError,
    LeafRef, PredicateContext, Reason, Sign, exact_inputs,
};
mod algebra;
use algebra::*;
fn jets(
    h: &[Vec<[Expansion; 4]>],
    k: usize,
    c: &mut PredicateContext<'_>,
) -> Result<[P; 6], Reason> {
    let q = h[0].len() - 1;
    let f = power(&h.iter().map(|r| r[0][k].clone()).collect(), c)?;
    let mut a = Vec::new();
    let mut aa = Vec::new();
    for row in h {
        a.push(
            row[1][k]
                .sub(&row[0][k], c)?
                .mul(&Expansion::integer(q as u64, c)?, c)?,
        );
        aa.push(if q < 2 {
            zero()
        } else {
            row[2][k]
                .sub(&row[1][k].mul(&Expansion::scalar(2.), c)?, c)?
                .add(&row[0][k], c)?
                .mul(&Expansion::integer((q * (q - 1)) as u64, c)?, c)?
        });
    }
    let a = power(&a, c)?;
    let aa = power(&aa, c)?;
    let u = derivative(&f, c)?;
    Ok([
        f,
        u.clone(),
        a.clone(),
        derivative(&u, c)?,
        derivative(&a, c)?,
        aa,
    ])
}
fn graph(
    h: &[Vec<[Expansion; 4]>],
    axis: usize,
    order: usize,
    c: &mut PredicateContext<'_>,
) -> Result<Vec<Fraction>, Reason> {
    let w = jets(h, 3, c)?;
    let ww = mul(&w[0], &w[0], c)?;
    let mut a = Vec::new();
    let mut b = Vec::new();
    let mut second = Vec::new();
    for k in [(axis + 1) % 3, (axis + 2) % 3, axis] {
        let h = jets(h, k, c)?;
        a.push(minor(&h[1], &w[0], &h[0], &w[1], c)?);
        b.push(minor(&h[2], &w[0], &h[0], &w[2], c)?);
        let mut s = Vec::new();
        for (i, j, ij) in [(1, 1, 3), (1, 2, 4), (2, 2, 5)] {
            let sum = add(
                &add(&mul(&h[i], &w[j], c)?, &mul(&h[j], &w[i], c)?, false, c)?,
                &mul(&h[0], &w[ij], c)?,
                false,
                c,
            )?;
            let n = add(&mul(&h[ij], &ww, c)?, &mul(&sum, &w[0], c)?, true, c)?;
            s.push(add(
                &n,
                &times(&mul(&h[0], &mul(&w[i], &w[j], c)?, c)?, 2, c)?,
                false,
                c,
            )?);
        }
        second.push(s);
    }
    let d = minor(&a[0], &b[1], &a[1], &b[0], c)?;
    if d.iter().all(|v| v.sign() == Sign::Zero) {
        return Err(Reason::MissingProof);
    }
    let gx = minor(&a[2], &b[1], &b[2], &a[1], c)?;
    let gy = minor(&a[0], &b[2], &b[0], &a[2], c)?;
    if order == 1 {
        return Ok(vec![
            Fraction::new(gx, d.clone(), c)?,
            Fraction::new(gy, d, c)?,
        ]);
    }
    let mut r = Vec::new();
    for i in 0..3 {
        r.push(add(
            &minor(&d, &second[2][i], &gx, &second[0][i], c)?,
            &mul(&gy, &second[1][i], c)?,
            true,
            c,
        )?);
    }
    let d3 = mul(&mul(&d, &d, c)?, &d, c)?;
    let xx = add(
        &minor(
            &mul(&b[1], &b[1], c)?,
            &r[0],
            &times(&mul(&b[1], &a[1], c)?, 2, c)?,
            &r[1],
            c,
        )?,
        &mul(&mul(&a[1], &a[1], c)?, &r[2], c)?,
        false,
        c,
    )?;
    let xy = add(
        &minor(
            &add(&mul(&b[1], &a[0], c)?, &mul(&a[1], &b[0], c)?, false, c)?,
            &r[1],
            &mul(&b[0], &b[1], c)?,
            &r[0],
            c,
        )?,
        &mul(&mul(&a[0], &a[1], c)?, &r[2], c)?,
        true,
        c,
    )?;
    let yy = add(
        &minor(
            &mul(&b[0], &b[0], c)?,
            &r[0],
            &times(&mul(&b[0], &a[0], c)?, 2, c)?,
            &r[1],
            c,
        )?,
        &mul(&mul(&a[0], &a[0], c)?, &r[2], c)?,
        false,
        c,
    )?;
    Ok(vec![
        Fraction::new(gx, d.clone(), c)?,
        Fraction::new(gy, d.clone(), c)?,
        Fraction::new(mul(&w[0], &xx, c)?, d3.clone(), c)?,
        Fraction::new(mul(&w[0], &xy, c)?, d3.clone(), c)?,
        Fraction::new(mul(&w[0], &yy, c)?, d3, c)?,
    ])
}
/// Rows already follow the canonical direction; columns start at the original
/// natural meridian side. Maps retain original forward carrier weights A,B
/// and original UV-line weights C,D. No rounded composed weight is accepted.
/// Reversed canonical maps use (DA,CB), forward maps (CB,DA) as inverse factors.
/// Identity is checked on ALL coefficients, not a station/sample collection.
/// Exact graph gradients alone. This does not assert Hessian compatibility,
/// positional ownership, regularity, or finite endpoint limits.
pub fn rational_natural_meridian_g1(
    ctx: &mut PredicateContext<'_>,
    surfaces: [&[Vec<[LeafRef; 4]>]; 2],
    maps: [[LeafRef; 4]; 2],
    reversed: [bool; 2],
    axis: usize,
) -> Result<BezierIdentityDecision, InputError> {
    identity(ctx, surfaces, maps, reversed, axis, 1)
}
pub fn rational_natural_meridian_g2(
    ctx: &mut PredicateContext<'_>,
    surfaces: [&[Vec<[LeafRef; 4]>]; 2],
    maps: [[LeafRef; 4]; 2],
    reversed: [bool; 2],
    axis: usize,
) -> Result<BezierIdentityDecision, InputError> {
    identity(ctx, surfaces, maps, reversed, axis, 2)
}
fn identity(
    ctx: &mut PredicateContext<'_>,
    surfaces: [&[Vec<[LeafRef; 4]>]; 2],
    maps: [[LeafRef; 4]; 2],
    reversed: [bool; 2],
    axis: usize,
    order: usize,
) -> Result<BezierIdentityDecision, InputError> {
    if axis > 2
        || surfaces.iter().any(|s| {
            !(2..=9).contains(&s.len())
                || s.first().is_none_or(|r| !(2..=9).contains(&r.len()))
                || s.iter().any(|r| r.len() != s[0].len())
        })
    {
        return Err(InputError::InvalidInput(
            "Choose original degree 1..8 meridian tensors and a world graph axis",
        ));
    }
    let mut values = Vec::new();
    for s in surfaces {
        for row in s {
            for p in row {
                for &leaf in p {
                    values.push(ctx.resolve(leaf)?.clone());
                }
            }
        }
    }
    for map in maps {
        for leaf in map {
            values.push(ctx.resolve(leaf)?.clone());
        }
    }
    values.push(AuthoredScalar::Binary64Bits(1f64.to_bits()));
    let result = (|| -> Result<bool, Reason> {
        ctx.charge(values.len() as u64)?;
        let x = exact_inputs(&values, ctx)?;
        let unit = x.last().unwrap();
        let map_at = surfaces
            .iter()
            .map(|s| s.len() * s[0].len() * 4)
            .sum::<usize>();
        let mut offset = 0;
        let mut fields = Vec::new();
        for (side, s) in surfaces.into_iter().enumerate() {
            let raw = &x[map_at + 4 * side..map_at + 4 * side + 4];
            if raw.iter().any(|v| v.sign() != Sign::Positive) {
                return Err(Reason::MissingProof);
            }
            let cb = raw[2].mul(&raw[1], ctx)?;
            let da = raw[3].mul(&raw[0], ctx)?;
            let factors = if reversed[side] { [da, cb] } else { [cb, da] };
            let p = s.len() - 1;
            let mut all = Vec::new();
            for i in 0..s.len() {
                let mut factor = Expansion::scalar(1.);
                for _ in 0..p - i {
                    factor = factor.mul(&factors[0], ctx)?;
                }
                for _ in 0..i {
                    factor = factor.mul(&factors[1], ctx)?;
                }
                for j in 0..s[0].len() {
                    let at = offset + (i * s[0].len() + j) * 4;
                    let weight = &x[at + 3];
                    if weight.sign() != Sign::Positive {
                        return Err(Reason::MissingProof);
                    }
                    for k in 0..3 {
                        all.push(
                            x[at + k]
                                .sub(&x[offset + k], ctx)?
                                .mul(weight, ctx)?
                                .mul(&factor, ctx)?,
                        );
                    }
                    all.push(unit.mul(weight, ctx)?.mul(&factor, ctx)?);
                }
            }
            Expansion::normalize_common(&mut all, ctx)?;
            let h: Vec<Vec<[Expansion; 4]>> = all
                .chunks_exact(s[0].len() * 4)
                .map(|r| {
                    r.chunks_exact(4)
                        .map(|p| std::array::from_fn(|k| p[k].clone()))
                        .collect()
                })
                .collect();
            fields.push(graph(&h, axis, order, ctx)?);
            offset += s.len() * s[0].len() * 4;
        }
        for (a, b) in fields[0].iter().zip(&fields[1]) {
            if !a.equal(b, ctx)? {
                return Ok(false);
            }
        }
        ctx.charge(0)?;
        Ok(true)
    })();
    Ok(BezierIdentityDecision {
        outcome: match result {
            Ok(true) => BezierIdentity::Equal,
            Ok(false) => BezierIdentity::Different,
            Err(r) => BezierIdentity::Indeterminate(r),
        },
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}
#[cfg(test)]
mod tests;
