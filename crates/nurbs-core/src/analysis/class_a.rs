//! Численные метрики качества поверхности класса A (пункты 959–964).
//!
//! Всё read-only и чисто численное: highlight/reflection-линии марширующим
//! методом на (u,v)-сетке, изофотный анализ, зебра-валидация шва двух
//! поверхностей, пороговый класс-A валидатор и детекция вмятин по карте
//! гауссовой кривизны. Метрики — не сертификаты: выборка сеточная, выводы
//! справочные (для интервальных доказательств см. `surface_shape_operator`).
use crate::{
    Result, check, numeric, resource,
    foundation::guards::{Budget, require_finite_f64},
    surface::{Evaluation, Surface},
};
use math_core::{cross, dot, norm};

/// Бюджеты выборки: защита от чрезмерных сеток и сегментов.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Максимум ячеек сетки по одной параметрической оси.
    pub max_grid_axis: usize,
    /// Максимум извлечённых сегментов изолиний на один запрос.
    pub max_segments: usize,
    /// Максимум точек в сцепленной полилинии.
    pub max_polyline_points: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_grid_axis: 256,
            max_segments: 100_000,
            max_polyline_points: 20_000,
        }
    }
}

/// Тип дефекта качества поверхности.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefectKind {
    /// Излом/разрыв бликовой линии.
    HighlightKink,
    /// Скачок второй производной reflection-линии.
    ReflectionJump,
    /// Неравномерность шага изофот (индикатор вмятины).
    IsophoteUneven,
    /// Позиционный зазор на шве (G0 нарушен).
    SeamPositionGap,
    /// Угол между нормалями на шве выше порога (G1 нарушен).
    SeamNormalAngle,
    /// Относительный скачок кривизны поперёк шва (G2 нарушен).
    SeamCurvatureJump,
    /// Локальная вмятина/выпуклость по остатку гауссовой кривизны.
    Dent,
}

/// Одно нарушение/дефект с локализацией.
#[derive(Clone, Debug)]
pub struct Defect {
    pub kind: DefectKind,
    /// (u,v) на поверхности; для шва — (t, NaN), где t в [0,1] вдоль шва.
    pub uv: [f64; 2],
    /// Измеренная величина (рад, длина, относительный скачок...).
    pub value: f64,
    /// Порог, который превышен.
    pub threshold: f64,
}

/// Выборка одной изолинии: полилинии в (u,v) + дефекты непрерывности.
#[derive(Clone, Debug)]
pub struct IsoLineReport {
    pub polylines: Vec<Vec<[f64; 2]>>,
    pub defects: Vec<Defect>,
}

