#![doc = include_str!("../README.md")]
//! Shared bounded 2D sketch solver. Independent of polygon and NURBS geometry.
#[cfg(feature = "codec")]
mod serialization;
pub use math_core::{Error, Result};
const INVALID_INPUT: &str = "SKETCH_INVALID_INPUT";
fn error(message: impl Into<String>) -> Error {
    Error::new(INVALID_INPUT, message)
}
#[derive(Clone, Debug)]
pub struct Circle {
    pub center: usize,
    pub radius: f64,
}

#[derive(Clone, Debug)]
pub enum Constraint {
    Fix {
        point: usize,
        at: [f64; 2],
    },
    Horizontal {
        a: usize,
        b: usize,
    },
    Vertical {
        a: usize,
        b: usize,
    },
    Coincident {
        a: usize,
        b: usize,
    },
    Distance {
        a: usize,
        b: usize,
        value: f64,
    },
    Parallel {
        a: usize,
        b: usize,
        c: usize,
        d: usize,
    },
    Perpendicular {
        a: usize,
        b: usize,
        c: usize,
        d: usize,
    },
    EqualLength {
        a: usize,
        b: usize,
        c: usize,
        d: usize,
    },
    Angle {
        a: usize,
        b: usize,
        c: usize,
        d: usize,
        radians: f64,
    },
    PointToLineDistance {
        point: usize,
        a: usize,
        b: usize,
        value: f64,
    },
    Midpoint {
        point: usize,
        a: usize,
        b: usize,
    },
    Symmetric {
        a: usize,
        b: usize,
        axis_a: usize,
        axis_b: usize,
    },
    Collinear {
        a: usize,
        b: usize,
        c: usize,
    },
    Radius {
        circle: usize,
        value: f64,
    },
    Concentric {
        a: usize,
        b: usize,
    },
    EqualRadius {
        a: usize,
        b: usize,
    },
    PointOnCircle {
        point: usize,
        circle: usize,
    },
    TangentLineCircle {
        a: usize,
        b: usize,
        circle: usize,
    },
    TangentCircles {
        a: usize,
        b: usize,
        internal: bool,
    },
    /// Incident point on the infinite line through `a` and `b`.
    PointOnLine {
        point: usize,
        a: usize,
        b: usize,
    },
    /// Line segment `a -> b` at fixed angle `radians` to the +X axis.
    FixAngle {
        a: usize,
        b: usize,
        radians: f64,
    },
    /// Equal cosine of angle between two pairs of lines `(a->b, c->d)` and `(e->f, g->h)`
    /// or equal directed angle between `(a->b, c->d)` (4 points: angle(a->b, c->d) == angle(c->d, a->b) is trivial,
    /// so we store two line pairs or 4 points where `(a->b)` and `(c->d)` have equal angle to +X or two pairs `(a->b, c->d)` and `(e->f, g->h)`).
    EqualAngle {
        a: usize,
        b: usize,
        c: usize,
        d: usize,
        e: usize,
        f: usize,
        g: usize,
        h: usize,
    },
    /// Incident point on circular arc (defined by circle `circle` and start/end angles `start_angle`, `end_angle`).
    PointOnArc {
        point: usize,
        circle: usize,
        start_angle: f64,
        end_angle: f64,
    },
}

#[derive(Clone, Debug)]
pub struct Sketch {
    pub points: Vec<[f64; 2]>,
    pub circles: Vec<Circle>,
    pub constraints: Vec<Constraint>,
}

#[derive(Clone, Debug)]
pub struct Solution {
    pub sketch: Sketch,
    pub status: &'static str,
    pub iterations: usize,
    pub max_residual: f64,
    pub residuals: Vec<Vec<f64>>,
    pub degrees_of_freedom: usize,
    pub convergence_certified: bool,
}

fn length(v: [f64; 2]) -> f64 {
    v[0].hypot(v[1])
}
fn subtract(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn solve_linear_in_place(a: &mut [f64], b: &mut [f64], x: &mut [f64], n: usize) -> bool {
    for i in 0..n {
        let pivot = (i..n)
            .max_by(|&j, &k| a[j * n + i].abs().total_cmp(&a[k * n + i].abs()))
            .unwrap();
        if a[pivot * n + i].abs() < 1e-18 {
            return false;
        }
        if i != pivot {
            for col in i..n {
                a.swap(i * n + col, pivot * n + col);
            }
            b.swap(i, pivot);
        }
        let inv_diag = 1.0 / a[i * n + i];
        for j in i + 1..n {
            let f = a[j * n + i] * inv_diag;
            if f != 0.0 {
                let (before, after) = a.split_at_mut(j * n);
                let pivot_row = &before[i * n + i..i * n + n];
                let target_row = &mut after[i..n];
                for (value, &pivot_val) in target_row.iter_mut().zip(pivot_row) {
                    *value -= f * pivot_val;
                }
                b[j] -= f * b[i];
            }
        }
    }
    for i in (0..n).rev() {
        let row = &a[i * n..i * n + n];
        let mut sum = 0.0;
        for j in i + 1..n {
            sum += row[j] * x[j];
        }
        x[i] = (b[i] - sum) / row[i];
    }
    true
}



fn rank_flat(a: &mut [f64], m: usize, n: usize) -> usize {
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



#[cfg(test)]
#[path = "tests/tests_integration.rs"]
mod tests;

#[path = "constraint_jacobian.rs"]
mod constraint_jacobian;
use constraint_jacobian::jacobian_into;
use constraint_jacobian::constraint_active_vars;
pub use constraint_jacobian::analyze_dof;

#[path = "constraint_evaluation.rs"]
mod constraint_evaluation;
