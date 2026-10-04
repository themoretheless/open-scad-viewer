//! Sparse linear algebra on a compressed sparse row (CSR) matrix (items
//! 901-904): triplet assembly with duplicate accumulation, sparse
//! matrix-vector product (SpMV), index-mask subblock extraction, and a
//! diagonally preconditioned conjugate gradient (CG) solver for symmetric
//! positive definite systems.
//!
//! The matrix stores binary64 values; assembly is deterministic (triplets
//! are sorted and merged row by row, so the result does not depend on
//! insertion order). CG runs under an explicit iteration budget; exhaustion
//! of the budget before the residual target is reached is a numeric error,
//! never a silently approximate answer. A nonpositive diagonal entry (which
//! would break both the Jacobi preconditioner and the SPD premise) is
//! reported as a numeric error as well.
use crate::{Result, check, numeric};

/// Compressed sparse row matrix with sorted column indices per row.
#[derive(Clone, Debug, PartialEq)]
pub struct CsrMatrix {
    nrows: usize,
    ncols: usize,
    values: Vec<f64>,
    columns: Vec<usize>,
    row_offsets: Vec<usize>,
}

impl CsrMatrix {
    /// Number of rows.
    pub fn nrows(&self) -> usize {
        self.nrows
    }

    /// Number of columns.
    pub fn ncols(&self) -> usize {
        self.ncols
    }

    /// Number of stored (structural) entries.
    pub fn nnz(&self) -> usize {
        self.values.len()
    }

    /// Row slice as (column, value) pairs with strictly ascending columns.
    pub fn row(&self, row: usize) -> Result<Vec<(usize, f64)>> {
        check(row < self.nrows, "CSR row index out of range")?;
        let (lo, hi) = (self.row_offsets[row], self.row_offsets[row + 1]);
        Ok(self.columns[lo..hi]
            .iter()
            .copied()
            .zip(self.values[lo..hi].iter().copied())
            .collect())
    }

    /// Value at (row, column), zero when structurally absent.
    pub fn at(&self, row: usize, column: usize) -> Result<f64> {
        check(
            row < self.nrows && column < self.ncols,
            "CSR index out of range",
        )?;
        let (lo, hi) = (self.row_offsets[row], self.row_offsets[row + 1]);
        Ok(match self.columns[lo..hi].binary_search(&column) {
            Ok(pos) => self.values[lo + pos],
            Err(_) => 0.,
        })
    }

    /// Sparse matrix-vector product y = A·x.
    pub fn spmv(&self, x: &[f64]) -> Result<Vec<f64>> {
        check(x.len() == self.ncols, "SpMV operand width mismatch")?;
        let mut y = vec![0.; self.nrows];
        for row in 0..self.nrows {
            let mut sum = 0.;
            for e in self.row_offsets[row]..self.row_offsets[row + 1] {
                sum += self.values[e] * x[self.columns[e]];
            }
            y[row] = sum;
        }
        numeric(
            y.iter().all(|v| v.is_finite()),
            "SpMV exhausted finite numeric range",
        )?;
        Ok(y)
    }

    /// Principal subblock A[keep][keep] for an ascending duplicate-free
    /// index mask. Every kept index must be a valid row/column of a square
    /// matrix.
    pub fn subblock(&self, keep: &[usize]) -> Result<CsrMatrix> {
        check(
            self.nrows == self.ncols,
            "Subblock extraction requires a square matrix",
        )?;
        for w in keep.windows(2) {
            check(w[0] < w[1], "Subblock mask must be ascending without duplicates")?;
        }
        for &i in keep {
            check(i < self.nrows, "Subblock mask index out of range")?;
        }
        // Position of each row inside the mask, nrows for dropped rows.
        let mut slot = vec![usize::MAX; self.nrows];
        for (s, &i) in keep.iter().enumerate() {
            slot[i] = s;
        }
        let mut builder = CsrBuilder::new(keep.len(), keep.len());
        for (s, &row) in keep.iter().enumerate() {
            for e in self.row_offsets[row]..self.row_offsets[row + 1] {
                let col = self.columns[e];
                if slot[col] != usize::MAX {
                    builder.push(s, slot[col], self.values[e]);
                }
            }
        }
        builder.build()
    }

    /// Diagonal entries of a square matrix; a structurally missing diagonal
    /// entry reads as zero.
    pub fn diagonal(&self) -> Result<Vec<f64>> {
        check(self.nrows == self.ncols, "Diagonal requires a square matrix")?;
        let mut diag = vec![0.; self.nrows];
        for row in 0..self.nrows {
            for e in self.row_offsets[row]..self.row_offsets[row + 1] {
                if self.columns[e] == row {
                    diag[row] = self.values[e];
                    break;
                }
                if self.columns[e] > row {
                    break;
                }
            }
        }
        Ok(diag)
    }
}

