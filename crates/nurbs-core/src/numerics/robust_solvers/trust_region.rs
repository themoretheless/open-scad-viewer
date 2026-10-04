use super::{
    LeastSquaresProblem, SolverOptions, SolverReport, StepStrategy, StopReason,
    check_problem_consistency, cholesky_solve, dot, norm2, normal_equations, predicted_reduction,
};
use crate::{Result, check};

/// Cauchy step `d_u = −(gᵀg / ‖Jg‖²) g` and its model length along −g.
fn cauchy_step(j: &[Vec<f64>], g: &[f64]) -> Vec<f64> {
    let gg = dot(g, g);
    let mut jg = vec![0.; j.len()];
    for (row, out) in jg.iter_mut().enumerate() {
        *out = dot(&j[row], g);
    }
    let denom = dot(&jg, &jg);
    if denom <= 0. || gg <= 0. {
        return vec![0.; g.len()];
    }
    let alpha = gg / denom;
    g.iter().map(|x| -alpha * x).collect()
}

/// Dogleg step between the Cauchy step and the Gauss–Newton step `d_gn`,
/// truncated at radius `delta`. Falls back to the (scaled) Cauchy segment
/// when the normal equations are not numerically SPD.
fn dogleg_step(j: &[Vec<f64>], f: &[f64], delta: f64) -> (Vec<f64>, Vec<f64>) {
    let (a, g) = normal_equations(j, f);
    let d_u = cauchy_step(j, &g);
    let nu = norm2(&d_u);
    let d_gn = cholesky_solve(&a, &g)
        .map(|x| x.iter().map(|v| -v).collect::<Vec<_>>());
    match d_gn {
        Some(d_gn) if norm2(&d_gn) > 0. => {
            let ngn = norm2(&d_gn);
            if ngn <= delta {
                return (d_gn, g);
            }
            if nu >= delta {
                let scale = delta / nu;
                return (d_u.iter().map(|v| v * scale).collect(), g);
            }
            // Dogleg segment d_u + t·(d_gn − d_u), ‖·‖ = Δ.
            let diff: Vec<f64> = d_gn.iter().zip(&d_u).map(|(a, b)| a - b).collect();
            let a_dd = dot(&diff, &diff);
            let b_dd = 2. * dot(&d_u, &diff);
            let c_dd = nu * nu - delta * delta;
            let disc = (b_dd * b_dd - 4. * a_dd * c_dd).max(0.);
            let t = (-b_dd + disc.sqrt()) / (2. * a_dd);
            let step: Vec<f64> = d_u
                .iter()
                .zip(&diff)
                .map(|(u, d)| u + t * d)
                .collect();
            (step, g)
        }
        _ => {
            if nu >= delta && nu > 0. {
                let scale = delta / nu;
                (d_u.iter().map(|v| v * scale).collect(), g)
            } else {
                (d_u, g)
            }
        }
    }
}

/// Hook step: find `λ ≥ 0` with `‖d(λ)‖ ≈ Δ`, `d(λ)` solving
/// `(JᵀJ + λI) d = −g`. A few secant iterations on `φ(λ) = ‖d(λ)‖ − Δ`.
fn hook_step(a: &[Vec<f64>], g: &[f64], delta: f64) -> (Vec<f64>, Vec<f64>) {
    let n = g.len();
    let identity_scaled = |lambda: f64| -> Vec<Vec<f64>> {
        let mut m = a.to_vec();
        for i in 0..n {
            m[i][i] += lambda;
        }
        m
    };
    // Undamped Gauss–Newton step: use it directly when already inside.
    if let Some(d) = cholesky_solve(a, g) {
        let d: Vec<f64> = d.iter().map(|v| -v).collect();
        if norm2(&d) <= delta {
            return (d, g.to_vec());
        }
    }
    let mut lambda: f64 = 1e-3;
    let mut step = vec![0.; n];
    let mut found = false;
    for _ in 0..64 {
        let Some(d) = cholesky_solve(&identity_scaled(lambda), g) else {
            lambda = (lambda * 4.).max(1e-12);
            continue;
        };
        let d: Vec<f64> = d.iter().map(|v| -v).collect();
        let nd = norm2(&d);
        step = d;
        if (nd - delta).abs() <= 0.1 * delta {
            found = true;
            break;
        }
        // Secant-ish update: λ ∝ nd/Δ keeps positivity and converges fast
        // enough for the small dense systems targeted here.
        lambda *= (nd / delta).max(1.0001);
        if !lambda.is_finite() {
            break;
        }
    }
    if !found && norm2(&step) == 0. {
        // Last resort: scaled steepest descent.
        let ng = norm2(g);
        if ng > 0. {
            step = g.iter().map(|v| -v * delta / ng).collect();
        }
    }
    (step, g.to_vec())
}

