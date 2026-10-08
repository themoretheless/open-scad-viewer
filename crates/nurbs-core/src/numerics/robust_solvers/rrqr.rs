use super::{back_substitute, dot, norm2};
use crate::{Result, check};

/// Diagnostics of the numerical rank decision.
#[derive(Clone, Debug)]
pub struct RankDiagnostics {
    /// Numerical rank.
    pub rank: usize,
    /// Absolute tolerance used against the pivot diagonal.
    pub tolerance: f64,
    /// Diagonal of `R` (pivot magnitudes, non-increasing in modulus).
    pub diagonal: Vec<f64>,
    /// Ratio `|r_kk| / |r_00|` at the detected rank boundary (0 if full rank).
    pub boundary_ratio: f64,
}

/// Rank-revealing QR with column pivoting: `A·P = Q·R`, thin `Q` (`m × n`),
/// `R` upper trapezoidal (`n × n` over the first `min(m,n)` rows stored).
#[derive(Clone, Debug)]
pub struct Rrqr {
    /// Thin orthogonal factor, `m × n`.
    pub q: Vec<Vec<f64>>,
    /// Upper factor, `n × n` (rows ≥ m are zero for tall-skinny input).
    pub r: Vec<Vec<f64>>,
    /// Column permutation: column `j` of `A·P` is column `permutation[j]` of `A`.
    pub permutation: Vec<usize>,
    /// Row / column counts of the input.
    pub rows: usize,
    pub cols: usize,
    /// Rank decision diagnostics.
    pub diagnostics: RankDiagnostics,
}

/// Rank-revealing QR with column pivoting (Householder, LAPACK `geqp3`
/// pivoting rule: at each step the column with the largest trailing norm is
/// pivoted to the front, so `|r_00| ≥ |r_11| ≥ …`).
///
/// `tolerance`, when `None`, defaults to `max(m,n)·ε·|r_00|`.
pub fn rrqr(matrix: &[Vec<f64>], tolerance: Option<f64>) -> Result<Rrqr> {
    let m = matrix.len();
    check(m > 0, "RRQR matrix must have at least one row")?;
    let n = matrix[0].len();
    check(n > 0, "RRQR matrix must have at least one column")?;
    check(
        matrix.iter().all(|row| row.len() == n)
            && matrix
                .iter()
                .flatten()
                .all(|value| value.is_finite()),
        "RRQR matrix must be rectangular and finite",
    )?;
    let mut a: Vec<Vec<f64>> = matrix.to_vec();
    let mut permutation: Vec<usize> = (0..n).collect();
    // Householder vectors: v[k] stored with implicit leading 1.
    let mut reflectors: Vec<(Vec<f64>, f64)> = Vec::with_capacity(n.min(m));
    let steps = n.min(m);
    for k in 0..steps {
        // Pivot: largest trailing column norm over rows k..m.
        let pivot = (k..n)
            .max_by(|&p, &q| {
                let np: f64 = (k..m).map(|i| a[i][p] * a[i][p]).sum();
                let nq: f64 = (k..m).map(|i| a[i][q] * a[i][q]).sum();
                np.total_cmp(&nq)
            })
            .unwrap();
        if pivot != k {
            for row in a.iter_mut() {
                row.swap(k, pivot);
            }
            permutation.swap(k, pivot);
        }
        // Householder reflector for a[k..m][k].
        let mut v: Vec<f64> = (k..m).map(|i| a[i][k]).collect();
        let sigma = norm2(&v);
        if sigma == 0. {
            reflectors.push((vec![0.; m - k], 0.));
            continue;
        }
        let beta = if v[0] >= 0. { -sigma } else { sigma };
        v[0] -= beta;
        let tau = if norm2(&v) == 0. {
            0.
        } else {
            2. / dot(&v, &v)
        };
        // Apply H = I − τ v vᵀ to trailing columns.
        for j in k + 1..n {
            let mut s = 0.;
            for (t, &vt) in v.iter().enumerate() {
                s += vt * a[k + t][j];
            }
            s *= tau;
            for (t, &vt) in v.iter().enumerate() {
                a[k + t][j] -= s * vt;
            }
        }
        for (t, _) in v.iter().enumerate() {
            a[k + t][k] = if t == 0 { beta } else { 0. };
        }
        reflectors.push((v, tau));
    }
    // Assemble thin Q (m × steps) by applying reflectors to e_j.
    let mut q = vec![vec![0.; steps]; m];
    for col in 0..steps {
        q[col][col] = 1.;
        for (k, (v, tau)) in reflectors.iter().enumerate().take(col + 1).rev() {
            if *tau == 0. {
                continue;
            }
            let mut s = 0.;
            for (t, &vt) in v.iter().enumerate() {
                s += vt * q[k + t][col];
            }
            s *= tau;
            for (t, &vt) in v.iter().enumerate() {
                q[k + t][col] -= s * vt;
            }
        }
    }
    // R: steps × n upper trapezoidal, padded to n × n.
    let mut r = vec![vec![0.; n]; n];
    for i in 0..steps {
        for j in i..n {
            r[i][j] = a[i][j];
        }
    }
    let diagonal: Vec<f64> = (0..steps).map(|i| r[i][i]).collect();
    let scale = diagonal[0].abs();
    let tolerance = tolerance.unwrap_or(m.max(n) as f64 * f64::EPSILON * scale);
    let mut rank = 0;
    let mut boundary_ratio = 0.;
    while rank < steps && diagonal[rank].abs() > tolerance {
        rank += 1;
    }
    if rank < steps && scale > 0. {
        boundary_ratio = diagonal[rank].abs() / scale;
    }
    Ok(Rrqr {
        q,
        r,
        permutation,
        rows: m,
        cols: n,
        diagnostics: RankDiagnostics {
            rank,
            tolerance,
            diagonal,
            boundary_ratio,
        },
    })
}

