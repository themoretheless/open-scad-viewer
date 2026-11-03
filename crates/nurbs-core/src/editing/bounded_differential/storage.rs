//! Finite storage candidates. Only the subsequent residual proof admits them.
use super::*;
use num_traits::ToPrimitive;
fn rounded(q: &Q, w: &mut Work) -> PResult<f64> {
    w.charge(1 + (q.numer().bits() + q.denom().bits()) / 64)?;
    q.to_f64()
        .filter(|x| x.is_finite())
        .ok_or(Refusal::InvalidOutput)
}
pub(super) fn candidate(channels: &[Vec<Q>], w: &mut Work) -> PResult<(Vec<Vec<f64>>, Vec<f64>)> {
    match super::super::rational_edit::store(channels, w) {
        Ok(out) => return Ok(out),
        Err(super::super::rational_factor::Refusal::NotRepresentable) => {}
        Err(e) => return Err(e.into()),
    }
    let weights = channels.last().unwrap();
    let maximum = weights.iter().max().unwrap();
    let mut points = Vec::new();
    let mut stored_weights = Vec::new();
    for (i, weight) in weights.iter().enumerate() {
        let mut point = Vec::new();
        for row in &channels[..channels.len() - 1] {
            let q = w.op(&row[i], weight, |a, b| a / b)?;
            point.push(rounded(&q, w)?);
        }
        let q = w.op(weight, maximum, |a, b| a / b)?;
        stored_weights.push(rounded(&q, w)?);
        points.push(point);
    }
    Ok((points, stored_weights))
}
