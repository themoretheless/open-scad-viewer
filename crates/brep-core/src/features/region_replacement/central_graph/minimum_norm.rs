//! Minimum norm in ORIGINAL control units for the retained QR row space.
//! A second Householder QR of the transposed, row-equilibrated constraints
//! avoids normal equations and the arbitrary free-variable-zero completion.
use super::*;
pub(super) fn complete(
    r: &[Vec<f64>],
    rhs: &[f64],
    order: &[usize],
    scale: &[f64],
    rank: usize,
) -> Result<Vec<f64>> {
    let n = order.len();
    let mut columns = Vec::with_capacity(rank);
    let mut target = Vec::with_capacity(rank);
    for i in 0..rank {
        let mut row = (0..n)
            .map(|j| if j < i { 0. } else { r[i][j] * scale[order[j]] })
            .collect::<Vec<_>>();
        let norm = row.iter().map(|x| x * x).sum::<f64>().sqrt();
        if !norm.is_finite() || norm == 0. {
            return Err(refuse("Invalid original-unit row scale"));
        }
        for x in &mut row {
            *x /= norm;
        }
        columns.push(row);
        target.push(rhs[i] / norm);
    }
    let mut reflectors = Vec::with_capacity(rank);
    let mut retained = rank;
    for k in 0..rank {
        let pivot = (k..rank)
            .max_by(|&a, &b| {
                let norm = |j: usize| columns[j][k..].iter().map(|x| x * x).sum::<f64>();
                norm(a).total_cmp(&norm(b))
            })
            .unwrap();
        columns.swap(k, pivot);
        target.swap(k, pivot);
        let norm = columns[k][k..].iter().map(|x| x * x).sum::<f64>().sqrt();
        if !norm.is_finite() {
            return Err(refuse(
                "Minimum-norm control completion is numerically singular",
            ));
        }
        // Scaling back to original control units can reveal dependencies
        // hidden by column equilibration. Keep the resolved physical row
        // space; independent geometry inspection still owns all admission.
        if norm <= 1e-13 {
            retained = k;
            break;
        }
        let alpha = -norm.copysign(columns[k][k]);
        let mut v = columns[k][k..].to_vec();
        v[0] -= alpha;
        let beta = 2. / v.iter().map(|x| x * x).sum::<f64>();
        for column in columns.iter_mut().skip(k + 1) {
            let dot = beta * v.iter().zip(&column[k..]).map(|(a, b)| a * b).sum::<f64>();
            for (x, a) in column[k..].iter_mut().zip(&v) {
                *x -= dot * a;
            }
        }
        columns[k][k] = alpha;
        for x in &mut columns[k][k + 1..] {
            *x = 0.;
        }
        reflectors.push((v, beta));
    }
    let mut x = vec![0.; n];
    for i in 0..retained {
        x[i] = (target[i] - (0..i).map(|j| columns[i][j] * x[j]).sum::<f64>()) / columns[i][i];
    }
    for (k, (v, beta)) in reflectors.iter().enumerate().rev() {
        let dot = beta * v.iter().zip(&x[k..]).map(|(a, b)| a * b).sum::<f64>();
        for (x, a) in x[k..].iter_mut().zip(v) {
            *x -= dot * a;
        }
    }
    if x.iter().any(|x| !x.is_finite()) {
        return Err(refuse("Nonfinite minimum-norm controls"));
    }
    let mut result = vec![0.; n];
    for (i, &j) in order.iter().enumerate() {
        result[j] = x[i];
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn minimizes_physical_controls_after_column_scaling_and_permutation() {
        // Original constraint 2*x+y=1; independent Lagrange solution (.4,.2).
        let x = complete(&[vec![0.125, 0.5]], &[1.], &[1, 0], &[4., 8.], 1).unwrap();
        assert!((x[0] - 0.4).abs() < 1e-12 && (x[1] - 0.2).abs() < 1e-12);
    }
    #[test]
    fn two_constraints_choose_the_balanced_interior_mode() {
        let x = complete(
            &[vec![1., 1., 0.], vec![0., 1., 1.]],
            &[1., 1.],
            &[0, 1, 2],
            &[1.; 3],
            2,
        )
        .unwrap();
        for (actual, expected) in x.iter().zip([1. / 3., 2. / 3., 1. / 3.]) {
            assert!((actual - expected).abs() < 1e-12);
        }
    }
}

#[cfg(test)]
mod dependent_tests {
    use super::*;
    #[test]
    fn nearly_dependent_physical_constraints_choose_bounded_controls() {
        let x = complete(
            &[vec![1., 1., 0.], vec![0., 1., 1.]],
            &[1., 1.],
            &[0, 1, 2],
            &[1e-16, 1., 1e-16],
            2,
        )
        .unwrap();
        assert!(x.iter().all(|v| v.abs() < 1.01));
        assert!((1e-16 * x[0] + x[1] - 1.).abs() < 1e-12);
        assert!((x[1] + 1e-16 * x[2] - 1.).abs() < 1e-12);
    }
}
