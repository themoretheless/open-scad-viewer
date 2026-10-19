//! Shared error-free ORIGINAL affine coefficient construction; failures refuse.
pub(crate) fn exact_add(a: f64, b: f64) -> Option<f64> {
    let s = a + b;
    let bv = s - a;
    let av = s - bv;
    let br = b - bv;
    let ar = a - av;
    (s.is_finite() && ar + br == 0.).then_some(s)
}
pub(crate) fn exact_mul(a: f64, b: f64) -> Option<f64> {
    if a == 1. && b.is_finite() {
        return Some(b);
    }
    if b == 1. && a.is_finite() {
        return Some(a);
    }
    if a == 0. || b == 0. {
        return Some(0.);
    }
    let p = a * b;
    (p.is_finite()
        && p.abs() >= 1e-100
        && a.abs() >= 1e-100
        && b.abs() >= 1e-100
        && a.mul_add(b, -p) == 0.)
        .then_some(p)
}
pub(crate) fn image(pose: [[f64; 3]; 4], p: &[f64], origin: [f64; 3]) -> Option<[f64; 3]> {
    let p = [
        exact_add(p[0], -origin[0])?,
        exact_add(p[1], -origin[1])?,
        exact_add(p[2], -origin[2])?,
    ];
    let mut q = pose[3];
    for col in 0..3 {
        for k in 0..3 {
            q[k] = exact_add(q[k], exact_mul(pose[col][k], p[col])?)?;
        }
    }
    Some(q)
}