impl Rrqr {
    /// Least-squares solve. Full column rank: the classic `x = P R⁻¹ Qᵀ b`.
    /// Rank-deficient or underdetermined: the *minimum-norm* least-squares
    /// solution via the complete orthogonal factorization — `R₁` (the leading
    /// `rank × n` block of `R`) is re-factored as `R₁ᵀ = Q₂ R₂` and the
    /// solution is `x = P Q₂ (R₂⁻ᵀ Q₁ᵀ b, 0)`.
    pub fn solve_least_squares(&self, b: &[f64]) -> Result<Vec<f64>> {
        check(
            b.len() == self.rows && b.iter().all(|v| v.is_finite()),
            "Right-hand side must match the matrix rows and be finite",
        )?;
        let n = self.cols;
        let k = self.diagnostics.rank;
        // y = Q₁ᵀ b (first k entries used).
        let mut y = vec![0.; self.r.len().min(self.rows)];
        for (col, ycol) in y.iter_mut().enumerate() {
            *ycol = (0..self.rows).map(|i| self.q[i][col] * b[i]).sum();
        }
        let z: Vec<f64> = if k == n {
            // Full column rank: back-substitute on the leading n × n block.
            back_substitute(&self.r[..n], &y[..n])?
        } else if k == 0 {
            vec![0.; n]
        } else {
            // Complete orthogonal factorization on R₁ (k × n): with the
            // monotone pivot diagonal produced by column pivoting, a plain
            // unpivoted thin QR of R₁ᵀ (n × k, full column rank) is stable:
            // R₁ᵀ = Q₂ R₂, then R₂ᵀ w = y₁ and x = P (Q₂ w).
            let r1: Vec<Vec<f64>> = (0..k).map(|i| self.r[i].clone()).collect();
            let (q2, r2) = thin_qr(&transpose(&r1));
            // Solve R₂ᵀ w = y[0..k] (lower triangular).
            let mut w = y[..k].to_vec();
            for i in 0..k {
                for j in 0..i {
                    w[i] -= r2[j][i] * w[j];
                }
                check(
                    r2[i][i].abs() > 0.,
                    "Rank-revealing factor is singular within the reported rank",
                )?;
                w[i] /= r2[i][i];
            }
            // z = Q₂ w (n-vector).
            let mut z = vec![0.; n];
            for row in 0..n {
                z[row] = (0..k).map(|j| q2[row][j] * w[j]).sum();
            }
            z
        };
        // Un-permute: x[permutation[j]] = z[j].
        let mut x = vec![0.; n];
        for (j, &source) in self.permutation.iter().enumerate() {
            x[source] = z[j];
        }
        Ok(x)
    }
}

fn transpose(a: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let m = a.len();
    let n = a[0].len();
    (0..n)
        .map(|j| (0..m).map(|i| a[i][j]).collect())
        .collect()
}

