//! Fixed-basis positive-weight NURBS fitting with hard point/derivative/tangent-
//! plane constraints and discrete thin-plate regularization. No XNURBS code.
//! `solve` enforces supplied point parameters. The boundary and network owners
//! separately admit complete original two-jets and curve constraints.
use crate::{Error, Result, curve::basis, surface::Surface};
pub mod boundary;
mod jet;
mod linear;
pub use jet::JetEquation;
pub mod adaptive;
pub mod network;
pub mod rational;
pub mod unparameterized;
/// Reference-metric linearized shell energy and dead loads; owned by surfaces.
pub mod variational;

#[derive(Clone, Copy, Debug)]
pub enum Axis {
    U,
    V,
}
#[derive(Clone, Debug)]
pub enum Constraint {
    /// One scalar linear equation in actual first/second parametric jets.
    Jet {
        uv: [f64; 2],
        equation: JetEquation,
    },
    /// Exact Cartesian control coefficient. Positive-weight proposals may vary
    /// its homogeneous weight, but final Cartesian data is restored/rechecked.
    ControlPoint {
        index: [usize; 2],
        point: [f64; 3],
    },
    Point {
        uv: [f64; 2],
        point: [f64; 3],
    },
    Derivative {
        uv: [f64; 2],
        axis: Axis,
        value: [f64; 3],
    },
    TangentPlane {
        uv: [f64; 2],
        normal: [f64; 3],
    },
}
#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub anchor_weight: f64,
    pub fairness_weight: f64,
    pub position_tolerance: f64,
    pub derivative_tolerance: f64,
    /// Unitless 1-abs(dot(actual_unit_normal, requested_unit_normal)).
    pub normal_tolerance: f64,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            anchor_weight: 1.,
            fairness_weight: 0.01,
            position_tolerance: 1e-8,
            derivative_tolerance: 1e-8,
            normal_tolerance: 1e-8,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Report {
    pub surface: Surface,
    pub equations: usize,
    pub constraint_rank: usize,
    pub max_position_residual: f64,
    pub max_derivative_residual: f64,
    pub max_normal_misalignment: f64,
    pub sampled_regularity_only: bool,
}
fn error(message: impl Into<String>) -> Error {
    Error::new("NURBS_CONSTRAINT_REFUSED", message)
}
fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn norm(v: &[f64]) -> f64 {
    if v.iter().any(|x| !x.is_finite()) {
        return f64::INFINITY;
    }
    v.iter().fold(0f64, |n, v| n.hypot(*v))
}
fn rational_basis(s: &Surface, uv: [f64; 2], axis: Option<Axis>) -> Result<Vec<f64>> {
    let nu = s.control_points.len();
    let nv = s.control_points[0].len();
    let u = basis(s.degree_u, &s.knots_u, nu, uv[0], s.periodic_u)?;
    let v = basis(s.degree_v, &s.knots_v, nv, uv[1], s.periodic_v)?;
    if axis.is_some()
        && ((matches!(axis, Some(Axis::U)) && u.derivative_status != "available")
            || (matches!(axis, Some(Axis::V)) && v.derivative_status != "available"))
    {
        return Err(error("Derivative constraint at an unresolved knot"));
    }
    let mut a = Vec::new();
    let mut da = Vec::new();
    for i in 0..nu {
        for j in 0..nv {
            a.push(u.basis[i] * v.basis[j] * s.weights[i][j]);
            da.push(match axis {
                None => 0.,
                Some(Axis::U) => u.d1[i] * v.basis[j] * s.weights[i][j],
                Some(Axis::V) => u.basis[i] * v.d1[j] * s.weights[i][j],
            });
        }
    }
    let w = a.iter().sum::<f64>();
    let dw = da.iter().sum::<f64>();
    if !w.is_finite() || w <= 0. {
        return Err(error("Invalid rational denominator"));
    }
    let row = if axis.is_none() {
        a.iter().map(|a| a / w).collect()
    } else {
        a.iter()
            .zip(&da)
            .map(|(a, da)| (da - a * dw / w) / w)
            .collect::<Vec<_>>()
    };
    if row.iter().any(|v| !v.is_finite()) {
        return Err(error("Constraint basis overflow"));
    }
    Ok(row)
}
fn cholesky(a: &[Vec<f64>]) -> Result<Vec<Vec<f64>>> {
    let n = a.len();
    let mut l = vec![vec![0.; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let sum = a[i][j] - (0..j).map(|k| l[i][k] * l[j][k]).sum::<f64>();
            if i == j {
                if !sum.is_finite() || sum <= 0. {
                    return Err(error("Fairness system is not resolved positive definite"));
                }
                l[i][j] = sum.sqrt();
            } else {
                l[i][j] = sum / l[j][j];
            }
        }
    }
    Ok(l)
}
fn triangular_solve(l: &[Vec<f64>], b: &[f64]) -> Vec<f64> {
    let n = b.len();
    let mut x = vec![0.; n];
    for i in 0..n {
        x[i] = (b[i] - (0..i).map(|j| l[i][j] * x[j]).sum::<f64>()) / l[i][i];
    }
    for i in (0..n).rev() {
        x[i] = (x[i] - (i + 1..n).map(|j| l[j][i] * x[j]).sum::<f64>()) / l[i][i];
    }
    x
}
pub fn solve(seed: &Surface, constraints: &[Constraint], options: Options) -> Result<Report> {
    seed.validate()?;
    let nu = seed.control_points.len();
    let nv = seed.control_points[0].len();
    let n = nu * nv;
    if n > 256 || constraints.len() > 384 {
        return Err(Error::new(
            "NURBS_RESOURCE_LIMIT",
            "Constrained surface allows <=256 controls and <=384 constraints",
        ));
    }
    if seed.control_points.iter().flatten().any(|p| p.len() != 3)
        || seed.periodic_u
        || seed.periodic_v
    {
        return Err(error("Nonperiodic 3D surface required"));
    }
    if !options.anchor_weight.is_finite()
        || options.anchor_weight <= 0.
        || !options.fairness_weight.is_finite()
        || options.fairness_weight < 0.
        || [
            options.position_tolerance,
            options.derivative_tolerance,
            options.normal_tolerance,
        ]
        .iter()
        .any(|t| !t.is_finite() || *t <= 0.)
    {
        return Err(error("Invalid objective weights or residual tolerances"));
    }
    let mut h = vec![vec![0.; n]; n];
    for (i, row) in h.iter_mut().enumerate() {
        row[i] = options.anchor_weight;
    }
    let mut penalize = |indices: [usize; 3]| {
        for (i, a) in indices.iter().zip([1., -2., 1.]) {
            for (j, b) in indices.iter().zip([1., -2., 1.]) {
                h[*i][*j] += options.fairness_weight * a * b;
            }
        }
    };
    for i in 0..nu {
        for j in 1..nv - 1 {
            penalize([i * nv + j - 1, i * nv + j, i * nv + j + 1]);
        }
    }
    for i in 1..nu - 1 {
        for j in 0..nv {
            penalize([(i - 1) * nv + j, i * nv + j, (i + 1) * nv + j]);
        }
    }
    let l = cholesky(&h)?;
    let mut x0 = vec![0.; 3 * n];
    for axis in 0..3 {
        let rhs = seed
            .control_points
            .iter()
            .flatten()
            .map(|p| p[axis] * options.anchor_weight)
            .collect::<Vec<_>>();
        let x = triangular_solve(&l, &rhs);
        x0[axis * n..(axis + 1) * n].copy_from_slice(&x);
    }
    let (rows, targets) = equations(seed, constraints, options)?;
    let rhs = targets
        .iter()
        .zip(&rows)
        .map(|(target, row)| target - dot(row, &x0))
        .collect::<Vec<_>>();
    let (correction, rank) = linear::correction(&l, &rows, &rhs)?;
    let mut x = x0;
    for (a, b) in x.iter_mut().zip(correction) {
        *a += b;
    }
    let mut surface = seed.clone();
    for i in 0..nu {
        for j in 0..nv {
            for axis in 0..3 {
                surface.control_points[i][j][axis] = x[axis * n + i * nv + j];
            }
        }
    }
    // Fixed Cartesian coefficients are exact authored data, not fitting sites.
    // Restore them before checking ALL equations on the actual saved geometry;
    // an incompatible position/jet row must still refuse after restoration.
    for c in constraints {
        if let Constraint::ControlPoint {
            index: [i, j],
            point,
        } = c
        {
            surface.control_points[*i][*j] = point.to_vec();
        }
    }
    surface.validate()?;
    check_solution(surface, constraints, options, rows.len(), rank)
}

