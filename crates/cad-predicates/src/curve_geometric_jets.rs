//! Exact geometric joins of original rational Bezier curves, without surfaces,
//! sampled normals, square roots or rounded parameter-speed ratios.
use crate::{
    Algebra, AuthoredScalar, BezierIdentity, BezierIdentityDecision, Expansion, InputError,
    LeafRef, PredicateContext, Reason, Sign, exact_inputs,
};

/// Both control arrays start at their common endpoint and point INTO their own
/// side. G1 requires opposite nonzero Cartesian tangents. G2 permits any positive
/// local speed ratio and a tangential second-derivative defect. Whole-curve
/// regularity, original extraction and adjacency remain caller obligations.
pub fn rational_bezier_geometric_endpoint_identity(
    ctx: &mut PredicateContext<'_>,
    a: &[[LeafRef; 4]],
    b: &[[LeafRef; 4]],
    order: usize,
) -> Result<BezierIdentityDecision, InputError> {
    if !(1..=2).contains(&order) || !(2..=33).contains(&a.len()) || !(2..=33).contains(&b.len()) {
        return Err(InputError::InvalidInput(
            "Invalid rational curve endpoint jets",
        ));
    }
    let mut values = a
        .iter()
        .chain(b)
        .flatten()
        .copied()
        .map(|r| ctx.resolve(r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    values.push(AuthoredScalar::Binary64Bits(1f64.to_bits()));
    let result = (|| -> Result<BezierIdentity, Reason> {
        ctx.charge(values.len() as u64)?;
        let x = exact_inputs(&values, ctx)?;
        let unit = x.last().unwrap();
        if x[..x.len() - 1]
            .chunks_exact(4)
            .any(|p| p[3].sign() != Sign::Positive)
        {
            return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));
        }
        let build = |start: usize,
                     count: usize,
                     ctx: &mut PredicateContext<'_>|
         -> Result<([Expansion; 4], [Expansion; 3], [Expansion; 3]), Reason> {
            let zero = || std::array::from_fn(|_| Expansion::scalar(0.));
            let h = |i: usize, ctx: &mut PredicateContext<'_>| -> Result<[Expansion; 4], Reason> {
                let p = &x[start + 4 * i..start + 4 * i + 4];
                let mut out = zero();
                for k in 0..4 {
                    out[k] = if k == 3 {
                        p[3].mul(unit, ctx)?
                    } else {
                        p[k].mul(&p[3], ctx)?
                    };
                }
                Ok(out)
            };
            let h0 = h(0, ctx)?;
            let h1 = h(1, ctx)?;
            let h2 = if count > 2 { h(2, ctx)? } else { zero() };
            let p = Expansion::integer((count - 1) as u64, ctx)?;
            let p2 = Expansion::integer(((count - 1) * (count - 2)) as u64, ctx)?;
            let two = Expansion::integer(2, ctx)?;
            let mut j1 = zero();
            let mut j2 = zero();
            for k in 0..4 {
                j1[k] = h1[k].sub(&h0[k], ctx)?.mul(&p, ctx)?;
                if count > 2 {
                    j2[k] = h2[k]
                        .sub(&h1[k], ctx)?
                        .sub(&h1[k].sub(&h0[k], ctx)?, ctx)?
                        .mul(&p2, ctx)?;
                }
            }
            // D/W² and E/W³ are Cartesian first and second derivatives.
            let w = &h0[3];
            let w2 = w.mul(w, ctx)?;
            let w1sq = j1[3].mul(&j1[3], ctx)?;
            let mut d = std::array::from_fn(|_| Expansion::scalar(0.));
            let mut e = d.clone();
            for k in 0..3 {
                d[k] = j1[k].mul(w, ctx)?.sub(&h0[k].mul(&j1[3], ctx)?, ctx)?;
                e[k] = j2[k]
                    .mul(&w2, ctx)?
                    .sub(&h0[k].mul(&j2[3], ctx)?.mul(w, ctx)?, ctx)?
                    .sub(&j1[k].mul(w, ctx)?.mul(&j1[3], ctx)?.mul(&two, ctx)?, ctx)?
                    .add(&h0[k].mul(&w1sq, ctx)?.mul(&two, ctx)?, ctx)?;
            }
            Ok((h0, d, e))
        };
        let (ha, da, ea) = build(0, a.len(), ctx)?;
        let (hb, db, eb) = build(4 * a.len(), b.len(), ctx)?;
        for k in 0..3 {
            if ha[k]
                .mul(&hb[3], ctx)?
                .sub(&hb[k].mul(&ha[3], ctx)?, ctx)?
                .sign()
                != Sign::Zero
            {
                return Ok(BezierIdentity::Different);
            }
        }
        let Some(k) = (0..3).find(|&k| da[k].sign() != Sign::Zero) else {
            return Ok(BezierIdentity::Indeterminate(Reason::MissingProof));
        };
        if db[k].sign() == Sign::Zero {
            return Ok(BezierIdentity::Different);
        }
        if da[k].sign() == db[k].sign() {
            return Ok(BezierIdentity::Different);
        }
        for i in 0..3 {
            if da[i]
                .mul(&db[k], ctx)?
                .sub(&db[i].mul(&da[k], ctx)?, ctx)?
                .sign()
                != Sign::Zero
            {
                return Ok(BezierIdentity::Different);
            }
        }
        if order == 1 {
            return Ok(BezierIdentity::Equal);
        }
        // lambda=-Db[k]*Wa²/(Da[k]*Wb²)>0. Remove all denominators:
        // B''-lambda² A'' is tangent iff R=Da[k]² Wb Eb-Db[k]² Wa Ea
        // is parallel to Da. This allows nonbinary speed ratios exactly.
        let dak2 = da[k].mul(&da[k], ctx)?;
        let dbk2 = db[k].mul(&db[k], ctx)?;
        let mut r = std::array::from_fn::<_, 3, _>(|_| Expansion::scalar(0.));
        for i in 0..3 {
            r[i] = dak2
                .mul(&hb[3], ctx)?
                .mul(&eb[i], ctx)?
                .sub(&dbk2.mul(&ha[3], ctx)?.mul(&ea[i], ctx)?, ctx)?;
        }
        for i in 0..3 {
            if r[i]
                .mul(&da[k], ctx)?
                .sub(&r[k].mul(&da[i], ctx)?, ctx)?
                .sign()
                != Sign::Zero
            {
                return Ok(BezierIdentity::Different);
            }
        }
        Ok(BezierIdentity::Equal)
    })();
    let result = result.and_then(|r| ctx.charge(0).map(|_| r));
    Ok(BezierIdentityDecision {
        outcome: result.unwrap_or_else(BezierIdentity::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}

#[cfg(test)]
mod tests;
