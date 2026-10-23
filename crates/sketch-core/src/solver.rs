use math_core::Result;
use crate::{Constraint, Sketch, Solution, error, length, subtract};
use crate::residuals::jacobian_into;

pub(crate) fn solve_linear_in_place(a: &mut [f64], b: &mut [f64], x: &mut [f64], n: usize) -> bool {
    for i in 0..n {
        let i_base = i * n;
        let pivot = (i..n)
            .max_by(|&j, &k| a[j * n + i].abs().total_cmp(&a[k * n + i].abs()))
            .unwrap();
        let pivot_base = pivot * n;
        if a[pivot_base + i].abs() < 1e-18 {
            return false;
        }
        if i != pivot {
            let (before, after) = a.split_at_mut(pivot_base);
            before[i_base + i..i_base + n].swap_with_slice(&mut after[i..n]);
            b.swap(i, pivot);
        }
        let inv_diag = 1.0 / a[i_base + i];
        let bi = b[i];
        let (before, after) = a.split_at_mut(i_base + n);
        let pivot_row = &before[i_base + i..i_base + n];
        for (j_offset, row_chunk) in after.chunks_exact_mut(n).enumerate() {
            let j = i + 1 + j_offset;
            let f = row_chunk[i] * inv_diag;
            if f != 0.0 {
                for (value, &pivot_val) in row_chunk[i..n].iter_mut().zip(pivot_row) {
                    *value -= f * pivot_val;
                }
                b[j] -= f * bi;
            }
        }
    }
    for i in (0..n).rev() {
        let row = &a[i * n..i * n + n];
        let mut sum = 0.0;
        for (&r_val, &x_val) in row[i + 1..n].iter().zip(&x[i + 1..n]) {
            sum += r_val * x_val;
        }
        x[i] = (b[i] - sum) / row[i];
    }
    true
}

pub(crate) fn rank_flat(a: &mut [f64], m: usize, n: usize) -> usize {
    let mut row = 0;
    for col in 0..n {
        if row >= m {
            break;
        }
        let pivot = (row..m)
            .max_by(|&i, &j| a[i * n + col].abs().total_cmp(&a[j * n + col].abs()))
            .unwrap();
        if a[pivot * n + col].abs() < 1e-7 {
            continue;
        }
        if row != pivot {
            for c in col..n {
                a.swap(row * n + c, pivot * n + c);
            }
        }
        let inv_diag = 1.0 / a[row * n + col];
        for i in row + 1..m {
            let f = a[i * n + col] * inv_diag;
            if f != 0.0 {
                let (before, after) = a.split_at_mut(i * n);
                let pivot_row = &before[row * n + col..row * n + n];
                let target_row = &mut after[col..n];
                for (value, &pivot_val) in target_row.iter_mut().zip(pivot_row) {
                    *value -= f * pivot_val;
                }
            }
        }
        row += 1;
    }
    row
}

/// Bounded numerical solver policy, shared by native and named adapters.
#[derive(Clone, Copy, Debug)]
pub struct SolverOptions {
    pub tolerance: f64,
    pub max_iterations: usize,
}

pub fn solve(sketch: &Sketch, tolerance: f64) -> Result<Solution> {
    solve_with_options(
        sketch,
        SolverOptions {
            tolerance,
            max_iterations: 96,
        },
    )
}

