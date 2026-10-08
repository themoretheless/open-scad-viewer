use super::dot;
use crate::{Result, check};

/// SVD-based rank diagnostics of a constraint Jacobian. `conflicting_rows`
/// are the rows that participate in near-null left singular vectors — the
/// constraints that (numerically) contradict each other, intended as input to
/// a future sketch solver.
#[derive(Clone, Debug)]
pub struct ConstraintRankReport {
    /// Singular values, non-increasing.
    pub singular_values: Vec<f64>,
    /// Numerical rank at `tolerance`.
    pub rank: usize,
    /// Absolute tolerance used (`max(m,n)·ε·σ₀` by default).
    pub tolerance: f64,
    /// Indices of rows involved in dependencies: components of left singular
    /// vectors (of singular values below the tolerance) whose magnitude is at
    /// least `1/√m` of the vector's largest component.
    pub conflicting_rows: Vec<usize>,
    /// Condition estimate `σ₀/σ_{r−1}` at the numerical rank (`∞` when the
    /// matrix is numerically zero).
    pub condition: f64,
}

/// Cyclic one-sided Jacobi eigen decomposition of the symmetric PSD matrix
/// `AᵀA`, returning (eigenvalues descending, eigenvectors as columns of V).
fn jacobi_eigen_psd(gram: &[Vec<f64>]) -> (Vec<f64>, Vec<Vec<f64>>) {
    let n = gram.len();
    let mut a = gram.to_vec();
    let mut v = vec![vec![0.; n]; n];
    for i in 0..n {
        v[i][i] = 1.;
    }
    for _sweep in 0..200 {
        let mut off = 0.;
        for p in 0..n {
            for q in p + 1..n {
                off += a[p][q] * a[p][q];
            }
        }
        // Converge well below the √ε reporting tolerance of the callers: the
        // residue of a null eigenvalue is O(off), and callers take square
        // roots of the eigenvalues.
        let trace_sq: f64 = a
            .iter()
            .enumerate()
            .map(|(i, r)| r[i] * r[i])
            .sum::<f64>()
            .max(f64::MIN_POSITIVE);
        if off <= 0.01 * f64::EPSILON * f64::EPSILON * trace_sq {
            break;
        }
        for p in 0..n {
            for q in p + 1..n {
                if a[p][q] == 0. {
                    continue;
                }
                let theta = (a[q][q] - a[p][p]) / (2. * a[p][q]);
                let t = theta.signum() / (theta.abs() + (theta * theta + 1.).sqrt());
                let c = 1. / (t * t + 1.).sqrt();
                let s = t * c;
                for k in 0..n {
                    let akp = a[k][p];
                    let akq = a[k][q];
                    a[k][p] = c * akp - s * akq;
                    a[k][q] = s * akp + c * akq;
                }
                for k in 0..n {
                    let apk = a[p][k];
                    let aqk = a[q][k];
                    a[p][k] = c * apk - s * aqk;
                    a[q][k] = s * apk + c * aqk;
                }
                for k in 0..n {
                    let vkp = v[k][p];
                    let vkq = v[k][q];
                    v[k][p] = c * vkp - s * vkq;
                    v[k][q] = s * vkp + c * vkq;
                }
            }
        }
    }
    let mut pairs: Vec<(f64, usize)> = (0..n).map(|i| (a[i][i].max(0.), i)).collect();
    pairs.sort_by(|x, y| y.0.total_cmp(&x.0));
    let values: Vec<f64> = pairs.iter().map(|p| p.0).collect();
    let mut vectors = vec![vec![0.; n]; n];
    for (new, &(_, old)) in pairs.iter().enumerate() {
        for k in 0..n {
            vectors[k][new] = v[k][old];
        }
    }
    (values, vectors)
}

