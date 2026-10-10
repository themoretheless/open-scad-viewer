//! The one copy of the tiny binary64 3-vector helpers the intersection engine,
//! the NURBS surface/surface certifier and the analytic pair modules share.
//! The component-wise operations are `math-core`'s; `norm` keeps the
//! `hypot`-chain the intersection reports were frozen with (not `sqrt(dot)`),
//! so no report byte moves by routing through here.
pub use math_core::{add, cross, dot, scale, sub};

/// First three coordinates of a jet point or control point.
#[inline(always)]
pub fn point3(p: &[f64]) -> [f64; 3] {
    [p[0], p[1], p[2]]
}

/// Euclidean length as a `hypot` chain (overflow-safe, correctly rounded per step).
#[inline(always)]
pub fn norm(a: [f64; 3]) -> f64 {
    a[0].hypot(a[1]).hypot(a[2])
}

/// Unit direction, or `None` for a zero or non-finite vector.
pub fn normalize(a: [f64; 3]) -> Option<[f64; 3]> {
    let n = norm(a);
    if !n.is_finite() || n == 0. {
        None
    } else {
        Some([a[0] / n, a[1] / n, a[2] / n])
    }
}
