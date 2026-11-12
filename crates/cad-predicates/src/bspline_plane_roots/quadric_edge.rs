//! Exact projected-coordinate signs at original curve/quadric roots.
use super::*;
#[derive(Clone, Debug)]
pub struct OriginalQuadricRootSignDecision {
    pub sign: Result<Sign, Reason>,
    pub work_used: u64,
    pub context: ContextIdentity,
}
/// One span of a rational B-spline curve given by source leaves: homogeneous controls, knots,
/// degree and span index.
#[derive(Clone, Copy)]
pub struct RationalSpanRefs<'a> {
    pub controls: &'a [[LeafRef; 4]],
    pub knots: &'a [LeafRef],
    pub degree: usize,
    pub span: usize,
}
/// A quadric given by source leaves: its homogeneous matrix, the authored products it uses,
/// the projection frame and the two projected axes.
#[derive(Clone, Copy)]
pub struct QuadricRefs<'a> {
    pub matrix: [[LeafRef; 4]; 4],
    pub products: &'a [[LeafRef; 2]],
    pub frame: [[LeafRef; 3]; 3],
    pub axes: [usize; 2],
}
/// Sign of normalized projected affine coordinate minus side (edges 0/1 for
/// the first coordinate, 2/3 for the second) at the unique original quadric
/// root in the supplied closed span box. Frame points [origin,u-end,v-end]
/// remain original leaves. Its determinant orientation is included exactly.
/// This proves no target surface membership, chart coverage or Body claim.
pub fn rational_bspline_quadric_projected_edge_sign(
    ctx: &mut PredicateContext<'_>,
    curve: RationalSpanRefs<'_>,
    quadric: QuadricRefs<'_>,
    edge: usize,
    interval: [f64; 2],
) -> Result<OriginalQuadricRootSignDecision, InputError> {
    let RationalSpanRefs { controls, knots, degree, span } = curve;
    let QuadricRefs { matrix, products, frame, axes } = quadric;
    if edge > 3
        || axes[0] > 2
        || axes[1] > 2
        || axes[0] == axes[1]
        || !interval.iter().all(|v| v.is_finite())
        || interval[0] < 0.
        || interval[1] > 1.
        || interval[0] > interval[1]
    {
        return Err(InputError::InvalidInput(
            "Invalid original quadric coordinate-sign query",
        ));
    }
    let validation = super::quadric::rational_bspline_quadric_roots(
        ctx,
        crate::SplineLeafRefs {
            controls,
            knots,
            degree,
        },
        span,
        matrix,
        products,
        0.25,
    )?;
    let computed = (|| {
        match validation.outcome {
            super::quadric::OriginalQuadricRootsOutcome::Roots(_) => {}
            super::quadric::OriginalQuadricRootsOutcome::Coincident => {
                return Err(Reason::MissingProof);
            }
            super::quadric::OriginalQuadricRootsOutcome::Indeterminate(r) => return Err(r),
        }
        let refs: Vec<_> = controls
            .iter()
            .flatten()
            .chain(knots)
            .chain(matrix.iter().flatten())
            .chain(products.iter().flatten())
            .chain(frame.iter().flatten())
            .copied()
            .collect();
        ctx.charge(refs.len() as u64)?;
        let values = refs
            .iter()
            .map(|&r| match ctx.resolve(r) {
                Ok(AuthoredScalar::Binary64Bits(b)) => Ok(f64::from_bits(*b)),
                _ => Err(Reason::MissingProof),
            })
            .collect::<Result<Vec<_>, _>>()?;
        let x: Vec<_> = values.iter().map(|v| Expansion::scalar(*v)).collect();
        let n = controls.len();
        let tail = n * 4 + knots.len();
        let f = tail + 16 + products.len() * 2;
        let first =
            super::quadric::polynomial(ctx, &x[..f], &values[n * 4..tail], n, degree, span)?;
        let [i, j] = axes;
        let origin = &x[f..f + 3];
        let u = [
            x[f + 3 + i].sub(&origin[i], ctx)?,
            x[f + 3 + j].sub(&origin[j], ctx)?,
        ];
        let v = [
            x[f + 6 + i].sub(&origin[i], ctx)?,
            x[f + 6 + j].sub(&origin[j], ctx)?,
        ];
        let det = u[0].mul(&v[1], ctx)?.sub(&u[1].mul(&v[0], ctx)?, ctx)?;
        if det.sign() == Sign::Zero {
            return Err(Reason::MissingProof);
        }
        let mut plane = std::array::from_fn(|_| Expansion::scalar(0.));
        if edge < 2 {
            plane[i] = v[1].clone();
            plane[j] = v[0].negated();
        } else {
            plane[i] = u[1].negated();
            plane[j] = u[0].clone();
        }
        for k in 0..3 {
            plane[3] = plane[3].add(&plane[k].mul(&origin[k], ctx)?, ctx)?;
        }
        if edge % 2 == 1 {
            plane[3] = plane[3].add(&det, ctx)?;
        }
        let second = super::polynomial::original_polynomial(
            ctx,
            &x[..tail],
            &values[n * 4..tail],
            n,
            degree,
            span,
            &plane,
        )?;
        let mut sign =
            crate::planar_bezier_roots::unique_root_sign(&first, &second, interval, ctx)?;
        if det.sign() == Sign::Negative {
            sign = match sign {
                Sign::Positive => Sign::Negative,
                Sign::Negative => Sign::Positive,
                Sign::Zero => Sign::Zero,
            };
        }
        ctx.charge(0)?;
        Ok(sign)
    })();
    Ok(OriginalQuadricRootSignDecision {
        sign: computed,
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}
