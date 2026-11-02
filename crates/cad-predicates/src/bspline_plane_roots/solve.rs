//! Shared exact scalar polynomial isolation from expansion plane coefficients.
use super::*;
pub(super) fn solve(
    ctx: &mut PredicateContext<'_>,
    x: &[Expansion],
    uv: &[f64],
    n: usize,
    degree: usize,
    span: usize,
    plane: &[Expansion; 4],
    tolerance: f64,
) -> Result<OriginalPlaneRootsOutcome, Reason> {
    let bezier = uv[span - degree + 1] == uv[span] && uv[span + degree] == uv[span + 1];
    let polynomial = if bezier {
        let mut residuals = Vec::new();
        for i in span - degree..=span {
            let mut f = Expansion::scalar(0.).sub(&plane[3], ctx)?;
            for axis in 0..3 {
                f = f.add(&x[i * 4 + axis].mul(&plane[axis], ctx)?, ctx)?;
            }
            residuals.push(f.mul(&x[i * 4 + 3], ctx)?);
        }
        if residuals.iter().all(|v| v.sign() == Sign::Zero) {
            return Ok(OriginalPlaneRootsOutcome::Coincident);
        }
        if residuals[0].sign() != Sign::Zero {
            let mut geometric = true;
            for i in 2..residuals.len() {
                if residuals[i]
                    .mul(&residuals[0], ctx)?
                    .sub(&residuals[i - 1].mul(&residuals[1], ctx)?, ctx)?
                    .sign()
                    != Sign::Zero
                {
                    geometric = false;
                    break;
                }
            }
            if geometric {
                // Exact binomial identity: Hi=H0*r^i gives
                // H0*((1-q)+r*q)^p, including non-midpoint contacts.
                let linear = vec![residuals[0].clone(), residuals[1].sub(&residuals[0], ctx)?];
                let roots = isolate(&linear, tolerance, ctx)?
                    .into_iter()
                    .map(|r| OriginalPlaneRoot {
                        interval: r.interval,
                        multiplicity: degree,
                    })
                    .collect();
                return Ok(OriginalPlaneRootsOutcome::Roots(roots));
            }
        }
        power(&residuals, ctx)?
    } else {
        super::polynomial::original_polynomial(ctx, x, uv, n, degree, span, plane)?
    };
    if polynomial.iter().all(|v| v.sign() == Sign::Zero) {
        return Ok(OriginalPlaneRootsOutcome::Coincident);
    }
    let roots = isolate(&polynomial, tolerance, ctx)?
        .into_iter()
        .map(|r| OriginalPlaneRoot {
            interval: r.interval,
            multiplicity: r.multiplicity,
        })
        .collect();
    Ok(OriginalPlaneRootsOutcome::Roots(roots))
}
