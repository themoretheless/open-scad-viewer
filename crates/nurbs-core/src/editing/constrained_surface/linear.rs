//! Minimum-energy equality corrections without squaring row conditioning.
//! Pivoted, twice-reorthogonalized modified Gram-Schmidt in the whitened
//! Cholesky coordinates. Numerical rank never substitutes for geometric checks.
use super::{Result, dot, error, norm};
struct Equation {
    row: Vec<f64>,
    target: f64,
    dual: Option<Vec<f64>>,
}
fn lower(l: &[Vec<f64>], b: &[f64]) -> Vec<f64> {
    let mut x = vec![0.; b.len()];
    for i in 0..b.len() {
        x[i] = (b[i] - (0..i).map(|j| l[i][j] * x[j]).sum::<f64>()) / l[i][i];
    }
    x
}
fn upper(l: &[Vec<f64>], b: &[f64]) -> Vec<f64> {
    let mut x = vec![0.; b.len()];
    for i in (0..b.len()).rev() {
        x[i] = (b[i] - (i + 1..b.len()).map(|j| l[j][i] * x[j]).sum::<f64>()) / l[i][i];
    }
    x
}
fn remove(row: &mut Equation, q: &Equation) {
    let coefficient = dot(&row.row, &q.row);
    for (a, b) in row.row.iter_mut().zip(&q.row) {
        *a -= coefficient * b;
    }
    row.target -= coefficient * q.target;
    if let (Some(a), Some(b)) = (&mut row.dual, &q.dual) {
        for (x, y) in a.iter_mut().zip(b) {
            *x -= coefficient * y;
        }
    }
}
pub(super) fn correction(
    l: &[Vec<f64>],
    rows: &[Vec<f64>],
    rhs: &[f64],
) -> Result<(Vec<f64>, usize)> {
    let (x, rank, _) = correction_impl(l, rows, rhs, 3, false)?;
    Ok((x, rank))
}
/// One dense Cartesian block, including coupling between world components.
pub(super) fn dense_correction(
    l: &[Vec<f64>],
    rows: &[Vec<f64>],
    rhs: &[f64],
) -> Result<(Vec<f64>, usize)> {
    let (x, rank, _) = correction_impl(l, rows, rhs, 1, false)?;
    Ok((x, rank))
}
pub(super) fn homogeneous_correction(
    l: &[Vec<f64>],
    rows: &[Vec<f64>],
    rhs: &[f64],
) -> Result<(Vec<f64>, usize, Vec<f64>)> {
    correction_impl(l, rows, rhs, 4, true)
}
fn correction_impl(
    l: &[Vec<f64>],
    rows: &[Vec<f64>],
    rhs: &[f64],
    components: usize,
    dual: bool,
) -> Result<(Vec<f64>, usize, Vec<f64>)> {
    let n = l.len();
    let mut pending = Vec::new();
    for (index, (row, &target)) in rows.iter().zip(rhs).enumerate() {
        let mut r = Vec::new();
        for axis in 0..components {
            r.extend(lower(l, &row[axis * n..(axis + 1) * n]));
        }
        let length = norm(&r);
        if !length.is_finite() || length <= 0. || !target.is_finite() {
            return Err(error("Constraint whitening is numerically unresolved"));
        }
        for value in &mut r {
            *value /= length;
        }
        pending.push(Equation {
            row: r,
            target: target / length,
            dual: dual.then(|| {
                let mut x = vec![0.; rows.len()];
                x[index] = 1. / length;
                x
            }),
        });
    }
    let mut basis = Vec::<Equation>::new();
    while !pending.is_empty() {
        let pivot = (0..pending.len())
            .max_by(|&a, &b| norm(&pending[a].row).total_cmp(&norm(&pending[b].row)))
            .unwrap();
        if norm(&pending[pivot].row) <= 1e-11 {
            break;
        }
        let mut next = pending.swap_remove(pivot);
        for _ in 0..2 {
            for q in &basis {
                remove(&mut next, q);
            }
        }
        let length = norm(&next.row);
        if !length.is_finite() {
            return Err(error("Constraint row orthogonalization overflow"));
        }
        if length <= 1e-11 {
            continue;
        }
        for x in &mut next.row {
            *x /= length;
        }
        next.target /= length;
        if let Some(x) = &mut next.dual {
            for a in x {
                *a /= length;
            }
        }
        if !next.target.is_finite() {
            return Err(error("Constraint target normalization overflow"));
        }
        for row in &mut pending {
            remove(row, &next);
        }
        basis.push(next);
        if basis.len() == components * n {
            break;
        }
    }
    // In the whitened objective, orthonormal equality rows fix independent
    // coordinates. Setting the orthogonal complement to zero is the unique
    // minimum-energy correction on the numerically resolved row space.
    let mut y = vec![0.; components * n];
    for q in &basis {
        for (v, &a) in y.iter_mut().zip(&q.row) {
            *v += a * q.target;
        }
    }
    let mut x = Vec::new();
    for axis in 0..components {
        x.extend(upper(l, &y[axis * n..(axis + 1) * n]));
    }
    if x.iter().any(|v| !v.is_finite()) {
        return Err(error("Constraint correction overflow"));
    }
    let mut multipliers = vec![0.; if dual { rows.len() } else { 0 }];
    for q in &basis {
        if let Some(a) = &q.dual {
            for (x, y) in multipliers.iter_mut().zip(a) {
                *x += q.target * y;
            }
        }
    }
    Ok((x, basis.len(), multipliers))
}