/// Unpivoted thin Householder QR of a full-column-rank `m × n` matrix
/// (`m ≥ n`), returning (`Q` m×n, `R` n×n).
fn thin_qr(matrix: &[Vec<f64>]) -> (Vec<Vec<f64>>, Vec<Vec<f64>>) {
    let m = matrix.len();
    let n = matrix[0].len();
    let mut a = matrix.to_vec();
    let mut reflectors: Vec<(Vec<f64>, f64)> = Vec::with_capacity(n);
    for k in 0..n {
        let mut v: Vec<f64> = (k..m).map(|i| a[i][k]).collect();
        let sigma = norm2(&v);
        if sigma == 0. {
            reflectors.push((vec![0.; m - k], 0.));
            continue;
        }
        let beta = if v[0] >= 0. { -sigma } else { sigma };
        v[0] -= beta;
        let tau = 2. / dot(&v, &v);
        for j in k + 1..n {
            let mut s = 0.;
            for (t, &vt) in v.iter().enumerate() {
                s += vt * a[k + t][j];
            }
            s *= tau;
            for (t, &vt) in v.iter().enumerate() {
                a[k + t][j] -= s * vt;
            }
        }
        a[k][k] = beta;
        reflectors.push((v, tau));
    }
    let mut q = vec![vec![0.; n]; m];
    for col in 0..n {
        q[col][col] = 1.;
        for (k, (v, tau)) in reflectors.iter().enumerate().take(col + 1).rev() {
            if *tau == 0. {
                continue;
            }
            let mut s = 0.;
            for (t, &vt) in v.iter().enumerate() {
                s += vt * q[k + t][col];
            }
            s *= tau;
            for (t, &vt) in v.iter().enumerate() {
                q[k + t][col] -= s * vt;
            }
        }
    }
    let r: Vec<Vec<f64>> = (0..n)
        .map(|i| (0..n).map(|j| if j >= i { a[i][j] } else { 0. }).collect())
        .collect();
    (q, r)
}

#[cfg(test)]
mod tests {
    use super::super::norm2;
    use super::super::test_helpers::approx_eq;
    use super::rrqr;

    #[test]
    fn rrqr_detects_rank_of_rank_deficient_matrix() {
        // rank(A) = 2: third column is the sum of the first two.
        let a = vec![
            vec![1., 2., 3.],
            vec![4., 5., 9.],
            vec![7., 8., 15.],
            vec![1., 1., 2.],
        ];
        let f = rrqr(&a, None).unwrap();
        assert_eq!(f.diagnostics.rank, 2);
        // Reconstruction A·P ≈ Q·R on the full-rank part is tight.
        let (q, r, p) = (&f.q, &f.r, &f.permutation);
        for i in 0..4 {
            for j in 0..3 {
                let qr: f64 = (0..3).map(|k| q[i][k] * r[k][j]).sum();
                assert!(approx_eq(qr, a[i][p[j]], 1e-10));
            }
        }
    }

    #[test]
    fn rrqr_full_rank_least_squares() {
        // Overdetermined consistent system: x = (1, −2), b = A x exactly.
        let a = vec![vec![2., 0.], vec![-1., 3.], vec![0., 4.]];
        let b = vec![2., -7., -8.];
        let f = rrqr(&a, None).unwrap();
        assert_eq!(f.diagnostics.rank, 2);
        let x = f.solve_least_squares(&b).unwrap();
        assert!(approx_eq(x[0], 1., 1e-10));
        assert!(approx_eq(x[1], -2., 1e-10));
    }

    #[test]
    fn rrqr_minimum_norm_for_underdetermined() {
        // Underdetermined: x + y + z = 3, x − y = 0 → min-norm (1, 1, 1).
        let a = vec![vec![1., 1., 1.], vec![1., -1., 0.]];
        let b = vec![3., 0.];
        let f = rrqr(&a, None).unwrap();
        assert_eq!(f.diagnostics.rank, 2);
        let x = f.solve_least_squares(&b).unwrap();
        assert!(approx_eq(x[0], 1., 1e-10), "{x:?}");
        assert!(approx_eq(x[1], 1., 1e-10), "{x:?}");
        assert!(approx_eq(x[2], 1., 1e-10), "{x:?}");
    }

    #[test]
    fn rrqr_rank_deficient_minimum_norm_least_squares() {
        // Rank 1 matrix: rows are multiples of (1, 2). b = (3, 6) lies in the
        // column space; min-norm solution of x + 2y = 3 is (0.6, 1.2).
        let a = vec![vec![1., 2.], vec![2., 4.], vec![-1., -2.]];
        let b = vec![3., 6., -3.];
        let f = rrqr(&a, None).unwrap();
        assert_eq!(f.diagnostics.rank, 1);
        let x = f.solve_least_squares(&b).unwrap();
        assert!(approx_eq(x[0] + 2. * x[1], 3., 1e-9), "{x:?}");
        assert!(approx_eq(norm2(&x), (0.6f64.hypot(1.2)), 1e-9), "{x:?}");
    }
}
