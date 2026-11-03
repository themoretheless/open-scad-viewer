use super::*;
use num_traits::ToPrimitive;
fn coefficient(c: &Q) -> Result<I> {
    let value = c.to_f64().filter(|x| x.is_finite()).ok_or_else(|| {
        crate::input("Exact SS coefficient does not have a finite interval image")
    })?;
    let exact = q(value)?;
    I::new(
        if exact > *c { value.next_down() } else { value },
        if exact < *c { value.next_up() } else { value },
    )
}
pub(super) fn evaluate(p: &ExactSsPolynomial, d: [[f64; 2]; 4], w: &mut IntervalWork) -> Result<I> {
    let mut sum = I::point(0.);
    for (e, c) in &p.terms {
        w.charge()?;
        let mut term = coefficient(c)?;
        for k in 0..4 {
            let coordinate = I::new(d[k][0], d[k][1])?;
            let mut exponent = e[k];
            let mut power = I::point(1.);
            let mut base = coordinate;
            while exponent > 0 {
                if exponent % 2 == 1 {
                    w.charge()?;
                    power = power.mul(base)?;
                }
                exponent /= 2;
                if exponent > 0 {
                    w.charge()?;
                    let squared = base.mul(base)?;
                    base = I::new(squared.lo.max(0.), squared.hi)?;
                }
            }
            w.charge()?;
            term = term.mul(power)?;
        }
        w.charge()?;
        sum = sum.add(term)?;
    }
    Ok(sum)
}
pub(super) fn derivative(
    p: &ExactSsPolynomial,
    axis: usize,
    w: &mut Work,
) -> Result<ExactSsPolynomial> {
    let mut out = ExactSsPolynomial::default();
    for (e, c) in &p.terms {
        if e[axis] > 0 {
            w.charge()?;
            let coefficient = c * Q::from_integer(e[axis].into());
            w.coefficient(&coefficient)?;
            let mut power = *e;
            power[axis] -= 1;
            out.insert(power, coefficient, w)?;
        }
    }
    Ok(out)
}
pub(super) fn project(
    equations: &[ExactSsPolynomial],
    r: [[f64; 3]; 3],
    w: &mut Work,
) -> Result<[ExactSsPolynomial; 3]> {
    let mut out = std::array::from_fn(|_| ExactSsPolynomial::default());
    for i in 0..3 {
        for k in 0..3 {
            let scale = q(r[i][k])?;
            if scale.is_zero() {
                continue;
            }
            let term = equations[k].scale_monomial([0; 4], &scale, w)?;
            for (e, c) in term.terms {
                out[i].insert(e, c, w)?;
            }
        }
    }
    Ok(out)
}
/// Exact translation precedes interval evaluation, retaining cancellations in
/// the original coefficients; the translated polynomial is only an enclosure
/// device and never replaces the owned source equations.
pub(super) fn centered(
    p: &ExactSsPolynomial,
    d: [[f64; 2]; 4],
    iw: &mut IntervalWork,
    w: &mut Work,
) -> Result<I> {
    let centers: [f64; 4] = d.map(|v| v[0] * 0.5 + v[1] * 0.5);
    let mut translated = ExactSsPolynomial::default();
    for (e, c) in &p.terms {
        let mut term = ExactSsPolynomial::default();
        term.insert([0; 4], c.clone(), w)?;
        for k in 0..4 {
            let mut factor = ExactSsPolynomial::default();
            factor.insert([0; 4], q(centers[k])?, w)?;
            let mut linear = [0; 4];
            linear[k] = 1;
            factor.insert(linear, Q::one(), w)?;
            for _ in 0..e[k] {
                term = term.product(&factor, w)?;
            }
        }
        for (e, c) in term.terms {
            translated.insert(e, c, w)?;
        }
    }
    let mut delta = [[0.; 2]; 4];
    for k in 0..4 {
        iw.charge()?;
        let v = I::new(d[k][0], d[k][1])?.sub(I::point(centers[k]))?;
        delta[k] = [v.lo, v.hi];
    }
    evaluate(&translated, delta, iw)
}
