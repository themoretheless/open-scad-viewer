//! Constraint jacobian: preserves the original arithmetic and proof budgets.
use super::*;

pub(super) fn jacobian_into(
    sketch: &Sketch,
    row_offsets: &[usize],
    scratch: &mut [f64],
    matrix: &mut [f64],
    n: usize,
) {
    matrix.fill(0.0);
    for (ci, constraint) in sketch.constraints.iter().enumerate() {
        let r0 = row_offsets[ci];
        let r0_idx = r0 * n;
        let r1_idx = (r0 + 1) * n;
        match *constraint {
            Constraint::Fix { point, .. } => {
                matrix[r0_idx + 2 * point] = 1.0;
                matrix[r1_idx + 2 * point + 1] = 1.0;
            }
            Constraint::Horizontal { a, b } => {
                matrix[r0_idx + 2 * a + 1] -= 1.0;
                matrix[r0_idx + 2 * b + 1] += 1.0;
            }
            Constraint::Vertical { a, b } => {
                matrix[r0_idx + 2 * a] -= 1.0;
                matrix[r0_idx + 2 * b] += 1.0;
            }
            Constraint::FixAngle { a, b, radians } => {
                let (sin_t, cos_t) = radians.sin_cos();
                matrix[r0_idx + 2 * a] -= sin_t;
                matrix[r0_idx + 2 * a + 1] += cos_t;
                matrix[r0_idx + 2 * b] += sin_t;
                matrix[r0_idx + 2 * b + 1] -= cos_t;
            }
            Constraint::PointOnLine { point, a, b } => {
                let pa = [scratch[2 * a], scratch[2 * a + 1]];
                let pb = [scratch[2 * b], scratch[2 * b + 1]];
                let pp = [scratch[2 * point], scratch[2 * point + 1]];
                let ux = pb[0] - pa[0];
                let uy = pb[1] - pa[1];
                let vx = pp[0] - pa[0];
                let vy = pp[1] - pa[1];
                let lu = ux.hypot(uy);
                if lu > 1e-12 {
                    let cross = ux * vy - uy * vx;
                    let inv_lu = 1.0 / lu;
                    let inv_lu3 = inv_lu * inv_lu * inv_lu;
                    let d_pbx = vy * inv_lu - cross * ux * inv_lu3;
                    let d_pby = -vx * inv_lu - cross * uy * inv_lu3;
                    let d_ppx = -uy * inv_lu;
                    let d_ppy = ux * inv_lu;
                    matrix[r0_idx + 2 * b] += d_pbx;
                    matrix[r0_idx + 2 * b + 1] += d_pby;
                    matrix[r0_idx + 2 * point] += d_ppx;
                    matrix[r0_idx + 2 * point + 1] += d_ppy;
                    matrix[r0_idx + 2 * a] -= d_pbx + d_ppx;
                    matrix[r0_idx + 2 * a + 1] -= d_pby + d_ppy;
                }
            }
            Constraint::Coincident { a, b } => {
                matrix[r0_idx + 2 * a] += 1.0;
                matrix[r0_idx + 2 * b] -= 1.0;
                matrix[r1_idx + 2 * a + 1] += 1.0;
                matrix[r1_idx + 2 * b + 1] -= 1.0;
            }
            Constraint::Midpoint { point, a, b } => {
                matrix[r0_idx + 2 * point] += 1.0;
                matrix[r0_idx + 2 * a] -= 0.5;
                matrix[r0_idx + 2 * b] -= 0.5;
                matrix[r1_idx + 2 * point + 1] += 1.0;
                matrix[r1_idx + 2 * a + 1] -= 0.5;
                matrix[r1_idx + 2 * b + 1] -= 0.5;
            }
            Constraint::Radius { circle, .. } => {
                matrix[r0_idx + 2 * sketch.points.len() + circle] = 1.0;
            }
            Constraint::Concentric { a, b } => {
                let ca = sketch.circles[a].center;
                let cb = sketch.circles[b].center;
                matrix[r0_idx + 2 * ca] += 1.0;
                matrix[r0_idx + 2 * cb] -= 1.0;
                matrix[r1_idx + 2 * ca + 1] += 1.0;
                matrix[r1_idx + 2 * cb + 1] -= 1.0;
            }
            Constraint::EqualRadius { a, b } => {
                let base = 2 * sketch.points.len();
                matrix[r0_idx + base + a] += 1.0;
                matrix[r0_idx + base + b] -= 1.0;
            }
            Constraint::Distance { a, b, .. } => {
                let pa = [scratch[2 * a], scratch[2 * a + 1]];
                let pb = [scratch[2 * b], scratch[2 * b + 1]];
                let dx = pa[0] - pb[0];
                let dy = pa[1] - pb[1];
                let len = dx.hypot(dy);
                if len > 1e-12 {
                    let gx = dx / len;
                    let gy = dy / len;
                    matrix[r0_idx + 2 * a] += gx;
                    matrix[r0_idx + 2 * a + 1] += gy;
                    matrix[r0_idx + 2 * b] -= gx;
                    matrix[r0_idx + 2 * b + 1] -= gy;
                }
            }
            Constraint::EqualLength { a, b, c, d } => {
                let pa = [scratch[2 * a], scratch[2 * a + 1]];
                let pb = [scratch[2 * b], scratch[2 * b + 1]];
                let pc = [scratch[2 * c], scratch[2 * c + 1]];
                let pd = [scratch[2 * d], scratch[2 * d + 1]];
                let ux = pb[0] - pa[0];
                let uy = pb[1] - pa[1];
                let vx = pd[0] - pc[0];
                let vy = pd[1] - pc[1];
                let lu = ux.hypot(uy);
                let lv = vx.hypot(vy);
                if lu > 1e-12 {
                    matrix[r0_idx + 2 * a] -= ux / lu;
                    matrix[r0_idx + 2 * a + 1] -= uy / lu;
                    matrix[r0_idx + 2 * b] += ux / lu;
                    matrix[r0_idx + 2 * b + 1] += uy / lu;
                }
                if lv > 1e-12 {
                    matrix[r0_idx + 2 * c] += vx / lv;
                    matrix[r0_idx + 2 * c + 1] += vy / lv;
                    matrix[r0_idx + 2 * d] -= vx / lv;
                    matrix[r0_idx + 2 * d + 1] -= vy / lv;
                }
            }
            Constraint::PointOnCircle { point, circle } => {
                let pp = [scratch[2 * point], scratch[2 * point + 1]];
                let center_idx = sketch.circles[circle].center;
                let pc = [scratch[2 * center_idx], scratch[2 * center_idx + 1]];
                let dx = pp[0] - pc[0];
                let dy = pp[1] - pc[1];
                let len = dx.hypot(dy);
                if len > 1e-12 {
                    let gx = dx / len;
                    let gy = dy / len;
                    matrix[r0_idx + 2 * point] += gx;
                    matrix[r0_idx + 2 * point + 1] += gy;
                    matrix[r0_idx + 2 * center_idx] -= gx;
                    matrix[r0_idx + 2 * center_idx + 1] -= gy;
                }
                matrix[r0_idx + 2 * sketch.points.len() + circle] -= 1.0;
            }
            Constraint::TangentCircles { a, b, internal } => {
                let ca = sketch.circles[a].center;
                let cb = sketch.circles[b].center;
                let pca = [scratch[2 * ca], scratch[2 * ca + 1]];
                let pcb = [scratch[2 * cb], scratch[2 * cb + 1]];
                let dx = pca[0] - pcb[0];
                let dy = pca[1] - pcb[1];
                let len = dx.hypot(dy);
                if len > 1e-12 {
                    let gx = dx / len;
                    let gy = dy / len;
                    matrix[r0_idx + 2 * ca] += gx;
                    matrix[r0_idx + 2 * ca + 1] += gy;
                    matrix[r0_idx + 2 * cb] -= gx;
                    matrix[r0_idx + 2 * cb + 1] -= gy;
                }
                let base = 2 * sketch.points.len();
                if internal {
                    let ra = scratch[base + a];
                    let rb = scratch[base + b];
                    let s = if ra >= rb { 1.0 } else { -1.0 };
                    matrix[r0_idx + base + a] -= s;
                    matrix[r0_idx + base + b] += s;
                } else {
                    matrix[r0_idx + base + a] -= 1.0;
                    matrix[r0_idx + base + b] -= 1.0;
                }
            }
            _ => {
                let (active, active_len) = constraint_active_vars(sketch, constraint);
                for &j in &active[..active_len] {
                    let orig = scratch[j];
                    let h = (orig.abs() * 1e-7).max(1e-6);
                    scratch[j] = orig + h;
                    let (plus, len) = sketch.eval_constraint_residual(constraint, scratch);
                    scratch[j] = orig - h;
                    let (minus, _) = sketch.eval_constraint_residual(constraint, scratch);
                    scratch[j] = orig;
                    let inv_2h = 1.0 / (2.0 * h);
                    for k in 0..len {
                        matrix[(r0 + k) * n + j] = (plus[k] - minus[k]) * inv_2h;
                    }
                }
            }
        }
    }
}

