//! Sufficient original isoparametric trace identity in a common along basis.
//! Failure of a constant homogeneous gauge is unresolved, not a mismatch witness.
use crate::bspline_internal_jets::{Rational as Q, series};
use crate::{
    Algebra, AuthoredScalar, BezierIdentity, BezierIdentityDecision, BsplineStripLocation,
    Expansion, InputError, LeafRef, PredicateContext, Reason, Sign, exact_inputs,
};

pub fn rational_bspline_isoparametric_trace_identity(
    ctx: &mut PredicateContext<'_>,
    curve: &[[LeafRef; 4]],
    strip: &[Vec<[LeafRef; 4]>],
    knots: &[LeafRef],
    degree: usize,
    location: BsplineStripLocation,
    periodic_cross: bool,
) -> Result<BezierIdentityDecision, InputError> {
    if !(2..=33).contains(&curve.len())
        || curve.len() != strip.len()
        || !(1..=32).contains(&degree)
        || strip.iter().any(|r| r.len() != strip[0].len())
        || !(degree + 1..=33).contains(&strip[0].len())
        || knots.len() != strip[0].len() + degree + 1
        || location.span < degree
        || location.span >= strip[0].len()
    {
        return Err(InputError::InvalidInput("Invalid original B-spline trace"));
    }
    let count = curve.len();
    let n = strip[0].len();
    let mut leaves = curve
        .iter()
        .flatten()
        .chain(strip.iter().flatten().flatten())
        .chain(knots)
        .copied()
        .collect::<Vec<_>>();
    let ki = count * 4 + count * n * 4;
    let ti = leaves.len();
    leaves.push(location.parameter);
    let mut values = leaves
        .iter()
        .map(|r| ctx.resolve(*r).cloned())
        .collect::<Result<Vec<AuthoredScalar>, _>>()?;
    values.push(AuthoredScalar::Binary64Bits(1f64.to_bits()));
    let result = (|| -> Result<BezierIdentity, Reason> {
        ctx.charge(values.len() as u64)?;
        let x = exact_inputs(&values, ctx)?;
        let unit = x.last().unwrap();
        let homogeneous_unit = Q::scalar(unit.mul(unit, ctx)?);
        let u = &x[ki..ki + knots.len()];
        let t = &x[ti];
        if u[n].sub(&u[degree], ctx)?.sign() != Sign::Positive
            || u[location.span + 1].sub(&u[location.span], ctx)?.sign() != Sign::Positive
            || t.sub(&u[location.span], ctx)?.sign() == Sign::Negative
            || u[location.span + 1].sub(t, ctx)?.sign() == Sign::Negative
        {
            return Err(Reason::MissingProof);
        }
        for pair in u.windows(2) {
            if pair[1].sub(&pair[0], ctx)?.sign() == Sign::Negative {
                return Err(Reason::MissingProof);
            }
        }
        if x[..ki]
            .chunks_exact(4)
            .any(|c| c[3].sign() != Sign::Positive)
        {
            return Err(Reason::MissingProof);
        }
        let point = |row: usize,
                     span: usize,
                     t: &Expansion,
                     ctx: &mut PredicateContext<'_>|
         -> Result<[Q; 4], Reason> {
            let at = count * 4 + row * n * 4;
            let s = series(&x[at..at + n * 4], u, degree, span, t, 0, ctx)?;
            let mut h = s[0].clone();
            h[3] = h[3].mul(&Q::scalar(unit.clone()), ctx)?;
            for q in &mut h {
                *q = q.div_positive(&homogeneous_unit, ctx)?;
            }
            Ok(h)
        };
        let mut actual = Vec::new();
        let mut target = Vec::new();
        let mut starts = Vec::new();
        let first = (degree..n)
            .find(|&i| {
                u[i + 1]
                    .sub(&u[i], ctx)
                    .is_ok_and(|v| v.sign() == Sign::Positive)
            })
            .ok_or(Reason::MissingProof)?;
        for i in 0..count {
            actual.push(std::array::from_fn::<_, 4, _>(|_| Q::zero()));
            for k in 0..4 {
                actual[i][k] =
                    Q::scalar(x[4 * i + 3].mul(if k == 3 { unit } else { &x[4 * i + k] }, ctx)?)
                        .div_positive(&homogeneous_unit, ctx)?;
            }
            let h = point(i, location.span, t, ctx)?;
            if periodic_cross {
                let start = point(i, first, &u[degree], ctx)?;
                if h[3].n.sign() != Sign::Positive || start[3].n.sign() != Sign::Positive {
                    return Err(Reason::MissingProof);
                }
                starts.push(start);
            }
            target.push(h);
        }
        if periodic_cross {
            for i in 0..count {
                for k in 0..4 {
                    if !target[i][k]
                        .mul(&starts[0][3], ctx)?
                        .equal(&starts[i][k].mul(&target[0][3], ctx)?, ctx)?
                    {
                        return Err(Reason::MissingProof);
                    }
                }
            }
        }
        for i in 0..count {
            for k in 0..4 {
                if !actual[i][k]
                    .mul(&target[0][3], ctx)?
                    .equal(&target[i][k].mul(&actual[0][3], ctx)?, ctx)?
                {
                    return Err(Reason::MissingProof);
                }
            }
        }
        ctx.charge(0)?;
        Ok(BezierIdentity::Equal)
    })();
    Ok(BezierIdentityDecision {
        outcome: result.unwrap_or_else(BezierIdentity::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}
