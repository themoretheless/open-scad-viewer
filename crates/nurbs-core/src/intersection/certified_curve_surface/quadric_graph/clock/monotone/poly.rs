//! Bounded exact rational polynomial algebra; ascending powers.
use super::*;
pub(super) type Poly = Vec<Q>;
pub(super) fn trim(p: &mut Poly) {
    while p.last().is_some_and(|x| x.is_zero()) {
        p.pop();
    }
}
pub(super) fn derivative(p: &Poly, w: &mut Work) -> Option<Poly> {
    let mut d = Vec::new();
    for (i, a) in p.iter().enumerate().skip(1) {
        d.push(w.keep(a * q(i as f64))?);
    }
    trim(&mut d);
    Some(d)
}
pub(super) fn normalize(p: &Poly, positive: bool, w: &mut Work) -> Option<Poly> {
    let mut d = p.last()?.clone();
    if positive && d < q(0.) {
        d = -d;
    }
    let mut out = Vec::new();
    for x in p {
        out.push(w.keep(x / &d)?);
    }
    Some(out)
}
pub(super) fn divide(a: &Poly, b: &Poly, w: &mut Work) -> Option<(Poly, Poly)> {
    if b.is_empty() {
        return None;
    }
    let mut r = a.clone();
    let mut out = vec![q(0.); a.len().saturating_sub(b.len()) + 1];
    while !r.is_empty() && r.len() >= b.len() {
        let shift = r.len() - b.len();
        let c = w.keep(r.last()? / b.last()?)?;
        out[shift] = c.clone();
        for i in 0..b.len() {
            let product = w.keep(&c * &b[i])?;
            r[i + shift] = w.keep(&r[i + shift] - product)?;
        }
        trim(&mut r);
    }
    trim(&mut out);
    Some((out, r))
}
pub(super) fn quotient(a: &Poly, b: &Poly, w: &mut Work) -> Option<Poly> {
    let (q, r) = divide(a, b, w)?;
    if !r.is_empty() {
        return None;
    }
    Some(q)
}
pub(super) fn gcd(a: &Poly, b: &Poly, w: &mut Work) -> Option<Poly> {
    let mut a = a.clone();
    let mut b = b.clone();
    while !b.is_empty() {
        let r = divide(&a, &b, w)?.1;
        a = b;
        b = r;
    }
    normalize(&a, false, w)
}
pub(super) fn multiply(a: &Poly, b: &Poly, w: &mut Work) -> Option<Poly> {
    if a.is_empty() || b.is_empty() {
        return Some(Vec::new());
    }
    let mut out = vec![q(0.); a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            let p = w.keep(x * y)?;
            out[i + j] = w.keep(&out[i + j] + p)?;
        }
    }
    trim(&mut out);
    Some(out)
}
pub(super) fn at_one(p: &Poly, w: &mut Work) -> Option<Q> {
    let mut a = q(0.);
    for x in p {
        a = w.keep(a + x)?;
    }
    Some(a)
}
