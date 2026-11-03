use super::*;
fn sum(a: Vec<Q>, b: Vec<Q>, w: &mut Work) -> PResult<Vec<Q>> {
    let mut out = vec![Q::zero(); a.len().max(b.len())];
    for (i, value) in out.iter_mut().enumerate() {
        let zero = Q::zero();
        *value = w.op(
            a.get(i).unwrap_or(&zero),
            b.get(i).unwrap_or(&zero),
            |a, b| a + b,
        )?;
    }
    uni::trim(&mut out);
    Ok(out)
}
/// Local Cox-de Boor polynomial basis for the ACTIVE span, with exact source
/// knots. This admits unclamped/periodic encodings without rounded trimming.
fn basis(
    p: usize,
    k: usize,
    knots: &[f64],
    a: f64,
    width: &Q,
    w: &mut Work,
) -> PResult<Vec<Vec<Q>>> {
    let a = w.lift(a)?;
    let mut prev = vec![vec![Q::one()]];
    for degree in 1..=p {
        let start = k - degree;
        let mut next = Vec::new();
        for i in start..=k {
            let mut value = Vec::new();
            if i > start {
                let lo = w.lift(knots[i])?;
                let hi = w.lift(knots[i + degree])?;
                let den = w.op(&hi, &lo, |a, b| a - b)?;
                if !den.is_zero() {
                    let distance = w.op(&a, &lo, |a, b| a - b)?;
                    let linear = vec![
                        w.op(&distance, &den, |a, b| a / b)?,
                        w.op(width, &den, |a, b| a / b)?,
                    ];
                    value = uni::multiply(&linear, &prev[i - start - 1], w)?;
                }
            }
            if i < k {
                let lo = w.lift(knots[i + 1])?;
                let hi = w.lift(knots[i + degree + 1])?;
                let den = w.op(&hi, &lo, |a, b| a - b)?;
                if !den.is_zero() {
                    let distance = w.op(&hi, &a, |a, b| a - b)?;
                    let linear = vec![
                        w.op(&distance, &den, |a, b| a / b)?,
                        w.op(width, &den, |a, b| -a / b)?,
                    ];
                    let right = uni::multiply(&linear, &prev[i - start], w)?;
                    value = sum(value, right, w)?;
                }
            }
            next.push(value);
        }
        prev = next;
    }
    Ok(prev)
}
pub(super) fn cell(
    s: &Surface,
    ku: usize,
    kv: usize,
    domains: [[f64; 2]; 2],
    widths: &[Q],
    w: &mut Work,
) -> PResult<Vec<Poly>> {
    let u = basis(s.degree_u, ku, &s.knots_u, domains[0][0], &widths[0], w)?;
    let v = basis(s.degree_v, kv, &s.knots_v, domains[1][0], &widths[1], w)?;
    let mut out = vec![Poly::new(); 4];
    for (i, bu) in u.iter().enumerate() {
        for (j, bv) in v.iter().enumerate() {
            let row = ku - s.degree_u + i;
            let col = kv - s.degree_v + j;
            let weight = w.lift(s.weights[row][col])?;
            let mut h = Vec::new();
            for &x in &s.control_points[row][col] {
                let x = w.lift(x)?;
                h.push(w.op(&x, &weight, |x, w| x * w)?);
            }
            h.push(weight);
            for (a, x) in bu.iter().enumerate() {
                if x.is_zero() {
                    continue;
                }
                for (b, y) in bv.iter().enumerate() {
                    if y.is_zero() {
                        continue;
                    }
                    let coefficient = w.op(x, y, |a, b| a * b)?;
                    for axis in 0..4 {
                        let value = w.op(&coefficient, &h[axis], |a, b| a * b)?;
                        polynomial::accumulate(&mut out[axis], (a, b), &value, w)?;
                    }
                }
            }
        }
    }
    Ok(out)
}
