use super::*;
pub(super) fn construct(
    ctx: &mut PredicateContext<'_>,
    x: &[Expansion],
    uv: &[f64],
    n: usize,
    degree: usize,
    span: usize,
) -> Result<Vec<Expansion>, Reason> {
    let tail = n * 4 + uv.len();
    let mut q = x[tail..tail + 16].to_vec();
    for factors in x[tail + 16..].chunks_exact(2) {
        q[15] = q[15].add(&factors[0].mul(&factors[1], ctx)?, ctx)?;
    }
    let h = super::homogeneous(ctx, &x[..tail], uv, n, degree, span)?;
    let mut coefficients = vec![Rational::zero(); 2 * degree + 1];
    for a in 0..4 {
        for b in 0..4 {
            if q[a * 4 + b].sign() == Sign::Zero {
                continue;
            }
            let factor = Rational::scalar(q[a * 4 + b].clone());
            for i in 0..h[a].len() {
                for j in 0..h[b].len() {
                    let term = h[a][i].mul(&h[b][j], ctx)?.mul(&factor, ctx)?;
                    coefficients[i + j] = coefficients[i + j].add(&term, ctx)?;
                }
            }
        }
    }
    let mut polynomial = Vec::new();
    for (i, c) in coefficients.iter().enumerate() {
        if c.d.sign() != Sign::Positive {
            return Err(Reason::MissingProof);
        }
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