pub fn solve_with_options(sketch: &Sketch, options: SolverOptions) -> Result<Solution> {
    let SolverOptions {
        tolerance,
        max_iterations,
    } = options;
    if !(1..=96).contains(&max_iterations) {
        return Err(error("Invalid solver iteration limit"));
    }

    sketch.validate()?;
    if !tolerance.is_finite() || !(1e-8..=0.1).contains(&tolerance) {
        return Err(error("Invalid solver tolerance"));
    }
    let mut x: Vec<_> = sketch
        .points
        .iter()
        .flatten()
        .copied()
        .chain(sketch.circles.iter().map(|c| c.radius))
        .collect();
    let n = x.len();
    let mut row_offsets = Vec::with_capacity(sketch.constraints.len());
    let mut m = 0;
    for c in &sketch.constraints {
        row_offsets.push(m);
        m += Sketch::constraint_residual_len(c);
    }

    let mut damping = 1e-3;
    let mut iterations = 0;
    let mut residual = vec![0.0; m];
    let mut cand_residual = vec![0.0; m];
    let mut j_flat = vec![0.0; m * n];
    let mut scratch = x.clone();
    let mut a = vec![0.0; n * n];
    let mut b = vec![0.0; n];
    let mut delta = vec![0.0; n];
    let mut candidate = vec![0.0; n];

    sketch.residual_into(&x, &mut residual);
    let mut current_obj: f64 = residual.iter().map(|v| v * v).sum();

    for _ in 0..max_iterations {
        if residual.iter().all(|v| v.abs() <= tolerance) {
            break;
        }
        iterations += 1;
        scratch.copy_from_slice(&x);
        jacobian_into(sketch, &row_offsets, &mut scratch, &mut j_flat, n);
        a.fill(0.0);
        b.fill(0.0);
        for k in 0..m {
            let row = &j_flat[k * n..(k + 1) * n];
            let rk = residual[k];
            for i in 0..n {
                let row_i = row[i];
                if row_i != 0.0 {
                    b[i] -= row_i * rk;
                    let a_row = &mut a[i * n + i..i * n + n];
                    for (val, &row_l) in a_row.iter_mut().zip(&row[i..n]) {
                        *val += row_i * row_l;
                    }
                }
            }
        }
        for i in 0..n {
            for l in i + 1..n {
                a[l * n + i] = a[i * n + l];
            }
            a[i * n + i] += damping;
        }
        if !solve_linear_in_place(&mut a, &mut b, &mut delta, n) {
            damping *= 10.;
            continue;
        }
        for (cand, (&xv, &dv)) in candidate.iter_mut().zip(x.iter().zip(&delta)) {
            *cand = xv + dv;
        }
        let valid = candidate.iter().all(|v| v.is_finite() && v.abs() <= 1e6)
            && candidate[sketch.points.len() * 2..]
                .iter()
                .all(|r| *r > 1e-9);
        if valid {
            sketch.residual_into(&candidate, &mut cand_residual);
            let cand_obj: f64 = cand_residual.iter().map(|v| v * v).sum();
            if cand_obj < current_obj {
                x.copy_from_slice(&candidate);
                std::mem::swap(&mut residual, &mut cand_residual);
                current_obj = cand_obj;
                damping = (damping / 3.).max(1e-12);
                continue;
            }
        }
        damping = (damping * 10.).min(1e12);
    }
    let residuals = sketch.residual(&x);
    let max = residual.iter().map(|v| v.abs()).fold(0., f64::max);
    scratch.copy_from_slice(&x);
    jacobian_into(sketch, &row_offsets, &mut scratch, &mut j_flat, n);
    let dof = n - rank_flat(&mut j_flat, m, n);
    let mut result = sketch.clone();
    for (i, p) in result.points.iter_mut().enumerate() {
        *p = [x[2 * i], x[2 * i + 1]];
    }
    for (i, c) in result.circles.iter_mut().enumerate() {
        c.radius = x[sketch.points.len() * 2 + i];
    }
    let degenerate = has_degenerate_constraint(&result, tolerance);
    let status = if degenerate {
        "degenerate"
    } else if max > tolerance {
        "not_converged"
    } else if dof > 0 {
        "underconstrained"
    } else {
        "solved"
    };
    Ok(Solution {
        sketch: result,
        status,
        iterations,
        max_residual: max,
        residuals,
        degrees_of_freedom: dof,
        convergence_certified: false,
    })
}

