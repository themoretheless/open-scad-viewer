//! Bounded exact linear algebra for affine construction proofs.
use super::exact_conics::Refusal;
use crate::core::bounded_rational::Work;
use num_rational::BigRational as Q;
use num_traits::Zero;
type PResult<T> = std::result::Result<T, Refusal>;
pub(super) fn dot(a: &[Q], b: &[Q], w: &mut Work) -> PResult<Q> {
    let mut result = Q::zero();
    for (a, b) in a.iter().zip(b) {
        let p = w.op(a, b, |a, b| a * b)?;
        result = w.op(&result, &p, |a, b| a + b)?;
    }
    Ok(result)
}
pub(super) fn cross(a: &[Q], b: &[Q], w: &mut Work) -> PResult<Vec<Q>> {
    [(1, 2), (2, 0), (0, 1)]
        .into_iter()
        .map(|(i, j)| {
            let x = w.op(&a[i], &b[j], |a, b| a * b)?;
            let y = w.op(&a[j], &b[i], |a, b| a * b)?;
            Ok(w.op(&x, &y, |a, b| a - b)?)
        })
        .collect()
}
