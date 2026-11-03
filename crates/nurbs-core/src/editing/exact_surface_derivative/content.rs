//! One common univariate content across EVERY bivariate coefficient channel.
use super::*;
fn coordinates(key: (usize, usize), axis: Axis) -> (usize, usize) {
    match axis {
        Axis::U => key,
        Axis::V => (key.1, key.0),
    }
}
fn column(p: &Poly, axis: Axis, index: usize, w: &mut Work) -> PResult<Vec<Q>> {
    let degree = p
        .keys()
        .map(|&key| coordinates(key, axis).0)
        .max()
        .unwrap_or(0);
    w.charge(p.len() as u64 + degree as u64 + 1)?;
    let mut out = vec![Q::zero(); degree + 1];
    for (&key, x) in p {
        let (power, other) = coordinates(key, axis);
        if other == index {
            out[power] = x.clone();
        }
    }
    uni::trim(&mut out);
    Ok(out)
}
fn other_degree(p: &Poly, axis: Axis) -> usize {
    p.keys()
        .map(|&key| coordinates(key, axis).1)
        .max()
        .unwrap_or(0)
}
fn cancel_axis(h: &[Poly], axis: Axis, w: &mut Work) -> PResult<Vec<Poly>> {
    let mut common = Vec::new();
    // Starting with the denominator ensures an early stop when it is constant.
    // Any constant coefficient slice rules out a nonconstant common axis factor.
    for p in h.iter().rev() {
        for index in 0..=other_degree(p, axis) {
            let values = column(p, axis, index, w)?;
            if values.is_empty() {
                continue;
            }
            common = uni::gcd(common, values, w)?;
            if common.len() <= 1 {
                return Ok(h.to_vec());
            }
        }
    }
    if common.len() <= 1 {
        return Ok(h.to_vec());
    }
    let mut out = Vec::new();
    for p in h {
        let mut reduced = Poly::new();
        for index in 0..=other_degree(p, axis) {
            let values = column(p, axis, index, w)?;
            let (quotient, remainder) = uni::divide(&values, &common, w)?;
            if !remainder.is_empty() || uni::multiply(&quotient, &common, w)? != values {
                return Err(Refusal::UnprovedIdentity);
            }
            for (power, x) in quotient.into_iter().enumerate() {
                if !x.is_zero() {
                    reduced.insert(
                        match axis {
                            Axis::U => (power, index),
                            Axis::V => (index, power),
                        },
                        x,
                    );
                }
            }
        }
        out.push(reduced);
    }
    Ok(out)
}
/// Cancels only factors depending exclusively on U or V, not a general
/// bivariate GCD. No per-row/column scale is chosen: each factor is shared by
/// the complete numerator/denominator net. The caller proves the actual stored
/// result against its original uncancelled target with the same Work ledger.
pub(super) fn cancel(h: &[Poly], w: &mut Work) -> PResult<Vec<Poly>> {
    let u = cancel_axis(h, Axis::U, w)?;
    cancel_axis(&u, Axis::V, w)
}