fn is_constraint_degenerate(sketch: &Sketch, c: &Constraint, tolerance: f64) -> bool {
    match *c {
        Constraint::Parallel { a, b, c, d }
        | Constraint::Perpendicular { a, b, c, d }
        | Constraint::Angle { a, b, c, d, .. } => {
            length(subtract(sketch.points[a], sketch.points[b])) <= tolerance
                || length(subtract(sketch.points[c], sketch.points[d])) <= tolerance
        }
        Constraint::PointToLineDistance { a, b, .. }
        | Constraint::Collinear { a, b, .. }
        | Constraint::TangentLineCircle { a, b, .. }
        | Constraint::PointOnLine { a, b, .. }
        | Constraint::FixAngle { a, b, .. } => {
            length(subtract(sketch.points[a], sketch.points[b])) <= tolerance
        }
        Constraint::EqualAngle {
            a,
            b,
            c,
            d,
            e,
            f,
            g,
            h,
        } => {
            length(subtract(sketch.points[a], sketch.points[b])) <= tolerance
                || length(subtract(sketch.points[c], sketch.points[d])) <= tolerance
                || length(subtract(sketch.points[e], sketch.points[f])) <= tolerance
                || length(subtract(sketch.points[g], sketch.points[h])) <= tolerance
        }
        Constraint::Symmetric { axis_a, axis_b, .. } => {
            length(subtract(sketch.points[axis_a], sketch.points[axis_b])) <= tolerance
        }
        Constraint::TangentCircles {
            a,
            b,
            internal: true,
        } => (sketch.circles[a].radius - sketch.circles[b].radius).abs() <= tolerance,
        _ => false,
    }
}

fn has_degenerate_constraint(sketch: &Sketch, tolerance: f64) -> bool {
    sketch
        .constraints
        .iter()
        .any(|c| is_constraint_degenerate(sketch, c, tolerance))
}

fn degenerate_constraint_indices(sketch: &Sketch, tolerance: f64) -> Vec<usize> {
    sketch
        .constraints
        .iter()
        .enumerate()
        .filter_map(|(i, c)| is_constraint_degenerate(sketch, c, tolerance).then_some(i))
        .collect()
}

/// Constraint diagnostics for named sketch adapters, independent of transport.
#[derive(Clone, Debug)]
pub struct Diagnostics {
    pub redundant_equations: usize,
    pub degenerate_constraints: Vec<usize>,
    pub constraint_residuals: Vec<f64>,
    pub inconsistent: bool,
}

pub fn solve_with_diagnostics(sketch: &Sketch, tolerance: f64) -> Result<(Solution, Diagnostics)> {
    solve_with_diagnostics_options(
        sketch,
        SolverOptions {
            tolerance,
            max_iterations: 96,
        },
    )
}

pub fn solve_with_diagnostics_options(
    sketch: &Sketch,
    options: SolverOptions,
) -> Result<(Solution, Diagnostics)> {
    let tolerance = options.tolerance;
    let solution = solve_with_options(sketch, options)?;
    let solved = &solution.sketch;
    let mut x: Vec<_> = solved
        .points
        .iter()
        .flatten()
        .copied()
        .chain(solved.circles.iter().map(|c| c.radius))
        .collect();
    let n = x.len();
    let mut row_offsets = Vec::with_capacity(solved.constraints.len());
    let mut m = 0;
    for c in &solved.constraints {
        row_offsets.push(m);
        m += Sketch::constraint_residual_len(c);
    }
    let mut j_flat = vec![0.0; m * n];
    jacobian_into(solved, &row_offsets, &mut x, &mut j_flat, n);
    let mut j_rank_buf = j_flat.clone();
    let jacobian_rank = rank_flat(&mut j_rank_buf, m, n);
    let residual: Vec<_> = solution.residuals.iter().flatten().copied().collect();
    let linear = solved.constraints.iter().all(|c| {
        matches!(
            c,
            Constraint::Fix { .. }
                | Constraint::Horizontal { .. }
                | Constraint::Vertical { .. }
                | Constraint::Coincident { .. }
                | Constraint::Midpoint { .. }
                | Constraint::Concentric { .. }
                | Constraint::EqualRadius { .. }
                | Constraint::FixAngle { .. }
        )
    });
    let inconsistent = if solution.max_residual > tolerance && linear {
        let mut aug = Vec::with_capacity(m * (n + 1));
        for (row, &r) in j_flat.chunks_exact(n).zip(&residual) {
            aug.extend_from_slice(row);
            aug.push(r);
        }
        rank_flat(&mut aug, m, n + 1) > jacobian_rank
    } else {
        false
    };
    let degenerate_constraints = degenerate_constraint_indices(solved, tolerance);
    let diagnostics = Diagnostics {
        redundant_equations: residual.len().saturating_sub(jacobian_rank),
        degenerate_constraints,
        constraint_residuals: solution
            .residuals
            .iter()
            .map(|r| r.iter().map(|x| x.abs()).fold(0., f64::max))
            .collect(),
        inconsistent,
    };
    Ok((solution, diagnostics))
}

