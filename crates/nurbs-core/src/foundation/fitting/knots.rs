//! Bounded knot optimization and native refitting diagnostics.
use super::*;

/// Report of `optimize_knots_report` (item 215).
pub struct FitCurveReport {
    /// Best-seen curve (least sum-of-squared site residuals encountered).
    pub curve: Curve,
    /// Max-norm site residual after refitting controls at the initial knots.
    pub initial_residual_max: f64,
    /// Honest max-norm site residual of the returned curve, recomputed on the
    /// full data set at its chordal parameters.
    pub residual_max: f64,
    /// Gauss-Newton iterations actually performed (≤ the requested budget).
    pub iterations_run: usize,
    /// True when the loop stopped on residual stagnation rather than budget.
    pub converged: bool,
    /// Per-interior-knot displacement (returned knots minus initial knots).
    pub knot_deltas: Vec<f64>,
}

struct KnotFit {
    curve: Curve,
    /// Sum of squared site residuals at the (possibly subsampled) loop sites.
    sse: f64,
    /// Per-site residual vectors at the loop sites.
    residuals: Vec<[f64; 3]>,
}

/// Least-squares control-point refit for fixed knots at fixed parameters,
/// reusing the design-matrix + robust_solve machinery of this module.
fn refit_at_knots(
    degree: usize,
    knots: &[f64],
    control_count: usize,
    parameters: &[f64],
    points: &[[f64; 3]],
) -> Result<KnotFit> {
    let design = parameters
        .iter()
        .map(|&u| {
            crate::curve::basis(degree, knots, control_count, u, false)
                .map(|basis| basis.basis)
        })
        .collect::<Result<Vec<Vec<f64>>>>()?;
    let matrix = (0..control_count)
        .map(|i| {
            (0..control_count)
                .map(|j| design.iter().map(|row| row[i] * row[j]).sum())
                .collect()
        })
        .collect();
    let rhs = (0..control_count)
        .map(|i| {
            (0..3)
                .map(|axis| {
                    design
                        .iter()
                        .zip(points)
                        .map(|(row, p)| row[i] * p[axis])
                        .sum()
                })
                .collect()
        })
        .collect();
    let (controls, rank, _) = robust_solve(matrix, rhs)?;
    check(
        rank == control_count,
        "Knot optimization produced a rank-deficient fitting system",
    )?;
    let curve = Curve {
        degree,
        knots: knots.to_vec(),
        control_points: controls[..control_count].to_vec(),
        weights: vec![1.; control_count],
        periodic: false,
    };
    curve.validate()?;
    let mut sse = 0.;
    let mut residuals = Vec::with_capacity(points.len());
    for (&u, point) in parameters.iter().zip(points) {
        let value = curve.evaluate(u)?.point;
        let residual = [value[0] - point[0], value[1] - point[1], value[2] - point[2]];
        sse += residual.iter().map(|x| x * x).sum::<f64>();
        residuals.push(residual);
    }
    numeric(sse.is_finite(), "Knot-fit residual exhausted numeric range")?;
    Ok(KnotFit {
        curve,
        sse,
        residuals,
    })
}