/// Triplet assembler for a [`CsrMatrix`]. Duplicate (row, column) triplets
/// are accumulated by summation at [`CsrBuilder::build`].
#[derive(Clone, Debug)]
pub struct CsrBuilder {
    nrows: usize,
    ncols: usize,
    triplets: Vec<(usize, usize, f64)>,
}

impl CsrBuilder {
    pub fn new(nrows: usize, ncols: usize) -> Self {
        Self {
            nrows,
            ncols,
            triplets: Vec::new(),
        }
    }

    /// Add a single triplet; duplicates are summed at build time. Indices
    /// are validated here, values are validated for finiteness at build.
    pub fn push(&mut self, row: usize, column: usize, value: f64) {
        self.triplets.push((row, column, value));
    }

    /// Symmetric assembly helper: pushes (r, c, v) and, for r ≠ c, the
    /// mirrored triplet (c, r, v).
    pub fn push_symmetric(&mut self, row: usize, column: usize, value: f64) {
        self.triplets.push((row, column, value));
        if row != column {
            self.triplets.push((column, row, value));
        }
    }

    /// Sort, merge duplicates by summation and pack into CSR. Zero-sum
    /// duplicates are dropped from the structure.
    pub fn build(mut self) -> Result<CsrMatrix> {
        for &(row, column, value) in &self.triplets {
            check(
                row < self.nrows && column < self.ncols,
                "CSR triplet index out of range",
            )?;
            numeric(value.is_finite(), "CSR triplet value is not finite")?;
        }
        // Deterministic ordering: by row, then column. Values of equal keys
        // are summed left to right in sorted order, so the result is
        // independent of the original insertion order.
        self.triplets
            .sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));
        let mut values: Vec<f64> = Vec::with_capacity(self.triplets.len());
        let mut columns: Vec<usize> = Vec::with_capacity(self.triplets.len());
        let mut row_offsets = vec![0usize; self.nrows + 1];
        let mut cursor = 0usize;
        for (row, column, value) in self.triplets {
            if let Some(&last) = columns.last() {
                if cursor == row && last == column {
                    let merged = values.last_mut().expect("columns and values are parallel");
                    *merged += value;
                    continue;
                }
            }
            while cursor < row {
                cursor += 1;
                row_offsets[cursor] = values.len();
            }
            columns.push(column);
            values.push(value);
        }
        while cursor < self.nrows {
            cursor += 1;
            row_offsets[cursor] = values.len();
        }
        // Drop entries that summed to exactly zero to keep the structure lean.
        let keep: Vec<bool> = values.iter().map(|&v| v != 0.).collect();
        if keep.iter().all(|&k| k) {
            return Ok(CsrMatrix {
                nrows: self.nrows,
                ncols: self.ncols,
                values,
                columns,
                row_offsets,
            });
        }
        let mut out_values = Vec::new();
        let mut out_columns = Vec::new();
        let mut out_offsets = vec![0usize; self.nrows + 1];
        for row in 0..self.nrows {
            out_offsets[row + 1] = out_offsets[row];
            for e in row_offsets[row]..row_offsets[row + 1] {
                if keep[e] {
                    out_columns.push(columns[e]);
                    out_values.push(values[e]);
                    out_offsets[row + 1] += 1;
                }
            }
        }
        Ok(CsrMatrix {
            nrows: self.nrows,
            ncols: self.ncols,
            values: out_values,
            columns: out_columns,
            row_offsets: out_offsets,
        })
    }
}

/// CG controls: iteration budget and relative residual target.
#[derive(Clone, Copy, Debug)]
pub struct CgOptions {
    /// Hard iteration budget; exhaustion is a numeric error.
    pub max_iterations: usize,
    /// Stop when ‖r‖₂ ≤ tolerance·‖b‖₂.
    pub tolerance: f64,
}

impl CgOptions {
    pub fn new(max_iterations: usize, tolerance: f64) -> Result<Self> {
        check(
            max_iterations >= 1,
            "CG needs an iteration budget of at least 1",
        )?;
        check(
            tolerance.is_finite() && tolerance > 0. && tolerance < 1.,
            "CG tolerance must lie in (0, 1)",
        )?;
        Ok(Self {
            max_iterations,
            tolerance,
        })
    }
}

