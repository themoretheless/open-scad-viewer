//! Equality-constrained correction in the QR objective metric. No normal
//! equations, penalty weights, or geometry admission are involved here.
use super::*;
// Compensate both product rounding and cancellation. This is proposal-solver
// arithmetic only; stored geometry still needs independent interval proofs.
fn dot(a: &[f64], b: &[f64]) -> f64 {
    let mut sum = 0f64;
    let mut error = 0f64;
    for (&x, &y) in a.iter().zip(b) {
        let p = x * y;
        let next = sum + p;
        error += if sum.abs() >= p.abs() {
            (sum - next) + p
        } else {
            (p - next) + sum
        };
        error += x.mul_add(y, -p);
        sum = next;
    }
    sum + error
}
pub(super) fn correct(
    r: &[Vec<f64>],
    order: &[usize],
    scale: &[f64],
    x: &mut [f64],
    constraints: &[(Vec<f64>, f64)],
) -> Result<()> {
    correct_refined(r, order, scale, x, constraints, 3)
}
fn correct_refined(
    r: &[Vec<f64>],
    order: &[usize],
    scale: &[f64],
    x: &mut [f64],
    constraints: &[(Vec<f64>, f64)],
    remaining: usize,
) -> Result<()> {
    if constraints.is_empty() {
        return Ok(());
    }
    let n = x.len();
    if constraints.len() > 64
        || constraints
            .iter()
            .any(|(c, d)| c.len() != n || !d.is_finite() || c.iter().any(|v| !v.is_finite()))
    {
        return Err(refuse("Invalid hard graph fitting constraints"));
    }
    let mut basis: Vec<Vec<f64>> = Vec::new();
    let mut coordinates: Vec<f64> = Vec::new();
    for (c, target) in constraints {
        // Match the physical coefficients that the caller actually stores:
        // divide the unknown once before multiplying by its authored row.
        let physical = order
            .iter()
            .enumerate()
            .map(|(i, &j)| x[i] / scale[j])
            .collect::<Vec<_>>();
        let authored = order.iter().map(|&j| c[j]).collect::<Vec<_>>();
        let residual = target - dot(&authored, &physical);
        let c: Vec<_> = order.iter().map(|&j| c[j] / scale[j]).collect();
        // w = R^-T c, so c*delta_x = w*(R*delta_x).
        let mut w = vec![0.; n];
        for i in 0..n {
            w[i] = (c[i] - (0..i).map(|j| r[j][i] * w[j]).sum::<f64>()) / r[i][i];
        }
        let initial = w.iter().fold(0f64, |a, v| a.hypot(*v));
        if !initial.is_finite() {
            return Err(refuse("Singular hard graph objective metric"));
        }
        let mut projections = vec![0.; basis.len()];
        // Reorthogonalization preserves near-dependent row constraints much
        // better than one-pass Gram-Schmidt; unresolved rows refuse below.
        for _ in 0..2 {
            for (j, q) in basis.iter().enumerate() {
                let dot = q.iter().zip(&w).map(|(a, b)| a * b).sum::<f64>();
                projections[j] += dot;
                for (v, a) in w.iter_mut().zip(q) {
                    *v -= dot * a;
                }
            }
        }
        let remaining = residual
            - projections
                .iter()
                .zip(&coordinates)
                .map(|(a, b)| a * b)
                .sum::<f64>();
        let norm = w.iter().fold(0f64, |a, v| a.hypot(*v));
        if norm <= 1e-12 * initial {
            if remaining.abs() > 1e-10 * (1. + target.abs()) {
                return Err(refuse("Hard graph anchors are dependent or inconsistent"));
            }
            continue;
        }
        for v in &mut w {
            *v /= norm;
        }
        basis.push(w);
        coordinates.push(remaining / norm);
    }
    let mut correction = vec![0.; n];
    for (q, y) in basis.iter().zip(coordinates) {
        for (v, a) in correction.iter_mut().zip(q) {
            *v += y * a;
        }
    }
    for i in (0..n).rev() {
        correction[i] =
            (correction[i] - (i + 1..n).map(|j| r[i][j] * correction[j]).sum::<f64>()) / r[i][i];
    }
    let mut proposed: Vec<_> = x.iter().zip(correction).map(|(a, b)| a + b).collect();
    if proposed.iter().any(|v| !v.is_finite()) {
        return Err(refuse("Nonfinite hard graph correction"));
    }
    // Numerical interpolation is checked independently of the row-space
    // solve, in physical control units. This is not an interval certificate.
    for (c, target) in constraints {
        let physical = order
            .iter()
            .enumerate()
            .map(|(i, &j)| proposed[i] / scale[j])
            .collect::<Vec<_>>();
        let authored = order.iter().map(|&j| c[j]).collect::<Vec<_>>();
        let actual = dot(&authored, &physical);
        if !actual.is_finite() || (actual - target).abs() > 1e-10 * (1. + target.abs()) {
            if actual.is_finite() && remaining > 0 {
                // Recompute the physical residual and solve for a correction;
                // no tolerance is relaxed, and the caller stays unchanged if
                // all bounded refinement attempts fail.
                correct_refined(r, order, scale, &mut proposed, constraints, remaining - 1)?;
                x.copy_from_slice(&proposed);
                return Ok(());
            }
            return Err(refuse(&format!(
                "Hard graph anchor residual remains unresolved: error={}, target={target}",
                (actual - target).abs()
            )));
        }
    }
    x.copy_from_slice(&proposed);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn physical_metric_does_not_chase_a_near_null_mode() {
        let c = vec![(vec![1., 1e-8], 1.)];
        let mut objective = vec![0.; 2];
        correct(
            &[vec![1., 0.], vec![0., 1e-16]],
            &[0, 1],
            &[1.; 2],
            &mut objective,
            &c,
        )
        .unwrap();
        assert!(objective[1].abs() > 1e7);
        let mut physical = vec![0.; 2];
        correct(
            &[vec![1., 0.], vec![0., 1.]],
            &[0, 1],
            &[1.; 2],
            &mut physical,
            &c,
        )
        .unwrap();
        assert!((physical[0] - 1.).abs() < 1e-12 && physical[1].abs() < 2e-8);
    }
    #[test]
    fn residual_retains_cancellation_and_product_roundoff() {
        assert_eq!(dot(&[1e16, 1., -1e16], &[1., 1., 1.]), 1.);
        let e = 2f64.powi(-27);
        assert_eq!(dot(&[1. + e, -1.], &[1. - e, 1.]), -e * e);
    }
    #[test]
    fn thirty_six_junction_rows_follow_the_physical_column_mapping() {
        let n = 36;
        let order = (0..n).rev().collect::<Vec<_>>();
        let scale = (0..n).map(|i| 1. + i as f64 / 10.).collect::<Vec<_>>();
        let r = (0..n)
            .map(|i| {
                (0..n)
                    .map(|j| {
                        if i == j {
                            2.
                        } else if j == i + 1 {
                            0.25
                        } else {
                            0.
                        }
                    })
                    .collect()
            })
            .collect::<Vec<Vec<f64>>>();
        let constraints = (0..n)
            .map(|i| {
                let mut c = vec![0.; n];
                c[i] = 1.;
                (c, i as f64 / 10.)
            })
            .collect::<Vec<_>>();
        let mut x = vec![0.; n];
        correct(&r, &order, &scale, &mut x, &constraints).unwrap();
        for (i, &physical) in order.iter().enumerate() {
            assert!((x[i] / scale[physical] - physical as f64 / 10.).abs() < 1e-10);
        }
    }
    #[test]
    fn equality_uses_objective_metric_instead_of_large_penalties() {
        // Minimize 4*x²+y² subject to x+y=1: x=0.2, y=0.8.
        let mut x = vec![0.; 2];
        correct(
            &[vec![2., 0.], vec![0., 1.]],
            &[0, 1],
            &[1., 1.],
            &mut x,
            &[(vec![1., 1.], 1.)],
        )
        .unwrap();
        assert!((x[0] - 0.2).abs() < 1e-12 && (x[1] - 0.8).abs() < 1e-12);
    }
    #[test]
    fn constraints_follow_physical_scaling_and_permutation() {
        let mut x = vec![2., -3.];
        correct(
            &[vec![2., 1.], vec![0., 3.]],
            &[1, 0],
            &[4., 8.],
            &mut x,
            &[(vec![1., 1.], 1.), (vec![1., -1.], 0.25)],
        )
        .unwrap();
        assert!((x[1] / 4. - 0.625).abs() < 1e-12 && (x[0] / 8. - 0.375).abs() < 1e-12);
    }
    #[test]
    fn dependent_consistent_rows_pass_but_conflicts_preserve_input() {
        for target in [2., 3.] {
            let mut x = vec![0.; 2];
            let result = correct(
                &[vec![1., 0.], vec![0., 1.]],
                &[0, 1],
                &[1., 1.],
                &mut x,
                &[(vec![1., 1.], 1.), (vec![2., 2.], target)],
            );
            if target == 2. {
                assert!(result.is_ok());
                assert!((x[0] - 0.5).abs() < 1e-12);
            } else {
                assert!(result.is_err());
                assert_eq!(x, vec![0.; 2]);
            }
        }
    }
}
