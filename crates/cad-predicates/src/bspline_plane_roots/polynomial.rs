//! Original-span polynomial from exact homogeneous derivatives; no fitted data.
use super::*;
pub(super) fn original_polynomial(
    ctx: &mut PredicateContext<'_>,
    x: &[Expansion],
    uv: &[f64],
    n: usize,
    degree: usize,
    span: usize,
    plane: &[Expansion; 4],
) -> Result<Vec<Expansion>, Reason> {
    if uv[span - degree + 1] == uv[span] && uv[span + degree] == uv[span + 1] {
        let mut residuals = Vec::new();
        for i in span - degree..=span {
            let mut value = plane[3].negated();
            for axis in 0..3 {
                if plane[axis].sign() == Sign::Zero {
                    continue;
                }
                // Exact unit coefficients need no floating product. In
                // particular ±1 times an authored subnormal is the original
                // exact expansion (or its negation), not a guarded splitter.
                let coordinate = &x[i * 4 + axis];
                let term = if plane[axis].sub(&Expansion::scalar(1.), ctx)?.sign() == Sign::Zero {
                    coordinate.clone()
                } else if plane[axis].add(&Expansion::scalar(1.), ctx)?.sign() == Sign::Zero {
                    coordinate.negated()
                } else {
                    coordinate.mul(&plane[axis], ctx)?
                };
                value = value.add(&term, ctx)?;
            }
            residuals.push(value);
        }
        // A common positive power-of-two scale preserves the exact sign and
        // zeros. Apply it BEFORE Bernstein differentiation/weight products so
        // subnormal authored residuals need not underflow during cancellation.
        Expansion::normalize_common(&mut residuals, ctx)?;
        for (j, value) in residuals.iter_mut().enumerate() {
            *value = value.mul(&x[(span - degree + j) * 4 + 3], ctx)?;
        }
        Expansion::normalize_common(&mut residuals, ctx)?;
        return power(&residuals, ctx);
    }
    let knots = &x[n * 4..n * 4 + uv.len()];
    let h = series(&x[..n * 4], knots, degree, span, &knots[span], degree, ctx)?;
    let width = knots[span + 1].sub(&knots[span], ctx)?;
    let mut factor = Expansion::scalar(1.);
    let mut coefficients = Vec::new();
    for order in 0..=degree {
        let mut value = Rational::zero();
        for axis in 0..3 {
            value = value.add(
                &h[order][axis].mul(&Rational::scalar(plane[axis].clone()), ctx)?,
                ctx,
            )?;
        }
        value = value.sub(
            &h[order][3].mul(&Rational::scalar(plane[3].clone()), ctx)?,
            ctx,
        )?;
        value.n = value.n.mul(&factor, ctx)?;
        if value.d.sign() != Sign::Positive {
            return Err(Reason::MissingProof);
        }
        coefficients.push(value);
        if order < degree {
            factor = factor.mul(&width, ctx)?;
        }
    }
    let mut polynomial = Vec::new();
    for (i, c) in coefficients.iter().enumerate() {
        let mut value = c.n.clone();
        if value.sign() != Sign::Zero {
            for (j, d) in coefficients.iter().enumerate() {
                if i != j {
                    value = value.mul(&d.d, ctx)?;
                }
            }
        }
        polynomial.push(value);
    }
    Ok(polynomial)
}
