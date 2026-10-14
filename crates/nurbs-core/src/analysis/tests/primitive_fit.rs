use super::*;

/// Детерминированный SplitMix64.
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// Приближённо нормальный шум (сумма 4 равномерных, Ирвин–Холл).
    fn gauss(&mut self) -> f64 {
        (0..4).map(|_| self.f64()).sum::<f64>() - 2.
    }
}

fn basis(axis: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    let a = unit(axis);
    let seed = if a[0].abs() < 0.9 { [1., 0., 0.] } else { [0., 1., 0.] };
    let u = unit([
        a[1] * seed[2] - a[2] * seed[1],
        a[2] * seed[0] - a[0] * seed[2],
        a[0] * seed[1] - a[1] * seed[0],
    ]);
    let v = [
        a[1] * u[2] - a[2] * u[1],
        a[2] * u[0] - a[0] * u[2],
        a[0] * u[1] - a[1] * u[0],
    ];
    (u, v)
}

fn sphere_points(center: [f64; 3], radius: f64, n: usize, noise: f64, seed: u64) -> Vec<[f64; 3]> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|_| {
            let u = 2. * rng.f64() - 1.;
            let phi = 2. * std::f64::consts::PI * rng.f64();
            let s = (1. - u * u).sqrt();
            let r = radius + noise * rng.gauss();
            [
                center[0] + r * s * phi.cos(),
                center[1] + r * s * phi.sin(),
                center[2] + r * u,
            ]
        })
        .collect()
}

fn cylinder_points(
    q: [f64; 3],
    axis: [f64; 3],
    radius: f64,
    height: f64,
    n: usize,
    noise: f64,
    seed: u64,
) -> Vec<[f64; 3]> {
    let mut rng = Rng::new(seed);
    let (u, v) = basis(axis);
    (0..n)
        .map(|_| {
            let phi = 2. * std::f64::consts::PI * rng.f64();
            let t = (rng.f64() - 0.5) * height;
            let r = radius + noise * rng.gauss();
            [
                q[0] + r * (u[0] * phi.cos() + v[0] * phi.sin()) + axis[0] * t,
                q[1] + r * (u[1] * phi.cos() + v[1] * phi.sin()) + axis[1] * t,
                q[2] + r * (u[2] * phi.cos() + v[2] * phi.sin()) + axis[2] * t,
            ]
        })
        .collect()
}

fn cone_points(
    vertex: [f64; 3],
    axis: [f64; 3],
    half_angle: f64,
    height: f64,
    n: usize,
    noise: f64,
    seed: u64,
) -> Vec<[f64; 3]> {
    let mut rng = Rng::new(seed);
    let (u, v) = basis(axis);
    let (sa, ca) = half_angle.sin_cos();
    (0..n)
        .map(|_| {
            // Длина образующей от вершины; минимальный срез, чтобы не
            // вырождаться в точку.
            let l = 0.3 * height / ca + rng.f64() * height / ca;
            let phi = 2. * std::f64::consts::PI * rng.f64();
            let radial = [
                u[0] * phi.cos() + v[0] * phi.sin(),
                u[1] * phi.cos() + v[1] * phi.sin(),
                u[2] * phi.cos() + v[2] * phi.sin(),
            ];
            let mut p = [0.; 3];
            for k in 0..3 {
                p[k] = vertex[k] + l * (ca * axis[k] + sa * radial[k]);
            }
            let g = noise * rng.gauss();
            for k in 0..3 {
                p[k] += g * radial[k];
            }
            p
        })
        .collect()
}

fn torus_points(
    center: [f64; 3],
    axis: [f64; 3],
    big: f64,
    small: f64,
    n: usize,
    noise: f64,
    seed: u64,
) -> Vec<[f64; 3]> {
    let mut rng = Rng::new(seed);
    let (u, v) = basis(axis);
    (0..n)
        .map(|_| {
            let phi = 2. * std::f64::consts::PI * rng.f64();
            let psi = 2. * std::f64::consts::PI * rng.f64();
            let r = small + noise * rng.gauss();
            let ring = big + r * psi.cos();
            [
                center[0] + ring * u[0] * phi.cos() + ring * v[0] * phi.sin()
                    + r * psi.sin() * axis[0],
                center[1] + ring * u[1] * phi.cos() + ring * v[1] * phi.sin()
                    + r * psi.sin() * axis[1],
                center[2] + ring * u[2] * phi.cos() + ring * v[2] * phi.sin()
                    + r * psi.sin() * axis[2],
            ]
        })
        .collect()
}

#[test]
fn fits_exact_sphere() {
    let c = [1., -2., 3.5];
    let pts = sphere_points(c, 2.5, 400, 0., 7);
    let fit = fit_sphere(&pts).unwrap();
    for k in 0..3 {
        assert!((fit.center[k] - c[k]).abs() < 1e-6, "center[{k}] = {}", fit.center[k]);
    }
    assert!((fit.radius - 2.5).abs() < 1e-6);
    assert!(fit.report.rms < 1e-8);
    assert!(fit.report.converged);
}

#[test]
fn fits_noisy_sphere() {
    let c = [0.5, 1., -1.];
    let pts = sphere_points(c, 3., 2000, 0.01, 11);
    let fit = fit_sphere(&pts).unwrap();
    for k in 0..3 {
        assert!((fit.center[k] - c[k]).abs() < 0.02, "center[{k}] = {}", fit.center[k]);
    }
    assert!((fit.radius - 3.).abs() < 0.02);
    assert!(fit.report.rms < 0.03);
}

