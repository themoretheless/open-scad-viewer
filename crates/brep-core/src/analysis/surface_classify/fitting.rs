use super::*;

/// Jacobi eigendecomposition of a symmetric 3×3 matrix.
/// Returns (eigenvalues, eigenvectors as columns), ascending by eigenvalue.
pub(super) fn eigen_symmetric3(m: [[f64; 3]; 3]) -> ([f64; 3], [[f64; 3]; 3]) {
    let mut a = m;
    let mut v = [[0.; 3]; 3];
    for i in 0..3 {
        v[i][i] = 1.;
    }
    for _ in 0..64 {
        // Largest off-diagonal entry.
        let (mut p, mut q, mut biggest) = (0, 1, 0f64);
        for i in 0..3 {
            for j in (i + 1)..3 {
                if a[i][j].abs() > biggest {
                    biggest = a[i][j].abs();
                    p = i;
                    q = j;
                }
            }
        }
        if biggest <= 1e-300 {
            break;
        }
        let theta = 0.5 * (a[q][q] - a[p][p]) / a[p][q];
        let t = theta.signum() / (theta.abs() + (theta * theta + 1.).sqrt());
        let c = 1. / (t * t + 1.).sqrt();
        let s = t * c;
        for k in 0..3 {
            let (apk, aqk) = (a[p][k], a[q][k]);
            a[p][k] = c * apk - s * aqk;
            a[q][k] = s * apk + c * aqk;
        }
        for k in 0..3 {
            let (akp, akq) = (a[k][p], a[k][q]);
            a[k][p] = c * akp - s * akq;
            a[k][q] = s * akp + c * akq;
        }
        for k in 0..3 {
            let (vkp, vkq) = (v[k][p], v[k][q]);
            v[k][p] = c * vkp - s * vkq;
            v[k][q] = s * vkp + c * vkq;
        }
    }
    let mut order = [0usize, 1, 2];
    order.sort_by(|&i, &j| a[i][i].total_cmp(&a[j][j]));
    let values = [a[order[0]][order[0]], a[order[1]][order[1]], a[order[2]][order[2]]];
    let vectors = std::array::from_fn(|r| std::array::from_fn(|c| v[r][order[c]]));
    (values, vectors)
}

pub(super) fn eigenvector(eigen: &([f64; 3], [[f64; 3]; 3]), which: usize) -> [f64; 3] {
    [eigen.1[0][which], eigen.1[1][which], eigen.1[2][which]]
}

/// Solve a small dense linear system by Gaussian elimination with partial
/// pivoting. Returns `None` when the system is numerically singular.
pub(super) fn solve_small(matrix: &[Vec<f64>], rhs: &[f64]) -> Option<Vec<f64>> {
    let n = rhs.len();
    let mut a: Vec<Vec<f64>> = matrix.to_vec();
    let mut b = rhs.to_vec();
    for col in 0..n {
        let pivot = (col..n).max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))?;
        if a[pivot][col].abs() <= 1e-300 {
            return None;
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        for row in (col + 1)..n {
            let f = a[row][col] / a[col][col];
            for k in col..n {
                a[row][k] -= f * a[col][k];
            }
            b[row] -= f * b[col];
        }
    }
    let mut x = vec![0.; n];
    for row in (0..n).rev() {
        let tail: f64 = (row + 1..n).map(|k| a[row][k] * x[k]).sum();
        x[row] = (b[row] - tail) / a[row][row];
    }
    x.iter().all(|v| v.is_finite()).then_some(x)
}

/// Algebraic (Kasa) circle fit: returns (center, radius).
pub(super) fn fit_circle_2d(points: &[[f64; 2]]) -> Option<([f64; 2], f64)> {
    let n = points.len() as f64;
    let (mut sx, mut sy, mut sxx, mut syy, mut sxy, mut sxz, mut syz, mut sz) =
        (0., 0., 0., 0., 0., 0., 0., 0.);
    for p in points {
        let (x, y) = (p[0], p[1]);
        let z = x * x + y * y;
        sx += x;
        sy += y;
        sxx += x * x;
        syy += y * y;
        sxy += x * y;
        sxz += x * z;
        syz += y * z;
        sz += z;
    }
    let m = vec![
        vec![sxx, sxy, sx],
        vec![sxy, syy, sy],
        vec![sx, sy, n],
    ];
    let x = solve_small(&m, &[-sxz, -syz, -sz])?;
    let (a, b, c) = (x[0], x[1], x[2]);
    let center = [-a / 2., -b / 2.];
    // x²+y² + a·x + b·y + c = 0  →  r² = (a²+b²)/4 − c.
    let r2 = center[0] * center[0] + center[1] * center[1] - c;
    (r2 > 0.).then_some((center, r2.sqrt()))
}

/// Algebraic sphere fit over 3D points: returns (center, radius).
pub(super) fn fit_sphere(points: &[[f64; 3]]) -> Option<([f64; 3], f64)> {
    let n = points.len() as f64;
    let mut m = vec![vec![0.; 4]; 4];
    let mut rhs = vec![0.; 4];
    for p in points {
        let z = dot(*p, *p);
        let row = [p[0], p[1], p[2], 1.];
        for i in 0..4 {
            for j in 0..4 {
                m[i][j] += row[i] * row[j];
            }
            rhs[i] += row[i] * z;
        }
    }
    let _ = n;
    let x = solve_small(&m, &rhs)?;
    let center = [x[0] / 2., x[1] / 2., x[2] / 2.];
    let r2 = x[3] + dot(center, center);
    (r2 > 0.).then_some((center, r2.sqrt()))
}
