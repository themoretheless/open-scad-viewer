//! Shared-root proof for two original authored plane equations.
use super::*;
#[derive(Clone, Debug)]
pub enum OriginalPlaneCommonRootOutcome {
    /// The first polynomial has exactly one distinct root in the closed box,
    /// and this very root also zeros the second polynomial.
    Common,
    /// The first polynomial has exactly one distinct root in the closed box,
    /// and no common root lies there.
    Distinct,
    Indeterminate(Reason),
}
#[derive(Clone, Debug)]
pub struct OriginalPlaneCommonRootDecision {
    pub outcome: OriginalPlaneCommonRootOutcome,
    pub work_used: u64,
    pub context: ContextIdentity,
}
/// Verify common-root membership on an original normalized B-spline span.
/// Plane equations are n·P=d using authored binary64 source leaves. No
/// interval-overlap matching: exact PRS gcd plus closed Sturm counts proves
/// that the root is identical. Multiple roots count once; their orders in the
/// two equations need not agree. A box containing zero/multiple first roots
/// is indeterminate, never evidence for common or distinct membership.
pub fn rational_bspline_plane_common_root(
    ctx: &mut PredicateContext<'_>,
    controls: &[[LeafRef; 4]],
    knots: &[LeafRef],
    degree: usize,
    span: usize,
    planes: [[LeafRef; 4]; 2],
    interval: [f64; 2],
) -> Result<OriginalPlaneCommonRootDecision, InputError> {
    let values = prepare(ctx, controls, knots, degree, span, planes, interval)?;
    let computed = (|| {
        let polys = polynomials(ctx, &values, controls.len(), knots.len(), degree, span)?;
        let common =
            crate::planar_bezier_roots::unique_common_root(&polys[0], &polys[1], interval, ctx)?;
        ctx.charge(0)?;
        Ok(if common {
            OriginalPlaneCommonRootOutcome::Common
        } else {
            OriginalPlaneCommonRootOutcome::Distinct
        })
    })();
    Ok(OriginalPlaneCommonRootDecision {
        outcome: computed.unwrap_or_else(OriginalPlaneCommonRootOutcome::Indeterminate),
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}

pub(super) fn prepare(
    ctx: &PredicateContext<'_>,
    controls: &[[LeafRef; 4]],
    knots: &[LeafRef],
    degree: usize,
    span: usize,
    planes: [[LeafRef; 4]; 2],
    interval: [f64; 2],
) -> Result<Vec<f64>, InputError> {
    let n = controls.len();
    if !(1..=25).contains(&degree)
        || n <= degree
        || n > 256
        || knots.len() != n + degree + 1
        || span < degree
        || span >= n
        || !interval.iter().all(|x| x.is_finite())
        || interval[0] < 0.
        || interval[1] > 1.
        || interval[0] > interval[1]
    {
        return Err(InputError::InvalidInput(
            "Invalid original common-root query",
        ));
    }
    let values = controls
        .iter()
        .flatten()
        .chain(knots)
        .copied()
        .chain(planes.into_iter().flatten())
        .map(|r| match ctx.resolve(r)? {
            AuthoredScalar::Binary64Bits(b) => Ok(f64::from_bits(*b)),
            _ => Err(InputError::InvalidInput(
                "Common-root proof requires binary64 source leaves",
            )),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let tail = n * 4 + knots.len();
    let uv = &values[n * 4..tail];
    if values[..n * 4].chunks_exact(4).any(|p| p[3] <= 0.)
        || uv.windows(2).any(|p| p[0] > p[1])
        || uv[degree] >= uv[n]
        || uv[span] >= uv[span + 1]
        || values[tail..]
            .chunks_exact(4)
            .any(|p| p[..3].iter().all(|v| *v == 0.))
    {
        return Err(InputError::InvalidInput("Invalid original source or plane"));
    }
    Ok(values)
}
pub(super) fn polynomials(
    ctx: &mut PredicateContext<'_>,
    values: &[f64],
    n: usize,
    knot_count: usize,
    degree: usize,
    span: usize,
) -> Result<Vec<Vec<Expansion>>, Reason> {
    let tail = n * 4 + knot_count;
    let uv = &values[n * 4..tail];
    ctx.charge(values.len() as u64)?;
    let x: Vec<_> = values.iter().map(|v| Expansion::scalar(*v)).collect();
    let mut polys = Vec::new();
    for i in 0..2 {
        let plane = std::array::from_fn(|j| x[tail + i * 4 + j].clone());
        polys.push(super::polynomial::original_polynomial(
            ctx,
            &x[..tail],
            uv,
            n,
            degree,
            span,
            &plane,
        )?);
    }
    Ok(polys)
}
