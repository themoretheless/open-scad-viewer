//! Exact floating-point expansion arithmetic (Shewchuk) shared by the
//! certified tangency classifiers. Every operation here is exact as long as
//! no component overflows or underflows, which `finite` checks; callers must
//! refuse instead of deciding a sign when it fails.

pub(crate) type Expansion = Vec<f64>;

pub(crate) fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let bb = s - a;
    (s, (a - (s - bb)) + (b - bb))
}
/// Shewchuk grow-expansion with zero elimination: nonoverlapping output in
/// increasing magnitude, so the sign is the sign of the last component.
pub(crate) fn grow(e: &[f64], b: f64) -> Expansion {
    let mut q = b;
    let mut out = Vec::with_capacity(e.len() + 1);
    for &x in e {
        let (s, h) = two_sum(q, x);
        if h != 0. {
            out.push(h);
        }
        q = s;
    }
    if q != 0. {
        out.push(q);
    }
    out
}
pub(crate) fn add(a: &[f64], b: &[f64]) -> Expansion {
    b.iter().fold(a.to_vec(), |acc, &x| grow(&acc, x))
}
pub(crate) fn neg(a: &[f64]) -> Expansion {
    a.iter().map(|x| -x).collect()
}
pub(crate) fn sub(a: &[f64], b: &[f64]) -> Expansion {
    add(a, &neg(b))
}
pub(crate) fn scale(a: &[f64], b: f64) -> Expansion {
    let mut out = Vec::new();
    for &x in a {
        let p = x * b;
        let e = x.mul_add(b, -p);
        out = grow(&out, e);
        out = grow(&out, p);
    }
    out
}
pub(crate) fn mul(a: &[f64], b: &[f64]) -> Expansion {
    b.iter().fold(Vec::new(), |acc, &x| add(&acc, &scale(a, x)))
}
pub(crate) fn sign(a: &[f64]) -> i8 {
    match a.last() {
        None => 0,
        Some(x) if *x > 0. => 1,
        Some(_) => -1,
    }
}
/// No component overflowed, and none is small enough that a product error
/// term could have been lost to gradual underflow.
pub(crate) fn finite(a: &[f64]) -> bool {
    a.iter()
        .all(|x| x.is_finite() && (*x == 0. || x.abs() >= f64::MIN_POSITIVE * 4.))
}
/// Binary64 approximation of an expansion (summed from the smallest term).
pub(crate) fn approximate(a: &[f64]) -> f64 {
    a.iter().sum()
}