/// Shared authored fixed-weight hard equations; numerical rank grants no geometry proof.
fn equations(
    seed: &Surface,
    constraints: &[Constraint],
    options: Options,
) -> Result<(Vec<Vec<f64>>, Vec<f64>)> {
    let nu = seed.control_points.len();
    let nv = seed.control_points[0].len();
    let n = nu * nv;
    let mut rows = Vec::<Vec<f64>>::new();
    let mut targets = Vec::new();
    let mut add_row = |mut row: Vec<f64>, target: f64| -> Result<()> {
        let length = norm(&row);
        if !length.is_finite() || !target.is_finite() {
            return Err(error("Nonfinite constraint"));
        }
        if length == 0. {
            if target.abs() > options.derivative_tolerance {
                return Err(error("Inconsistent zero derivative constraint"));
            }
            return Ok(());
        }
        for a in &mut row {
            *a /= length;
        }
        rows.push(row);
        targets.push(target / length);
        Ok(())
    };
    for c in constraints {
        match c {
            Constraint::Jet { uv, equation } => {
                add_row(jet::row(seed, *uv, equation)?, equation.value)?
            }
            Constraint::ControlPoint {
                index: [i, j],
                point,
            } => {
                if *i >= nu || *j >= nv {
                    return Err(error("Unknown fixed control coefficient"));
                }
                for axis in 0..3 {
                    let mut row = vec![0.; 3 * n];
                    row[axis * n + i * nv + j] = 1.;
                    add_row(row, point[axis])?;
                }
            }
            Constraint::Point { uv, point } => {
                let b = rational_basis(seed, *uv, None)?;
                for axis in 0..3 {
                    let mut row = vec![0.; 3 * n];
                    row[axis * n..(axis + 1) * n].copy_from_slice(&b);
                    add_row(row, point[axis])?;
                }
            }
            Constraint::Derivative { uv, axis, value } => {
                let b = rational_basis(seed, *uv, Some(*axis))?;
                for dim in 0..3 {
                    let mut row = vec![0.; 3 * n];
                    row[dim * n..(dim + 1) * n].copy_from_slice(&b);
                    add_row(row, value[dim])?;
                }
            }
            Constraint::TangentPlane { uv, normal } => {
                let length = norm(normal);
                if !length.is_finite() || length == 0. {
                    return Err(error("Nonzero finite tangent-plane normal required"));
                }
                for axis in [Axis::U, Axis::V] {
                    let b = rational_basis(seed, *uv, Some(axis))?;
                    let row = (0..3)
                        .flat_map(|dim| b.iter().map(move |a| a * normal[dim] / length))
                        .collect();
                    add_row(row, 0.)?;
                }
            }
        }
    }
    Ok((rows, targets))
}

