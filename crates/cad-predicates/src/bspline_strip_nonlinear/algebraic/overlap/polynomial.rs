//! Primitive polynomial remainder sequences over exact Q[source].
use super::*;
type T = Vec<P>;
fn divide(a: &Q, b: &Q, ctx: &mut PredicateContext<'_>) -> Result<Q, Reason> {
    if b.n.sign() == Sign::Negative {
        Q::zero()
            .sub(a, ctx)?
            .div_positive(&Q::zero().sub(b, ctx)?, ctx)
    } else {
        a.div_positive(b, ctx)
    }
}
fn division(mut a: P, b: &P, ctx: &mut PredicateContext<'_>) -> Result<(P, P), Reason> {
    if zero(b) {
        return Err(Reason::MissingProof);
    }
    let mut q = vec![Q::zero(); a.len().saturating_sub(b.len()) + 1];
    while !zero(&a) && a.len() >= b.len() {
        ctx.charge(1)?;
        let at = a.len() - b.len();
        let c = divide(a.last().unwrap(), b.last().unwrap(), ctx)?;
        q[at] = c.clone();
        for (i, v) in b.iter().enumerate() {
            a[i + at] = a[i + at].sub(&v.mul(&c, ctx)?, ctx)?;
        }
        a = trim(a);
    }
    Ok((trim(q), a))
}
fn monic(a: P, ctx: &mut PredicateContext<'_>) -> Result<P, Reason> {
    if zero(&a) {
        return Ok(a);
    }
    let lead = a.last().unwrap();
    a.iter().map(|x| divide(x, lead, ctx)).collect()
}
fn gcd(mut a: P, mut b: P, ctx: &mut PredicateContext<'_>) -> Result<P, Reason> {
    while !zero(&b) {
        let r = division(a, &b, ctx)?.1;
        a = b;
        b = r;
    }
    monic(a, ctx)
}
fn trim_t(mut a: T) -> T {
    while a.last().is_some_and(|x| zero(x)) {
        a.pop();
    }
    a
}
fn primitive(a: T, ctx: &mut PredicateContext<'_>) -> Result<T, Reason> {
    let a = trim_t(a);
    if a.is_empty() {
        return Ok(a);
    }
    let mut content = vec![Q::zero()];
    for c in &a {
        content = gcd(content, c.clone(), ctx)?;
        if content.len() == 1 {
            break;
        }
    }
    let mut out = Vec::new();
    for c in a {
        let (q, r) = division(c, &content, ctx)?;
        if !zero(&r) {
            return Err(Reason::MissingProof);
        }
        out.push(q);
    }
    // A constant normalization controls coefficient growth without a rounded pivot.
    let lead = out.last().unwrap().last().unwrap().clone();
    for c in &mut out {
        for q in c {
            *q = divide(q, &lead, ctx)?;
        }
    }
    Ok(out)
}
pub(super) fn to_t(a: &B) -> T {
    let mut out = Vec::new();
    for (&(i, j), q) in a {
        if out.len() <= j {
            out.resize_with(j + 1, || vec![Q::zero()]);
        }
        if out[j].len() <= i {
            out[j].resize(i + 1, Q::zero());
        }
        out[j][i] = q.clone();
    }
    out.into_iter().map(trim).collect()
}
fn from_t(a: T) -> B {
    let mut out = B::new();
    for (j, p) in a.into_iter().enumerate() {
        for (i, q) in p.into_iter().enumerate() {
            if q.n.sign() != Sign::Zero {
                out.insert((i, j), q);
            }
        }
    }
    out
}
pub(super) fn common(a: &B, b: &B, ctx: &mut PredicateContext<'_>) -> Result<B, Reason> {
    let mut a = primitive(to_t(a), ctx)?;
    let mut b = primitive(to_t(b), ctx)?;
    while !b.is_empty() {
        let mut r = a;
        while !r.is_empty() && r.len() >= b.len() {
            ctx.charge(1)?;
            let at = r.len() - b.len();
            let lc = r.last().unwrap().clone();
            let bc = b.last().unwrap();
            for row in &mut r {
                *row = mul(row, bc, ctx)?;
            }
            for (i, row) in b.iter().enumerate() {
                r[i + at] = sub(&r[i + at], &mul(row, &lc, ctx)?, ctx)?;
            }
            r = primitive(trim_t(r), ctx)?;
        }
        a = b;
        b = primitive(r, ctx)?;
    }
    Ok(from_t(primitive(a, ctx)?))
}

/// Remove only exactly verified even multiplicities. If p=r² h with a
/// nonconstant r, positivity of h certifies p>=0 with finitely many zeros.
/// The original p is divided again at the end; no approximate root is used.
pub(super) fn even_deflation(p: &P, ctx: &mut PredicateContext<'_>) -> Result<Option<P>, Reason> {
    if p.len() < 3 || zero(p) {
        return Ok(None);
    }
    let normalized = monic(p.clone(), ctx)?;
    let mut c = gcd(normalized.clone(), derivative(&normalized, ctx)?, ctx)?;
    if c.len() == 1 {
        return Ok(None);
    }
    let (mut w, r) = division(normalized, &c, ctx)?;
    if !zero(&r) {
        return Err(Reason::MissingProof);
    }
    let mut square = vec![one()];
    for multiplicity in 1..p.len() {
        ctx.charge(1)?;
        if w.len() == 1 {
            break;
        }
        let y = gcd(w.clone(), c.clone(), ctx)?;
        let (z, r) = division(w, &y, ctx)?;
        if !zero(&r) {
            return Err(Reason::MissingProof);
        }
        for _ in 0..multiplicity / 2 {
            square = mul(&square, &z, ctx)?;
        }
        let (next, r) = division(c, &y, ctx)?;
        if !zero(&r) {
            return Err(Reason::MissingProof);
        }
        w = y;
        c = next;
    }
    if w.len() != 1 || square.len() == 1 {
        return Ok(None);
    }
    let (q, r) = division(p.clone(), &mul(&square, &square, ctx)?, ctx)?;
    if !zero(&r) || zero(&q) {
        return Err(Reason::MissingProof);
    }
    Ok(Some(q))
}

#[cfg(test)]
mod deflation_tests {
    use super::*;
    use crate::{AuthoredScalar, Limits, SourceArena, ToleranceContext};
    #[test]
    fn even_multiplicity_is_removed_but_an_odd_sign_change_is_not() {
        let arena =
            SourceArena::authored("deflation", 1, vec![AuthoredScalar::Binary64Bits(0)]).unwrap();
        let tol = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&arena, &tol, Limits::default(), None);
        let root = vec![Q::scalar(Expansion::scalar(-0.5)), one()];
        let square = mul(&root, &root, &mut ctx).unwrap();
        let fourth = mul(&square, &square, &mut ctx).unwrap();
        let positive = mul(&fourth, &vec![one(), one()], &mut ctx).unwrap();
        let remainder = even_deflation(&positive, &mut ctx).unwrap().unwrap();
        assert_eq!(remainder.len(), 2);
        assert!(super::super::super::positive(&remainder, &Q::zero(), &one(), &mut ctx).unwrap());
        let odd = mul(&square, &root, &mut ctx).unwrap();
        let remainder = even_deflation(&odd, &mut ctx).unwrap().unwrap();
        assert!(!super::super::super::positive(&remainder, &Q::zero(), &one(), &mut ctx).unwrap());
        assert!(
            !super::super::super::positive(
                &neg(&remainder, &mut ctx).unwrap(),
                &Q::zero(),
                &one(),
                &mut ctx
            )
            .unwrap()
        );
    }
}
