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

use crate::foundation::guards::{Budget, require_finite_f64, require_finite_point};
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
    for p in points {
        require_finite_point(p, "points")?;
    }
    let c = centroid(points);
    let spread = points.iter().map(|p| norm(sub(*p, c))).fold(0., f64::max);
    numeric(spread > 1e-14, "Point cloud is degenerate (all points coincide)")
}

/// Метрики отклонений по финальным невязкам. Неконечная метрика в отчёте —
/// баг вычисления, отвергается как numeric-ошибка.
fn report_from_residuals(residuals: &[f64], iterations: usize, converged: bool) -> Result<FitReport> {
    let n = residuals.len() as f64;
    let rms = (residuals.iter().map(|r| r * r).sum::<f64>() / n).sqrt();
    let max_dev = residuals.iter().map(|r| r.abs()).fold(0., f64::max);
    require_finite_f64(rms, "fit report rms")?;
    require_finite_f64(max_dev, "fit report max_dev")?;
    Ok(FitReport { rms, max_dev, iterations, converged })
}

/// Единый бюджетный guard (пункт 1065) вокруг одного нелинейного solve:
/// лимит итераций солвера задаёт итерационную размерность бюджета, тик —
/// один solve-раунд, `check()` — страховка по wall-clock.
fn solver_guard(stage: &'static str, options: &SolverOptions) -> Result<crate::foundation::guards::BudgetGuard> {
    Ok(Budget::with_iterations(options.max_iterations.max(1))?.guard(stage))
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
    let options = solver_options();
    let mut guard = solver_guard("primitive-fit.sphere-lm", &options)?;
    guard.tick()?;
    let (x, rep) = levenberg_marquardt_solve(&problem, &x0, options)?;
    guard.check()?;
    x0.clone_from(&x);
    let residuals = (problem.residual)(&x)?;
    let report = report_from_residuals(&residuals, rep.iterations, rep.converged)?;
    numeric(x[3] > 0., "Sphere radius must stay positive")?;
    let center = [x0[0], x0[1], x0[2]];
    require_finite_point(&center, "sphere center")?;
    require_finite_f64(x0[3], "sphere radius")?;
    Ok(SphereFit {
        center,
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
    let options = solver_options();
    let mut guard = solver_guard("primitive-fit.cylinder-trust-region", &options)?;
    guard.tick()?;
    let (x, rep) = trust_region_solve(&problem, &x0, options)?;
    guard.check()?;
    let residuals = (problem.residual)(&x)?;
    let report = report_from_residuals(&residuals, rep.iterations, rep.converged)?;
    let q = [x[0], x[1], x[2]];
    let d = unit([x[3], x[4], x[5]]);
    // Приводим точку оси к ближайшей к центроиду.
    let t = dot(sub(c0, q), d);
    let point_on_axis = [q[0] + d[0] * t, q[1] + d[1] * t, q[2] + d[2] * t];
    require_finite_point(&point_on_axis, "cylinder point_on_axis")?;
    require_finite_point(&d, "cylinder direction")?;
    require_finite_f64(x[6], "cylinder radius")?;
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
    let options = solver_options();
    let mut guard = solver_guard("primitive-fit.cone-lm", &options)?;
    guard.tick()?;
    let (x, rep) = levenberg_marquardt_solve(&problem, &x0, options)?;
    guard.check()?;
    let residuals = (problem.residual)(&x)?;
    let report = report_from_residuals(&residuals, rep.iterations, rep.converged)?;
    let an = unit([x[3], x[4], x[5]]);
    let vertex = [x[0], x[1], x[2]];
    require_finite_point(&vertex, "cone vertex")?;
    require_finite_point(&an, "cone axis")?;
    require_finite_f64(x[6], "cone half_angle")?;
    Ok(ConeFit {
        vertex,
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
    let options = solver_options();
    let mut guard = solver_guard("primitive-fit.torus-lm", &options)?;
    guard.tick()?;
    let (x, rep) = levenberg_marquardt_solve(&problem, &x0, options)?;
    guard.check()?;
    let residuals = (problem.residual)(&x)?;
    let report = report_from_residuals(&residuals, rep.iterations, rep.converged)?;
    let center = [x[0], x[1], x[2]];
    let axis = unit([x[3], x[4], x[5]]);
    require_finite_point(&center, "torus center")?;
    require_finite_point(&axis, "torus axis")?;
    require_finite_f64(x[6], "torus major_radius")?;
    require_finite_f64(x[7], "torus minor_radius")?;
    Ok(TorusFit {
        center,
        axis,
        major_radius: x[6],
        minor_radius: x[7],
        report,
    })
}

#[cfg(test)]
#[path = "tests/primitive_fit.rs"]
mod tests;