/// SVD rank diagnostics of a constraint Jacobian `J` (`m × n`, row-major).
///
/// Singular values come from the cyclic-Jacobi eigendecomposition of the
/// smaller Gram matrix; `conflicting_rows` are recovered from the *left*
/// singular vectors (eigendecomposition of `J·Jᵀ`) whose singular values fall
/// below the tolerance: their significant components identify exactly the
/// rows participating in a near-linear dependency — i.e. the constraints that
/// contradict or duplicate each other, intended as input to a future sketch
/// solver. (For tall matrices with `m > n` the left null space is
/// structurally non-trivial, so redundancy witnesses are always reported.)
///
/// Because Gram formation squares the condition number, the default tolerance
/// is `max(m,n)·√ε·σ₀`.
pub fn constraint_rank_report(
    jacobian: &[Vec<f64>],
    tolerance: Option<f64>,
) -> Result<ConstraintRankReport> {
    let m = jacobian.len();
    check(m > 0, "Constraint Jacobian must have at least one row")?;
    let n = jacobian[0].len();
    check(n > 0, "Constraint Jacobian must have at least one column")?;
    check(
        jacobian.iter().all(|row| row.len() == n)
            && jacobian.iter().flatten().all(|v| v.is_finite()),
        "Constraint Jacobian must be rectangular and finite",
    )?;
    // Left Gram matrix J·Jᵀ (m × m): its eigenvectors are the left singular
    // vectors and its eigenvalues are the squared singular values.
    let mut left_gram = vec![vec![0.; m]; m];
    for i in 0..m {
        for j in 0..=i {
            left_gram[i][j] = dot(&jacobian[i], &jacobian[j]);
            left_gram[j][i] = left_gram[i][j];
        }
    }
    let (eigenvalues, u) = jacobi_eigen_psd(&left_gram);
    let sigma0 = eigenvalues[0].sqrt();
    let default_tol = (m.max(n) as f64) * f64::EPSILON.sqrt() * sigma0;
    let tolerance = tolerance.unwrap_or(default_tol);
    let singular_values: Vec<f64> = eigenvalues
        .iter()
        .take(m.min(n))
        .map(|e| e.sqrt())
        .collect();
    let rank = eigenvalues.iter().filter(|e| e.sqrt() > tolerance).count();
    let condition = if rank == 0 {
        f64::INFINITY
    } else {
        sigma0 / eigenvalues[rank - 1].sqrt()
    };
    // Conflicting rows: significant components of left singular vectors with
    // singular value below the tolerance.
    let mut conflict = vec![false; m];
    for (col, &eigenvalue) in eigenvalues.iter().enumerate() {
        if eigenvalue.sqrt() > tolerance {
            continue;
        }
        let ucol: Vec<f64> = (0..m).map(|k| u[k][col]).collect();
        let max_component = ucol.iter().map(|x| x.abs()).fold(0., f64::max);
        if max_component == 0. {
            continue;
        }
        let threshold = max_component / (m as f64).sqrt();
        for (i, &ui) in ucol.iter().enumerate() {
            if ui.abs() >= threshold * (1. - 1e-9) {
                conflict[i] = true;
            }
        }
    }
    Ok(ConstraintRankReport {
        singular_values,
        rank,
        tolerance,
        conflicting_rows: (0..m).filter(|&i| conflict[i]).collect(),
        condition,
    })
}

#[cfg(test)]
mod tests {
    use super::constraint_rank_report;

    #[test]
    fn constraint_rank_report_identifies_conflicting_rows() {
        // Row 2 = row 0 + row 1, row 3 independent → rank 3, rows {0,1,2}
        // participate in the dependency, row 3 does not.
        let j = vec![
            vec![1., 0., 0.],
            vec![0., 1., 0.],
            vec![1., 1., 0.],
            vec![0., 0., 5.],
        ];
        let report = constraint_rank_report(&j, None).unwrap();
        assert_eq!(report.rank, 3);
        assert_eq!(report.singular_values.len(), 3);
        assert!(report.conflicting_rows.contains(&0));
        assert!(report.conflicting_rows.contains(&1));
        assert!(report.conflicting_rows.contains(&2));
        assert!(!report.conflicting_rows.contains(&3));

        // Doubled constraint: rows 2 and 3 are (1,1) and (2,2); the
        // dependency witness must be reported among the redundant rows.
        let j2 = vec![vec![1., 0.], vec![0., 1.], vec![1., 1.], vec![2., 2.]];
        let report2 = constraint_rank_report(&j2, None).unwrap();
        assert_eq!(report2.rank, 2);
        // The left null space of this J is two-dimensional and both near-null
        // vectors involve the duplicated rows: at least two rows are flagged,
        // and the duplicated row 3 is among them.
        assert!(report2.conflicting_rows.len() >= 2);
        assert!(report2.conflicting_rows.contains(&3));
    }

    #[test]
    fn constraint_rank_report_full_row_rank_has_no_conflicts() {
        // Wide full-row-rank Jacobian: no left null space, no conflicts.
        let j = vec![vec![1., 0., 1.], vec![0., 1., 1.]];
        let report = constraint_rank_report(&j, None).unwrap();
        assert_eq!(report.rank, 2);
        assert!(report.conflicting_rows.is_empty());
        assert!(report.condition < 10.);
    }
}
