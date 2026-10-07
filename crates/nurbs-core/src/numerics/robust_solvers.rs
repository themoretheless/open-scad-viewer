//! Robust dense nonlinear least-squares solvers and rank diagnostics for the
//! kernel (checklist items 492–497).
//!
//! Contents:
//! - [`rrqr`]: rank-revealing QR with column pivoting for small/medium dense
//!   matrices (row-major `Vec<Vec<f64>>`, matching `foundation.rs` style),
//!   including least-squares and minimum-norm rank-deficient solves.
//! - [`trust_region_solve`]: trust-region nonlinear least-squares solver for
//!   `min ‖F(x)‖` with residual/jacobian callbacks, switchable step strategy
//!   ([`StepStrategy::Hook`] or Powell's [`StepStrategy::Dogleg`], the latter
//!   needs no per-iteration refactorization).
//! - [`levenberg_marquardt_solve`]: standalone Levenberg–Marquardt solver with
//!   adaptive damping (Nielsen update). This is a *general* solver over
//!   arbitrary residual/jacobian callbacks; the light Levenberg damping inside
//!   `foundation/fitting.rs` is a fixed small diagonal bias local to the
//!   fitting normal equations and exposes no adaptive radius / stopping
//!   diagnostics, so nothing is duplicated.
//! - [`trust_region_point_inversion_curve`] /
//!   [`trust_region_point_inversion_surface`]: robust parameter inversion
//!   `min_u ‖C(u) − P‖` and `min_(u,v) ‖S(u,v) − P‖` built on the crate's
//!   analytic `evaluate` derivatives.
//! - [`constraint_rank_report`]: SVD rank diagnostics of a constraint
//!   Jacobian: singular values, numerical rank and indices of conflicting
//!   (linearly dependent) rows.
//!
//! No `unsafe`, no new dependencies. All matrices are row-major
//! `Vec<Vec<f64>>`.

use crate::{Result, check};

mod inversion;
mod lm;
mod rank;
mod rrqr;
mod trust_region;
pub use inversion::{trust_region_point_inversion_curve, trust_region_point_inversion_surface};
pub use lm::levenberg_marquardt_solve;
pub use rank::{ConstraintRankReport, constraint_rank_report};
pub use rrqr::{RankDiagnostics, Rrqr, rrqr};
pub use trust_region::trust_region_solve;

// ---------------------------------------------------------------------------
// Small dense linear algebra helpers (safe, row-major).
// ---------------------------------------------------------------------------

pub(super) fn norm2(v: &[f64]) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
}

pub(super) fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// `Jᵀ J` and `Jᵀ f` for an `m × n` row-major Jacobian.
pub(super) fn normal_equations(j: &[Vec<f64>], f: &[f64]) -> (Vec<Vec<f64>>, Vec<f64>) {
    let m = j.len();
    let n = if m == 0 { 0 } else { j[0].len() };
    let mut a = vec![vec![0.; n]; n];
    let mut g = vec![0.; n];
    for row in 0..m {
        for p in 0..n {
            g[p] += j[row][p] * f[row];
            for q in p..n {
                a[p][q] += j[row][p] * j[row][q];
            }
        }
    }
    for p in 0..n {
        for q in 0..p {
            a[p][q] = a[q][p];
        }
    }
    (a, g)
}

/// Cholesky solve `A x = b` for symmetric positive definite `A` (lower `L`
/// with `A = L Lᵀ`). Returns `None` when a pivot is non-positive (matrix not
/// numerically SPD); callers then damp or fall back.
pub(super) fn cholesky_solve(a: &[Vec<f64>], b: &[f64]) -> Option<Vec<f64>> {
    let n = a.len();
    let mut l = vec![vec![0.; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let mut s = a[i][j];
            for k in 0..j {
                s -= l[i][k] * l[j][k];
            }
            if i == j {
                if !(s > 0.) || !s.is_finite() {
                    return None;
                }
                l[i][j] = s.sqrt();
            } else {
                l[i][j] = s / l[j][j];
            }
        }
    }
    let mut y = b.to_vec();
    for i in 0..n {
        for k in 0..i {
            y[i] -= l[i][k] * y[k];
        }
        y[i] /= l[i][i];
    }
    let mut x = y;
    for i in (0..n).rev() {
        for k in i + 1..n {
            x[i] -= l[k][i] * x[k];
        }
        x[i] /= l[i][i];
    }
    Some(x)
}

/// Solve upper-triangular `R x = b` (rows beyond `R.len()` ignored).
pub(super) fn back_substitute(r: &[Vec<f64>], b: &[f64]) -> Result<Vec<f64>> {
    let n = r.len();
    let mut x = b.to_vec();
    for i in (0..n).rev() {
        check(
            r[i][i].abs() > 0. && r[i][i].is_finite(),
            "Triangular factor has a zero pivot",
        )?;
        for k in i + 1..n {
            x[i] -= r[i][k] * x[k];
        }
        x[i] /= r[i][i];
    }
    Ok(x)
}

// ---------------------------------------------------------------------------
// Shared problem/options/report types for the nonlinear solvers.
// ---------------------------------------------------------------------------

