//! Original rational spline point lift; equality here is point-only.
use crate::bspline_internal_jets::Rational as Q;
use crate::{
    AuthoredScalar, BezierIdentity, ContextIdentity, Expansion, InputError, LeafRef,
    PredicateContext, Reason, Sign, exact_inputs,
};

#[derive(Clone, Debug)]
pub struct SplinePointLiftDecision {
    /// Equality at the specified normalized parameter, never a curve identity.
    pub point_equality: BezierIdentity,
    pub work_used: u64,
    pub context: ContextIdentity,
}
fn positive(x: &Q) -> bool {
    x.n.sign() == Sign::Positive
}
fn sign(a: &Q, b: &Q, c: &mut PredicateContext<'_>) -> Result<Sign, Reason> {
    Ok(a.sub(b, c)?.n.sign())
}
fn same(a: &[Q], b: &[Q], c: &mut PredicateContext<'_>) -> Result<bool, Reason> {
    let w = a.len() - 1;
    if !positive(&a[w]) || !positive(&b[w]) {
        return Err(Reason::MissingProof);
    }
    for k in 0..w {
        if !a[k].mul(&b[w], c)?.equal(&b[k].mul(&a[w], c)?, c)? {
            return Ok(false);
        }
    }
    Ok(true)
}
fn point(
    poles: &[Vec<Q>],
    knots: &[Q],
    degree: usize,
    t: &Q,
    c: &mut PredicateContext<'_>,
) -> Result<Vec<Q>, Reason> {
    let count = poles.len();
    for pair in knots.windows(2) {
        if sign(&pair[1], &pair[0], c)? == Sign::Negative {
            return Err(Reason::MissingProof);
        }
    }
    if sign(&knots[count], &knots[degree], c)? != Sign::Positive
        || sign(t, &knots[degree], c)? == Sign::Negative
        || sign(&knots[count], t, c)? == Sign::Negative
    {
        return Err(Reason::MissingProof);
    }
    let mut answer: Option<Vec<Q>> = None;
    for span in degree..count {
        c.charge(1)?;
        if sign(&knots[span + 1], &knots[span], c)? != Sign::Positive
            || sign(t, &knots[span], c)? == Sign::Negative
            || sign(&knots[span + 1], t, c)? == Sign::Negative
        {
            continue;
        }
        let mut d = poles[span - degree..=span].to_vec();
        for r in 1..=degree {
            for j in (r..=degree).rev() {
                let i = span - degree + j;
                let gap = knots[i + degree - r + 1].sub(&knots[i], c)?;
                let a = t.sub(&knots[i], c)?.div_positive(&gap, c)?;
                let b = Q::scalar(Expansion::scalar(1.)).sub(&a, c)?;
                d[j] = (0..d[j].len())
                    .map(|k| d[j - 1][k].mul(&b, c)?.add(&d[j][k].mul(&a, c)?, c))
                    .collect::<Result<_, Reason>>()?;
            }
        }
        let value = d.pop().unwrap();
        // Refuse a discontinuous/ambiguous original knot, never select one side.
        if let Some(old) = &answer {
            for (a, b) in old.iter().zip(&value) {
                if !a.equal(b, c)? {
                    return Err(Reason::MissingProof);
                }
            }
        }
        answer = Some(value);
    }
    answer.ok_or(Reason::MissingProof)
}
/// Euclidean XYZ+W curve/surface, UV+W pcurve, original knot arrays C,P,U,V.
/// The normalized parameter is authored in [0,1]; reversing affects C only.
/// No caller-selected span or pre-evaluated XYZ/UV is accepted as authority.
pub fn rational_bspline_curve_surface_point_identity(
    ctx: &mut PredicateContext<'_>,
    curve: &[[LeafRef; 4]],
    pcurve: &[[LeafRef; 3]],
    surface: &[Vec<[LeafRef; 4]>],
    knots: [&[LeafRef]; 4],
    degrees: [usize; 4],
    normalized_parameter: LeafRef,
    reversed: bool,
) -> Result<SplinePointLiftDecision, InputError> {
    let sizes = [
        curve.len(),
        pcurve.len(),
        surface.len(),
        surface.first().map_or(0, Vec::len),
    ];
    if (0..4).any(|i| {
        !(1..=32).contains(&degrees[i])
            || sizes[i] <= degrees[i]
            || sizes[i] > 4096
            || knots[i].len() != sizes[i] + degrees[i] + 1
    }) || sizes[2].saturating_mul(sizes[3]) > 4096
        || surface.iter().any(|r| r.len() != sizes[3])
    {
        return Err(InputError::InvalidInput(
            "Invalid original spline point lift layout",
        ));
    }
    let refs = curve
        .iter()
        .flatten()
        .chain(pcurve.iter().flatten())
        .chain(surface.iter().flatten().flatten())
        .chain(knots.iter().flat_map(|k| k.iter()))
        .chain(std::iter::once(&normalized_parameter));
    let mut raw = refs
        .map(|r| ctx.resolve(*r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    raw.push(AuthoredScalar::Binary64Bits(1f64.to_bits()));
    let result = (|| -> Result<BezierIdentity, Reason> {
        ctx.charge(raw.len() as u64)?;
        let exact = exact_inputs(&raw, ctx)?;
        let unit = Q::scalar(exact.last().unwrap().clone());
        // Undo common input normalization exactly, including original knots.
        let x = exact[..exact.len() - 1]
            .iter()
            .map(|e| Q::scalar(e.clone()).div_positive(&unit, ctx))
            .collect::<Result<Vec<_>, _>>()?;
        let mut offset = 0;
        let mut controls = Vec::new();
        for (count, dim) in [(sizes[0], 4), (sizes[1], 3), (sizes[2] * sizes[3], 4)] {
            let mut rows = Vec::new();
            for _ in 0..count {
                let w = &x[offset + dim - 1];
                if !positive(w) {
                    return Err(Reason::MissingProof);
                }
                let mut h = x[offset..offset + dim - 1]
                    .iter()
                    .map(|v| v.mul(w, ctx))
                    .collect::<Result<Vec<_>, _>>()?;
                h.push(w.clone());
                rows.push(h);
                offset += dim;
            }
            controls.push(rows);
        }
        let mut k = Vec::new();
        for row in knots {
            k.push(&x[offset..offset + row.len()]);
            offset += row.len();
        }
        let t = &x[offset];
        let one = Q::scalar(Expansion::scalar(1.));
        if sign(t, &Q::zero(), ctx)? == Sign::Negative || sign(&one, t, ctx)? == Sign::Negative {
            return Err(Reason::MissingProof);
        }
        let clock = |axis: usize, t: &Q, ctx: &mut PredicateContext<'_>| -> Result<Q, Reason> {
            let low = &k[axis][degrees[axis]];
            low.add(&k[axis][sizes[axis]].sub(low, ctx)?.mul(t, ctx)?, ctx)
        };
        let ct = if reversed {
            one.sub(t, ctx)?
        } else {
            t.clone()
        };
        let xyz = point(&controls[0], k[0], degrees[0], &clock(0, &ct, ctx)?, ctx)?;
        let uv = point(&controls[1], k[1], degrees[1], &clock(1, t, ctx)?, ctx)?;
        let u = uv[0].div_positive(&uv[2], ctx)?;
        let v = uv[1].div_positive(&uv[2], ctx)?;
        let rows = controls[2]
            .chunks_exact(sizes[3])
            .map(|row| point(row, k[3], degrees[3], &v, ctx))
            .collect::<Result<Vec<_>, _>>()?;
        let lifted = point(&rows, k[2], degrees[2], &u, ctx)?;
        Ok(if same(&xyz, &lifted, ctx)? {
            BezierIdentity::Equal
        } else {
            BezierIdentity::Different
        })
    })()
    .and_then(|r| ctx.charge(0).map(|_| r));
    Ok(SplinePointLiftDecision {
        point_equality: result.unwrap_or_else(BezierIdentity::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}
