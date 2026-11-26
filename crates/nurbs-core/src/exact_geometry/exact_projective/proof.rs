use super::*;
/// Exact rank-four admission; no numerical determinant tolerance.
pub(super) fn invertible(m: &[Vec<Q>], w: &mut Work) -> PResult<()> {
    let mut a = m.to_vec();
    for column in 0..4 {
        let pivot = (column..4)
            .find(|&r| !a[r][column].is_zero())
            .ok_or(Refusal::SingularMatrix)?;
        a.swap(column, pivot);
        for row in column + 1..4 {
            let factor = w.op(&a[row][column], &a[column][column], |a, b| a / b)?;
            for j in column..4 {
                let term = w.op(&factor, &a[column][j], |a, b| a * b)?;
                a[row][j] = w.op(&a[row][j], &term, |a, b| a - b)?;
            }
        }
    }
    Ok(())
}
/// Re-lift actual stored binary64 values and check ONE global proportionality
/// factor over every homogeneous control. Basis linearity then proves identity
/// on every span; positive basis partition and strict weights prove finiteness.
pub(super) fn stored(
    h: &[Vec<Q>],
    points: &[Vec<f64>],
    weights: &[f64],
    w: &mut Work,
) -> PResult<()> {
    let first = w.lift(weights[0])?;
    let scale = w.op(&first, &h[3][0], |a, b| a / b)?;
    if scale.is_zero() {
        return Err(Refusal::UnprovedIdentity);
    }
    for (i, (p, &weight)) in points.iter().zip(weights).enumerate() {
        let weight = w.lift(weight)?;
        for axis in 0..4 {
            let actual = if axis == 3 {
                weight.clone()
            } else {
                let x = w.lift(p[axis])?;
                w.op(&x, &weight, |a, b| a * b)?
            };
            let expected = w.op(&h[axis][i], &scale, |a, b| a * b)?;
            if actual != expected {
                return Err(Refusal::UnprovedIdentity);
            }
        }
    }
    Ok(())
}
