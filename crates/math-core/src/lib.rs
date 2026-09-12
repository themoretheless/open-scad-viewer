//! Small dense linear algebra shared by the geometry and photogrammetry cores. No external runtime.
pub type V2 = [f64; 2];
pub type V3 = [f64; 3];
pub type M3 = [[f64; 3]; 3];
pub const ID: M3 = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];

/// Compute placement. `Cpu` is the deterministic reference; `Gpu` is opt-in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Acceleration {
    #[default]
    Cpu,
    Gpu,
}

/// Shared geometry error. Codes stay crate-specific; the type is one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub code: &'static str,
    pub message: String,
}
impl Error {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
    pub fn contains(&self, needle: &str) -> bool {
        self.message.contains(needle)
    }
    /// Shared precondition check. Codes stay crate-specific at the call site.
    #[inline]
    pub fn ensure(ok: bool, code: &'static str, message: impl Into<String>) -> Result<()> {
        if ok {
            Ok(())
        } else {
            Err(Self::new(code, message))
        }
    }
}
/// Same as [`Error::ensure`]; free function for crate `check` wrappers.
#[inline]
pub fn ensure(ok: bool, code: &'static str, message: impl Into<String>) -> Result<()> {
    Error::ensure(ok, code, message)
}
impl From<Error> for String {
    fn from(error: Error) -> Self {
        error.message
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;

/// Leaf helpers on the geometry/photogrammetry hotpath: force inlining so call
/// overhead and missed branch hints do not dominate tiny f64 kernels.
#[inline(always)]
pub fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
#[inline(always)]
pub fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
#[inline(always)]
pub fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
#[inline(always)]
pub fn scale(a: V3, s: f64) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}
#[inline(always)]
pub fn norm(a: V3) -> f64 {
    dot(a, a).sqrt()
}
#[inline(always)]
pub fn unit(a: V3) -> V3 {
    scale(a, 1. / norm(a).max(1e-15))
}
#[inline(always)]
pub fn finite(p: V3) -> bool {
    p.iter().all(|v| v.is_finite() && v.abs() <= 1e6)
}
#[inline(always)]
pub fn add2(a: V2, b: V2) -> V2 {
    [a[0] + b[0], a[1] + b[1]]
}
#[inline(always)]
pub fn sub2(a: V2, b: V2) -> V2 {
    [a[0] - b[0], a[1] - b[1]]
}
#[inline(always)]
pub fn scale2(a: V2, s: f64) -> V2 {
    [a[0] * s, a[1] * s]
}
#[inline(always)]
pub fn dot2(a: V2, b: V2) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
#[inline(always)]
pub fn cross2(a: V2, b: V2) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
#[inline(always)]
pub fn norm2(a: V2) -> f64 {
    a[0].hypot(a[1])
}
#[inline(always)]
pub fn unit2(a: V2) -> V2 {
    scale2(a, 1. / norm2(a).max(1e-15))
}
#[inline(always)]
pub fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
#[inline(always)]
pub fn mv(a: M3, b: V3) -> V3 {
    [dot(a[0], b), dot(a[1], b), dot(a[2], b)]
}
#[inline(always)]
pub fn tr(a: M3) -> M3 {
    [
        [a[0][0], a[1][0], a[2][0]],
        [a[0][1], a[1][1], a[2][1]],
        [a[0][2], a[1][2], a[2][2]],
    ]
}
#[inline(always)]
pub fn mm(a: M3, b: M3) -> M3 {
    let bt = tr(b);
    [
        [dot(a[0], bt[0]), dot(a[0], bt[1]), dot(a[0], bt[2])],
        [dot(a[1], bt[0]), dot(a[1], bt[1]), dot(a[1], bt[2])],
        [dot(a[2], bt[0]), dot(a[2], bt[1]), dot(a[2], bt[2])],
    ]
}
#[inline(always)]
pub fn det(a: M3) -> f64 {
    dot(a[0], cross(a[1], a[2]))
}
#[inline(always)]
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
/// Fixed stack storage avoids allocating for the bounded 9/12-dimensional systems.
pub fn eigen<const N: usize>(mut a: [[f64; N]; N]) -> ([f64; N], [[f64; N]; N]) {
    let n = N;
    let mut v = [[0.; N]; N];
    for i in 0..n {
        v[i][i] = 1.;
    }
    for _ in 0..(80 * n * n) {
        let mut p = 0;
        let mut q = 1;
        let mut largest = 0.;
        for i in 0..n {
            for j in i + 1..n {
                if a[i][j].abs() > largest {
                    largest = a[i][j].abs();
                    p = i;
                    q = j;
                }
            }
        }
        let diag = (0..n).map(|i| a[i][i].abs()).fold(0., f64::max);
        if largest < 1e-13 * diag.max(1e-30) {
            break;
        }
        let theta = 0.5 * (2. * a[p][q]).atan2(a[q][q] - a[p][p]);
        let (c, s) = (theta.cos(), theta.sin());
        let (app, aqq, apq) = (a[p][p], a[q][q], a[p][q]);
        for k in 0..n {
            if k != p && k != q {
                let (kp, kq) = (a[k][p], a[k][q]);
                a[k][p] = c * kp - s * kq;
                a[p][k] = a[k][p];
                a[k][q] = s * kp + c * kq;
                a[q][k] = a[k][q];
            }
        }
        a[p][p] = c * c * app - 2. * s * c * apq + s * s * aqq;
        a[q][q] = s * s * app + 2. * s * c * apq + c * c * aqq;
        a[p][q] = 0.;
        a[q][p] = 0.;
        for k in 0..n {
            let (kp, kq) = (v[k][p], v[k][q]);
            v[k][p] = c * kp - s * kq;
            v[k][q] = s * kp + c * kq;
        }
    }
    (std::array::from_fn(|i| a[i][i]), v)
}
/// Accumulate normal equations directly from fixed-width rows. No row matrix is
/// materialized; the order of products and additions matches the original solver.
pub fn smallest<const N: usize>(rows: impl IntoIterator<Item = [f64; N]>) -> [f64; N] {
    let mut a = [[0.; N]; N];
    for r in rows {
        for i in 0..N {
            for j in 0..N {
                a[i][j] += r[i] * r[j];
            }
        }
    }
    let (d, v) = eigen(a);
    let k = (0..N).min_by(|&i, &j| d[i].total_cmp(&d[j])).unwrap();
    std::array::from_fn(|i| v[i][k])
}
pub fn svd(a: M3) -> (M3, V3, M3) {
    let ata = mm(tr(a), a);
    let (d, v) = eigen(ata);
    let mut order = [0, 1, 2];
    order.sort_by(|&i, &j| d[j].total_cmp(&d[i]));
    let cols: [V3; 3] = order.map(|k| std::array::from_fn(|i| v[i][k]));
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
        let a = [[3., 1., 0.], [1., 2., 1.], [0., 1., 4.]];
        let (d, v) = eigen(a);
        for k in 0..3 {
            for i in 0..3 {
                let av = (0..3).map(|j| a[i][j] * v[j][k]).sum::<f64>();
                assert!((av - d[k] * v[i][k]).abs() < 1e-9);
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

#[cfg(test)]
mod hotpath_bench {
    use super::*;
    use std::time::Instant;

    #[test]
    fn leaf_math_throughput() {
        let mut acc = 0.0f64;
        let a = [1.1, -2.2, 3.3];
        let b = [0.4, 0.5, -0.6];
        let m = [[1.0, 0.2, 0.0], [0.1, 1.0, 0.3], [0.0, 0.1, 1.0]];
        // warmup
        for _ in 0..50_000 {
            let c = cross(a, b);
            acc += dot(unit(add(c, scale(b, 0.1))), mv(m, a)) + det(mm(m, rotation(a)));
        }
        let start = Instant::now();
        const N: u32 = 2_000_000;
        for _ in 0..N {
            let c = cross(a, b);
            acc += dot(unit(add(c, scale(b, 0.1))), mv(m, a)) + det(mm(m, rotation(a)));
        }
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        eprintln!("math-core hotpath: {ms:.3} ms for {N} iters, acc={acc}");
        assert!(acc.is_finite());
    }
}