#[test]
fn fits_noisy_cylinder() {
    let q = [0.3, -0.7, 1.1];
    let a = unit([0.2, 0.4, 1.]);
    let pts = cylinder_points(q, a, 1.5, 6., 2000, 0.01, 13);
    let fit = fit_cylinder(&pts).unwrap();
    assert!(dot(fit.direction, a).abs() > 0.9999, "dir = {:?}", fit.direction);
    assert!((fit.radius - 1.5).abs() < 0.02, "r = {}", fit.radius);
    // Точка оси близка к истинной оси.
    let w = sub(fit.point_on_axis, q);
    let t = dot(w, a);
    let axial = [a[0] * t, a[1] * t, a[2] * t];
    assert!(norm(sub(w, axial)) < 0.02);
    assert!(fit.report.rms < 0.03);
}

#[test]
fn fits_noisy_cone() {
    let v = [0., 0., 0.];
    let a = unit([0., 0., 1.]);
    let alpha = 0.35;
    let pts = cone_points(v, a, alpha, 3., 2500, 0.005, 17);
    let fit = fit_cone(&pts).unwrap();
    assert!(dot(fit.axis, a) > 0.999, "axis = {:?}", fit.axis);
    assert!((fit.half_angle - alpha).abs() < 0.01, "alpha = {}", fit.half_angle);
    assert!(norm(sub(fit.vertex, v)) < 0.05, "vertex = {:?}", fit.vertex);
    assert!(fit.report.rms < 0.02);
}

#[test]
fn fits_noisy_torus() {
    let c = [0.5, -0.5, 2.];
    let a = unit([0., 0.3, 1.]);
    let pts = torus_points(c, a, 2., 0.5, 2500, 0.005, 19);
    let fit = fit_torus(&pts).unwrap();
    assert!(dot(fit.axis, a).abs() > 0.999, "axis = {:?}", fit.axis);
    assert!((fit.major_radius - 2.).abs() < 0.02, "R = {}", fit.major_radius);
    assert!((fit.minor_radius - 0.5).abs() < 0.02, "r = {}", fit.minor_radius);
    for k in 0..3 {
        assert!((fit.center[k] - c[k]).abs() < 0.02, "center[{k}] = {}", fit.center[k]);
    }
    assert!(fit.report.rms < 0.02);
}

#[test]
fn rejects_degenerate_input() {
    // Меньше минимума.
    assert!(fit_sphere(&[[0.; 3], [1., 0., 0.], [0., 1., 0.]]).is_err());
    // Все точки совпадают.
    let same = vec![[1., 2., 3.]; 20];
    assert!(fit_sphere(&same).is_err());
    assert!(fit_cylinder(&same).is_err());
    assert!(fit_cone(&same).is_err());
    assert!(fit_torus(&same).is_err());
    // Неконечные координаты.
    let mut bad = sphere_points([0.; 3], 1., 10, 0., 3);
    bad[0][0] = f64::NAN;
    assert!(fit_sphere(&bad).is_err());
}

#[test]
fn rejects_nan_point_with_param_name() {
    let mut bad = sphere_points([0.; 3], 1., 10, 0., 3);
    bad[4][2] = f64::INFINITY;
    let err = fit_sphere(&bad).unwrap_err();
    assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
    assert!(err.contains("points"), "{err}");
    // То же на всех fit'ах.
    assert!(fit_cylinder(&bad).is_err());
    assert!(fit_cone(&bad).is_err());
    assert!(fit_torus(&bad).is_err());
}

#[test]
fn solver_guard_carries_named_stage_and_budget() {
    // Guard вокруг solve-раунда: истощение — типизированный resource().
    let options = solver_options();
    let mut guard = solver_guard("primitive-fit.sphere-lm", &options).unwrap();
    guard.tick().unwrap();
    guard.check().unwrap();
    let tiny = solver_guard("primitive-fit.cone-lm", &SolverOptions {
        max_iterations: 1,
        ..solver_options()
    })
    .unwrap();
    let mut tiny = tiny;
    tiny.tick().unwrap();
    let err = tiny.tick().unwrap_err();
    assert_eq!(err.code, crate::RESOURCE_LIMIT, "{err:?}");
    assert!(err.contains("primitive-fit.cone-lm"), "{err}");
}

#[test]
fn fit_reports_carry_finite_metrics() {
    let pts = sphere_points([1., 2., 3.], 2., 300, 0.005, 41);
    let fit = fit_sphere(&pts).unwrap();
    assert!(fit.report.rms.is_finite());
    assert!(fit.report.max_dev.is_finite());
    assert!(fit.center.iter().all(|v| v.is_finite()));
    assert!(fit.radius.is_finite());
}

#[test]
fn rejects_complanar_sphere_points() {
    // Компланарные точки: нормальные уравнения Пратта сингулярны.
    let mut rng = Rng::new(23);
    let pts: Vec<[f64; 3]> = (0..50)
        .map(|_| [rng.f64() * 4. - 2., rng.f64() * 4. - 2., 1.])
        .collect();
    assert!(fit_sphere(&pts).is_err());
}
