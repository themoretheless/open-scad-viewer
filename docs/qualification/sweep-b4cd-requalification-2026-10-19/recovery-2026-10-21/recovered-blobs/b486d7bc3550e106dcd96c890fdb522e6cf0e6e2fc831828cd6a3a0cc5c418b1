//! Linear (eigenvalue) buckling core shared by the frame and truss solvers.
//!
//! Buckling load factors of a reference load state solve (K + λ·Kg)φ = 0,
//! where Kg is the geometric stiffness built from the reference axial forces.
//! With the equilibrated Cholesky K = D·L·Lᵀ·D on the free DOFs and
//! G = D⁻¹·Kg·D⁻¹, the substitution φ = D⁻¹L⁻ᵀy turns this into the standard
//! symmetric eigenproblem A·y = μ·y with A = −L⁻¹·G·L⁻ᵀ and λ = 1/μ.
//!
//! Positive load factors buckle under the reference load scaled up; negative
//! factors buckle under the reversed load. Modes come back ordered by
//! ascending |λ| — the first entry is the critical one. The dense symmetric
//! eigensolver is O(n³) on the free DOFs, which the crate limits keep bounded.

use crate::{Error, Result};
use nalgebra::{Cholesky, DMatrix, Dyn, SymmetricEigen};

/// Eigenmodes returned per buckling or modal call.
pub const MAX_MODES: usize = 8;

fn numeric() -> Error {
    Error::new(
        "BUCKLING_NUMERIC_RANGE",
        "Buckling calculation exceeds finite numeric range",
    )
}

/// One buckling mode of the reference load state.
#[derive(Clone, Debug)]
pub struct BucklingMode {
    /// Signed factor on the reference loads; buckling load = λ·reference.
    /// Negative means the structure buckles under the reversed reference load.
    pub load_factor: f64,
    /// Mode shape over every DOF of the model (restrained DOFs are zero),
    /// normalized to max |component| = 1.
    pub shape: Vec<f64>,
    /// ‖Kφ + λKgφ‖∞ / (‖Kφ‖∞ + ‖λKgφ‖∞) on the free DOFs.
    pub relative_residual: f64,
}

/// Solve (K + λKg)φ = 0 given the equilibrated Cholesky factorization of the
/// elastic stiffness (as produced by the frame/truss assembly) and the
/// full-space geometric stiffness. Returns up to `modes` modes, ascending |λ|.
pub(crate) fn solve_modes(
    factor: &Cholesky<f64, Dyn>,
    scales: &[f64],
    free: &[usize],
    stiffness: &DMatrix<f64>,
    geometric: &DMatrix<f64>,
    modes: usize,
) -> Result<Vec<BucklingMode>> {
    let n = free.len();
    if n == 0 || modes == 0 {
        return Ok(Vec::new());
    }
    // Equilibrated geometric stiffness on the free DOFs.
    let g = DMatrix::from_fn(n, n, |i, j| {
        geometric[(free[i], free[j])] / scales[i] / scales[j]
    });
    if g.iter().any(|v| !v.is_finite()) {
        return Err(numeric());
    }
    // A = −L⁻¹·G·L⁻ᵀ via two triangular solves; symmetrize against roundoff.
    let l = factor.l();
    let x = l.solve_lower_triangular(&g).ok_or_else(numeric)?;
    let at = l.solve_lower_triangular(&(-x.transpose())).ok_or_else(numeric)?;
    let a = 0.5 * (&at + at.transpose());
    if a.iter().any(|v| !v.is_finite()) {
        return Err(numeric());
    }
    let eigen = SymmetricEigen::new(a);
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&i, &j| {
        eigen.eigenvalues[j]
            .abs()
            .total_cmp(&eigen.eigenvalues[i].abs())
    });
    let max_mu = eigen.eigenvalues[order[0]].abs();
    if max_mu == 0. {
        // No axial force anywhere: nothing to buckle.
        return Ok(Vec::new());
    }
    let mu_floor = 1e-9 * max_mu;
    let mut modes_out = Vec::new();
    for &index in order.iter().take(modes) {
        let mu = eigen.eigenvalues[index];
        if !mu.is_finite() || mu.abs() <= mu_floor {
            continue;
        }
        let lambda = 1. / mu;
        let y = eigen.eigenvectors.column(index).into_owned();
        // φ = D⁻¹·L⁻ᵀ·y, mapped back into the full DOF space.
        let psi = l
            .transpose()
            .solve_upper_triangular(&y)
            .ok_or_else(numeric)?;
        let mut shape = vec![0f64; stiffness.nrows()];
        for (i, &dof) in free.iter().enumerate() {
            shape[dof] = psi[i] / scales[i];
        }
        let peak = shape.iter().map(|v| v.abs()).fold(0., f64::max);
        if !peak.is_finite() || peak == 0. {
            return Err(numeric());
        }
        for v in &mut shape {
            *v /= peak;
        }
        let relative_residual = mode_residual(stiffness, geometric, free, &shape, lambda)?;
        if relative_residual > 1e-6 {
            return Err(numeric());
        }
        modes_out.push(BucklingMode {
            load_factor: lambda,
            shape,
            relative_residual,
        });
    }
    Ok(modes_out)
}

