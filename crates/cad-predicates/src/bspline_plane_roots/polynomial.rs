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
