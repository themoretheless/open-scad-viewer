//! Exact positive tensor-geometric Bernstein weights. The corresponding
//! rational basis is a polynomial Bernstein basis after independent increasing
//! Mobius clocks. No fitted ratio or rounded control-point replacement.
use num_rational::BigRational as Q;
const MAX_BITS: u64 = 8192;
fn charge(used: &mut usize, max: usize) -> bool {
    if *used >= max {
        false
    } else {
        *used += 1;
        true
    }
}
fn bounded(q: &Q) -> bool {
    q.numer().bits() <= MAX_BITS && q.denom().bits() <= MAX_BITS
}
fn equal_products(a: &Q, b: &Q, c: &Q, d: &Q, used: &mut usize, max: usize) -> bool {
    if !charge(used, max) {
        return false;
    }
    let lhs = a * b;
    if !bounded(&lhs) {
        return false;
    }
    if !charge(used, max) {
        return false;
    }
    let rhs = c * d;
    if !bounded(&rhs) {
        return false;
    }
    if !charge(used, max) {
        return false;
    }
    lhs == rhs
}
pub(crate) fn tensor(weights: &[Vec<f64>], used: &mut usize, max: usize) -> bool {
    if weights.is_empty()
        || weights.len() > 33
        || weights[0].is_empty()
        || weights[0].len() > 33
        || weights.iter().any(|r| r.len() != weights[0].len())
    {
        return false;
    }
    let mut exact = Vec::with_capacity(weights.len());
    for row in weights {
        let mut out = Vec::with_capacity(row.len());
        for &w in row {
            if !charge(used, max) || !w.is_finite() || w <= 0. {
                return false;
            }
            let Some(q) = Q::from_float(w) else {
                return false;
            };
            if !bounded(&q) {
                return false;
            }
            out.push(q);
        }
        exact.push(out);
    }
    for i in 0..exact.len() {
        for j in 0..exact[0].len() {
            if i > 0
                && !equal_products(
                    &exact[i][j],
                    &exact[0][0],
                    &exact[i - 1][j],
                    &exact[1][0],
                    used,
                    max,
                )
            {
                return false;
            }
            if j > 0
                && !equal_products(
                    &exact[i][j],
                    &exact[0][0],
                    &exact[i][j - 1],
                    &exact[0][1],
                    used,
                    max,
                )
            {
                return false;
            }
        }
    }
    true
}
pub(crate) fn sequence(weights: &[f64], used: &mut usize, max: usize) -> bool {
    tensor(&[weights.to_vec()], used, max)
}