/// Normwise residual of one eigenpair on the free DOFs.
fn mode_residual(
    stiffness: &DMatrix<f64>,
    geometric: &DMatrix<f64>,
    free: &[usize],
    shape: &[f64],
    lambda: f64,
) -> Result<f64> {
    let mut max_residual = 0f64;
    let mut max_scale = 0f64;
    for &i in free {
        let mut elastic = 0f64;
        let mut geometric_term = 0f64;
        for &j in free {
            elastic += stiffness[(i, j)] * shape[j];
            geometric_term += geometric[(i, j)] * shape[j];
        }
        let residual = elastic + lambda * geometric_term;
        if !residual.is_finite() {
            return Err(numeric());
        }
        max_residual = max_residual.max(residual.abs());
        max_scale = max_scale.max(elastic.abs() + (lambda * geometric_term).abs());
    }
    Ok(if max_scale > 0. {
        max_residual / max_scale
    } else {
        0.
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Diagonal K = I, diagonal Kg: eigenvalues are exact and independent.
    fn diagonal_setup(g: &[f64]) -> (Cholesky<f64, Dyn>, Vec<f64>, Vec<usize>, DMatrix<f64>, DMatrix<f64>) {
        let n = g.len();
        let stiffness = DMatrix::<f64>::identity(n, n);
        let factor = stiffness.clone().cholesky().unwrap();
        let geometric = DMatrix::from_diagonal(&nalgebra::DVector::from_column_slice(g));
        let free: Vec<usize> = (0..n).collect();
        (factor, vec![1.; n], free, stiffness, geometric)
    }

    #[test]
    fn diagonal_problem_recovers_exact_load_factors_ordered_by_magnitude() {
        // A = −Kg: μ = 2, −1, 0.5 → λ = 0.5, −1, 2 ordered by |λ| ascending.
        let (factor, scales, free, stiffness, geometric) = diagonal_setup(&[-2., 1., -0.5]);
        let modes = solve_modes(&factor, &scales, &free, &stiffness, &geometric, 3).unwrap();
        assert_eq!(modes.len(), 3);
        for (mode, expected) in modes.iter().zip([0.5, -1., 2.]) {
            assert!((mode.load_factor - expected).abs() < 1e-9);
            assert!(mode.relative_residual < 1e-12);
        }
        // First mode shape is e1 (the μ = 2 eigenvector), normalized; the
        // eigenvector sign is arbitrary.
        assert_eq!(modes[0].shape.iter().map(|v| v.abs()).collect::<Vec<_>>(), vec![1., 0., 0.]);
    }

    #[test]
    fn zero_geometric_stiffness_yields_no_modes() {
        let (factor, scales, free, stiffness, geometric) = diagonal_setup(&[0., 0.]);
        assert!(
            solve_modes(&factor, &scales, &free, &stiffness, &geometric, 4)
                .unwrap()
                .is_empty()
        );
        assert!(
            solve_modes(&factor, &scales, &free, &stiffness, &geometric, 0)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn coupled_problem_eigenvectors_annihilate_the_pencil() {
        // K with off-diagonal coupling, Kg full: residuals must stay tiny.
        let stiffness = DMatrix::from_row_slice(3, 3, &[3., 1., 0., 1., 4., 1., 0., 1., 5.]);
        let geometric =
            DMatrix::from_row_slice(3, 3, &[-1., 0.2, 0., 0.2, -2., 0.1, 0., 0.1, -0.5]);
        let factor = stiffness.clone().cholesky().unwrap();
        let free: Vec<usize> = (0..3).collect();
        let scales = vec![1.; 3];
        let modes = solve_modes(&factor, &scales, &free, &stiffness, &geometric, 3).unwrap();
        assert_eq!(modes.len(), 3);
        for mode in &modes {
            assert!(mode.load_factor > 0.);
            assert!(mode.relative_residual < 1e-10);
            let peak = mode.shape.iter().map(|v| v.abs()).fold(0., f64::max);
            assert_eq!(peak, 1.);
        }
        assert!(modes[0].load_factor <= modes[1].load_factor);
        assert!(modes[1].load_factor <= modes[2].load_factor);
    }
}
