//! Crate-private `[f64; 3]` helpers whose semantics deliberately differ from
//! `math_core::linalg`: an overflow-safe `hypot`-based norm, and checked
//! normalization that reports a `Result` error with a caller-chosen message
//! instead of `math_core::unit`'s silent `1e-15` clamp.


