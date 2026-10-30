//! airfoil algorithm: naca5_camber.
use super::*;
/// Solve the 5-digit mean-line constants for a design max-camber position
/// `position` (chord fraction, 0.05..0.3) and max camber `camber`.
/// Bracketing bisection followed by Newton polish under SOLVER_BUDGET.
pub fn naca5_camber(position: f64, camber: f64) -> Result<Naca5Camber> {
    require_finite_f64(position, "naca5_position")?;
    require_finite_f64(camber, "naca5_camber")?;
    check(
        (0.05..=0.3).contains(&position),
        "NACA 5-digit camber position must be a chord fraction in [0.05, 0.3]",
    )?;
    check(
        camber > 0. && camber <= 0.06,
        "NACA 5-digit camber must be in (0, 0.06]",
    )?;
    let p = position;
    let f = |m: f64| 3. * p * p - 6. * m * p + m * m * (3. - m);
    let df = |m: f64| -6. * p + 6. * m - 3. * m * m;
    // f(p) = -p^3 < 0 and f(1) = 3p^2 - 6p + 2 > 0 for p in [0.05, 0.3];
    // f' = -3((m-1)^2 + 2p - 1) > 0 on (p, 1), so the root is unique.
    let mut solver_budget =
        Budget::with_iterations(2 * SOLVER_BUDGET)?.guard("naca5-camber-k1-solver");
    let mut lo = p;
    let mut hi = 1.;
    let mut iterations = 0_usize;
    let mut m = 0.5 * (lo + hi);
    for step in 0..SOLVER_BUDGET {
        solver_budget.tick()?;
        iterations = step + 1;
        m = 0.5 * (lo + hi);
        if f(m) > 0. {
            hi = m;
        } else {
            lo = m;
        }
        if hi - lo < 1e-14 {
            break;
        }
    }
    // Newton polish from the bisection midpoint.
    for step in 0..SOLVER_BUDGET {
        solver_budget.tick()?;
        let d = df(m);
        if d.abs() < 1e-300 {
            break;
        }
        let next = m - f(m) / d;
        if !next.is_finite() || next <= p || next >= 1. {
            break;
        }
        let delta = (next - m).abs();
        m = next;
        iterations += step + 1;
        if delta < 1e-15 {
            break;
        }
    }
    crate::numeric(
        (f(m)).abs() < 1e-8,
        "NACA 5-digit camber solver did not converge within budget",
    )?;
    let shape = p * p * p - 3. * m * p * p + m * m * (3. - m) * p;
    crate::numeric(
        shape.is_finite() && shape.abs() > 1e-12,
        "NACA 5-digit camber shape factor is degenerate",
    )?;
    let k1 = 6. * camber / shape;
    Ok(Naca5Camber {
        transition: m,
        k1,
        iterations,
    })
}