/// Trust-region solver for `min ‖F(x)‖₂` with residual/Jacobian callbacks.
///
/// Stops when the backward step is small (`StepTolerance`), the residual is
/// small (`ResidualTolerance`), the gradient is small (`GradientTolerance`),
/// or the iteration budget is exhausted (`MaxIterations`).
pub fn trust_region_solve<P: LeastSquaresProblem>(
    problem: &P,
    x0: &[f64],
    options: SolverOptions,
) -> Result<(Vec<f64>, SolverReport)> {
    let n = x0.len();
    check(
        n > 0 && x0.iter().all(|v| v.is_finite()),
        "Initial guess must be non-empty and finite",
    )?;
    check(
        options.max_iterations > 0
            && options.step_epsilon > 0.
            && options.initial_radius > 0.
            && options.max_radius >= options.initial_radius,
        "Invalid trust-region options",
    )?;
    let mut x = x0.to_vec();
    let mut f = problem.residual(&x)?;
    let mut j = problem.jacobian(&x)?;
    check_problem_consistency(&f, &j, n)?;
    let mut f_norm = norm2(&f);
    let mut radius = options.initial_radius;
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
        let (step, g) = match options.strategy {
            StepStrategy::Dogleg => dogleg_step(&j, &f, radius),
            StepStrategy::Hook => hook_step(&a, &g, radius),
        };
        let step_norm = norm2(&step);
        let predicted = predicted_reduction(&j, &g, &step);
        let mut x_trial = x.clone();
        for (xt, &d) in x_trial.iter_mut().zip(&step) {
            *xt += d;
        }
        let f_trial = problem.residual(&x_trial)?;
        check(f_trial.iter().all(|v| v.is_finite()), "Residual became non-finite")?;
        let trial_norm = norm2(&f_trial);
        let actual = 0.5 * (f_norm * f_norm - trial_norm * trial_norm);
        let rho = if predicted > 0. { actual / predicted } else { 0. };
        if rho > options.acceptance_ratio && trial_norm < f_norm {
            x = x_trial;
            f = f_trial;
            j = problem.jacobian(&x)?;
            check_problem_consistency(&f, &j, n)?;
            if rho > 0.75 && step_norm >= 0.99 * radius {
                radius = (2. * radius).min(options.max_radius);
            }
            f_norm = trial_norm;
            if step_norm <= options.step_epsilon * (norm2(&x) + options.step_epsilon) {
                iterations += 1;
                reason = StopReason::StepTolerance;
                break;
            }
        } else {
            // Stagnation at the model precision floor: the step is below the
            // backward-step tolerance but no trial improves the residual to
            // machine-representable accuracy — this is convergence, not
            // failure.
            if step_norm <= options.step_epsilon * (norm2(&x) + options.step_epsilon) {
                iterations += 1;
                reason = StopReason::StepTolerance;
                break;
            }
            radius *= 0.25;
            if radius <= f64::MIN_POSITIVE * options.max_radius {
                // Radius exhausted: treat as an accept-everything tiny step.
                reason = StopReason::MaxIterations;
                break;
            }
        }
        iterations += 1;
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
            radius,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::{Rosenbrock, approx_eq};
    use super::super::{
        LeastSquaresProblem, SolverOptions, StepStrategy, StopReason, cholesky_solve, norm2,
        normal_equations,
    };
    use super::trust_region_solve;
    use crate::Result;

    /// Bare Gauss–Newton with full steps — known to overshoot/diverge on
    /// Rosenbrock from the far start used below.
    fn gauss_newton(x0: [f64; 2], iterations: usize) -> ([f64; 2], f64) {
        let p = Rosenbrock;
        let mut x = x0;
        for _ in 0..iterations {
            let f = p.residual(&x).unwrap();
            let j = p.jacobian(&x).unwrap();
            let (a, g) = normal_equations(&j, &f);
            let Some(d) = cholesky_solve(&a, &g) else { break };
            x[0] -= d[0];
            x[1] -= d[1];
            if !x[0].is_finite() || !x[1].is_finite() {
                break;
            }
        }
        let f = p.residual(&x).unwrap_or(vec![f64::INFINITY, f64::INFINITY]);
        (x, norm2(&f))
    }

    #[test]
    fn trust_region_converges_on_rosenbrock_where_gauss_newton_struggles() {
        // From this start the full GN step immediately jumps to a much worse
        // point (‖f‖ grows), while trust region contracts and converges.
        let x0 = [-1.2, 1.0];
        let (gn_x, gn_norm) = gauss_newton(x0, 1);
        let initial_norm = norm2(&Rosenbrock.residual(&x0).unwrap());
        assert!(
            gn_norm > initial_norm,
            "test premise: first GN step should worsen the residual ({gn_norm} vs {initial_norm}, x={gn_x:?})"
        );
        let (x, report) =
            trust_region_solve(&Rosenbrock, &x0, SolverOptions::default()).unwrap();
        assert!(report.converged, "{report:?}");
        assert!(report.residual_norm < 1e-10, "{report:?}");
        assert!(approx_eq(x[0], 1., 1e-6), "{x:?}");
        assert!(approx_eq(x[1], 1., 1e-6), "{x:?}");
    }

    #[test]
    fn dogleg_and_hook_strategies_both_converge() {
        for strategy in [StepStrategy::Dogleg, StepStrategy::Hook] {
            let (x, report) = trust_region_solve(
                &Rosenbrock,
                &[-1.2, 1.0],
                SolverOptions {
                    strategy,
                    ..SolverOptions::default()
                },
            )
            .unwrap();
            assert!(report.converged, "{strategy:?} {report:?}");
            assert!(approx_eq(x[0], 1., 1e-5), "{strategy:?} {x:?}");
            assert!(approx_eq(x[1], 1., 1e-5), "{strategy:?} {x:?}");
        }
    }

    #[test]
    fn stopping_criteria_fire() {
        // Zero-residual problem: residual criterion fires immediately.
        struct Zero;
        impl LeastSquaresProblem for Zero {
            fn residual(&self, _x: &[f64]) -> Result<Vec<f64>> {
                Ok(vec![0., 0.])
            }
            fn jacobian(&self, _x: &[f64]) -> Result<Vec<Vec<f64>>> {
                Ok(vec![vec![1., 0.], vec![0., 1.]])
            }
        }
        let (_, report) = trust_region_solve(&Zero, &[5., 5.], SolverOptions::default()).unwrap();
        assert_eq!(report.reason, StopReason::ResidualTolerance);
        assert!(report.converged);

        // Iteration budget: force max-iterations with a flat-residual problem
        // whose Jacobian is zero — the gradient criterion should fire first;
        // use instead a slow problem with a tiny budget.
        struct Slow;
        impl LeastSquaresProblem for Slow {
            fn residual(&self, x: &[f64]) -> Result<Vec<f64>> {
                Ok(vec![x[0].powi(3) - 2.])
            }
            fn jacobian(&self, x: &[f64]) -> Result<Vec<Vec<f64>>> {
                Ok(vec![vec![3. * x[0] * x[0]]])
            }
        }
        let (_, report) = trust_region_solve(
            &Slow,
            &[10.],
            SolverOptions {
                max_iterations: 2,
                ..SolverOptions::default()
            },
        )
        .unwrap();
        assert_eq!(report.reason, StopReason::MaxIterations);
        assert!(!report.converged);

        // Step criterion: solve x² − 2 = 0 to full precision. Unlike x³ − 2
        // (whose root cubes back to exactly 2.0 in binary64), √2 squared is
        // 2.0000000000000004 — the residual can never reach exactly zero, and
        // with the gradient criterion disabled the backward-step stagnation
        // criterion is the only way out.
        struct Sqrt2;
        impl LeastSquaresProblem for Sqrt2 {
            fn residual(&self, x: &[f64]) -> Result<Vec<f64>> {
                Ok(vec![x[0] * x[0] - 2.])
            }
            fn jacobian(&self, x: &[f64]) -> Result<Vec<Vec<f64>>> {
                Ok(vec![vec![2. * x[0]]])
            }
        }
        let (x, report) = trust_region_solve(
            &Sqrt2,
            &[2.],
            SolverOptions {
                step_epsilon: 1e-13,
                gradient_epsilon: 0.,
                residual_epsilon: 0.,
                ..SolverOptions::default()
            },
        )
        .unwrap();
        assert_eq!(report.reason, StopReason::StepTolerance, "{report:?}");
        assert!(report.converged);
        assert!(approx_eq(x[0], 2f64.sqrt(), 1e-12), "{x:?}");

        // Sanity: on x³ − 2 (whose root *does* reproduce 2.0 exactly in
        // binary64) the residual criterion legitimately fires at the root —
        // any of the convergence criteria is correct at the precision floor.
        let (x, report) = trust_region_solve(
            &Slow,
            &[2.],
            SolverOptions {
                step_epsilon: 1e-13,
                gradient_epsilon: 1e-14,
                residual_epsilon: 0.,
                ..SolverOptions::default()
            },
        )
        .unwrap();
        assert!(
            matches!(
                report.reason,
                StopReason::StepTolerance
                    | StopReason::GradientTolerance
                    | StopReason::ResidualTolerance
            ),
            "{report:?}"
        );
        assert!(report.converged);
        assert!(approx_eq(x[0], 2f64.cbrt(), 1e-8), "{x:?}");
    }
}
