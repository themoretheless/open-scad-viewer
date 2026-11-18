//! Absolute-clock rational mixed derivatives, no rounded intermediate fields.
use super::*;
pub(super) fn target(
    h: &[Poly],
    orders: [usize; 2],
    widths: &[Q],
    work: &mut Work,
) -> PResult<Vec<Poly>> {
    let denominator = &h[3];
    let mut numerators = h[..3].to_vec();
    let mut exponent = 1usize;
    for (index, axis) in [Axis::U, Axis::V].into_iter().enumerate() {
        if orders[index] == 0 {
            continue;
        }
        let dw = polynomial::derivative(denominator, axis, &widths[index], work)?;
        for _ in 0..orders[index] {
            let mut next = Vec::new();
            for n in &numerators {
                // d(N/W^k) = ((dN)W-kN(dW))/W^(k+1).
                let dn = polynomial::derivative(n, axis, &widths[index], work)?;
                let left = polynomial::multiply(&dn, denominator, work)?;
                let mut right = polynomial::multiply(n, &dw, work)?;
                for value in right.values_mut() {
                    *value = work.op(value, &Q::from_integer(exponent.into()), |a, b| a * b)?;
                }
                next.push(polynomial::subtract(left, &right, work)?);
            }
            numerators = next;
            exponent += 1;
        }
    }
    let mut weight = BTreeMap::from([((0, 0), Q::one())]);
    for _ in 0..exponent {
        weight = polynomial::multiply(&weight, denominator, work)?;
    }
    numerators.push(weight);
    Ok(numerators)
}
