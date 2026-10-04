//! МНК-fitting аналитических поверхностей (сфера, цилиндр, конус, тор) по
//! 3D-точкам.
//!
//! Каждый fit: алгебраическая/моментная начальная оценка + нелинейное
//! уточнение по геометрической дистанции через
//! [`numerics::robust_solvers`] (LM / trust-region). Метрики отклонений и
//! число итераций солвера возвращаются в [`FitReport`].
//!
//! Бюджет: ≤ 65536 точек на вызов. Вырождения (компланарность, недостаток
//! данных, неконечные значения) — ошибки `Result`, без паник и `unsafe`.

use crate::numerics::robust_solvers::{
    ClosureProblem, SolverOptions, levenberg_marquardt_solve, trust_region_solve,
};
use crate::{Result, check, numeric, numeric_err};
use math_core::{Acceleration, point_principal_axes};

/// Максимальное число точек на один fit.
pub const MAX_FIT_POINTS: usize = 65536;

/// Метрики отклонений и диагностика солвера для одного fit'а.
#[derive(Clone, Copy, Debug)]
pub struct FitReport {
    /// Среднеквадратичное отклонение точек от поверхности.
    pub rms: f64,
    /// Максимальное по модулю отклонение.
    pub max_dev: f64,
    /// Число итераций нелинейного солвера (0, если уточнение не запускалось).
    pub iterations: usize,
    /// Сошёлся ли солвер по допускам (false — бюджет итераций исчерпан).
    pub converged: bool,
}

/// Результат fit'а сферы.
#[derive(Clone, Copy, Debug)]
pub struct SphereFit {
    pub center: [f64; 3],
    pub radius: f64,
    pub report: FitReport,
}

/// Результат fit'а кругового цилиндра.
#[derive(Clone, Copy, Debug)]
pub struct CylinderFit {
    /// Точка на оси (ближайшая к центроиду).
    pub point_on_axis: [f64; 3],
    /// Единичное направление оси.
    pub direction: [f64; 3],
    pub radius: f64,
    pub report: FitReport,
}

/// Результат fit'а кругового конуса.
#[derive(Clone, Copy, Debug)]
pub struct ConeFit {
    pub vertex: [f64; 3],
    /// Единичная ось, направленная от вершины к раскрытию конуса.
    pub axis: [f64; 3],
    /// Половинный угол при вершине, радианы, в (0, π/2).
    pub half_angle: f64,
    pub report: FitReport,
}

/// Результат fit'а кругового тора.
#[derive(Clone, Copy, Debug)]
pub struct TorusFit {
    pub center: [f64; 3],
    /// Единичная ось симметрии.
    pub axis: [f64; 3],
    /// Большой радиус (центр трубки).
    pub major_radius: f64,
    /// Малый радиус трубки.
    pub minor_radius: f64,
    pub report: FitReport,
}

// ---------------------------------------------------------------------------
// Малая векторная арифметика (f64, [f64; 3]).
// ---------------------------------------------------------------------------

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

fn unit(a: [f64; 3]) -> [f64; 3] {
    let l = norm(a);
    [a[0] / l, a[1] / l, a[2] / l]
}

fn centroid(points: &[[f64; 3]]) -> [f64; 3] {
    let mut c = [0.; 3];
    for p in points {
        for k in 0..3 {
            c[k] += p[k] / points.len() as f64;
        }
    }
    c
}

/// Проверка входа: конечность, бюджет, минимальное число точек, ненулевой
/// размер облака (не все точки совпадают).
fn validate_points(points: &[[f64; 3]], min_count: usize) -> Result<()> {
    check(
        points.len() >= min_count && points.len() <= MAX_FIT_POINTS,
        "Point count is below the minimum or exceeds the 65536 budget",
    )?;
    check(
        points.iter().flatten().all(|v| v.is_finite()),
        "Point coordinates must be finite",
    )?;
    let c = centroid(points);
    let spread = points.iter().map(|p| norm(sub(*p, c))).fold(0., f64::max);
    numeric(spread > 1e-14, "Point cloud is degenerate (all points coincide)")
}

/// Метрики отклонений по финальным невязкам.
fn report_from_residuals(residuals: &[f64], iterations: usize, converged: bool) -> FitReport {
    let n = residuals.len() as f64;
    let rms = (residuals.iter().map(|r| r * r).sum::<f64>() / n).sqrt();
    let max_dev = residuals.iter().map(|r| r.abs()).fold(0., f64::max);
    FitReport { rms, max_dev, iterations, converged }
}

