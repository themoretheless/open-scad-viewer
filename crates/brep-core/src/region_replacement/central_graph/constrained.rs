//! Equality-constrained correction in the QR objective metric. No normal
//! equations, penalty weights, or geometry admission are involved here.
use super::*;
pub(super) fn correct(
    r: &[Vec<f64>],
    order: &[usize],
    scale: &[f64],
    x: &mut [f64],
    constraints: &[(Vec<f64>, f64)],
) -> Result<()> {
    if constraints.is_empty() {
        return Ok(());
    }
    let n = x.len();
    if constraints.len() > 32
        || constraints
            .iter()
            .any(|(c, d)| c.len() != n || !d.is_finite() || c.iter().any(|v| !v.is_finite()))
    {
        return Err(refuse("Invalid hard graph fitting constraints"));
    }
    let mut basis: Vec<Vec<f64>> = Vec::new();
    let mut coordinates: Vec<f64> = Vec::new();
    for (c, target) in constraints {
        let c: Vec<_> = order.iter().map(|&j| c[j] / scale[j]).collect();
        let residual = target - c.iter().zip(x.iter()).map(|(a, b)| a * b).sum::<f64>();
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
    let proposed: Vec<_> = x.iter().zip(correction).map(|(a, b)| a + b).collect();
    if proposed.iter().any(|v| !v.is_finite()) {
        return Err(refuse("Nonfinite hard graph correction"));
    }
    // Numerical interpolation is checked independently of the row-space
    // solve, in physical control units. This is not an interval certificate.
    for (c, target) in constraints {
        let actual = order
            .iter()
            .enumerate()
            .map(|(i, &j)| c[j] * proposed[i] / scale[j])
            .sum::<f64>();
        if !actual.is_finite() || (actual - target).abs() > 1e-10 * (1. + target.abs()) {
            return Err(refuse("Hard graph anchor residual remains unresolved"));
        }
    }
    x.copy_from_slice(&proposed);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
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