/// Residual + Jacobian callback contract for `min ‖F(x)‖₂`, `x ∈ Rⁿ`,
/// `F: Rⁿ → Rᵐ`. The Jacobian is row-major `m × n`.
pub trait LeastSquaresProblem {
    fn residual(&self, x: &[f64]) -> Result<Vec<f64>>;
    fn jacobian(&self, x: &[f64]) -> Result<Vec<Vec<f64>>>;
}

/// Adapter so plain closures satisfy [`LeastSquaresProblem`].
pub struct ClosureProblem<F, G> {
    pub residual: F,
    pub jacobian: G,
}

impl<F, G> LeastSquaresProblem for ClosureProblem<F, G>
where
    F: Fn(&[f64]) -> Result<Vec<f64>>,
    G: Fn(&[f64]) -> Result<Vec<Vec<f64>>>,
{
    fn residual(&self, x: &[f64]) -> Result<Vec<f64>> {
        (self.residual)(x)
    }
    fn jacobian(&self, x: &[f64]) -> Result<Vec<Vec<f64>>> {
        (self.jacobian)(x)
    }
}

/// Step strategy inside the trust-region solver.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepStrategy {
    /// Hook step: solve `(JᵀJ + λI) d = −Jᵀf` with `λ` iterated so that
    /// `‖d‖ ≈ Δ` (Moré's exact trust-region step; needs a Cholesky
    /// factorization per λ trial).
    Hook,
    /// Powell's dogleg: piecewise-linear interpolation between the Cauchy
    /// (steepest-descent) step and the Gauss–Newton step, truncated at `Δ`.
    /// No factorization per radius adjustment — only one normal-equation
    /// solve per accepted model build.
    Dogleg,
}

/// Trust-region / LM solver options.
#[derive(Clone, Copy, Debug)]
pub struct SolverOptions {
    /// Maximum number of iterations.
    pub max_iterations: usize,
    /// Step stopping tolerance: stop when `‖d‖ ≤ step_epsilon·(‖x‖+step_epsilon)`.
    pub step_epsilon: f64,
    /// Residual stopping tolerance on `‖F(x)‖₂`.
    pub residual_epsilon: f64,
    /// Gradient stopping tolerance on `‖JᵀF‖∞`.
    pub gradient_epsilon: f64,
    /// Initial trust radius (or initial damping for LM).
    pub initial_radius: f64,
    /// Maximum trust radius.
    pub max_radius: f64,
    /// Acceptance threshold on the gain ratio ρ.
    pub acceptance_ratio: f64,
    /// Step strategy (trust-region solver only).
    pub strategy: StepStrategy,
}

impl Default for SolverOptions {
    fn default() -> Self {
        Self {
            max_iterations: 200,
            step_epsilon: 1e-12,
            residual_epsilon: 1e-12,
            gradient_epsilon: 1e-12,
            initial_radius: 1.,
            max_radius: 1e12,
            acceptance_ratio: 1e-4,
            strategy: StepStrategy::Dogleg,
        }
    }
}

/// Why the solver stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    /// `‖d‖ ≤ eps·(‖x‖ + eps)` — backward step vanishes.
    StepTolerance,
    /// `‖F‖ ≤ residual_epsilon`.
    ResidualTolerance,
    /// `‖JᵀF‖∞ ≤ gradient_epsilon`.
    GradientTolerance,
    /// Iteration budget exhausted.
    MaxIterations,
}

/// Solver diagnostics.
#[derive(Clone, Debug)]
pub struct SolverReport {
    pub converged: bool,
    pub iterations: usize,
    pub residual_norm: f64,
    pub reason: StopReason,
    /// Final trust radius (final damping λ for LM).
    pub radius: f64,
}

pub(super) fn check_problem_consistency(f: &[f64], j: &[Vec<f64>], n: usize) -> Result<()> {
    check(
        j.len() == f.len()
            && j.iter().all(|row| row.len() == n)
            && f.iter().all(|v| v.is_finite())
            && j.iter().flatten().all(|v| v.is_finite()),
        "Residual/Jacobian callback returned inconsistent or non-finite data",
    )
}

/// Predicted reduction of the linear model at step `d`:
/// `−(gᵀd + ½ dᵀ JᵀJ d)`, evaluated without forming JᵀJ explicitly.
pub(super) fn predicted_reduction(j: &[Vec<f64>], g: &[f64], d: &[f64]) -> f64 {
    let mut jd = vec![0.; j.len()];
    for (out, row) in jd.iter_mut().zip(j) {
        *out = dot(row, d);
    }
    -(dot(g, d) + 0.5 * dot(&jd, &jd))
}

// ---------------------------------------------------------------------------
// Shared test fixtures used by the solver test modules.
// ---------------------------------------------------------------------------

#[cfg(test)]
pub(crate) mod test_helpers {
    use super::LeastSquaresProblem;
    use crate::Result;

    pub(crate) fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    /// Rosenbrock residuals: f₁ = 10(x₂ − x₁²), f₂ = 1 − x₁.
    pub(crate) struct Rosenbrock;
    impl LeastSquaresProblem for Rosenbrock {
        fn residual(&self, x: &[f64]) -> Result<Vec<f64>> {
            Ok(vec![10. * (x[1] - x[0] * x[0]), 1. - x[0]])
        }
        fn jacobian(&self, x: &[f64]) -> Result<Vec<Vec<f64>>> {
            Ok(vec![vec![-20. * x[0], 10.], vec![-1., 0.]])
        }
    }
}
