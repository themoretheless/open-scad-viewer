//! Common-root proof with an edge coordinate from original patch expressions.
use super::common::{OriginalPlaneCommonRootDecision, OriginalPlaneCommonRootOutcome as Outcome};
use super::*;
/// Edges 0/1 are affine-u=0/1; edges 2/3 are affine-v=0/1.
/// The original patch identity and the unique plane root in the supplied
/// closed normalized interval are checked independently. No rounded plane
/// coefficient or root-midpoint comparison enters this proof.
pub fn rational_bspline_affine_patch_edge_root(
    ctx: &mut PredicateContext<'_>,
    curve: super::quadric_edge::RationalSpanRefs<'_>,
    patch: [[LeafRef; 3]; 4],
    edge: usize,
    interval: [f64; 2],
) -> Result<OriginalPlaneCommonRootDecision, InputError> {
    let d = query(ctx, curve, patch, edge, interval, false)?;
    Ok(OriginalPlaneCommonRootDecision {
        outcome: match d.sign {
            Ok(Sign::Zero) => Outcome::Common,
            Ok(_) => Outcome::Distinct,
            Err(r) => Outcome::Indeterminate(r),
        },
        work_used: d.work_used,
        context: d.context,
    })
}
/// Sign of affine-u-side (edges 0/1), or affine-v-side (edges 2/3),
/// at the unique original-plane root. The determinant orientation is included;
/// this is the coordinate sign, not the unoriented plane numerator sign.
pub fn rational_bspline_affine_patch_edge_sign(
    ctx: &mut PredicateContext<'_>,
    curve: super::quadric_edge::RationalSpanRefs<'_>,
    patch: [[LeafRef; 3]; 4],
    edge: usize,
    interval: [f64; 2],
) -> Result<super::sign::OriginalPlaneRootSignDecision, InputError> {
    query(ctx, curve, patch, edge, interval, true)
}
fn query(
    ctx: &mut PredicateContext<'_>,
    curve: super::quadric_edge::RationalSpanRefs<'_>,
    patch: [[LeafRef; 3]; 4],
    edge: usize,
    interval: [f64; 2],
    signed: bool,
) -> Result<super::sign::OriginalPlaneRootSignDecision, InputError> {
    let super::quadric_edge::RationalSpanRefs {
        controls,
        knots,
        degree,
        span,
    } = curve;
    if edge > 3
        || !interval.iter().all(|v| v.is_finite())
        || interval[0] < 0.
        || interval[1] > 1.
        || interval[0] > interval[1]
    {
        return Err(InputError::InvalidInput("Invalid patch edge root query"));
    }
    // Reuse original-source and exact parallelogram validation. Any failed
    // predicate (including work/precision exhaustion) propagates fail-closed.
    let validation = patch::rational_bspline_affine_patch_roots(
        ctx, controls, knots, degree, span, patch, 0.25,
    )?;
    let computed = (|| {
        match validation.outcome {
            OriginalPlaneRootsOutcome::Roots(_) => {}
            OriginalPlaneRootsOutcome::Indeterminate(r) => return Err(r),
            OriginalPlaneRootsOutcome::Coincident => return Err(Reason::MissingProof),
        }
        let refs: Vec<_> = controls
            .iter()
            .flatten()
            .chain(knots)
            .copied()
            .chain(patch.into_iter().flatten())
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
        let mut u = Vec::new();
        let mut v = Vec::new();
        for k in 0..3 {
            u.push(x[tail + 3 + k].sub(&x[tail + k], ctx)?);
            v.push(x[tail + 6 + k].sub(&x[tail + k], ctx)?);
        }
        let mut plane = std::array::from_fn(|_| Expansion::scalar(0.));
        for k in 0..3 {
            plane[k] = u[(k + 1) % 3]
                .mul(&v[(k + 2) % 3], ctx)?
                .sub(&u[(k + 2) % 3].mul(&v[(k + 1) % 3], ctx)?, ctx)?;
        }
        for k in 0..3 {
            plane[3] = plane[3].add(&plane[k].mul(&x[tail + k], ctx)?, ctx)?;
        }
        let mut coordinate = None;
        let mut orientation = Sign::Positive;
        for (i, j) in [(0, 1), (0, 2), (1, 2)] {
            let det = u[i].mul(&v[j], ctx)?.sub(&u[j].mul(&v[i], ctx)?, ctx)?;
            if det.sign() == Sign::Zero {
                continue;
            }
            let mut c = std::array::from_fn(|_| Expansion::scalar(0.));
            if edge < 2 {
                c[i] = v[j].clone();
                c[j] = v[i].negated();
            } else {
                c[i] = u[j].negated();
                c[j] = u[i].clone();
            }
            for k in 0..3 {
                c[3] = c[3].add(&c[k].mul(&x[tail + k], ctx)?, ctx)?;
            }
            if edge % 2 == 1 {
                c[3] = c[3].add(&det, ctx)?;
            }
            orientation = det.sign();
            coordinate = Some(c);
            break;
        }
        let coordinate = coordinate.ok_or(Reason::MissingProof)?;
        let uv = &values[n * 4..tail];
        let spline = ExactSpan {
            x: &x[..tail],
            uv,
            n,
            degree,
            span,
        };
        let first = super::polynomial::original_polynomial(ctx, spline, &plane)?;
        let second = super::polynomial::original_polynomial(ctx, spline, &coordinate)?;
        let sign = if signed {
            let sign =
                crate::planar_bezier_roots::unique_root_sign(&first, &second, interval, ctx)?;
            if orientation == Sign::Negative {
                match sign {
                    Sign::Positive => Sign::Negative,
                    Sign::Negative => Sign::Positive,
                    Sign::Zero => Sign::Zero,
                }
            } else {
                sign
            }
        } else if crate::planar_bezier_roots::unique_common_root(&first, &second, interval, ctx)? {
            Sign::Zero
        } else {
            Sign::Positive
        };
        ctx.charge(0)?;
        Ok(sign)
    })();
    Ok(super::sign::OriginalPlaneRootSignDecision {
        sign: computed,
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}
