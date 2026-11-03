use super::*;
pub(super) fn squared_error(
    s: &Surface,
    target: &[Poly],
    denominator: &[Q],
    w: &mut Work,
) -> PResult<Q> {
    let stored = storage::powers(s, w)?;
    let mut minimum = None;
    for row in &s.weights {
        for &weight in row {
            let q = w.lift(weight)?;
            if minimum.as_ref().is_none_or(|a| &q < a) {
                minimum = Some(q);
            }
        }
    }
    let lower = w.op(
        denominator.iter().min().unwrap(),
        minimum.as_ref().unwrap(),
        |a, b| a * b,
    )?;
    if !lower.is_positive() {
        return Err(Refusal::UnprovedDenominator);
    }
    let mut sum = Q::zero();
    for axis in 0..3 {
        let a = poly::multiply(&target[axis], &stored[3], w)?;
        let b = poly::multiply(&stored[axis], &target[3], w)?;
        let residual = poly::subtract(a, &b, w)?;
        let pu = residual.keys().map(|p| p.0).max().unwrap_or(0);
        let pv = residual.keys().map(|p| p.1).max().unwrap_or(0);
        if pu > 50 || pv > 50 {
            return Err(Refusal::OutputDegreeLimit);
        }
        let bound = storage::bernstein(&residual, pu, pv, w)?
            .into_iter()
            .map(|x| x.abs())
            .max()
            .unwrap();
        let error = w.op(&bound, &lower, |a, b| a / b)?;
        let square = w.op(&error, &error, |a, b| a * b)?;
        sum = w.op(&sum, &square, |a, b| a + b)?;
    }
    Ok(sum)
}