/// Решение квадратной системы Гауссом с частичным выбором ведущего.
fn solve_dense(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Result<Vec<f64>> {
    let n = b.len();
    for col in 0..n {
        let mut piv = col;
        for r in col + 1..n {
            if a[r][col].abs() > a[piv][col].abs() {
                piv = r;
            }
        }
        numeric(a[piv][col].abs() > 1e-300, "Normal equations are singular")?;
        if piv != col {
            a.swap(piv, col);
            b.swap(piv, col);
        }
        for r in col + 1..n {
            let f = a[r][col] / a[col][col];
            for c in col..n {
                a[r][c] -= f * a[col][c];
            }
            b[r] -= f * b[col];
        }
    }
    let mut x = vec![0.; n];
    for i in (0..n).rev() {
        let mut s = b[i];
        for c in i + 1..n {
            s -= a[i][c] * x[c];
        }
        x[i] = s / a[i][i];
    }
    numeric(
        x.iter().all(|v| v.is_finite()),
        "Normal-equation solve produced non-finite values",
    )?;
    Ok(x)
}

/// Конечно-разностный якобиан для колбэка невязок (центральные разности).
fn fd_jacobian(
    f: &dyn Fn(&[f64]) -> Result<Vec<f64>>,
    x: &[f64],
    base: &[f64],
) -> Result<Vec<Vec<f64>>> {
    let n = x.len();
    let m = base.len();
    let mut j = vec![vec![0.; n]; m];
    for c in 0..n {
        let h = 1e-7 * x[c].abs().max(1e-3);
        let mut xp = x.to_vec();
        let mut xm = x.to_vec();
        xp[c] += h;
        xm[c] -= h;
        let fp = f(&xp)?;
        let fm = f(&xm)?;
        for r in 0..m {
            j[r][c] = (fp[r] - fm[r]) / (2. * h);
        }
    }
    Ok(j)
}

fn solver_options() -> SolverOptions {
    SolverOptions {
        max_iterations: 100,
        step_epsilon: 1e-14,
        residual_epsilon: 1e-14,
        gradient_epsilon: 1e-12,
        ..SolverOptions::default()
    }
}

// ---------------------------------------------------------------------------
// Сфера
// ---------------------------------------------------------------------------

/// Fit сферы: алгебраическая оценка Пратта + LM-уточнение по геометрической
/// дистанции `|p − c| − r`.
pub fn fit_sphere(points: &[[f64; 3]]) -> Result<SphereFit> {
    validate_points(points, 4)?;
    // Пратт: минимум Σ(|p|² − 2c·p + s)², s = |c|² − r².
    let mut a = vec![vec![0.; 4]; 4];
    let mut b = vec![0.; 4];
    for p in points {
        let row = [-2. * p[0], -2. * p[1], -2. * p[2], 1.];
        let rhs = -(p[0] * p[0] + p[1] * p[1] + p[2] * p[2]);
        for i in 0..4 {
            b[i] += row[i] * rhs;
            for j in 0..4 {
                a[i][j] += row[i] * row[j];
            }
        }
    }
    let x = solve_dense(a, b)?;
    let c0 = [x[0], x[1], x[2]];
    let r2 = dot(c0, c0) - x[3];
    numeric(r2 > 0., "Degenerate sphere fit (non-positive radius squared)")?;
    let mut x0 = vec![c0[0], c0[1], c0[2], r2.sqrt()];

    let residual = move |x: &[f64]| -> Result<Vec<f64>> {
        let c = [x[0], x[1], x[2]];
        let r = x[3];
        numeric(r > 0. && r.is_finite(), "Sphere radius must stay positive")?;
        Ok(points.iter().map(|p| norm(sub(*p, c)) - r).collect())
    };
    let jacobian = move |x: &[f64]| -> Result<Vec<Vec<f64>>> {
        let c = [x[0], x[1], x[2]];
        let mut j = Vec::with_capacity(points.len());
        for p in points {
            let d = sub(*p, c);
            let l = norm(d);
            numeric(l > 1e-300, "Point coincides with sphere center")?;
            j.push(vec![-d[0] / l, -d[1] / l, -d[2] / l, -1.]);
        }
        Ok(j)
    };
    let problem = ClosureProblem { residual, jacobian };
    let (x, rep) = levenberg_marquardt_solve(&problem, &x0, solver_options())?;
    x0.clone_from(&x);
    let residuals = (problem.residual)(&x)?;
    let report = report_from_residuals(&residuals, rep.iterations, rep.converged);
    numeric(x[3] > 0., "Sphere radius must stay positive")?;
    Ok(SphereFit {
        center: [x0[0], x0[1], x0[2]],
        radius: x0[3],
        report,
    })
}

// ---------------------------------------------------------------------------
// Цилиндр
// ---------------------------------------------------------------------------

/// Fit кругового цилиндра. Начальная ось — собственный вектор ковариации с
/// наиболее постоянным радиальным расстоянием; уточнение trust-region по
/// геометрической дистанции. Параметры солвера: точка на оси (3), сырое
/// направление (3, нормируется в невязках), радиус (1).
pub fn fit_cylinder(points: &[[f64; 3]]) -> Result<CylinderFit> {
    validate_points(points, 5)?;
    let c0 = centroid(points);
    let axes = point_principal_axes(points, Acceleration::Cpu)
        .map_err(|e| numeric_err(format!("PCA failed: {e}")))?;
    // Выбираем ось по минимальному относительному разбросу радиальных
    // расстояний — устойчиво к длине цилиндрического пояса.
    let mut best_dir = axes.axes[0];
    let mut best_cv = f64::INFINITY;
    for cand in &axes.axes {
        let mut dists = Vec::with_capacity(points.len());
        for p in points {
            let w = sub(*p, c0);
            let t = dot(w, *cand);
            let axial = [cand[0] * t, cand[1] * t, cand[2] * t];
            dists.push(norm(sub(w, axial)));
        }
        let mean = dists.iter().sum::<f64>() / dists.len() as f64;
        if mean <= 1e-14 {
            continue;
        }
        let var = dists.iter().map(|d| (d - mean) * (d - mean)).sum::<f64>()
            / dists.len() as f64;
        let cv = var.sqrt() / mean;
        if cv < best_cv {
            best_cv = cv;
            best_dir = *cand;
        }
    }
    let mut r0 = 0.;
    for p in points {
        let w = sub(*p, c0);
        let t = dot(w, best_dir);
        let axial = [best_dir[0] * t, best_dir[1] * t, best_dir[2] * t];
        r0 += norm(sub(w, axial)) / points.len() as f64;
    }
    numeric(r0 > 1e-14, "Degenerate cylinder fit (zero radius)")?;

    let residual = move |x: &[f64]| -> Result<Vec<f64>> {
        let q = [x[0], x[1], x[2]];
        let d = [x[3], x[4], x[5]];
        let dl = norm(d);
        numeric(dl > 1e-12, "Cylinder direction must stay non-zero")?;
        let dn = [d[0] / dl, d[1] / dl, d[2] / dl];
        let r = x[6];
        numeric(r > 0., "Cylinder radius must stay positive")?;
        Ok(points
            .iter()
            .map(|p| {
                let w = sub(*p, q);
                let t = dot(w, dn);
                let axial = [dn[0] * t, dn[1] * t, dn[2] * t];
                norm(sub(w, axial)) - r
            })
            .collect())
    };
    let x0 = vec![c0[0], c0[1], c0[2], best_dir[0], best_dir[1], best_dir[2], r0];
    let jac = {
        let residual = residual.clone();
        move |x: &[f64]| -> Result<Vec<Vec<f64>>> {
            let base = residual(x)?;
            fd_jacobian(&residual, x, &base)
        }
    };
    let problem = ClosureProblem { residual, jacobian: jac };
    let (x, rep) = trust_region_solve(&problem, &x0, solver_options())?;
    let residuals = (problem.residual)(&x)?;
    let report = report_from_residuals(&residuals, rep.iterations, rep.converged);
    let q = [x[0], x[1], x[2]];
    let d = unit([x[3], x[4], x[5]]);
    // Приводим точку оси к ближайшей к центроиду.
    let t = dot(sub(c0, q), d);
    let point_on_axis = [q[0] + d[0] * t, q[1] + d[1] * t, q[2] + d[2] * t];
    Ok(CylinderFit { point_on_axis, direction: d, radius: x[6], report })
}

// ---------------------------------------------------------------------------
// Конус
// ---------------------------------------------------------------------------

/// Fit кругового конуса. Начальная ось — главный собственный вектор
/// ковариации, вершина — экстраполяция по проекциям, угол — регрессия
/// ρ/t; уточнение LM по знаковой геометрической дистанции
/// `(w·a)·sin α − |w − (w·a)a|·cos α`.
pub fn fit_cone(points: &[[f64; 3]]) -> Result<ConeFit> {
    validate_points(points, 6)?;
    let c0 = centroid(points);
    let axes = point_principal_axes(points, Acceleration::Cpu)
        .map_err(|e| numeric_err(format!("PCA failed: {e}")))?;
    let mut axis = axes.axes[0];
    // Вершина: ориентируем ось так, чтобы радиальное расстояние росло вдоль +a.
    let mut tmin = f64::INFINITY;
    let mut tmax = f64::NEG_INFINITY;
    for p in points {
        let t = dot(sub(*p, c0), axis);
        tmin = tmin.min(t);
        tmax = tmax.max(t);
    }
    let radial_at = |t_target: f64| -> f64 {
        let mut sum = 0.;
        let mut cnt = 0.;
        for p in points {
            let w = sub(*p, c0);
            let t = dot(w, axis);
            if (t - t_target).abs() < 0.2 * (tmax - tmin).max(1e-12) {
                let axial = [axis[0] * t, axis[1] * t, axis[2] * t];
                sum += norm(sub(w, axial));
                cnt += 1.;
            }
        }
        if cnt > 0. { sum / cnt } else { 0. }
    };
    if radial_at(tmin) > radial_at(tmax) {
        axis = [-axis[0], -axis[1], -axis[2]];
        std::mem::swap(&mut tmin, &mut tmax);
    }
    // Вершина: экстраполяция линейной регрессии ρ(t) на ρ = 0.
    let mut st = 0.;
    let mut sr = 0.;
    let mut stt = 0.;
    let mut str_ = 0.;
    let mut cnt = 0.;
    for p in points {
        let w = sub(*p, c0);
        let t = dot(w, axis);
        let axial = [axis[0] * t, axis[1] * t, axis[2] * t];
        let rho = norm(sub(w, axial));
        st += t;
        sr += rho;
        stt += t * t;
        str_ += t * rho;
        cnt += 1.;
    }
    let det = cnt * stt - st * st;
    numeric(det.abs() > 1e-300, "Degenerate cone initial estimate")?;
    let slope = (cnt * str_ - st * sr) / det;
    let intercept = (sr - slope * st) / cnt;
    numeric(slope > 1e-6, "Points do not widen along the axis (not a cone)")?;
    let t_vertex = -intercept / slope;
    let vertex0 = [
        c0[0] + axis[0] * t_vertex,
        c0[1] + axis[1] * t_vertex,
        c0[2] + axis[2] * t_vertex,
    ];
    let alpha0 = slope.atan().clamp(1e-3, std::f64::consts::FRAC_PI_2 - 1e-3);

    let residual = move |x: &[f64]| -> Result<Vec<f64>> {
        let v = [x[0], x[1], x[2]];
        let a = [x[3], x[4], x[5]];
        let al = norm(a);
        numeric(al > 1e-12, "Cone axis must stay non-zero")?;
        let an = [a[0] / al, a[1] / al, a[2] / al];
        let alpha = x[6];
        numeric(
            alpha > 1e-6 && alpha < std::f64::consts::FRAC_PI_2 - 1e-6,
            "Cone half-angle left the open interval (0, pi/2)",
        )?;
        let (sa, ca) = alpha.sin_cos();
        Ok(points
            .iter()
            .map(|p| {
                let w = sub(*p, v);
                let t = dot(w, an);
                let axial = [an[0] * t, an[1] * t, an[2] * t];
                let rho = norm(sub(w, axial));
                t * sa - rho * ca
            })
            .collect())
    };
    let x0 = vec![vertex0[0], vertex0[1], vertex0[2], axis[0], axis[1], axis[2], alpha0];
    let jac = {
        let residual = residual.clone();
        move |x: &[f64]| -> Result<Vec<Vec<f64>>> {
            let base = residual(x)?;
            fd_jacobian(&residual, x, &base)
        }
    };
    let problem = ClosureProblem { residual, jacobian: jac };
    let (x, rep) = levenberg_marquardt_solve(&problem, &x0, solver_options())?;
    let residuals = (problem.residual)(&x)?;
    let report = report_from_residuals(&residuals, rep.iterations, rep.converged);
    let an = unit([x[3], x[4], x[5]]);
    Ok(ConeFit {
        vertex: [x[0], x[1], x[2]],
        axis: an,
        half_angle: x[6],
        report,
    })
}

// ---------------------------------------------------------------------------
// Тор
// ---------------------------------------------------------------------------

/// Fit кругового тора. Ось — собственный вектор ковариации с наименьшей
/// дисперсией (ось симметрии), центр — центроид, R — средний планарный
/// радиус, r — среднее расстояние до окружности трубки; уточнение LM по
/// геометрической дистанции `sqrt((ρ − R)² + z²) − r`.
pub fn fit_torus(points: &[[f64; 3]]) -> Result<TorusFit> {
    validate_points(points, 6)?;
    let c0 = centroid(points);
    let axes = point_principal_axes(points, Acceleration::Cpu)
        .map_err(|e| numeric_err(format!("PCA failed: {e}")))?;
    let axis0 = axes.axes[2];
    let mut r_big = 0.;
    for p in points {
        let w = sub(*p, c0);
        let z = dot(w, axis0);
        let axial = [axis0[0] * z, axis0[1] * z, axis0[2] * z];
        let rho = norm(sub(w, axial));
        r_big += rho / points.len() as f64;
    }
    numeric(r_big > 1e-14, "Degenerate torus fit (zero major radius)")?;
    // r: среднее sqrt((ρ − R)² + z²).
    let mut r_small = 0.;
    for p in points {
        let w = sub(*p, c0);
        let z = dot(w, axis0);
        let axial = [axis0[0] * z, axis0[1] * z, axis0[2] * z];
        let rho = norm(sub(w, axial));
        r_small += ((rho - r_big).powi(2) + z * z).sqrt() / points.len() as f64;
    }
    numeric(r_small > 1e-14, "Degenerate torus fit (zero minor radius)")?;

    let residual = move |x: &[f64]| -> Result<Vec<f64>> {
        let c = [x[0], x[1], x[2]];
        let a = [x[3], x[4], x[5]];
        let al = norm(a);
        numeric(al > 1e-12, "Torus axis must stay non-zero")?;
        let an = [a[0] / al, a[1] / al, a[2] / al];
        let big = x[6];
        let small = x[7];
        numeric(big > 0. && small > 0., "Torus radii must stay positive")?;
        Ok(points
            .iter()
            .map(|p| {
                let w = sub(*p, c);
                let z = dot(w, an);
                let axial = [an[0] * z, an[1] * z, an[2] * z];
                let rho = norm(sub(w, axial));
                ((rho - big).powi(2) + z * z).sqrt() - small
            })
            .collect())
    };
    let x0 = vec![c0[0], c0[1], c0[2], axis0[0], axis0[1], axis0[2], r_big, r_small];
    let jac = {
        let residual = residual.clone();
        move |x: &[f64]| -> Result<Vec<Vec<f64>>> {
            let base = residual(x)?;
            fd_jacobian(&residual, x, &base)
        }
    };
    let problem = ClosureProblem { residual, jacobian: jac };
    let (x, rep) = levenberg_marquardt_solve(&problem, &x0, solver_options())?;
    let residuals = (problem.residual)(&x)?;
    let report = report_from_residuals(&residuals, rep.iterations, rep.converged);
    Ok(TorusFit {
        center: [x[0], x[1], x[2]],
        axis: unit([x[3], x[4], x[5]]),
        major_radius: x[6],
        minor_radius: x[7],
        report,
    })
}

#[cfg(test)]
mod tests {
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
    fn rejects_complanar_sphere_points() {
        // Компланарные точки: нормальные уравнения Пратта сингулярны.
        let mut rng = Rng::new(23);
        let pts: Vec<[f64; 3]> = (0..50)
            .map(|_| [rng.f64() * 4. - 2., rng.f64() * 4. - 2., 1.])
            .collect();
        assert!(fit_sphere(&pts).is_err());
    }
}
