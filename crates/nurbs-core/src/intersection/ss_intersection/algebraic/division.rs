//! Exact multivariate polynomial division in a fixed admissible lex order.
use super::*;
pub(super) fn lead(p: &ExactSsPolynomial) -> Option<([usize; 4], Q)> {
    p.terms.last_key_value().map(|(e, c)| (*e, c.clone()))
}
pub(super) fn monic(p: &ExactSsPolynomial, w: &mut Work) -> Result<ExactSsPolynomial> {
    let Some((_, c)) = lead(p) else {
        return Ok(p.clone());
    };
    w.charge()?;
    p.scale_monomial([0; 4], &(Q::one() / c), w)
}
pub(super) fn divide(
    a: &ExactSsPolynomial,
    b: &ExactSsPolynomial,
    w: &mut Work,
) -> Result<(ExactSsPolynomial, ExactSsPolynomial)> {
    let (be, bc) = lead(b).ok_or_else(|| crate::input("Exact SS division by zero"))?;
    let mut pending = a.clone();
    let mut quotient = ExactSsPolynomial::default();
    let mut remainder = ExactSsPolynomial::default();
    while let Some((e, c)) = lead(&pending) {
        w.charge()?;
        if (0..4).all(|k| be[k] <= e[k]) {
            let shift = std::array::from_fn(|k| e[k] - be[k]);
            w.charge()?;
            let scale = c / &bc;
            w.coefficient(&scale)?;
            let term = b.scale_monomial(shift, &scale, w)?;
            pending = pending.subtract(&term, w)?;
            quotient.insert(shift, scale, w)?;
        } else {
            pending.terms.remove(&e);
            remainder.insert(e, c, w)?;
        }
    }
    Ok((quotient, remainder))
}
pub(super) fn exact(
    a: &ExactSsPolynomial,
    b: &ExactSsPolynomial,
    w: &mut Work,
) -> Result<ExactSsPolynomial> {
    let (q, r) = divide(a, b, w)?;
    check(r.is_zero(), "Exact SS proposed divisor does not divide")?;
    Ok(q)
}
pub(super) fn constant(q: Q) -> ExactSsPolynomial {
    if q.is_zero() {
        ExactSsPolynomial::default()
    } else {
        ExactSsPolynomial {
            terms: std::collections::BTreeMap::from([([0; 4], q)]),
        }
    }
}
pub(super) fn is_one(p: &ExactSsPolynomial) -> bool {
    p.terms.len() == 1 && p.terms.get(&[0; 4]) == Some(&Q::one())
}