fn unit(v: [f64; 3]) -> Option<[f64; 3]> {
    let n = norm(v);
    (n > 1e-14).then(|| [v[0] / n, v[1] / n, v[2] / n])
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// Отражение направления света l от единичной нормали n: 2(n·l)n − l.
fn reflect(l: [f64; 3], n: [f64; 3]) -> [f64; 3] {
    let d = 2. * dot(n, l);
    [d * n[0] - l[0], d * n[1] - l[1], d * n[2] - l[2]]
}

/// Домен поверхности по узловым векторам (как `Curve::domain`).
fn surface_domains(s: &Surface) -> ([f64; 2], [f64; 2]) {
    (
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    )
}

/// Скалярное поле на равномерной (u,v)-сетке; NaN = выборка недоступна.
struct Field {
    nu: usize,
    nv: usize,
    inset: f64,
    domain_u: [f64; 2],
    domain_v: [f64; 2],
    values: Vec<f64>,
}





/// Метрика изломов полилиний: мировой угол между соседними звеньями
/// (точки вычисляются на поверхности) выше порога = разрыв/излом линии.
fn kink_defects(
    surface: &Surface,
    lines: &[Vec<[f64; 2]>],
    kind: DefectKind,
    kink_angle: f64,
) -> Vec<Defect> {
    let mut defects = Vec::new();
    for line in lines {
        for w in line.windows(3) {
            let (Ok(e0), Ok(e1), Ok(e2)) = (
                surface.evaluate(w[0][0], w[0][1]),
                surface.evaluate(w[1][0], w[1][1]),
                surface.evaluate(w[2][0], w[2][1]),
            ) else {
                continue;
            };
            let a = sub(e1.point, e0.point);
            let b = sub(e2.point, e1.point);
            // Мировой возврат (тройка не даёт чистого продвижения длиной
            // даже в одно звено) — артефакт сцепки марширующих квадратов
            // у вырождения параметризации (полюс), а не излом изолинии:
            // поворот изолинии гладкого поля на ячейке не превышает ~120°.
            let backtrack = norm(sub(e2.point, e0.point)) < norm(a).max(norm(b));
            if backtrack {
                continue;
            }
            let (Some(ua), Some(ub)) = (unit(a), unit(b)) else {
                continue;
            };
            let angle = dot(ua, ub).clamp(-1., 1.).acos();
            // Неконечный угол в отчёте — баг; такую тройку пропускаем.
            if !angle.is_finite() {
                continue;
            }
            if angle > kink_angle {
                defects.push(Defect {
                    kind,
                    uv: w[1],
                    value: angle,
                    threshold: kink_angle,
                });
            }
        }
    }
    defects
}

fn iso_lines(
    surface: &Surface,
    nu: usize,
    nv: usize,
    inset: f64,
    level: f64,
    kink_angle: f64,
    kind: DefectKind,
    f: impl Fn(&Evaluation) -> Option<f64>,
    limits: &Limits,
) -> Result<IsoLineReport> {
    require_finite_f64(level, "isoline level")?;
    require_finite_f64(kink_angle, "kink angle threshold")?;
    check(kink_angle > 0., "Kink angle threshold must be positive")?;
    let field = Field::sample(surface, nu, nv, inset, f, limits)?;
    let range = [
        field.domain_u[1] - field.domain_u[0],
        field.domain_v[1] - field.domain_v[0],
    ];
    // Допуск сцепки в масштабированном домене, переведённый обратно в (u,v):
    // половина диагонали ячейки по большей из осей.
    let cell = (range[0] / nu as f64).hypot(range[1] / nv as f64);
    let segments = field.marching_segments(level, limits)?;
    let polylines = chain_segments(segments, 0.51 * cell, limits)?;
    let defects = kink_defects(surface, &polylines, kind, kink_angle);
    Ok(IsoLineReport { polylines, defects })
}

/// Проверка направления света/взгляда.
fn direction(v: [f64; 3]) -> Result<[f64; 3]> {
    check(v.iter().all(|x| x.is_finite()), "Direction must be finite")?;
    let Some(u) = unit(v) else {
        return Err(crate::input("class_a direction vector is (near) zero"));
    };
    Ok(u)
}

/// (959) Highlight lines: гирлянда параллельных источников `lights`,
/// линии уровня `level` поля (отражённый луч · `view`).
/// Непрерывность измеряется изломами сцепленных полилиний.
pub fn highlight_lines(
    surface: &Surface,
    lights: &[[f64; 3]],
    view: [f64; 3],
    level: f64,
    grid: [usize; 2],
    kink_angle: f64,
    limits: &Limits,
) -> Result<Vec<IsoLineReport>> {
    check(!lights.is_empty(), "Highlight analysis needs at least one light")?;
    check(lights.len() <= 64, "Highlight garland is limited to 64 lights")?;
    let view = direction(view)?;
    let mut reports = Vec::with_capacity(lights.len());
    for &l in lights {
        let light = direction(l)?;
        reports.push(iso_lines(
            surface,
            grid[0],
            grid[1],
            0.02,
            level,
            kink_angle,
            DefectKind::HighlightKink,
            |ev| {
                let n = ev.unit_normal()?;
                Some(dot(reflect(light, n), view))
            },
            limits,
        )?);
    }
    Ok(reports)
}

/// (960) Reflection lines как функции параметра; скачки дискретной второй
/// производной (по длине дуги) локализуют дефектные зоны.
/// Возвращает (отчёт изолиний, дефекты скачка d²/ds²).
pub fn reflection_lines(
    surface: &Surface,
    light: [f64; 3],
    view: [f64; 3],
    level: f64,
    grid: [usize; 2],
    jump_threshold: f64,
    limits: &Limits,
) -> Result<(IsoLineReport, Vec<Defect>)> {
    check(jump_threshold > 0., "Reflection jump threshold must be positive")?;
    let light = direction(light)?;
    let view = direction(view)?;
    let report = iso_lines(
        surface,
        grid[0],
        grid[1],
        0.02,
        level,
        0.35,
        DefectKind::ReflectionJump,
        |ev| {
            let n = ev.unit_normal()?;
            Some(dot(reflect(light, n), view))
        },
        limits,
    )?;
    let (dom_u, dom_v) = surface_domains(surface);
    let scale = [dom_u[1] - dom_u[0], dom_v[1] - dom_v[0]];
    let mut jumps = Vec::new();
    for line in &report.polylines {
        // Дискретная кривизна: смена направления на единицу длины дуги.
        let mut curvature: Vec<([f64; 2], f64)> = Vec::new();
        for w in line.windows(3) {
            let a = [
                (w[1][0] - w[0][0]) * scale[0],
                (w[1][1] - w[0][1]) * scale[1],
            ];
            let b = [
                (w[2][0] - w[1][0]) * scale[0],
                (w[2][1] - w[1][1]) * scale[1],
            ];
            let (na, nb) = (a[0].hypot(a[1]), b[0].hypot(b[1]));
            if na < 1e-15 || nb < 1e-15 {
                continue;
            }
            let cos = ((a[0] * b[0] + a[1] * b[1]) / (na * nb)).clamp(-1., 1.);
            curvature.push((w[1], cos.acos() / (0.5 * (na + nb))));
        }
        for w in curvature.windows(2) {
            let jump = (w[1].1 - w[0].1).abs();
            if jump > jump_threshold {
                jumps.push(Defect {
                    kind: DefectKind::ReflectionJump,
                    uv: w[1].0,
                    value: jump,
                    threshold: jump_threshold,
                });
            }
        }
    }
    Ok((report, jumps))
}

/// (961) Изофотный анализ: изолинии n·L = cos θ для набора углов.
#[derive(Clone, Debug)]
pub struct IsophoteReport {
    /// Полилинии каждого уровня cos θ (порядок как во `cos_levels`).
    pub levels: Vec<IsoLineReport>,
    /// Средний шаг между соседними изофотами (по выборке ближайших точек).
    pub mean_spacing: f64,
    /// Амплитуда неравномерности шага: max|d_i − mean| / mean (0 = равномерно).
    pub spacing_unevenness: f64,
    /// Зоны, где локальный шаг отклоняется от среднего выше `uneven_tol`.
    pub defects: Vec<Defect>,
}

pub fn isophote_analysis(
    surface: &Surface,
    light: [f64; 3],
    cos_levels: &[f64],
    grid: [usize; 2],
    uneven_tol: f64,
    limits: &Limits,
) -> Result<IsophoteReport> {
    check(cos_levels.len() >= 2, "Isophote analysis needs at least two levels")?;
    check(
        cos_levels.iter().all(|c| (-1. ..=1.).contains(c)),
        "Isophote levels are cos(theta) and must lie in [-1, 1]",
    )?;
    check(uneven_tol > 0., "Isophote unevenness tolerance must be positive")?;
    let light = direction(light)?;
    let mut levels = Vec::with_capacity(cos_levels.len());
    for &c in cos_levels {
        levels.push(iso_lines(
            surface,
            grid[0],
            grid[1],
            0.02,
            c,
            0.35,
            DefectKind::IsophoteUneven,
            |ev| Some(dot(ev.unit_normal()?, light)),
            limits,
        )?);
    }
    // Шаг между соседними уровнями: выборка ближайших расстояний в (u,v),
    // масштабированных в длины области (грубая метрика равномерности).
    let (du, dv) = surface_domains(surface);
    let scale = [du[1] - du[0], dv[1] - dv[0]];
    let dist = |a: [f64; 2], b: [f64; 2]| {
        ((a[0] - b[0]) * scale[0]).hypot((a[1] - b[1]) * scale[1])
    };
    let stride = (grid[0].max(grid[1]) / 16).max(1);
    let mut samples: Vec<(f64, [f64; 2])> = Vec::new();
    for pair in levels.windows(2) {
        let (la, lb) = (&pair[0], &pair[1]);
        let points_b: Vec<[f64; 2]> = lb.polylines.iter().flatten().copied().collect();
        if points_b.is_empty() {
            continue;
        }
        for line in &la.polylines {
            for (k, p) in line.iter().enumerate() {
                if k % stride != 0 {
                    continue;
                }
                let mut best = f64::INFINITY;
                for &q in &points_b {
                    best = best.min(dist(*p, q));
                }
                if best.is_finite() && best > 0. {
                    samples.push((best, *p));
                }
            }
        }
    }
    if samples.is_empty() {
        return Ok(IsophoteReport {
            levels,
            mean_spacing: 0.,
            spacing_unevenness: 0.,
            defects: Vec::new(),
        });
    }
    let mean = samples.iter().map(|s| s.0).sum::<f64>() / samples.len() as f64;
    numeric(mean > 0., "Isophote spacing degenerated to zero")?;
    let mut unevenness: f64 = 0.;
    let mut defects = Vec::new();
    for &(d, uv) in &samples {
        let rel = (d - mean).abs() / mean;
        unevenness = unevenness.max(rel);
        if rel > uneven_tol {
            defects.push(Defect {
                kind: DefectKind::IsophoteUneven,
                uv,
                value: rel,
                threshold: uneven_tol,
            });
        }
    }
    Ok(IsophoteReport {
        levels,
        mean_spacing: mean,
        spacing_unevenness: unevenness,
        defects,
    })
}

mod seams;
pub use seams::{SeamEdge,SeamSample,SeamReport,ClassATolerances,SurfaceClass,SeamContinuity,classify_seam,ClassAReport,seam_zebra,validate_seam_class};


/// (964) Детекция вмятин/выпуклостей по карте гауссовой кривизны.
#[derive(Clone, Debug)]
pub struct DentZone {
    /// (u,v) максимума остатка в зоне.
    pub uv: [f64; 2],
    /// Максимальная амплитуда остатка |K − K_smooth| в зоне.
    pub amplitude: f64,
}

#[derive(Clone, Debug)]
pub struct DentReport {
    pub zones: Vec<DentZone>,
    /// Максимальный остаток по всей сетке.
    pub max_residual: f64,
}

pub fn dent_detection(
    surface: &Surface,
    grid: [usize; 2],
    sigma_cells: f64, // радиус гауссового сглаживания в ячейках
    threshold: f64,   // порог остатка гауссовой кривизны
    limits: &Limits,
) -> Result<DentReport> {
    check(sigma_cells > 0., "Dent smoothing sigma must be positive")?;
    check(threshold >= 0., "Dent residual threshold must be non-negative")?;
    let field = Field::sample(
        surface,
        grid[0],
        grid[1],
        0.02,
        |ev| ev.curvatures().map(|(k, _)| k),
        limits,
    )?;
    // Сепарабельное гауссово сглаживание; NaN-выборки исключаются из окна.
    let radius = (3. * sigma_cells).ceil() as usize;
    let weights: Vec<f64> = (0..=radius)
        .map(|k| (-0.5 * (k as f64 / sigma_cells).powi(2)).exp())
        .collect();
    let (nu, nv) = (field.nu, field.nv);
    let smooth_pass = |src: &[f64], horizontal: bool| -> Vec<f64> {
        let mut dst = vec![f64::NAN; src.len()];
        for j in 0..=nv {
            for i in 0..=nu {
                let mut sum = 0.;
                let mut wsum = 0.;
                for k in 0..=radius {
                    for &sign in &[1i32, -1] {
                        if sign < 0 && k == 0 {
                            continue;
                        }
                        let (ii, jj) = if horizontal {
                            (i as i32 + sign * k as i32, j as i32)
                        } else {
                            (i as i32, j as i32 + sign * k as i32)
                        };
                        if ii < 0 || jj < 0 || ii > nu as i32 || jj > nv as i32 {
                            continue;
                        }
                        let x = src[jj as usize * (nu + 1) + ii as usize];
                        if x.is_finite() {
                            sum += weights[k] * x;
                            wsum += weights[k];
                        }
                    }
                }
                if wsum > 0. {
                    dst[j * (nu + 1) + i] = sum / wsum;
                }
            }
        }
        dst
    };
    let tmp = smooth_pass(&field.values, true);
    let smooth = smooth_pass(&tmp, false);
    let mut residual = vec![f64::NAN; field.values.len()];
    let mut max_residual: f64 = 0.;
    for idx in 0..residual.len() {
        if field.values[idx].is_finite() && smooth[idx].is_finite() {
            let r = (field.values[idx] - smooth[idx]).abs();
            residual[idx] = r;
            max_residual = max_residual.max(r);
        }
    }
    // Зоны: связные кластеры ячеек с остатком выше порога (4-связность).
    let mut visited = vec![false; residual.len()];
    let mut zones = Vec::new();
    for j in 0..=nv {
        for i in 0..=nu {
            let start = j * (nu + 1) + i;
            if visited[start] || !residual[start].is_finite() || residual[start] <= threshold {
                continue;
            }
            let mut stack = vec![(i, j)];
            let mut best = (residual[start], (i, j));
            visited[start] = true;
            while let Some((ci, cj)) = stack.pop() {
                let r = residual[cj * (nu + 1) + ci];
                if r > best.0 {
                    best = (r, (ci, cj));
                }
                for (di, dj) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                    let (ni, nj) = (ci as i32 + di, cj as i32 + dj);
                    if ni < 0 || nj < 0 || ni > nu as i32 || nj > nv as i32 {
                        continue;
                    }
                    let nidx = nj as usize * (nu + 1) + ni as usize;
                    if !visited[nidx] && residual[nidx].is_finite() && residual[nidx] > threshold {
                        visited[nidx] = true;
                        stack.push((ni as usize, nj as usize));
                    }
                }
            }
            zones.push(DentZone {
                uv: field.uv(
                    best.1 .0 as f64 / nu as f64,
                    best.1 .1 as f64 / nv as f64,
                ),
                amplitude: best.0,
            });
        }
    }
    zones.sort_by(|a, b| b.amplitude.total_cmp(&a.amplitude));
    Ok(DentReport {
        zones,
        max_residual,
    })
}

