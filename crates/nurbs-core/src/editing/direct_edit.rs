//! Direct manipulation of B-spline and NURBS curves (checklist 196–197).
//!
//! A drag constraint moves a curve point `C(u)` to a target. With the weights
//! frozen, `C(u) = Σ R_{i,p}(u) P_i` is *linear* in the control points, where
//! `R_{i,p}(u) = N_{i,p}(u) w_i / Σ_j N_{j,p}(u) w_j` is the rational basis
//! row at `u`. The constraints therefore form the (typically underdetermined)
//! system `B·ΔP = ΔQ`, and the minimum-norm control displacement is the
//! classical pseudoinverse solution `ΔP = Bᵀ (B Bᵀ)⁻¹ ΔQ` with the small
//! `m × m` Gram matrix solved directly (`m ≤ 8`).
//!
//! Freezing the weights is the documented approximation: the solve uses only
//! control-point degrees of freedom. For a rational curve this is still exact
//! algebra (the residual is nonzero only when the constraints are mutually
//! inconsistent at machine precision), but no weight freedom is exploited.
//! Periodic curves are rejected because the wrapped control aliases would
//! break the shared-knot displacement bookkeeping; clamp first if needed.
use crate::{Result, check, curve::Curve, curve::basis, numeric};
use math_core::next_up;

/// One drag constraint: the curve point at `parameter` should move to `target`.
pub struct DragConstraint {
    pub parameter: f64,
    pub target: [f64; 3],
}

pub struct DirectEditReport {
    pub curve: Curve,
    pub max_control_displacement: f64,
    /// Max-norm residual at the constraint points, outward-rounded.
    pub residual: f64,
    /// Smallest singular value of the constraint design `B`; conditioning
    /// evidence. Near-zero values signal nearly redundant constraints.
    pub min_singular_value: f64,
}

/// Dense Gaussian elimination with partial pivoting for a small nonsingular
/// `n × n` system with multiple right-hand sides. The pivot threshold is
/// relative to the matrix scale so tiny-but-legitimate basis entries are not
/// mistaken for singularity.
fn dense_solve(mut matrix: Vec<Vec<f64>>, mut values: Vec<Vec<f64>>) -> Result<Vec<Vec<f64>>> {
    let n = matrix.len();
    let scale = matrix
        .iter()
        .flatten()
        .map(|x| x.abs())
        .fold(0_f64, f64::max);
    let threshold = 64. * f64::EPSILON * scale.max(f64::MIN_POSITIVE);
    for column in 0..n {
        let pivot = (column..n)
            .max_by(|&a, &b| matrix[a][column].abs().total_cmp(&matrix[b][column].abs()))
            .unwrap();
        numeric(
            matrix[pivot][column].abs() > threshold,
            "Direct-edit constraint system is rank deficient",
        )?;
        matrix.swap(column, pivot);
        values.swap(column, pivot);
        let divisor = matrix[column][column];
        for value in &mut matrix[column][column..] {
            *value /= divisor;
        }
        for value in &mut values[column] {
            *value /= divisor;
        }
        for row in 0..n {
            if row == column {
                continue;
            }
            let factor = matrix[row][column];
            for j in column..n {
                matrix[row][j] -= factor * matrix[column][j];
            }
            let pivot_values = values[column].clone();
            for (value, pivot_value) in values[row].iter_mut().zip(pivot_values) {
                *value -= factor * pivot_value;
            }
        }
    }
    Ok(values)
}

/// Smallest singular value of a design matrix via the eigenvalues of its
/// symmetric Gram matrix, estimated by cyclic Jacobi rotations. `m ≤ 8`, so
/// the classical two-sided sweep is ample; used only as conditioning evidence.
fn min_singular_value(gram: &[Vec<f64>]) -> f64 {
    let n = gram.len();
    let mut a = gram.to_vec();
    let scale = a
        .iter()
        .enumerate()
        .map(|(i, row)| row[i].abs())
        .fold(0_f64, f64::max)
        .max(f64::MIN_POSITIVE);
    for _ in 0..64 {
        let mut off = 0_f64;
        for i in 0..n {
            for j in i + 1..n {
                off = off.max(a[i][j].abs());
            }
        }
        if off <= 1e-14 * scale {
            break;
        }
        for p in 0..n {
            for q in p + 1..n {
                if a[p][q] == 0. {
                    continue;
                }
                let theta = 0.5 * (a[q][q] - a[p][p]) / a[p][q];
                let t = theta.signum() / (theta.abs() + (1. + theta * theta).sqrt());
                let c = 1. / (1. + t * t).sqrt();
                let s = t * c;
                for k in 0..n {
                    let (akp, akq) = (a[k][p], a[k][q]);
                    a[k][p] = c * akp - s * akq;
                    a[k][q] = s * akp + c * akq;
                }
                for k in 0..n {
                    let (apk, aqk) = (a[p][k], a[q][k]);
                    a[p][k] = c * apk - s * aqk;
                    a[q][k] = s * apk + c * aqk;
                }
            }
        }
    }
    let min_eigenvalue = (0..n).map(|i| a[i][i]).fold(f64::INFINITY, f64::min);
    min_eigenvalue.max(0.).sqrt()
}

