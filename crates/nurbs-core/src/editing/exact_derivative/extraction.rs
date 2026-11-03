use super::*;
/// Exact polynomial de Boor on u=a+width*t. No floating trim/decomposition or
/// rounded homogeneous products are admitted as original-source history.
pub(super) fn span(s: &Curve, k: usize, a: &Q, width: &Q, w: &mut Work) -> PResult<Vec<Poly>> {
    let p = s.degree;
    let dim = s.control_points[0].len();
    let mut net = Vec::new();
    for i in k - p..=k {
        let weight = w.lift(s.weights[i])?;
        let mut row = Vec::new();
        for &x in &s.control_points[i] {
            let x = w.lift(x)?;
            row.push(vec![w.op(&x, &weight, |x, w| x * w)?]);
        }
        row.push(vec![weight]);
        net.push(row);
    }
    for r in 1..=p {
        for j in (r..=p).rev() {
            let i = k - p + j;
            let left = w.lift(s.knots[i])?;
            let right = w.lift(s.knots[i + p - r + 1])?;
            let den = w.op(&right, &left, |a, b| a - b)?;
            if den <= Q::zero() {
                return Err(Refusal::InvalidOutput);
            }
            let start = w.op(a, &left, |a, b| a - b)?;
            let alpha = vec![
                w.op(&start, &den, |a, b| a / b)?,
                w.op(width, &den, |a, b| a / b)?,
            ];
            let beta = vec![
                w.op(&Q::one(), &alpha[0], |a, b| a - b)?,
                w.op(&Q::zero(), &alpha[1], |a, b| a - b)?,
            ];
            for axis in 0..=dim {
                let x = poly::multiply(&alpha, &net[j][axis], w)?;
                let y = poly::multiply(&beta, &net[j - 1][axis], w)?;
                net[j][axis] = add(&x, &y, false, w)?;
            }
        }
    }
    Ok(net.pop().unwrap())
}