/// CG result: solution and the achieved relative residual.
#[derive(Clone, Debug)]
pub struct CgSolution {
    pub x: Vec<f64>,
    pub iterations: usize,
    /// Achieved ‖r‖₂ / ‖b‖₂ at termination.
    pub relative_residual: f64,
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Diagonally (Jacobi) preconditioned conjugate gradient for SPD systems
/// A·x = b in CSR form. `x0` seeds the iteration (pass zeros when no guess
/// is available). Convergence to `options.tolerance` within the budget is
/// required; otherwise a numeric error is returned.
pub fn cg_solve(
    matrix: &CsrMatrix,
    rhs: &[f64],
    x0: &[f64],
    options: CgOptions,
) -> Result<CgSolution> {
    let n = matrix.nrows;
    check(matrix.nrows == matrix.ncols, "CG requires a square matrix")?;
    check(rhs.len() == n && x0.len() == n, "CG operand sizes mismatch")?;
    check(
        rhs.iter().chain(x0).all(|v| v.is_finite()),
        "CG operands must be finite",
    )?;
    let diag = matrix.diagonal()?;
    for &d in &diag {
        numeric(
            d > 0.,
            "CG needs a positive diagonal (SPD premise violated)",
        )?;
    }
    let norm_b = dot(rhs, rhs).sqrt();
    if norm_b == 0. {
        // The unique SPD solution of Ax = 0 is x = 0.
        return Ok(CgSolution {
            x: vec![0.; n],
            iterations: 0,
            relative_residual: 0.,
        });
    }
    let mut x = x0.to_vec();
    let ax = matrix.spmv(&x)?;
    let mut r: Vec<f64> = (0..n).map(|i| rhs[i] - ax[i]).collect();
    let mut z: Vec<f64> = (0..n).map(|i| r[i] / diag[i]).collect();
    let mut p = z.clone();
    let mut rz = dot(&r, &z);
    numeric(rz.is_finite(), "CG residual inner product overflowed")?;
    let target = options.tolerance * norm_b;
    let mut iterations = 0;
    while dot(&r, &r).sqrt() > target && iterations < options.max_iterations {
        let ap = matrix.spmv(&p)?;
        let pap = dot(&p, &ap);
        numeric(
            pap > 0.,
            "CG broke down: nonpositive curvature (matrix is not SPD)",
        )?;
        let alpha = rz / pap;
        for i in 0..n {
            x[i] += alpha * p[i];
            r[i] -= alpha * ap[i];
            z[i] = r[i] / diag[i];
        }
        let rz_next = dot(&r, &z);
        numeric(rz_next.is_finite(), "CG residual inner product overflowed")?;
        let beta = rz_next / rz;
        for i in 0..n {
            p[i] = z[i] + beta * p[i];
        }
        rz = rz_next;
        iterations += 1;
    }
    let relative_residual = dot(&r, &r).sqrt() / norm_b;
    numeric(
        relative_residual <= options.tolerance,
        "CG did not converge within the iteration budget",
    )?;
    Ok(CgSolution {
        x,
        iterations,
        relative_residual,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 5-point Laplacian on an n×n grid with Dirichlet boundary, an SPD
    /// matrix with bandwidth ~2n+1.
    fn laplacian_5pt(n: usize) -> CsrMatrix {
        let mut builder = CsrBuilder::new(n * n, n * n);
        let index = |i: usize, j: usize| i * n + j;
        for i in 0..n {
            for j in 0..n {
                let c = index(i, j);
                builder.push(c, c, 4.);
                if i > 0 {
                    builder.push(c, index(i - 1, j), -1.);
                }
                if i + 1 < n {
                    builder.push(c, index(i + 1, j), -1.);
                }
                if j > 0 {
                    builder.push(c, index(i, j - 1), -1.);
                }
                if j + 1 < n {
                    builder.push(c, index(i, j + 1), -1.);
                }
            }
        }
        builder.build().unwrap()
    }

    #[test]
    fn triplet_dedup_and_at() {
        let mut builder = CsrBuilder::new(3, 3);
        builder.push(0, 0, 1.);
        builder.push(0, 0, 2.);
        builder.push(2, 1, 5.);
        builder.push(2, 1, -5.);
        builder.push(1, 2, 7.);
        let m = builder.build().unwrap();
        assert_eq!(m.at(0, 0).unwrap(), 3.);
        assert_eq!(m.at(2, 1).unwrap(), 0.);
        assert_eq!(m.at(0, 1).unwrap(), 0.);
        assert_eq!(m.at(1, 2).unwrap(), 7.);
        // Zero-sum duplicate dropped from the structure.
        assert_eq!(m.nnz(), 2);
        // Insertion order does not matter.
        let mut builder2 = CsrBuilder::new(3, 3);
        builder2.push(1, 2, 7.);
        builder2.push(0, 0, 2.);
        builder2.push(0, 0, 1.);
        let m2 = builder2.build().unwrap();
        assert_eq!(m.at(0, 0).unwrap(), m2.at(0, 0).unwrap());
        assert!(CsrBuilder::new(1, 1).build().is_ok());
        let mut bad = CsrBuilder::new(2, 2);
        bad.push(5, 0, 1.);
        assert!(bad.build().is_err());
    }

    #[test]
    fn spmv_matches_dense_reference() {
        let m = laplacian_5pt(4);
        let x: Vec<f64> = (0..16).map(|i| (i as f64 * 0.37).sin()).collect();
        let y = m.spmv(&x).unwrap();
        for row in 0..16 {
            let mut dense = 0.;
            for (col, value) in m.row(row).unwrap() {
                dense += value * x[col];
            }
            assert!((y[row] - dense).abs() <= 1e-15 * dense.abs().max(1.));
        }
        assert!(m.spmv(&[1., 2.]).is_err());
    }

    #[test]
    fn cg_recovers_known_solution() {
        let m = laplacian_5pt(6);
        let n = m.nrows();
        let x_star: Vec<f64> = (0..n).map(|i| 0.1 * i as f64 - 1.7).collect();
        let b = m.spmv(&x_star).unwrap();
        let options = CgOptions::new(10 * n + 50, 1e-12).unwrap();
        let solved = cg_solve(&m, &b, &vec![0.; n], options).unwrap();
        assert!(solved.relative_residual <= 1e-12);
        for (got, want) in solved.x.iter().zip(&x_star) {
            assert!((got - want).abs() < 1e-8, "got {got}, want {want}");
        }
    }

    #[test]
    fn cg_converges_on_laplacian_and_respects_budget() {
        let m = laplacian_5pt(8);
        let n = m.nrows();
        let b: Vec<f64> = (0..n).map(|i| ((i * 7 + 3) % 11) as f64 - 5.).collect();
        let options = CgOptions::new(200, 1e-10).unwrap();
        let solved = cg_solve(&m, &b, &vec![0.; n], options).unwrap();
        // Residual check against the stored matrix, independent of internals.
        let ax = m.spmv(&solved.x).unwrap();
        let residual: f64 = (0..n).map(|i| (ax[i] - b[i]).powi(2)).sum::<f64>().sqrt();
        let norm_b: f64 = b.iter().map(|v| v * v).sum::<f64>().sqrt();
        assert!(residual / norm_b <= 1e-10);
        // Budget exhaustion is a numeric error, not a silent answer.
        let starved = CgOptions::new(1, 1e-14).unwrap();
        assert!(cg_solve(&m, &b, &vec![0.; n], starved).is_err());
        // Indefinite diagonal is rejected.
        let mut bad = CsrBuilder::new(2, 2);
        bad.push(0, 0, -1.);
        bad.push(1, 1, 1.);
        let bad = bad.build().unwrap();
        assert!(cg_solve(&bad, &[1., 1.], &[0., 0.], options).is_err());
        // Zero right-hand side solves exactly.
        let zero = cg_solve(&m, &vec![0.; n], &vec![0.; n], options).unwrap();
        assert_eq!(zero.iterations, 0);
        assert!(zero.x.iter().all(|&v| v == 0.));
    }

    #[test]
    fn subblock_selects_principal_minor() {
        let m = laplacian_5pt(3);
        // Keep the four corners of the 3x3 grid: indices 0, 2, 6, 8.
        let keep = [0, 2, 6, 8];
        let sub = m.subblock(&keep).unwrap();
        assert_eq!(sub.nrows(), 4);
        assert_eq!(sub.ncols(), 4);
        assert_eq!(sub.at(0, 0).unwrap(), 4.);
        assert_eq!(sub.at(3, 3).unwrap(), 4.);
        // Corners are not adjacent on the grid: no off-diagonals survive.
        assert_eq!(sub.nnz(), 4);
        // Keep a connected pair: edge survives.
        let pair = m.subblock(&[0, 1]).unwrap();
        assert_eq!(pair.at(0, 1).unwrap(), -1.);
        assert_eq!(pair.at(1, 0).unwrap(), -1.);
        // Unsorted / duplicate / out-of-range masks are rejected.
        assert!(m.subblock(&[2, 0]).is_err());
        assert!(m.subblock(&[1, 1]).is_err());
        assert!(m.subblock(&[99]).is_err());
    }
}
