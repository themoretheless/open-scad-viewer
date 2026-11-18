//! Monotone bounded coordinate pattern search in log-positive weights. Every
//! merit comparison uses the SAME original physical rest and Cartesian anchor.
use super::*;
pub(super) fn run(
    q: &mut Qualified,
    reference: &Surface,
    guides: &[network::CurveConstraint<'_>],
    curvatures: &[network::CurvatureConstraint<'_>],
    hard: &[Constraint],
    locks: Option<network::BoundaryLocks<'_>>,
    model: &variational::Model,
) -> Result<()> {
    let mut step = q.options.initial_log_step;
    let variables = q.variables.iter().copied().collect::<Vec<_>>();
    'search: while q.report.attempted < q.options.max_weight_trials
        && step >= q.options.min_log_step
        && !variables.is_empty()
    {
        let mut improved = false;
        for &(i, j) in &variables {
            for sign in [1., -1.] {
                if q.report.attempted == q.options.max_weight_trials {
                    break 'search;
                }
                let old = q.surface().weights[i][j];
                let weight = (old.ln() + sign * step).exp().clamp(
                    q.options.geometry.weight_bounds[0],
                    q.options.geometry.weight_bounds[1],
                );
                if weight == old {
                    continue;
                }
                let left = q.options.max_weight_trials - q.report.attempted;
                let divisor = left.min(4).max(1);
                let mut curves = q.options.geometry.network;
                curves.network.max_cells =
                    curves.network.max_cells.saturating_sub(q.fit.work.0) / divisor;
                curves.network.max_exact_work =
                    curves.network.max_exact_work.saturating_sub(q.fit.work.1) / divisor as u64;
                let assembly = q
                    .options
                    .max_total_assembly_terms
                    .saturating_sub(q.fit.loading_work)
                    / divisor as u64;
                if curves.network.max_cells == 0 || assembly == 0 {
                    q.report.resource_exhausted = true;
                    break 'search;
                }
                let mut anchor = q.original().clone();
                anchor.weights = q.surface().weights.clone();
                anchor.weights[i][j] = weight;
                q.report.attempted += 1;
                match network::loading::solve_with_reference(
                    &anchor,
                    reference,
                    network::ConstraintSet {
                        guides,
                        curvatures,
                        hard,
                        locks,
                    },
                    model,
                    q.options.loaded(curves, assembly),
                ) {
                    Ok(candidate) => {
                        q.report.admitted += 1;
                        q.fit.work.0 += candidate.work().0;
                        q.fit.work.1 += candidate.work().1;
                        q.fit.loading_work += candidate.loading_work();
                        let (value, _) = merit(&candidate, q.original(), &q.options)?;
                        if value < q.report.final_merit - 1e-12 * (1. + q.report.final_merit.abs())
                        {
                            q.fit.network = candidate;
                            q.report.final_merit = value;
                            q.report.accepted += 1;
                            q.trace.push(value);
                            improved = true;
                        }
                    }
                    Err(_) => {
                        q.report.numerically_unresolved_trials += 1;
                        q.fit.work.0 += curves.network.max_cells;
                        q.fit.work.1 += curves.network.max_exact_work;
                        q.fit.loading_work += assembly;
                    }
                }
            }
        }
        q.report.completed_sweeps += 1;
        if !improved {
            step *= 0.5;
        }
    }
    q.report.final_log_step = step;
    q.report.trial_budget_exhausted = q.report.attempted == q.options.max_weight_trials
        && step >= q.options.min_log_step
        && !variables.is_empty();
    let (value, penalty) = merit(q.network(), q.original(), &q.options)?;
    q.report.final_merit = value;
    q.report.weight_penalty = penalty;
    q.report.final_weights_at_bounds = q
        .surface()
        .weights
        .iter()
        .flatten()
        .filter(|w| {
            **w == q.options.geometry.weight_bounds[0] || **w == q.options.geometry.weight_bounds[1]
        })
        .count();
    if q.fit.work.0 > q.options.geometry.network.network.max_cells
        || q.fit.work.1 > q.options.geometry.network.network.max_exact_work
        || q.fit.loading_work > q.options.max_total_assembly_terms
    {
        return Err(error("Positive physical merit search exceeded shared work"));
    }
    Ok(())
}
