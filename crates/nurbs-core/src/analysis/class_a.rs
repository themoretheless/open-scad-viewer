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

impl Field {
    fn sample(
        surface: &Surface,
        nu: usize,
        nv: usize,
        inset: f64,
        f: impl Fn(&Evaluation) -> Option<f64>,
        limits: &Limits,
    ) -> Result<Self> {
        check(nu >= 2 && nv >= 2, "Grid needs at least 2 cells per axis")?;
        check(
            (0. ..0.5).contains(&inset),
            "Grid inset must lie in [0, 0.5)",
        )?;
        if nu > limits.max_grid_axis || nv > limits.max_grid_axis {
            return Err(resource("class_a grid axis exceeds the sampling budget"));
        }
        if nu.checked_mul(nv).map_or(true, |c| c > limits.max_segments) {
            return Err(resource("class_a grid cell count exceeds the sampling budget"));
        }
        let (du, dv) = surface_domains(surface);
        let mut values = Vec::with_capacity((nu + 1) * (nv + 1));
        for j in 0..=nv {
            let tv = inset + (1. - 2. * inset) * j as f64 / nv as f64;
            let v = dv[0] + (dv[1] - dv[0]) * tv;
            for i in 0..=nu {
                let tu = inset + (1. - 2. * inset) * i as f64 / nu as f64;
                let u = du[0] + (du[1] - du[0]) * tu;
                let value = match surface.evaluate(u, v) {
                    Ok(ev) => f(&ev).unwrap_or(f64::NAN),
                    Err(_) => f64::NAN,
                };
                values.push(value);
            }
        }
        Ok(Self {
            nu,
            nv,
            inset,
            domain_u: du,
            domain_v: dv,
            values,
        })
    }

    fn at(&self, i: usize, j: usize) -> f64 {
        self.values[j * (self.nu + 1) + i]
    }

    /// Доли ячеек (0..1 по сетке) → (u,v) с учётом inset-выборки.
    fn uv(&self, fu: f64, fv: f64) -> [f64; 2] {
        let map = |f: f64| self.inset + (1. - 2. * self.inset) * f;
        [
            self.domain_u[0] + (self.domain_u[1] - self.domain_u[0]) * map(fu),
            self.domain_v[0] + (self.domain_v[1] - self.domain_v[0]) * map(fv),
        ]
    }

    /// Марширующие квадраты: сегменты изолинии уровня `level` в (u,v).
    /// Число сегментов ограничено единым [`Budget`] поверх
    /// `limits.max_segments` (пункт 1065, стадия `class-a.marching-squares`).
    fn marching_segments(&self, level: f64, limits: &Limits) -> Result<Vec<([f64; 2], [f64; 2])>> {
        let mut guard = Budget::with_iterations(limits.max_segments)?
            .guard("class-a.marching-squares");
        let mut segments = Vec::new();
        for j in 0..self.nv {
            for i in 0..self.nu {
                let corner = [
                    self.at(i, j),
                    self.at(i + 1, j),
                    self.at(i + 1, j + 1),
                    self.at(i, j + 1),
                ];
                if corner.iter().any(|x| !x.is_finite()) {
                    continue;
                }
                // Узлы ячейки в долях области: 0=(i,j) 1=(i+1,j) 2=(i+1,j+1) 3=(i,j+1).
                let node = [
                    [i as f64 / self.nu as f64, j as f64 / self.nv as f64],
                    [(i + 1) as f64 / self.nu as f64, j as f64 / self.nv as f64],
                    [(i + 1) as f64 / self.nu as f64, (j + 1) as f64 / self.nv as f64],
                    [i as f64 / self.nu as f64, (j + 1) as f64 / self.nv as f64],
                ];
                let edges = [(0, 1), (1, 2), (2, 3), (3, 0)];
                let mut hits = Vec::with_capacity(4);
                for &(a, b) in &edges {
                    let (fa, fb) = (corner[a] - level, corner[b] - level);
                    // Строгая смена знака: переход ровно через узел сетки
                    // пропускаем в обеих соседних ячейках — иначе узел
                    // порождает дублирующиеся фрагменты изолинии.
                    if fa * fb < 0. {
                        let t = fa / (fa - fb);
                        hits.push(self.uv(
                            node[a][0] + t * (node[b][0] - node[a][0]),
                            node[a][1] + t * (node[b][1] - node[a][1]),
                        ));
                    }
                }
                if hits.len() == 2 {
                    guard.tick()?;
                    segments.push((hits[0], hits[1]));
                } else if hits.len() == 4 {
                    // Неоднозначная ячейка: сцепляем по парам соседних рёбер.
                    guard.tick()?;
                    segments.push((hits[0], hits[1]));
                    guard.tick()?;
                    segments.push((hits[2], hits[3]));
                }
            }
        }
        Ok(segments)
    }
}

/// Сцепка сегментов в полилинии по близости концов.
fn chain_segments(
    segments: Vec<([f64; 2], [f64; 2])>,
    tolerance: f64,
    limits: &Limits,
) -> Result<Vec<Vec<[f64; 2]>>> {
    let tol2 = tolerance * tolerance;
    let near = |a: [f64; 2], b: [f64; 2]| {
        let du = a[0] - b[0];
        let dv = a[1] - b[1];
        du * du + dv * dv <= tol2
    };
    let mut pool: Vec<[f64; 2]> = Vec::with_capacity(segments.len() * 2);
    let mut free: Vec<(usize, usize)> = Vec::with_capacity(segments.len());
    for (a, b) in segments {
        pool.push(a);
        pool.push(b);
        free.push((pool.len() - 2, pool.len() - 1));
    }
    let mut lines: Vec<Vec<[f64; 2]>> = Vec::new();
    // Единый бюджет точек сцепленной полилинии (стадия class-a.chaining).
    let mut guard = Budget::with_iterations(limits.max_polyline_points)?
        .guard("class-a.chaining");
    while let Some((ia, ib)) = free.pop() {
        let mut line = vec![pool[ia], pool[ib]];
        loop {
            if line.len() >= limits.max_polyline_points {
                return Err(resource("class_a polyline point budget exceeded"));
            }
            let mut grown = false;
            let mut k = 0;
            while k < free.len() {
                let (ca, cb) = free[k];
                let (head, tail) = (line[0], line[line.len() - 1]);
                // Кандидат (присоединяемая точка, идёт ли на хвост).
                let candidate: Option<(usize, bool)> = if near(pool[ca], tail) {
                    Some((cb, true))
                } else if near(pool[cb], tail) {
                    Some((ca, true))
                } else if near(pool[ca], head) {
                    Some((cb, false))
                } else if near(pool[cb], head) {
                    Some((ca, false))
                } else {
                    None
                };
                let Some((ipoint, at_tail)) = candidate else {
                    k += 1;
                    continue;
                };
                let p = pool[ipoint];
                // Отбрасываем мгновенный возврат назад (A-B-A): дубликат узла
                // марширующих квадратов, а не реальный излом на pi.
                let backtrack = if at_tail && line.len() >= 2 {
                    near(p, line[line.len() - 2])
                } else if !at_tail && line.len() >= 2 {
                    near(p, line[1])
                } else {
                    false
                };
                if backtrack {
                    free.swap_remove(k);
                    continue;
                }
                if at_tail {
                    guard.tick()?;
                    line.push(p);
                } else {
                    guard.tick()?;
                    line.insert(0, p);
                }
                free.swap_remove(k);
                grown = true;
            }
            if !grown {
                break;
            }
        }
        lines.push(line);
    }
    Ok(lines)
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