/// Detailed degrees-of-freedom and constraint conflict report for UI highlighting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DofAnalysis {
    pub degrees_of_freedom: usize,
    pub conflicting_constraints: Vec<usize>,
    pub underconstrained_points: Vec<usize>,
}

/// Analyzes degrees of freedom, conflicting constraints, and underconstrained points
/// for a sketch at the given tolerance.
pub fn analyze_dof(sketch: &Sketch, tolerance: f64) -> Result<DofAnalysis> {
    let (solution, _) = solve_with_diagnostics(sketch, tolerance)?;
    let solved = &solution.sketch;
    let mut x: Vec<_> = solved
        .points
        .iter()
        .flatten()
        .copied()
        .chain(solved.circles.iter().map(|c| c.radius))
        .collect();
    let n = x.len();
    let mut row_offsets = Vec::with_capacity(solved.constraints.len());
    let mut m = 0;
    for c in &solved.constraints {
        row_offsets.push(m);
        m += Sketch::constraint_residual_len(c);
    }
    let mut j_flat = vec![0.0; m * n];
    jacobian_into(solved, &row_offsets, &mut x, &mut j_flat, n);

    // Gaussian elimination with column pivot tracking to identify constrained variables.
    let mut a = j_flat;
    let mut pivot_cols = vec![false; n];
    let mut row = 0;
    for col in 0..n {
        if row >= m {
            break;
        }
        let pivot = (row..m)
            .max_by(|&i, &j| a[i * n + col].abs().total_cmp(&a[j * n + col].abs()))
            .unwrap();
        if a[pivot * n + col].abs() < 1e-7 {
            continue;
        }
        if row != pivot {
            for c in col..n {
                a.swap(row * n + c, pivot * n + c);
            }
        }
        pivot_cols[col] = true;
        let inv_diag = 1.0 / a[row * n + col];
        for i in row + 1..m {
            let f = a[i * n + col] * inv_diag;
            if f != 0.0 {
                let (before, after) = a.split_at_mut(i * n);
                let pivot_row = &before[row * n + col..row * n + n];
                let target_row = &mut after[col..n];
                for (value, &pivot_val) in target_row.iter_mut().zip(pivot_row) {
                    *value -= f * pivot_val;
                }
            }
        }
        row += 1;
    }

    let mut underconstrained_points = Vec::new();
    for p in 0..solved.points.len() {
        if !pivot_cols[2 * p] || !pivot_cols[2 * p + 1] {
            underconstrained_points.push(p);
        }
    }

    let conflicting_constraints = solution
        .residuals
        .iter()
        .enumerate()
        .filter_map(|(ci, res)| {
            let max_r = res.iter().map(|v| v.abs()).fold(0.0, f64::max);
            (max_r > tolerance).then_some(ci)
        })
        .collect();

    Ok(DofAnalysis {
        degrees_of_freedom: solution.degrees_of_freedom,
        conflicting_constraints,
        underconstrained_points,
    })
}