#[cfg(test)]
#[path = "tests/class_a.rs"]
mod tests;


/// Evaluates zebra / isophote reflection stripe intensity in $[0, 1]$ across a $(u, v)$ grid
/// (`grid_u` rows by `grid_v` columns) for Class-A surface smoothness inspection.
pub fn zebra_reflection_stripes(
    surface: &Surface,
    view_dir: [f64; 3],
    light_dir: [f64; 3],
    stripe_frequency: f64,
    grid_u: usize,
    grid_v: usize,
) -> Vec<Vec<f64>> {
    if grid_u == 0
        || grid_v == 0
        || !stripe_frequency.is_finite()
        || stripe_frequency <= 0.0
        || surface.validate().is_err()
    {
        return Vec::new();
    }
    let (Some(v_dir), Some(l_dir)) = (unit(view_dir), unit(light_dir)) else {
        return Vec::new();
    };
    let (du, dv) = surface_domains(surface);
    let span_u = du[1] - du[0];
    let span_v = dv[1] - dv[0];
    let pi_freq = std::f64::consts::PI * stripe_frequency;
    let mut v_vals = Vec::with_capacity(grid_v);
    for j in 0..grid_v {
        let tv = if grid_v == 1 {
            0.5
        } else {
            j as f64 / (grid_v - 1) as f64
        };
        v_vals.push(dv[0] + span_v * tv);
    }
    let mut grid = Vec::with_capacity(grid_u);
    for i in 0..grid_u {
        let tu = if grid_u == 1 {
            0.5
        } else {
            i as f64 / (grid_u - 1) as f64
        };
        let u = du[0] + span_u * tu;
        let mut row = Vec::with_capacity(grid_v);
        for &v in &v_vals {
            let intensity = surface
                .evaluate_validated(u, v)
                .ok()
                .and_then(|ev| {
                    let n = ev.unit_normal()?;
                    let r = reflect(v_dir, n);
                    let cos_theta = dot(r, l_dir).clamp(-1.0, 1.0);
                    Some(0.5 * (1.0 + (pi_freq * cos_theta).cos()))
                })
                .unwrap_or(0.0);
            row.push(intensity);
        }
        grid.push(row);
    }
    grid
}

