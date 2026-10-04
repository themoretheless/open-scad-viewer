//! Crate-private `[f64; 3]` helpers whose semantics deliberately differ from
//! `math_core::linalg`: an overflow-safe `hypot`-based norm, and checked
//! normalization that reports a `Result` error with a caller-chosen message
//! instead of `math_core::unit`'s silent `1e-15` clamp.
use crate::{Result, check};

/// Euclidean norm via `hypot` — no overflow for large components, unlike
/// `math_core::norm`'s `dot(a, a).sqrt()`.
pub(crate) fn norm(v: [f64; 3]) -> f64 {
    v[0].hypot(v[1]).hypot(v[2])
}

/// Normalize, failing with `message` when the norm is zero or non-finite.
pub(crate) fn unit(v: [f64; 3], message: &'static str) -> Result<[f64; 3]> {
    let n = norm(v);
    check(n.is_finite() && n > 0., message)?;
    Ok(v.map(|x| x / n))
}