/// Gauss–Newton optimization of interior knot positions for a clamped
/// non-periodic curve, minimizing the least-squares site residual at fixed
/// chordal data parameters. Each iteration re-solves the control points
/// (fixed-knot LSQ refit through the existing design-matrix/robust_solve
/// machinery), finite-differences the residual vector with respect to every
/// interior knot, and solves the damped normal equations augmented with
/// repulsion penalty rows for every knot gap below
/// `min_spacing · domain`. Steps are backtracked (1, ½, ¼, …) and projected
/// so the strict knot ordering with the minimum gap always holds; the loop
/// stops early on relative residual stagnation and always returns the
/// best-seen curve.
pub fn optimize_knots_report(
    points: &[[f64; 3]],
    initial: &Curve,
    iterations: usize,
    min_spacing: f64,
) -> Result<FitCurveReport> {
    initial.validate()?;
    check(
        points.len() >= 4 && points.len() <= 4096,
        "Knot optimization needs 4..4096 data sites",
    )?;
    for (i, point) in points.iter().enumerate() {
        for (axis, &x) in point.iter().enumerate() {
            require_finite_at(x, "knot_optimization_points", i * 3 + axis)?;
        }
    }
    check(
        !initial.periodic && initial.control_points[0].len() == 3,
        "Knot optimization needs a non-periodic 3D curve",
    )?;
    check(
        iterations <= 64,
        "Knot optimization iteration budget is at most 64",
    )?;
    check(
        min_spacing.is_finite() && (0. ..=0.25).contains(&min_spacing),
        "Minimum knot spacing must be a fraction of the domain in [0, 0.25]",
    )?;
    let p = initial.degree;
    let n = initial.control_points.len();
    check(
        initial.knots[0] == initial.knots[p] && initial.knots[n] == initial.knots[n + p],
        "Knot optimization requires clamped end knots",
    )?;
    check(
        points.len() >= n,
        "Knot optimization needs at least as many data sites as control points",
    )?;
    let [a, b] = initial.domain();
    let domain = b - a;
    // Interior knots must be simple and strictly inside the domain.
    let interior: Vec<f64> = initial.knots[p + 1..n].to_vec();
    let m = interior.len();
    check(
        m <= 24,
        "Knot optimization budget allows at most 24 interior knots",
    )?;
    check(
        interior.iter().all(|&k| k > a && k < b)
            && interior.array_windows().all(|[x, y]| x < y),
        "Interior knots must be strictly increasing inside the domain",
    )?;
    let gap_floor = min_spacing * domain;
    check(
        (m + 1) as f64 * gap_floor < domain * 0.5,
        "Minimum knot spacing leaves no feasible knot configuration",
    )?;
    // Fixed chordal data parameters mapped onto the curve domain.
    let mut parameters = vec![0.];
    for pair in points.windows(2) {
        parameters.push(
            parameters.last().unwrap() + distance(&pair[0].to_vec(), &pair[1].to_vec()),
        );
    }
    let total = *parameters.last().unwrap();
    let mut parameters: Vec<f64> = if total == 0. {
        (0..points.len())
            .map(|i| i as f64 / (points.len() - 1) as f64)
            .collect()
    } else {
        parameters.into_iter().map(|value| value / total).collect()
    };
    for parameter in &mut parameters {
        *parameter = a + domain * *parameter;
    }
    // Subsample the loop sites when the cloud is large; the final residual
    // is always recomputed on the full set.
    let stride = (points.len() / 1024).max(1);
    let loop_index: Vec<usize> = (0..points.len()).step_by(stride).collect();
    let loop_params: Vec<f64> = loop_index.iter().map(|&i| parameters[i]).collect();
    let loop_points: Vec<[f64; 3]> = loop_index.iter().map(|&i| points[i]).collect();

    let mut knots = initial.knots.clone();
    let mut best = refit_at_knots(p, &knots, n, &loop_params, &loop_points)?;
    let initial_residual_max = {
        let mut full = refit_at_knots(p, &knots, n, &parameters, points)?;
        full.residuals
            .drain(..)
            .map(|r| r.iter().map(|x| x * x).sum::<f64>().sqrt())
            .fold(0., f64::max)
    };
    let penalty_weight = 1e3 * (1. + best.sse.sqrt());
    let mut iterations_run = 0;
    let mut stagnant = 0_usize;
    let mut converged = false;
    // Unified budget guard on the requested Gauss–Newton iteration count
    // (already capped at 64 above); the guard turns exhaustion into a typed
    // BudgetExhausted payload if the loop is ever restructured.
    let mut gauss_newton = Budget::with_iterations(iterations.max(1))?
        .guard("knot-optimization-gauss-newton");
    for _ in 0..iterations {
        gauss_newton.tick()?;
        iterations_run += 1;
        // Finite-difference Jacobian columns: one refit per interior knot.
        let mut columns: Vec<Vec<[f64; 3]>> = Vec::with_capacity(m);
        for j in 0..m {
            let kidx = p + 1 + j;
            let h = 1e-6 * (knots[kidx + 1] - knots[kidx - 1]).max(1e-9 * domain);
            let mut perturbed = knots.clone();
            perturbed[kidx] = (knots[kidx] + h).min(knots[kidx + 1] - 1e-9 * domain.max(1.));
            if !(perturbed[kidx] > knots[kidx]) {
                columns.push(vec![[0.; 3]; best.residuals.len()]);
                continue;
            }
            let step = perturbed[kidx] - knots[kidx];
            let fit = refit_at_knots(p, &perturbed, n, &loop_params, &loop_points)?;
            columns.push(
                fit.residuals
                    .iter()
                    .zip(&best.residuals)
                    .map(|(r1, r0)| {
                        [
                            (r1[0] - r0[0]) / step,
                            (r1[1] - r0[1]) / step,
                            (r1[2] - r0[2]) / step,
                        ]
                    })
                    .collect(),
            );
        }
        let dot = |x: &[[f64; 3]], y: &[[f64; 3]]| -> f64 {
            x.iter()
                .zip(y)
                .map(|(a, b)| a[0] * b[0] + a[1] * b[1] + a[2] * b[2])
                .sum()
        };
        let mut jtj = vec![vec![0.; m]; m];
        let mut jtr = vec![0.; m];
        for i in 0..m {
            for j in 0..=i {
                let value = dot(&columns[i], &columns[j]);
                jtj[i][j] = value;
                jtj[j][i] = value;
            }
            jtr[i] = -dot(&columns[i], &best.residuals);
        }
        // Repulsion penalty rows for gaps below the floor: linearized
        // residual w·(g0 − gap) with ∂gap/∂knot = ±1 on the pair.
        let sequence: Vec<f64> = std::iter::once(a)
            .chain(knots[p + 1..n].iter().copied())
            .chain(std::iter::once(b))
            .collect();
        for q in 0..sequence.len() - 1 {
            let gap = sequence[q + 1] - sequence[q];
            if gap < gap_floor {
                let row = |jtj: &mut Vec<Vec<f64>>, jtr: &mut Vec<f64>| {
                    let lhs = q.checked_sub(1);
                    let rhs = if q < m { Some(q) } else { None };
                    if let Some(l) = lhs {
                        jtj[l][l] += penalty_weight * penalty_weight;
                        jtr[l] += penalty_weight * penalty_weight * (gap_floor - gap);
                    }
                    if let Some(r) = rhs {
                        jtj[r][r] += penalty_weight * penalty_weight;
                        jtr[r] -= penalty_weight * penalty_weight * (gap_floor - gap);
                    }
                    if let (Some(l), Some(r)) = (lhs, rhs) {
                        jtj[l][r] -= penalty_weight * penalty_weight;
                        jtj[r][l] -= penalty_weight * penalty_weight;
                    }
                };
                row(&mut jtj, &mut jtr);
            }
        }
        // Light Levenberg damping for rank safety.
        for j in 0..m {
            jtj[j][j] += 1e-12 * (1. + jtj[j][j]);
        }
        let rhs: Vec<Vec<f64>> = jtr.iter().map(|&v| vec![v]).collect();
        let Ok((delta, _, _)) = robust_solve(jtj, rhs) else {
            break;
        };
        let delta: Vec<f64> = delta[..m].iter().map(|row| row[0]).collect();
        if delta.iter().all(|d| !d.is_finite()) {
            break;
        }
        // Backtracking line search with feasibility projection.
        let mut accepted = false;
        for scale in [1., 0.5, 0.25, 0.1, 0.02] {
            let mut candidate_knots = knots.clone();
            for j in 0..m {
                candidate_knots[p + 1 + j] += scale * delta[j];
            }
            // Project to strict ordering with the minimum gap.
            let floor = gap_floor.max(1e-9 * domain);
            let mut feasible = true;
            for index in p + 1..n {
                let lower = candidate_knots[index - 1] + floor;
                if candidate_knots[index] < lower {
                    candidate_knots[index] = lower;
                }
                if candidate_knots[index] >= candidate_knots[index + 1] {
                    feasible = false;
                    break;
                }
            }
            if !feasible {
                continue;
            }
            let Ok(fit) = refit_at_knots(p, &candidate_knots, n, &loop_params, &loop_points)
            else {
                continue;
            };
            if fit.sse < best.sse {
                let improvement = (best.sse - fit.sse) / best.sse.max(1e-300);
                best = fit;
                knots = candidate_knots;
                accepted = true;
                stagnant = if improvement < 1e-10 { stagnant + 1 } else { 0 };
                break;
            }
        }
        if !accepted {
            stagnant += 1;
        }
        if stagnant >= 2 {
            converged = true;
            break;
        }
    }
    // Honest final residual on the full data set.
    let mut full = refit_at_knots(p, &best.curve.knots, n, &parameters, points)?;
    let residual_max = full
        .residuals
        .drain(..)
        .map(|r| r.iter().map(|x| x * x).sum::<f64>().sqrt())
        .fold(0., f64::max);
    let knot_deltas = best.curve.knots[p + 1..n]
        .iter()
        .zip(&interior)
        .map(|(new, old)| new - old)
        .collect();
    Ok(FitCurveReport {
        curve: full.curve,
        initial_residual_max: next_up(initial_residual_max),
        residual_max: next_up(residual_max),
        iterations_run,
        converged,
        knot_deltas,
    })
}

