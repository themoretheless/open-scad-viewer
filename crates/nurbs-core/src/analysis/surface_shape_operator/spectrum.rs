//! Conservative absolute spectrum of a symmetric world curvature tensor.
use crate::{Result, distance_bounds::Interval as I};
fn abs_lower(x: I) -> f64 {
    if x.lo <= 0. && x.hi >= 0. {
        0.
    } else {
        x.lo.abs().min(x.hi.abs())
    }
}
fn abs_upper(x: I) -> f64 {
    x.lo.abs().max(x.hi.abs())
}
// A floating Jacobi frame is only a proposal. The actual bound below uses
// interval Q^T A Q and a certified positive lower bound for Q^T Q, never
// assumes floating rotations/eigenvalues to be exact or orthonormal.
pub fn absolute_spectral_bounds(t: [[[f64; 2]; 3]; 3]) -> Result<Option<[f64; 2]>> {
    if t.iter().flatten().flatten().any(|x| !x.is_finite()) {
        return Ok(None);
    }
    let mut a = [[I::point(0.); 3]; 3];
    let mut m = [[0.; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            let lo = t[i][j][0].max(t[j][i][0]);
            let hi = t[i][j][1].min(t[j][i][1]);
            if lo > hi {
                return Ok(None);
            }
            a[i][j] = I::new(lo, hi)?;
            m[i][j] = lo * 0.5 + hi * 0.5;
        }
    }
    let mut q = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    for _ in 0..24 {
        let (p, r) = [(0, 1), (0, 2), (1, 2)]
            .into_iter()
            .max_by(|&(i, j), &(k, l)| m[i][j].abs().total_cmp(&m[k][l].abs()))
            .unwrap();
        if !m[p][r].is_finite() || m[p][r] == 0. {
            break;
        }
        let angle = 0.5 * (2. * m[p][r]).atan2(m[r][r] - m[p][p]);
        let (s, c) = angle.sin_cos();
        if !s.is_finite() || !c.is_finite() {
            break;
        }
        for row in &mut m {
            let (x, y) = (row[p], row[r]);
            row[p] = c * x - s * y;
            row[r] = s * x + c * y;
        }
        for j in 0..3 {
            let (x, y) = (m[p][j], m[r][j]);
            m[p][j] = c * x - s * y;
            m[r][j] = s * x + c * y;
        }
        for row in &mut q {
            let (x, y) = (row[p], row[r]);
            row[p] = c * x - s * y;
            row[r] = s * x + c * y;
        }
    }
    let mut gram = [[I::point(0.); 3]; 3];
    let mut b = gram;
    for k in 0..3 {
        for l in 0..3 {
            for i in 0..3 {
                gram[k][l] = gram[k][l].add(I::point(q[i][k]).mul(I::point(q[i][l]))?)?;
                for j in 0..3 {
                    b[k][l] =
                        b[k][l].add(I::point(q[i][k]).mul(a[i][j])?.mul(I::point(q[j][l]))?)?;
                }
            }
        }
    }
    let mut gram_lower = f64::INFINITY;
    let mut row_upper = 0f64;
    let mut lower = 0f64;
    for i in 0..3 {
        let mut off = I::point(0.);
        let mut row = I::point(0.);
        for j in 0..3 {
            if j != i {
                off = off.add(I::point(abs_upper(gram[i][j])))?;
            }
            row = row.add(I::point(abs_upper(b[i][j])))?;
        }
        gram_lower = gram_lower.min(I::point(gram[i][i].lo).sub(off)?.lo);
        row_upper = row_upper.max(row.hi);
        if gram[i][i].lo <= 0. {
            return Ok(None);
        }
        lower = lower.max(abs_lower(b[i][i].div(gram[i][i])?));
    }
    if gram_lower <= 0. || !row_upper.is_finite() {
        return Ok(None);
    }
    let upper = I::point(row_upper).div(I::point(gram_lower))?.hi;
    Ok(upper.is_finite().then_some([lower, upper]))
}