pub(crate) fn check_solution(
    surface: Surface,
    constraints: &[Constraint],
    options: Options,
    equations: usize,
    rank: usize,
) -> Result<Report> {
    surface.validate()?;
    let mut report = Report {
        surface,
        equations,
        constraint_rank: rank,
        max_position_residual: 0.,
        max_derivative_residual: 0.,
        max_normal_misalignment: 0.,
        sampled_regularity_only: true,
    };
    for c in constraints {
        match c {
            Constraint::Jet { uv, equation } => {
                report.max_derivative_residual = report
                    .max_derivative_residual
                    .max((jet::value(&report.surface, *uv, equation)? - equation.value).abs());
            }
            Constraint::ControlPoint {
                index: [i, j],
                point,
            } => {
                report.max_position_residual = report.max_position_residual.max(norm(
                    &report.surface.control_points[*i][*j]
                        .iter()
                        .zip(point)
                        .map(|(a, b)| a - b)
                        .collect::<Vec<_>>(),
                ));
            }
            Constraint::Point { uv, point } => {
                let p = report.surface.evaluate(uv[0], uv[1])?.point;
                report.max_position_residual = report.max_position_residual.max(norm(
                    &p.iter().zip(point).map(|(a, b)| a - b).collect::<Vec<_>>(),
                ));
            }
            Constraint::Derivative { uv, axis, value } => {
                let e = report.surface.evaluate(uv[0], uv[1])?;
                let (u, v) = e
                    .first_derivatives()
                    .ok_or_else(|| error("Derivative result unresolved"))?;
                let d = if matches!(axis, Axis::U) { u } else { v };
                report.max_derivative_residual = report.max_derivative_residual.max(norm(
                    &d.iter().zip(value).map(|(a, b)| a - b).collect::<Vec<_>>(),
                ));
            }
            Constraint::TangentPlane { uv, normal } => {
                let n = report
                    .surface
                    .evaluate(uv[0], uv[1])?
                    .unit_normal()
                    .ok_or_else(|| error("Tangent constraint produced singular surface"))?;
                report.max_normal_misalignment = report
                    .max_normal_misalignment
                    .max((1. - dot(&n, normal).abs() / norm(normal)).max(0.));
            }
        }
    }
    if report.max_position_residual > options.position_tolerance
        || report.max_derivative_residual > options.derivative_tolerance
        || report.max_normal_misalignment > options.normal_tolerance
    {
        return Err(error(format!(
            "Constraints inconsistent or numerically unresolved: point={}, derivative={}, normal={}",
            report.max_position_residual,
            report.max_derivative_residual,
            report.max_normal_misalignment
        )));
    }
    Ok(report)
}
