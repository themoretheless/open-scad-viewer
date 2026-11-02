//! Exact original-curve roots on a homogeneous quadric carrier.
use super::*;
#[derive(Clone, Debug)]
pub struct OriginalQuadricRoot {
    pub interval: [f64; 2],
    pub multiplicity: usize,
}
#[derive(Clone, Debug)]
pub enum OriginalQuadricRootsOutcome {
    Roots(Vec<OriginalQuadricRoot>),
    Coincident,
    Indeterminate(Reason),
}
#[derive(Clone, Debug)]
pub struct OriginalQuadricRootsDecision {
    pub outcome: OriginalQuadricRootsOutcome,
    pub work_used: u64,
    pub context: ContextIdentity,
}
/// All distinct roots of H^T Q H + sum(a*b)*W² on one original normalized
/// positive-weight B-spline span. Original XYZ+weight, knots, Q and constant
/// product factors must be authored binary64 leaves. Source degree 1..25,
/// scalar degree at most 50. Root multiplicity is the implicit residual order,
/// not a crossing sign. No finite surface-parameter, trim or Body admission.
pub fn rational_bspline_quadric_roots(
    ctx: &mut PredicateContext<'_>,
    controls: &[[LeafRef; 4]],
    knots: &[LeafRef],
    degree: usize,
    span: usize,
    matrix: [[LeafRef; 4]; 4],
    products: &[[LeafRef; 2]],
    tolerance: f64,
) -> Result<OriginalQuadricRootsDecision, InputError> {
    let n = controls.len();
    if !(1..=25).contains(&degree)
        || n <= degree
        || n > 256
        || knots.len() != n + degree + 1
        || span < degree
        || span >= n
        || products.len() > 16
        || !tolerance.is_finite()
        || tolerance <= 0.
        || tolerance >= 1.
    {
        return Err(InputError::InvalidInput(
            "Invalid original quadric root query",
        ));
    }
    let values = controls
        .iter()
        .flatten()
        .chain(knots)
        .chain(matrix.iter().flatten())
        .chain(products.iter().flatten())
        .map(|&r| match ctx.resolve(r)? {
            AuthoredScalar::Binary64Bits(b) => Ok(f64::from_bits(*b)),
            _ => Err(InputError::InvalidInput(
                "Quadric roots require original binary64 leaves",
            )),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let tail = n * 4 + knots.len();
    let uv = &values[n * 4..tail];
    if values[..n * 4].chunks_exact(4).any(|p| p[3] <= 0.)
        || uv.windows(2).any(|p| p[0] > p[1])
        || uv[degree] >= uv[n]
        || uv[span] >= uv[span + 1]
    {
        return Err(InputError::InvalidInput("Invalid original curve source"));
    }
    let computed = (|| {
        ctx.charge(values.len() as u64)?;
        let x: Vec<_> = values.iter().map(|v| Expansion::scalar(*v)).collect();
        let polynomial = polynomial(ctx, &x, uv, n, degree, span)?;
        if polynomial.iter().all(|v| v.sign() == Sign::Zero) {
            return Ok(OriginalQuadricRootsOutcome::Coincident);
        }
        let roots = isolate(&polynomial, tolerance, ctx)?
            .into_iter()
            .map(|r| OriginalQuadricRoot {
                interval: r.interval,
                multiplicity: r.multiplicity,
            })
            .collect();
        ctx.charge(0)?;
        Ok(OriginalQuadricRootsOutcome::Roots(roots))
    })()
    .and_then(|value| ctx.charge(0).map(|_| value));
    Ok(OriginalQuadricRootsDecision {
        outcome: computed.unwrap_or_else(OriginalQuadricRootsOutcome::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}
fn homogeneous(
    ctx: &mut PredicateContext<'_>,
    x: &[Expansion],
    uv: &[f64],
    n: usize,
    p: usize,
    span: usize,
) -> Result<Vec<Vec<Rational>>, Reason> {
    if uv[span - p + 1] == uv[span] && uv[span + p] == uv[span + 1] {
        let mut base = Vec::new();
        for axis in 0..4 {
            let mut poles = Vec::new();
            for i in span - p..=span {
                poles.push(if axis == 3 {
                    x[i * 4 + 3].clone()
                } else {
                    x[i * 4 + axis].mul(&x[i * 4 + 3], ctx)?
                });
            }
            base.push(poles);
        }
        // One common homogeneous gauge; never normalize Q with coordinates.
        let mut flat: Vec<_> = base.into_iter().flatten().collect();
        Expansion::normalize_common(&mut flat, ctx)?;
        return flat
            .chunks_exact(p + 1)
            .map(|b| Ok(power(b, ctx)?.into_iter().map(Rational::scalar).collect()))
            .collect();
    }
    let k = &x[n * 4..];
    let h = series(&x[..n * 4], k, p, span, &k[span], p, ctx)?;
    let width = k[span + 1].sub(&k[span], ctx)?;
    let mut scale = Expansion::scalar(1.);
    let mut result = vec![Vec::new(); 4];
    for order in 0..=p {
        for axis in 0..4 {
            let mut c = h[order][axis].clone();
            c.n = c.n.mul(&scale, ctx)?;
            if c.d.sign() != Sign::Positive {
                return Err(Reason::MissingProof);
            }
            result[axis].push(c);
        }
        if order < p {
            scale = scale.mul(&width, ctx)?;
        }
    }
    Ok(result)
}

#[path = "quadric/polynomial.rs"]
mod build;
pub(super) fn polynomial(
    ctx: &mut PredicateContext<'_>,
    x: &[Expansion],
    uv: &[f64],
    n: usize,
    degree: usize,
    span: usize,
) -> Result<Vec<Expansion>, Reason> {
    build::construct(ctx, x, uv, n, degree, span)
}
