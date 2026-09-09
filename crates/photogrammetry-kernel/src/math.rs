//! Small dense linear algebra for camera estimation. No external runtime.
pub type V3 = [f64; 3];
pub type M3 = [[f64; 3]; 3];
pub const ID: M3 = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
pub fn dot(a: V3, b: V3) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
pub fn add(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i] + b[i])
}
pub fn sub(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i] - b[i])
}
pub fn scale(a: V3, s: f64) -> V3 {
    a.map(|x| x * s)
}
pub fn norm(a: V3) -> f64 {
    dot(a, a).sqrt()
}
pub fn unit(a: V3) -> V3 {
    scale(a, 1. / norm(a).max(1e-15))
}
pub fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub fn mv(a: M3, b: V3) -> V3 {
    a.map(|r| dot(r, b))
}
pub fn tr(a: M3) -> M3 {
    std::array::from_fn(|i| std::array::from_fn(|j| a[j][i]))
}
pub fn mm(a: M3, b: M3) -> M3 {
    let bt = tr(b);
    std::array::from_fn(|i| std::array::from_fn(|j| dot(a[i], bt[j])))
}
pub fn det(a: M3) -> f64 {
    dot(a[0], cross(a[1], a[2]))
}
pub fn rotation(v: V3) -> M3 {
    let angle = norm(v);
    if angle < 1e-14 {
        return ID;
    }
    let [x, y, z] = scale(v, 1. / angle);
    let c = angle.cos();
    let s = angle.sin();
    let t = 1. - c;
    [
        [t * x * x + c, t * x * y - s * z, t * x * z + s * y],
        [t * x * y + s * z, t * y * y + c, t * y * z - s * x],
        [t * x * z - s * y, t * y * z + s * x, t * z * z + c],
    ]
}
/// Jacobi eigensystem of a real symmetric matrix, eigenvectors are columns.
pub fn eigen(mut a: Vec<f64>, n: usize) -> (Vec<f64>, Vec<f64>) {
    let mut v = vec![0.; n * n];
    for i in 0..n {
        v[i * n + i] = 1.;
    }
    for _ in 0..(80 * n * n) {
        let mut p = 0;
        let mut q = 1;
        let mut largest = 0.;
        for i in 0..n {
            for j in i + 1..n {
                if a[i * n + j].abs() > largest {
                    largest = a[i * n + j].abs();
                    p = i;
                    q = j;
                }
            }
        }
        let diag = (0..n).map(|i| a[i * n + i].abs()).fold(0., f64::max);
        if largest < 1e-13 * diag.max(1e-30) {
            break;
        }
        let theta = 0.5 * (2. * a[p * n + q]).atan2(a[q * n + q] - a[p * n + p]);
        let (c, s) = (theta.cos(), theta.sin());
        let (app, aqq, apq) = (a[p * n + p], a[q * n + q], a[p * n + q]);
        for k in 0..n {
            if k != p && k != q {
                let (kp, kq) = (a[k * n + p], a[k * n + q]);
                a[k * n + p] = c * kp - s * kq;
                a[p * n + k] = a[k * n + p];
                a[k * n + q] = s * kp + c * kq;
                a[q * n + k] = a[k * n + q];
            }
        }
        a[p * n + p] = c * c * app - 2. * s * c * apq + s * s * aqq;
        a[q * n + q] = s * s * app + 2. * s * c * apq + c * c * aqq;
        a[p * n + q] = 0.;
        a[q * n + p] = 0.;
        for k in 0..n {
            let (kp, kq) = (v[k * n + p], v[k * n + q]);
            v[k * n + p] = c * kp - s * kq;
            v[k * n + q] = s * kp + c * kq;
        }
    }
    ((0..n).map(|i| a[i * n + i]).collect(), v)
}
/// Accumulate normal equations directly from fixed-width rows. No row matrix is
/// materialized; the order of products and additions matches the original solver.
pub fn smallest<const N: usize>(rows: impl IntoIterator<Item = [f64; N]>) -> [f64; N] {
    let mut a = vec![0.; N * N];
    for r in rows {
        for i in 0..N {
            for j in 0..N {
                a[i * N + j] += r[i] * r[j];
            }
        }
    }
    let (d, v) = eigen(a, N);
    let k = (0..N).min_by(|&i, &j| d[i].total_cmp(&d[j])).unwrap();
    std::array::from_fn(|i| v[i * N + k])
}
pub fn svd(a: M3) -> (M3, V3, M3) {
    let ata = mm(tr(a), a);
    let (d, v) = eigen(ata.into_iter().flatten().collect(), 3);
    let mut order = [0, 1, 2];
    order.sort_by(|&i, &j| d[j].total_cmp(&d[i]));
    let cols: [V3; 3] = order.map(|k| std::array::from_fn(|i| v[i * 3 + k]));
    let s = order.map(|k| d[k].max(0.).sqrt());
    let u0 = unit(mv(a, cols[0]));
    let x = mv(a, cols[1]);
    let u1 = unit(sub(x, scale(u0, dot(x, u0))));
    let u2 = cross(u0, u1);
    (tr([u0, u1, u2]), s, tr(cols))
}
/// Pivoted elimination for the bounded 3/6/8-dimensional camera systems.
/// Fixed stack storage avoids allocating every row of each tiny system.
pub fn solve<const N: usize>(mut a: [[f64; N]; N], mut b: [f64; N]) -> Option<[f64; N]> {
    let n = N;
    for k in 0..n {
        let p = (k..n).max_by(|&i, &j| a[i][k].abs().total_cmp(&a[j][k].abs()))?;
        if a[p][k].abs() < 1e-14 {
            return None;
        }
        a.swap(k, p);
        b.swap(k, p);
        let pivot = a[k][k];
        for j in k..n {
            a[k][j] /= pivot;
        }
        b[k] /= pivot;
        for i in 0..n {
            if i == k {
                continue;
            }
            let f = a[i][k];
            for j in k..n {
                a[i][j] -= f * a[k][j];
            }
            b[i] -= f * b[k];
        }
    }
    Some(b)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn eigen_reconstructs() {
        let a = vec![3., 1., 0., 1., 2., 1., 0., 1., 4.];
        let (d, v) = eigen(a.clone(), 3);
        for k in 0..3 {
            for i in 0..3 {
                let av = (0..3).map(|j| a[i * 3 + j] * v[j * 3 + k]).sum::<f64>();
                assert!((av - d[k] * v[i * 3 + k]).abs() < 1e-9);
            }
        }
    }
    #[test]
    fn svd_rank_two() {
        let a = [[0., -1., 2.], [1., 0., -3.], [-2., 3., 0.]];
        let (u, s, v) = svd(a);
        let us: M3 = std::array::from_fn(|i| std::array::from_fn(|j| u[i][j] * s[j]));
        let b = mm(us, tr(v));
        for i in 0..3 {
            for j in 0..3 {
                assert!((a[i][j] - b[i][j]).abs() < 1e-6);
            }
        }
    }
}
