use super::{
    LeastSquaresProblem, SolverOptions, SolverReport, StopReason, check_problem_consistency,
    cholesky_solve, norm2, normal_equations, predicted_reduction,
};
use crate::{Result, check};

/// Standalone Levenberg–Marquardt solver with adaptive damping (Nielsen's
/// update: `λ ← λ·max(1/3, 1 − (2ρ−1)³)` on acceptance, `λ ← λ·ν`, `ν ← 2ν`
/// on rejection). Unlike the fixed light diagonal damping hard-wired into the
/// fitting normal equations of `foundation/fitting.rs`, this is a general
/// callback-driven solver with full stopping diagnostics.
pub fn levenberg_marquardt_solve<P: LeastSquaresProblem>(
    problem: &P,
    x0: &[f64],
    options: SolverOptions,
) -> Result<(Vec<f64>, SolverReport)> {
    let n = x0.len();
    check(
        n > 0 && x0.iter().all(|v| v.is_finite()),
        "Initial guess must be non-empty and finite",
    )?;
    let mut x = x0.to_vec();
    let mut f = problem.residual(&x)?;
    let mut j = problem.jacobian(&x)?;
    check_problem_consistency(&f, &j, n)?;
    let mut f_norm = norm2(&f);
    let mut lambda = options.initial_radius.max(1e-12);
    let mut nu = 2.;
    let mut iterations = 0;
    let reason;
    loop {
        if iterations >= options.max_iterations {
            reason = StopReason::MaxIterations;
            break;
        }
        if f_norm <= options.residual_epsilon {
            reason = StopReason::ResidualTolerance;
            break;
        }
        let (a, g) = normal_equations(&j, &f);
        if g.iter().map(|v| v.abs()).fold(0., f64::max) <= options.gradient_epsilon {
            reason = StopReason::GradientTolerance;
            break;
        }
        // Marquardt scaling: damp by λ·diag(JᵀJ) (floor at λ·1 to keep
        // scale-free behaviour on zero diagonal entries).
        let mut accepted = false;
        let mut step = vec![0.; n];
        let mut last_attempt_norm = f64::INFINITY;
        let mut trial = (x.clone(), f_norm);
        for _ in 0..32 {
            let mut damped = a.clone();
            for i in 0..n {
                damped[i][i] += lambda * a[i][i].max(1.);
            }
            let Some(d) = cholesky_solve(&damped, &g) else {
                lambda *= nu;
                nu *= 2.;
                continue;
            };
            let d: Vec<f64> = d.iter().map(|v| -v).collect();
            last_attempt_norm = norm2(&d);
            let mut x_trial = x.clone();
            for (xt, &dv) in x_trial.iter_mut().zip(&d) {
                *xt += dv;
            }
            let f_trial = problem.residual(&x_trial)?;
            check(f_trial.iter().all(|v| v.is_finite()), "Residual became non-finite")?;
            let trial_norm = norm2(&f_trial);
            let actual = 0.5 * (f_norm * f_norm - trial_norm * trial_norm);
            let predicted = predicted_reduction(&j, &g, &d);
            let rho = if predicted > 0. { actual / predicted } else { 0. };
            if rho > 0. {
                let gain = 1. - (2. * rho - 1.).powi(3);
                lambda *= gain.max(1. / 3.);
                nu = 2.;
                accepted = true;
                step = d;
                trial = (x_trial, trial_norm);
                break;
            }
            lambda *= nu;
            nu *= 2.;
            if !lambda.is_finite() {
                break;
            }
        }
        if !accepted {
            // Same stagnation rule as the trust-region solver: a step below
            // the backward-step tolerance with no representable improvement
            // is convergence at the precision floor, not failure.
            if last_attempt_norm <= options.step_epsilon * (norm2(&x) + options.step_epsilon) {
                reason = StopReason::StepTolerance;
            } else {
                reason = StopReason::MaxIterations;
            }
            break;
        }
        x = trial.0;
        f = problem.residual(&x)?;
        j = problem.jacobian(&x)?;
        check_problem_consistency(&f, &j, n)?;
        let step_norm = norm2(&step);
        f_norm = trial.1;
        iterations += 1;
        if step_norm <= options.step_epsilon * (norm2(&x) + options.step_epsilon) {
            reason = StopReason::StepTolerance;
            break;
        }
    }
    let converged = matches!(
        reason,
        StopReason::StepTolerance | StopReason::ResidualTolerance | StopReason::GradientTolerance
    );
    Ok((
        x,
        SolverReport {
            converged,
            iterations,
            residual_norm: f_norm,
            reason,
            radius: lambda,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::super::SolverOptions;
    use super::super::test_helpers::{Rosenbrock, approx_eq};
    use super::levenberg_marquardt_solve;

    #[test]
    fn levenberg_marquardt_converges_on_rosenbrock() {
        let (x, report) =
            levenberg_marquardt_solve(&Rosenbrock, &[-1.2, 1.0], SolverOptions::default())
                .unwrap();
        assert!(report.converged, "{report:?}");
        assert!(approx_eq(x[0], 1., 1e-5), "{x:?}");
        assert!(approx_eq(x[1], 1., 1e-5), "{x:?}");
    }
}