/// Evaluates Gaussian curvature $K = \kappa_1 \kappa_2$ and mean curvature $H = (\kappa_1 + \kappa_2) / 2$
/// across a $(u, v)$ grid (`grid_u` rows by `grid_v` columns).
pub fn gaussian_and_mean_curvature_grid(
    surface: &Surface,
    grid_u: usize,
    grid_v: usize,
) -> Vec<Vec<(f64, f64)>> {
    if grid_u == 0 || grid_v == 0 || surface.validate().is_err() {
        return Vec::new();
    }
    let (du, dv) = surface_domains(surface);
    let span_u = du[1] - du[0];
    let span_v = dv[1] - dv[0];
    let eps_u = 1e-6 * span_u;
    let eps_v = 1e-6 * span_v;
    let inset = 1e-4;
    let eval_kh = |u: f64, v: f64| -> Option<(f64, f64)> {
        surface.evaluate_validated(u, v).ok()?.curvatures()
    };
    let mut v_vals = Vec::with_capacity(grid_v);
    for j in 0..grid_v {
        let tv = if grid_v == 1 {
            0.5
        } else {
            inset + (1.0 - 2.0 * inset) * (j as f64 / (grid_v - 1) as f64)
        };
        v_vals.push(dv[0] + span_v * tv);
    }
    let mut grid = Vec::with_capacity(grid_u);
    for i in 0..grid_u {
        let tu = if grid_u == 1 {
            0.5
        } else {
            inset + (1.0 - 2.0 * inset) * (i as f64 / (grid_u - 1) as f64)
        };
        let u = du[0] + span_u * tu;
        let mut row = Vec::with_capacity(grid_v);
        for &v in &v_vals {
            let kh = eval_kh(u, v)
                .or_else(|| {
                    let u_nudged = (u + eps_u).clamp(du[0] + eps_u, du[1] - eps_u);
                    let v_nudged = (v + eps_v).clamp(dv[0] + eps_v, dv[1] - eps_v);
                    eval_kh(u_nudged, v)
                        .or_else(|| eval_kh(u, v_nudged))
                        .or_else(|| eval_kh(u_nudged, v_nudged))
                })
                .unwrap_or((0.0, 0.0));
            row.push(kh);
        }
        grid.push(row);
    }
    grid
}

#[path="class_a/methods_Field_sample.rs"]
mod methods_Field_sample;
#[path="class_a/algorithm_chain_segments.rs"]
mod algorithm_chain_segments;
use algorithm_chain_segments::chain_segments;
