//! Brent-style safeguarded root finding on a bracket (item 271).
//!
//! Inverse-quadratic/secant steps with a bisection fallback: the bracket is
//! retained at every iteration, so convergence is guaranteed whenever
//! `f(lo)·f(hi) ≤ 0`, while smooth functions still converge superlinearly.
//! Roots exactly at an endpoint return immediately; a zero-length bracket is
//! accepted only when it is itself a root; non-bracketing signs and iteration
//! exhaustion are errors (never panics).
use crate::{Result, check, numeric_err};

/// Safeguarded Brent root solve of `f` on `[lo, hi]`.
///
/// Returns a root whose enclosing bracket has shrunk to at most `xtol`, or an
/// error: `NURBS_INVALID_INPUT` for malformed brackets (unordered, non-finite,
/// same-sign endpoints, zero-length without an endpoint root) and
/// `NURBS_NUMERIC_ERROR` when `max_iterations` is exhausted or `f` becomes
/// non-finite mid-iteration. The error message carries the iteration count.
pub fn brent_bracketed(
    f: impl Fn(f64) -> f64,
    lo: f64,
    hi: f64,
    xtol: f64,
    max_iterations: usize,
) -> Result<f64> {
    check(
        lo.is_finite() && hi.is_finite(),
        "Bracket endpoints must be finite",
    )?;
    check(lo <= hi, "Bracket endpoints must be ordered")?;
    check(
        xtol.is_finite() && xtol > 0.,
        "Root tolerance must be positive and finite",
    )?;
    let mut a = lo;
    let mut b = hi;
    let mut fa = f(a);
    let mut fb = f(b);
    check(
        fa.is_finite() && fb.is_finite(),
        "Function must be finite at the bracket endpoints",
    )?;
    if fa == 0. {
        return Ok(a);
    }
    if fb == 0. {
        return Ok(b);
    }
    check(
        fa.signum() != fb.signum(),
        "Bracket endpoints do not straddle a root",
    )?;
    let mut c = b;
    let mut fc = fb;
    let mut d = 0.;
    let mut e = 0.;
    for _iteration in 1..=max_iterations {
        if (fb > 0.) == (fc > 0.) {
            c = a;
            fc = fa;
            d = b - a;
            e = d;
        }
        if fc.abs() < fb.abs() {
            a = b;
            b = c;
            c = a;
            fa = fb;
            fb = fc;
            fc = fa;
        }
        let tol1 = 0.5 * xtol;
        let xm = 0.5 * (c - b);
        if xm.abs() <= tol1 || fb == 0. {
            return Ok(b);
        }
        if e.abs() >= tol1 && fa.abs() > fb.abs() {
            // Inverse quadratic interpolation, falling back to a secant step.
            let s = fb / fa;
            let (p, q) = if a == c {
                (2. * xm * s, 1. - s)
            } else {
                let q = fa / fc;
                let r = fb / fc;
                (
                    s * (2. * xm * q * (q - r) - (b - a) * (r - 1.)),
                    (q - 1.) * (r - 1.) * (s - 1.),
                )
            };
            let (p, q) = if p > 0. { (p, -q) } else { (-p, q) };
            if 2. * p < (3. * xm * q - (tol1 * q).abs()).min((0.5 * e * q).abs()) {
                e = d;
                d = p / q;
            } else {
                d = xm;
                e = d;
            }
        } else {
            d = xm;
            e = d;
        }
        a = b;
        fa = fb;
        b += if d.abs() > tol1 {
            d
        } else if xm >= 0. {
            tol1
        } else {
            -tol1
        };
        fb = f(b);
        if !fb.is_finite() {
            return Err(numeric_err(format!(
                "Brent root finding hit a non-finite function value at iteration {_iteration}"
            )));
        }
    }
    Err(numeric_err(format!(
        "Brent root finding did not converge within {max_iterations} iterations"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cubic_root() {
        // x³ − x − 2 has its only real root at ≈ 1.5213797068045676.
        let root = brent_bracketed(|x| x * x * x - x - 2., 1., 2., 1e-13, 100).unwrap();
        assert!((root - 1.5213797068045676).abs() < 1e-12);
    }

    #[test]
    fn sine_root_at_pi() {
        let root = brent_bracketed(f64::sin, 3., 4., 1e-14, 100).unwrap();
        assert!((root - std::f64::consts::PI).abs() < 1e-13);
    }

    #[test]
    fn endpoint_roots_return_immediately() {
        assert_eq!(brent_bracketed(|x| x, 0., 1., 1e-12, 10).unwrap(), 0.);
        assert_eq!(brent_bracketed(|x| x - 1., 0., 1., 1e-12, 10).unwrap(), 1.);
        // Zero-length bracket that is itself a root.
        assert_eq!(brent_bracketed(|x| x - 2., 2., 2., 1e-12, 10).unwrap(), 2.);
    }

    #[test]
    fn non_bracketing_signs_are_rejected() {
        assert!(brent_bracketed(|x| x * x + 1., -1., 1., 1e-12, 100).is_err());
        // Zero-length bracket without a root.
        assert!(brent_bracketed(|x| x - 1., 0., 0., 1e-12, 100).is_err());
        // Unordered / non-finite brackets and bad tolerances.
        assert!(brent_bracketed(|x| x, 1., 0., 1e-12, 100).is_err());
        assert!(brent_bracketed(|x| x, f64::NAN, 1., 1e-12, 100).is_err());
        assert!(brent_bracketed(|x| x, -1., 1., 0., 100).is_err());
        // Same-sign endpoints are rejected even when a root region exists inside.
        assert!(brent_bracketed(|x| x * x, -1., 1., 1e-12, 100).is_err());
    }

    #[test]
    fn iteration_exhaustion_errors_not_panics() {
        let result = brent_bracketed(f64::cos, 0., 3., 1e-14, 1);
        let error = result.expect_err("one iteration cannot converge");
        assert!(error.contains("did not converge within 1 iterations"));
    }

    #[test]
    fn nearly_flat_function_still_converges() {
        // (x − 0.3)⁹ has a high-multiplicity-style flat root region.
        let root = brent_bracketed(|x| (x - 0.3).powi(9), 0., 1., 1e-12, 200).unwrap();
        assert!((root - 0.3).abs() < 1e-3);
    }
}
