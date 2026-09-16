//! Small dense linear algebra shared by the geometry and photogrammetry cores. No external runtime.
pub type V2 = [f64; 2];
pub type V3 = [f64; 3];
pub type M3 = [[f64; 3]; 3];
pub const ID: M3 = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];

/// Compute placement. `Cpu` is the deterministic reference; the others are
/// opt-in and fall back to the CPU reference when unavailable.
///
/// - `Gpu`: portable compute shaders (wgpu — Vulkan/DX12/Metal natively,
///   WebGPU in the browser). Reaches NVIDIA hardware through Vulkan/DX12.
/// - `Cuda`: the CUDA driver API on NVIDIA hardware (native `cuda` feature).
///   Kernels without a CUDA port run their `Gpu` shader instead, so `Cuda`
///   is never slower than `Gpu` on the same device; without a CUDA device
///   the `Gpu` path is tried, then the CPU reference.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Acceleration {
    #[default]
    Cpu,
    Gpu,
    Cuda,
}

impl Acceleration {
    /// True for every device placement (`Gpu` or `Cuda`); kernels that only
    /// have a portable shader use this instead of comparing against `Gpu`.
    #[inline]
    pub const fn is_gpu(self) -> bool {
        matches!(self, Self::Gpu | Self::Cuda)
    }

    /// Parses the CLI/environment spelling (`cpu`, `gpu`, `cuda`).
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "cpu" => Some(Self::Cpu),
            "gpu" | "wgpu" | "webgpu" => Some(Self::Gpu),
            "cuda" => Some(Self::Cuda),
            _ => None,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Gpu => "gpu",
            Self::Cuda => "cuda",
        }
    }
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
/// Batch `q = M*p + t` over a large point set. This is the CPU reference for
/// [`transform_points_accelerated`] and is exact (f64); the GPU/CUDA paths run
/// the same formula in f32 and only differ at that precision.
pub fn transform_points(points: &[V3], m: M3, t: V3) -> Vec<V3> {
    points.iter().map(|&p| add(mv(m, p), t)).collect()
}

/// WGSL source for the batch transform compute shader (feature `gpu`); the
/// `gpu` and `cuda` modules both target this exact formula.
pub const TRANSFORM_WGSL: &str = include_str!("transform.wgsl");

#[cfg(feature = "cuda")]
pub mod cuda;
#[cfg(feature = "gpu")]
pub mod gpu;

/// `transform_points` with an optional GPU/CUDA batch kernel. `Acceleration::Cuda`
/// runs the PTX port through the CUDA driver (feature `cuda`), then the wgpu
/// shader (feature `gpu`), then the CPU reference; anything unavailable or that
/// fails falls through to the next stage, so the CPU result is always returned.
pub fn transform_points_accelerated(
    points: &[V3],
    m: M3,
    t: V3,
    #[allow(unused_variables)] acceleration: Acceleration,
) -> Vec<V3> {
    #[cfg(feature = "gpu")]
    if acceleration.is_gpu() {
        #[cfg(feature = "cuda")]
        if acceleration == Acceleration::Cuda
            && let Some(values) = cuda::transform_points_cuda(points, m, t)
        {
            return values;
        }
        if let Some(values) = gpu::transform_points_gpu(points, m, t) {
            return values;
        }
    }
    transform_points(points, m, t)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn acceleration_parse_and_placement() {
        assert_eq!(Acceleration::parse("cpu"), Some(Acceleration::Cpu));
        assert_eq!(Acceleration::parse(" GPU "), Some(Acceleration::Gpu));
        assert_eq!(Acceleration::parse("webgpu"), Some(Acceleration::Gpu));
        assert_eq!(Acceleration::parse("cuda"), Some(Acceleration::Cuda));
        assert_eq!(Acceleration::parse("opencl"), None);
        assert!(!Acceleration::Cpu.is_gpu());
        assert!(Acceleration::Gpu.is_gpu());
        assert!(Acceleration::Cuda.is_gpu());
        for mode in [Acceleration::Cpu, Acceleration::Gpu, Acceleration::Cuda] {
            assert_eq!(Acceleration::parse(mode.label()), Some(mode));
        }
        assert_eq!(Acceleration::default(), Acceleration::Cpu);
    }
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
mod transform_tests {
    use super::*;
    #[test]
    fn transform_points_matches_scalar_mv_add() {
        let m = rotation([0.3, -0.2, 0.7]);
        let t = [1., -2., 0.5];
        let points: Vec<V3> = (0..37)
            .map(|i| {
                let f = i as f64;
                [f * 0.37 - 5., f * -0.11 + 2., f * 0.05]
            })
            .collect();
        let got = transform_points(&points, m, t);
        for (p, q) in points.iter().zip(&got) {
            let expected = add(mv(m, *p), t);
            for k in 0..3 {
                assert!((expected[k] - q[k]).abs() < 1e-12);
            }
        }
    }
    #[test]
    fn transform_points_accelerated_cpu_matches_reference() {
        let m = ID;
        let t = [1., 2., 3.];
        let points = vec![[0., 0., 0.], [1., 1., 1.], [-1., 2., -3.]];
        let got = transform_points_accelerated(&points, m, t, Acceleration::Cpu);
        let want = transform_points(&points, m, t);
        assert_eq!(got, want);
    }
    #[test]
    fn transform_points_accelerated_empty_input() {
        let got = transform_points_accelerated(&[], ID, [0., 0., 0.], Acceleration::Gpu);
        assert!(got.is_empty());
    }
    #[cfg(feature = "gpu")]
    #[test]
    fn transform_points_accelerated_gpu_dispatch_matches_cpu() {
        let m = rotation([-0.4, 0.2, 0.6]);
        let t = [3., -1., 0.25];
        let points: Vec<V3> = (0..64)
            .map(|i| {
                let f = i as f64;
                [f * 0.2 - 6., f * 0.05, (f * 0.11).cos() * 3.]
            })
            .collect();
        let got = transform_points_accelerated(&points, m, t, Acceleration::Gpu);
        let want = transform_points(&points, m, t);
        for (g, w) in got.iter().zip(&want) {
            for k in 0..3 {
                assert!((g[k] - w[k]).abs() < 5e-4, "{g:?} vs {w:?}");
            }
        }
    }
    #[cfg(feature = "cuda")]
    #[test]
    fn transform_points_accelerated_cuda_dispatch_matches_cpu() {
        let m = rotation([-0.4, 0.2, 0.6]);
        let t = [3., -1., 0.25];
        let points: Vec<V3> = (0..64)
            .map(|i| {
                let f = i as f64;
                [f * 0.2 - 6., f * 0.05, (f * 0.11).cos() * 3.]
            })
            .collect();
        let got = transform_points_accelerated(&points, m, t, Acceleration::Cuda);
        let want = transform_points(&points, m, t);
        for (g, w) in got.iter().zip(&want) {
            for k in 0..3 {
                assert!((g[k] - w[k]).abs() < 5e-4, "{g:?} vs {w:?}");
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

pub mod camera_gestures;
pub mod orbit_camera;
pub mod viewport;
