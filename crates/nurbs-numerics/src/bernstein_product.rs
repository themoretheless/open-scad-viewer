//! Products of Bernstein polynomials held as outward intervals, with exact binomials
//! where binary64 can represent them.
use crate::{Result, check, distance_bounds::Interval};

pub fn choose(n: usize, k: usize) -> f64 {
    // n <= 50: all integer intermediates and results fit u64, and the result
    // is below 2^53, hence exactly representable as binary64.
    let mut v = 1u64;
    for i in 0..k.min(n - k) {
        v = v * (n - i) as u64 / (i + 1) as u64;
    }
    v as f64
}
pub fn binomials(n: usize) -> Result<Vec<Interval>> {
    if n <= 48 {
        return Ok((0..=n).map(|k| Interval::point(choose(n, k))).collect());
    }
    // Above the exactly representable integer range, retain each division
    // and product as an outward interval. Never round a large binomial into
    // an allegedly exact authored scalar, or overflow the old u64 recurrence.
    let mut out = vec![Interval::point(1.); n + 1];
    for k in 1..=n / 2 {
        out[k] = out[k - 1]
            .mul(Interval::point((n - k + 1) as f64))?
            .div(Interval::point(k as f64))?;
        out[n - k] = out[k];
    }
    Ok(out)
}
pub fn mul(a: &[Interval], b: &[Interval]) -> Result<Vec<Interval>> {
    let (p, q) = (a.len() - 1, b.len() - 1);
    // Position composition keeps degree <=160; graph jet residuals need <=256.
    check(
        p + q <= 256,
        "Correlated composition polynomial work exceeds degree 256",
    )?;
    let ca = binomials(p)?;
    let cb = binomials(q)?;
    let cc = binomials(p + q)?;
    let mut out = vec![Interval::point(0.); p + q + 1];
    for (i, &x) in a.iter().enumerate() {
        for (j, &y) in b.iter().enumerate() {
            let factor = ca[i].mul(cb[j])?.div(cc[i + j])?;
            out[i + j] = out[i + j].add(x.mul(y)?.mul(factor)?)?;
        }
    }
    Ok(out)
}