pub(super) fn constraint_active_vars(sketch: &Sketch, constraint: &Constraint) -> ([usize; 16], usize) {
    let mut vars = [0usize; 16];
    let mut len = 0usize;
    let add_pt = |vars: &mut [usize; 16], len: &mut usize, p: usize| {
        vars[*len] = 2 * p;
        vars[*len + 1] = 2 * p + 1;
        *len += 2;
    };
    let n_pts = sketch.points.len();
    let add_cir = |vars: &mut [usize; 16], len: &mut usize, c: usize| {
        let center = sketch.circles[c].center;
        vars[*len] = 2 * center;
        vars[*len + 1] = 2 * center + 1;
        vars[*len + 2] = 2 * n_pts + c;
        *len += 3;
    };
    match *constraint {
        Constraint::Fix { point, .. } => add_pt(&mut vars, &mut len, point),
        Constraint::Horizontal { a, b }
        | Constraint::Vertical { a, b }
        | Constraint::Coincident { a, b }
        | Constraint::Distance { a, b, .. }
        | Constraint::FixAngle { a, b, .. } => {
            add_pt(&mut vars, &mut len, a);
            add_pt(&mut vars, &mut len, b);
        }
        Constraint::Parallel { a, b, c, d }
        | Constraint::Perpendicular { a, b, c, d }
        | Constraint::EqualLength { a, b, c, d }
        | Constraint::Angle { a, b, c, d, .. } => {
            add_pt(&mut vars, &mut len, a);
            add_pt(&mut vars, &mut len, b);
            add_pt(&mut vars, &mut len, c);
            add_pt(&mut vars, &mut len, d);
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
            for p in [a, b, c, d, e, f, g, h] {
                add_pt(&mut vars, &mut len, p);
            }
        }
        Constraint::PointToLineDistance { point, a, b, .. }
        | Constraint::Midpoint { point, a, b }
        | Constraint::PointOnLine { point, a, b } => {
            add_pt(&mut vars, &mut len, point);
            add_pt(&mut vars, &mut len, a);
            add_pt(&mut vars, &mut len, b);
        }
        Constraint::Symmetric {
            a,
            b,
            axis_a,
            axis_b,
        } => {
            add_pt(&mut vars, &mut len, a);
            add_pt(&mut vars, &mut len, b);
            add_pt(&mut vars, &mut len, axis_a);
            add_pt(&mut vars, &mut len, axis_b);
        }
        Constraint::Collinear { a, b, c } => {
            add_pt(&mut vars, &mut len, a);
            add_pt(&mut vars, &mut len, b);
            add_pt(&mut vars, &mut len, c);
        }
        Constraint::Radius { circle, .. } => {
            vars[len] = 2 * n_pts + circle;
            len += 1;
        }
        Constraint::Concentric { a, b } => {
            add_pt(&mut vars, &mut len, sketch.circles[a].center);
            add_pt(&mut vars, &mut len, sketch.circles[b].center);
        }
        Constraint::EqualRadius { a, b } => {
            vars[len] = 2 * n_pts + a;
            vars[len + 1] = 2 * n_pts + b;
            len += 2;
        }
        Constraint::PointOnCircle { point, circle }
        | Constraint::PointOnArc { point, circle, .. } => {
            add_pt(&mut vars, &mut len, point);
            add_cir(&mut vars, &mut len, circle);
        }
        Constraint::TangentLineCircle { a, b, circle } => {
            add_pt(&mut vars, &mut len, a);
            add_pt(&mut vars, &mut len, b);
            add_cir(&mut vars, &mut len, circle);
        }
        Constraint::TangentCircles { a, b, .. } => {
            add_cir(&mut vars, &mut len, a);
            add_cir(&mut vars, &mut len, b);
        }
    }
    let slice = &mut vars[..len];
    slice.sort_unstable();
    let mut unique_len = 0;
    for i in 0..len {
        if i == 0 || vars[i] != vars[unique_len - 1] {
            vars[unique_len] = vars[i];
            unique_len += 1;
        }
    }
    (vars, unique_len)
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
