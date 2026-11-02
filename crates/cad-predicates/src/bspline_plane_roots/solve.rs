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
    let tail = n * 4 + uv.len();
    let u = &x[n * 4..tail];
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
        let h = series(&x[..n * 4], u, degree, span, &u[span], degree, ctx)?;
        let width = u[span + 1].sub(&u[span], ctx)?;
        let mut factor = Expansion::scalar(1.);
        let mut coefficients = Vec::new();
        for q in 0..=degree {
            let mut value = Rational::zero();
            for axis in 0..3 {
                value = value.add(
                    &h[q][axis].mul(&Rational::scalar(plane[axis].clone()), ctx)?,
                    ctx,
                )?;
            }
            value = value.sub(&h[q][3].mul(&Rational::scalar(plane[3].clone()), ctx)?, ctx)?;
            value.n = value.n.mul(&factor, ctx)?;
            if value.d.sign() != Sign::Positive {
                return Err(Reason::MissingProof);
            }
            coefficients.push(value);
            if q < degree {
                factor = factor.mul(&width, ctx)?;
            }
        }
        let mut polynomial = Vec::new();
        for i in 0..coefficients.len() {
            let mut value = coefficients[i].n.clone();
            if value.sign() != Sign::Zero {
                for (j, c) in coefficients.iter().enumerate() {
                    if i != j {
                        value = value.mul(&c.d, ctx)?;
                    }
                }
            }
            polynomial.push(value);
        }
        polynomial
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