/// Rational basis row `R_{i,p}(u)` with the weights frozen.
fn rational_row(curve: &Curve, u: f64) -> Result<Vec<f64>> {
    let b = basis(
        curve.degree,
        &curve.knots,
        curve.control_points.len(),
        u,
        curve.periodic,
    )?;
    let weight: f64 = b
        .basis
        .iter()
        .zip(&curve.weights)
        .map(|(b, w)| b * w)
        .sum();
    numeric(
        weight > 0. && weight.is_finite(),
        "Rational denominator lost its positive finite value",
    )?;
    Ok(b
        .basis
        .iter()
        .zip(&curve.weights)
        .map(|(b, w)| b * w / weight)
        .collect())
}

/// Drag one or more points ON the curve to targets by the minimum-norm
/// displacement of the control points solving `B·ΔP = ΔQ`.
pub fn drag_curve_points_report(
    curve: &Curve,
    constraints: &[DragConstraint],
) -> Result<DirectEditReport> {
    curve.validate()?;
    check(
        (1..=8).contains(&constraints.len()),
        "Direct edit requires 1..=8 drag constraints",
    )?;
    check(
        !curve.periodic,
        "Direct edit does not support periodic storage; clamp the curve first",
    )?;
    check(
        curve.control_points[0].len() == 3,
        "Direct edit requires 3D control points",
    )?;
    let [a, b] = curve.domain();
    for constraint in constraints {
        check(
            constraint.parameter.is_finite()
                && constraint.parameter >= a
                && constraint.parameter <= b,
            "Drag parameter is outside the active knot domain",
        )?;
        check(
            constraint
                .target
                .iter()
                .all(|x| x.is_finite() && x.abs() <= 1e9),
            "Drag targets must be finite and bounded by 1e9",
        )?;
    }
    let n = curve.control_points.len();
    let m = constraints.len();
    let mut rows = Vec::with_capacity(m);
    let mut rhs = vec![vec![0.; 3]; m];
    for (k, constraint) in constraints.iter().enumerate() {
        rows.push(rational_row(curve, constraint.parameter)?);
        let point = curve.evaluate_validated(constraint.parameter)?.point;
        for axis in 0..3 {
            rhs[k][axis] = constraint.target[axis] - point[axis];
        }
    }
    // Gram matrix B Bᵀ (m × m) of the constraint design.
    let mut gram = vec![vec![0.; m]; m];
    for i in 0..m {
        for j in 0..=i {
            gram[i][j] = rows[i]
                .iter()
                .zip(&rows[j])
                .map(|(x, y)| x * y)
                .sum::<f64>();
            gram[j][i] = gram[i][j];
        }
    }
    numeric(
        gram.iter().flatten().all(|x| x.is_finite()),
        "Direct-edit Gram matrix overflowed finite precision",
    )?;
    let min_singular_value = min_singular_value(&gram);
    // Minimum-norm solution: ΔP = Bᵀ Λ with (B Bᵀ) Λ = ΔQ.
    let lambda = dense_solve(gram, rhs)?;
    let mut displacement = vec![[0.; 3]; n];
    for (k, row) in rows.iter().enumerate() {
        for i in 0..n {
            if row[i] == 0. {
                continue;
            }
            for axis in 0..3 {
                displacement[i][axis] += row[i] * lambda[k][axis];
            }
        }
    }
    numeric(
        displacement.iter().flatten().all(|x| x.is_finite()),
        "Direct-edit displacement exhausted finite precision",
    )?;
    let max_control_displacement = displacement
        .iter()
        .map(|d| d.iter().map(|x| x * x).sum::<f64>().sqrt())
        .fold(0_f64, f64::max);
    let mut output = curve.clone();
    for (point, d) in output.control_points.iter_mut().zip(&displacement) {
        for axis in 0..3 {
            point[axis] += d[axis];
        }
    }
    output.validate()?;
    let mut residual: f64 = 0.;
    for constraint in constraints {
        let point = output.evaluate_validated(constraint.parameter)?.point;
        residual = residual.max(
            point
                .iter()
                .zip(constraint.target)
                .map(|(x, t)| (x - t) * (x - t))
                .sum::<f64>()
                .sqrt(),
        );
    }
    numeric(residual.is_finite(), "Direct-edit residual is not finite")?;
    Ok(DirectEditReport {
        curve: output,
        max_control_displacement: next_up(max_control_displacement),
        residual: next_up(residual),
        min_singular_value,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cubic_bezier() -> Curve {
        Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![
                vec![0., 0., 0.],
                vec![1., 1., 0.],
                vec![2., 1., 0.],
                vec![3., 0., 0.],
            ],
            weights: vec![1.; 4],
            periodic: false,
        }
    }

    fn quarter_circle() -> Curve {
        let w = std::f64::consts::FRAC_1_SQRT_2;
        Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]],
            weights: vec![1., w, 1.],
            periodic: false,
        }
    }

    #[test]
    fn single_point_drag_hits_target_with_minimum_norm_displacement() {
        let curve = cubic_bezier();
        let current = curve.evaluate(0.5).unwrap().point;
        let target = [current[0] + 0.5, current[1] + 0.25, current[2] - 1.];
        let report = drag_curve_points_report(
            &curve,
            &[DragConstraint {
                parameter: 0.5,
                target,
            }],
        )
        .unwrap();
        // Curve passes through the target within the reported residual.
        let reached = report.curve.evaluate(0.5).unwrap().point;
        let distance = reached
            .iter()
            .zip(target)
            .map(|(x, t)| (x - t).abs())
            .fold(0_f64, f64::max);
        assert!(distance <= report.residual);
        assert!(report.residual < 1e-12);
        // Manual normal-equations solution: with one constraint,
        // ΔP_i = R_i · ΔQ / |R|² (per axis). Bernstein row at u=0.5 is
        // (1/8, 3/8, 3/8, 1/8) for a unit-weight cubic Bézier.
        let row = [0.125, 0.375, 0.375, 0.125];
        let norm2: f64 = row.iter().map(|x| x * x).sum();
        let dq = [
            target[0] - current[0],
            target[1] - current[1],
            target[2] - current[2],
        ];
        for i in 0..4 {
            for axis in 0..3 {
                let expected = row[i] * dq[axis] / norm2;
                let actual =
                    report.curve.control_points[i][axis] - curve.control_points[i][axis];
                assert!((actual - expected).abs() < 1e-12, "control {i} axis {axis}");
            }
        }
        let expected_displacement = 0.375 * (dq.iter().map(|x| x * x).sum::<f64>()).sqrt() / norm2;
        assert!((report.max_control_displacement - expected_displacement).abs() < 1e-12);
        assert!(report.min_singular_value > 0.);
    }

    #[test]
    fn multi_constraint_drag_satisfies_both_targets() {
        let curve = cubic_bezier();
        let p1 = curve.evaluate(0.25).unwrap().point;
        let p2 = curve.evaluate(0.75).unwrap().point;
        let targets = [
            [p1[0], p1[1] + 1., p1[2] + 0.5],
            [p2[0], p2[1] - 1., p2[2] - 0.5],
        ];
        let report = drag_curve_points_report(
            &curve,
            &[
                DragConstraint {
                    parameter: 0.25,
                    target: targets[0],
                },
                DragConstraint {
                    parameter: 0.75,
                    target: targets[1],
                },
            ],
        )
        .unwrap();
        for (u, target) in [(0.25, targets[0]), (0.75, targets[1])] {
            let reached = report.curve.evaluate(u).unwrap().point;
            let distance = reached
                .iter()
                .zip(target)
                .map(|(x, t)| (x - t).abs())
                .fold(0_f64, f64::max);
            assert!(distance <= report.residual);
        }
        assert!(report.residual < 1e-10);
        // The first control point is untouched: no basis support at 0.25/0.75
        // reaches index 0 strongly... it does reach, but endpoints move less
        // than the maximally displaced control.
        assert!(report.max_control_displacement > 0.);
        assert!(report.min_singular_value > 0.1);
    }

    #[test]
    fn rational_arc_drag_is_exact_with_frozen_weights() {
        let curve = quarter_circle();
        let u = 0.5;
        let current = curve.evaluate(u).unwrap().point;
        let target = [current[0] + 0.2, current[1] + 0.1, current[2]];
        let report = drag_curve_points_report(
            &curve,
            &[DragConstraint {
                parameter: u,
                target,
            }],
        )
        .unwrap();
        // Weights are unchanged (frozen-weight formulation).
        assert_eq!(report.curve.weights, curve.weights);
        let reached = report.curve.evaluate(u).unwrap().point;
        let distance = reached
            .iter()
            .zip(target)
            .map(|(x, t)| (x - t).abs())
            .fold(0_f64, f64::max);
        assert!(distance <= report.residual);
        assert!(report.residual < 1e-10);
        assert!(report.curve.validate().is_ok());
    }

    #[test]
    fn out_of_domain_constraint_is_rejected() {
        let curve = cubic_bezier();
        assert!(
            drag_curve_points_report(
                &curve,
                &[DragConstraint {
                    parameter: 1.5,
                    target: [0., 0., 0.],
                }],
            )
            .is_err()
        );
        assert!(
            drag_curve_points_report(
                &curve,
                &[DragConstraint {
                    parameter: f64::NAN,
                    target: [0., 0., 0.],
                }],
            )
            .is_err()
        );
        assert!(drag_curve_points_report(&curve, &[]).is_err());
    }
}
