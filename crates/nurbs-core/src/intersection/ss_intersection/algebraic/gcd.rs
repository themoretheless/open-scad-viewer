//! Multivariate gcd over Q via Gauss's lemma and Euclid over successive rational
//! function fields. Active variables are derived from the original equations;
//! no surface family or prescribed repeated-factor shape is recognized.
use super::division::{constant, exact, is_one, monic};
use super::*;
mod field;
use field::Fraction;
fn trim(coefficients: &mut Vec<Fraction>) {
    while coefficients.last().is_some_and(|c| c.numerator.is_zero()) {
        coefficients.pop();
    }
}
fn coefficients(
    p: &ExactSsPolynomial,
    axis: usize,
    w: &mut Work,
) -> Result<Vec<ExactSsPolynomial>> {
    let degree = p.terms.keys().map(|e| e[axis]).max().unwrap_or(0);
    if degree >= w.limits.max_terms {
        return Err(crate::resource(
            "Exact SS dense elimination degree exhausted",
        ));
    }
    let mut out = vec![ExactSsPolynomial::default(); degree + 1];
    for (e, c) in &p.terms {
        let mut a = *e;
        a[axis] = 0;
        out[e[axis]].insert(a, c.clone(), w)?;
    }
    Ok(out)
}
fn content(
    coefficients: &[ExactSsPolynomial],
    variables: &[usize],
    w: &mut Work,
) -> Result<ExactSsPolynomial> {
    let mut g = ExactSsPolynomial::default();
    for c in coefficients {
        if c.is_zero() {
            continue;
        }
        g = compute(&g, c, variables, w)?;
        if is_one(&g) {
            break;
        }
    }
    Ok(g)
}
fn remainder(
    mut a: Vec<Fraction>,
    b: &[Fraction],
    variables: &[usize],
    w: &mut Work,
) -> Result<Vec<Fraction>> {
    while a.len() >= b.len() && !a.is_empty() {
        w.charge()?;
        let shift = a.len() - b.len();
        let factor = a.last().unwrap().divide(b.last().unwrap(), variables, w)?;
        for i in 0..b.len() {
            let term = factor.product(&b[i], variables, w)?;
            a[i + shift] = a[i + shift].subtract(&term, variables, w)?;
        }
        trim(&mut a);
    }
    Ok(a)
}
pub(super) fn compute(
    a: &ExactSsPolynomial,
    b: &ExactSsPolynomial,
    variables: &[usize],
    w: &mut Work,
) -> Result<ExactSsPolynomial> {
    w.charge()?;
    if a.is_zero() {
        return monic(b, w);
    }
    if b.is_zero() {
        return monic(a, w);
    }
    if is_one(a) || is_one(b) {
        return Ok(constant(Q::one()));
    }
    let Some((&axis, rest)) = variables.split_last() else {
        return Ok(constant(Q::one()));
    };
    let ac = coefficients(a, axis, w)?;
    let bc = coefficients(b, axis, w)?;
    let ca = content(&ac, rest, w)?;
    let cb = content(&bc, rest, w)?;
    let common = compute(&ca, &cb, rest, w)?;
    let mut p = ac
        .iter()
        .map(|c| Ok(Fraction::polynomial(exact(c, &ca, w)?)))
        .collect::<Result<Vec<_>>>()?;
    let mut q = bc
        .iter()
        .map(|c| Ok(Fraction::polynomial(exact(c, &cb, w)?)))
        .collect::<Result<Vec<_>>>()?;
    trim(&mut p);
    trim(&mut q);
    while !q.is_empty() {
        let r = remainder(p, &q, rest, w)?;
        p = q;
        q = r;
    }
    let leading = p.last().unwrap().clone();
    for c in &mut p {
        *c = c.divide(&leading, rest, w)?;
    }
    // A common denominator need not be minimal. Primitive-content extraction
    // removes all introduced factors before restoring the original content gcd.
    let mut denominator = constant(Q::one());
    for c in &p {
        denominator = denominator.product(&c.denominator, w)?;
    }
    let mut cleared = Vec::new();
    for c in p {
        let scale = exact(&denominator, &c.denominator, w)?;
        cleared.push(c.numerator.product(&scale, w)?);
    }
    let primitive_content = content(&cleared, rest, w)?;
    let mut primitive = ExactSsPolynomial::default();
    for (i, c) in cleared.iter().enumerate() {
        let coefficient = exact(c, &primitive_content, w)?;
        let mut power = [0; 4];
        power[axis] = i;
        let term = coefficient.scale_monomial(power, &Q::one(), w)?;
        for (e, c) in term.terms {
            primitive.insert(e, c, w)?;
        }
    }
    monic(&primitive.product(&common, w)?, w)
}
pub(super) fn variables(p: &ExactSsPolynomial) -> Vec<usize> {
    (0..4)
        .filter(|&k| p.terms.keys().any(|e| e[k] > 0))
        .collect()
}
#[cfg(test)]
mod tests;
