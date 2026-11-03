use super::*;
/// Fresh stored output bits, full polynomial cross-products. The target is the
/// un-cancelled original-source derivative, not the construction's quotient.
pub(super) fn stored(c: &Curve, target: &[Poly], w: &mut Work) -> PResult<()> {
    let dim = c.control_points[0].len();
    let mut channels = vec![Vec::new(); dim + 1];
    for (p, &weight) in c.control_points.iter().zip(&c.weights) {
        let weight = w.lift(weight)?;
        for axis in 0..dim {
            let x = w.lift(p[axis])?;
            channels[axis].push(w.op(&x, &weight, |x, w| x * w)?);
        }
        channels[dim].push(weight);
    }
    let powers = channels
        .iter()
        .map(|row| poly::power(row, w).map_err(Into::into))
        .collect::<PResult<Vec<_>>>()?;
    for axis in 0..dim {
        let left = poly::multiply(&powers[axis], &target[dim], w)?;
        let right = poly::multiply(&target[axis], &powers[dim], w)?;
        if left != right {
            return Err(Refusal::UnprovedIdentity);
        }
    }
    Ok(())
}
