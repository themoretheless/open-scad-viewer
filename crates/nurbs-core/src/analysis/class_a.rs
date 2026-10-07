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

/// Граница патча для зебра-выборки шва.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeamEdge {
    UMin,
    UMax,
    VMin,
    VMax,
}

impl SeamEdge {
    /// (u,v) в долях домена для параметра шва t ∈ [0,1] и касательная d(u,v)/dt.
    fn point(&self, t: f64) -> ([f64; 2], [f64; 2]) {
        match self {
            SeamEdge::UMin => ([0., t], [0., 1.]),
            SeamEdge::UMax => ([1., t], [0., 1.]),
            SeamEdge::VMin => ([t, 0.], [1., 0.]),
            SeamEdge::VMax => ([t, 1.], [1., 0.]),
        }
    }
}

/// Одна выборка по обе стороны шва.
#[derive(Clone, Copy, Debug)]
pub struct SeamSample {
    pub t: f64,
    /// Позиционный зазор |p_a − p_b|.
    pub position_gap: f64,
    /// Угол между нормалями, рад.
    pub normal_angle: f64,
    /// Относительный скачок кривизны нормальных сечений поперёк шва.
    pub curvature_jump: f64,
}

/// (964) Зебра-выборка шва двух поверхностей по общей граничной кривой.
#[derive(Clone, Debug)]
pub struct SeamReport {
    pub samples: Vec<SeamSample>,
    pub max_position_gap: f64,
    pub max_normal_angle: f64,
    pub max_curvature_jump: f64,
}

/// Нормальная кривизна в мировом направлении d (единица, в касательной плоскости).
fn normal_curvature(ev: &Evaluation, d: [f64; 3]) -> Option<f64> {
    let (su, sv) = ev.first_derivatives()?;
    let (suu, suv, svv) = ev.second_derivatives()?;
    let n = ev.unit_normal()?;
    let (e, f, g) = (dot(su, su), dot(su, sv), dot(sv, sv));
    let det = e * g - f * f;
    if det <= 1e-28 {
        return None;
    }
    // Параметрическое направление: G·(du,dv) = (d·Su, d·Sv).
    let rhs_u = dot(d, su);
    let rhs_v = dot(d, sv);
    let du = (g * rhs_u - f * rhs_v) / det;
    let dv = (e * rhs_v - f * rhs_u) / det;
    let l = dot(n, suu);
    let m = dot(n, suv);
    let nn = dot(n, svv);
    let first = e * du * du + 2. * f * du * dv + g * dv * dv;
    if first <= 1e-28 {
        return None;
    }
    Some((l * du * du + 2. * m * du * dv + nn * dv * dv) / first)
}

pub fn seam_zebra(
    surface_a: &Surface,
    edge_a: SeamEdge,
    surface_b: &Surface,
    edge_b: SeamEdge,
    samples: usize,
    limits: &Limits,
) -> Result<SeamReport> {
    check(samples >= 2, "Seam zebra needs at least two samples")?;
    if samples > limits.max_polyline_points {
        return Err(resource("class_a seam sample count exceeds the budget"));
    }
    let eval_at = |s: &Surface, edge: SeamEdge, t: f64| -> Result<Evaluation> {
        let (frac, _) = edge.point(t);
        let (du, dv) = surface_domains(s);
        s.evaluate(
            du[0] + (du[1] - du[0]) * frac[0],
            dv[0] + (dv[1] - dv[0]) * frac[1],
        )
    };
    let mut report = SeamReport {
        samples: Vec::with_capacity(samples),
        max_position_gap: 0.,
        max_normal_angle: 0.,
        max_curvature_jump: 0.,
    };
    for k in 0..samples {
        let t = k as f64 / (samples - 1) as f64;
        let a = eval_at(surface_a, edge_a, t)?;
        let b = eval_at(surface_b, edge_b, t)?;
        let gap = norm(sub(a.point, b.point));
        let (Some(na), Some(nb)) = (a.unit_normal(), b.unit_normal()) else {
            return Err(crate::numeric_err(
                "class_a seam zebra hit a degenerate surface normal",
            ));
        };
        let angle = dot(na, nb).clamp(-1., 1.).acos();
        // Кривизна нормальных сечений поперёк шва: d ⟂ касательной шва.
        let (_, dir_a) = edge_a.point(t);
        let (_, dir_b) = edge_b.point(t);
        let tangent_a = a.first_derivatives().and_then(|(su, sv)| {
            unit([
                su[0] * dir_a[0] + sv[0] * dir_a[1],
                su[1] * dir_a[0] + sv[1] * dir_a[1],
                su[2] * dir_a[0] + sv[2] * dir_a[1],
            ])
        });
        let tangent_b = b.first_derivatives().and_then(|(su, sv)| {
            unit([
                su[0] * dir_b[0] + sv[0] * dir_b[1],
                su[1] * dir_b[0] + sv[1] * dir_b[1],
                su[2] * dir_b[0] + sv[2] * dir_b[1],
            ])
        });
        let jump = match tangent_a.zip(tangent_b) {
            Some((ta, tb)) => {
                let t_mid = unit([
                    0.5 * (ta[0] + tb[0]),
                    0.5 * (ta[1] + tb[1]),
                    0.5 * (ta[2] + tb[2]),
                ])
                .unwrap_or(ta);
                let da = unit(cross(na, t_mid));
                let db = unit(cross(nb, t_mid));
                match da.zip(db).and_then(|(da, db)| {
                    normal_curvature(&a, da).zip(normal_curvature(&b, db))
                }) {
                    Some((ka, kb)) => {
                        (ka - kb).abs() / ka.abs().max(kb.abs()).max(1e-12)
                    }
                    None => 0.,
                }
            }
            None => 0.,
        };
        report.max_position_gap = report.max_position_gap.max(gap);
        report.max_normal_angle = report.max_normal_angle.max(angle);
        report.max_curvature_jump = report.max_curvature_jump.max(jump);
        report.samples.push(SeamSample {
            t,
            position_gap: gap,
            normal_angle: angle,
            curvature_jump: jump,
        });
    }
    Ok(report)
}

/// (962) Конфиг допусков порогового класс-A валидатора.
#[derive(Clone, Copy, Debug)]
pub struct ClassATolerances {
    /// Класс A: позиционный зазор шва.
    pub a_position: f64,
    /// Класс A: угол между нормалями, рад.
    pub a_normal_angle: f64,
    /// Класс A: относительный скачок кривизны.
    pub a_curvature_jump: f64,
    /// Класс B (между A и C), те же величины.
    pub b_position: f64,
    pub b_normal_angle: f64,
    pub b_curvature_jump: f64,
}

impl Default for ClassATolerances {
    fn default() -> Self {
        Self {
            a_position: 0.01,
            a_normal_angle: 0.01,
            a_curvature_jump: 0.05,
            b_position: 0.05,
            b_normal_angle: 0.05,
            b_curvature_jump: 0.2,
        }
    }
}

/// Класс качества шва/грани.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SurfaceClass {
    /// Выше класса C (хуже B): не прошёл даже B-пороги.
    C = 0,
    B = 1,
    A = 2,
}

impl SurfaceClass {
    pub fn name(self) -> &'static str {
        match self {
            SurfaceClass::A => "A",
            SurfaceClass::B => "B",
            SurfaceClass::C => "C",
        }
    }
}

/// (962) Классификация шва G0/G1/G2 по конфигурируемым порогам.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeamContinuity {
    /// Даже G0 не достигнут.
    None,
    G0,
    G1,
    G2,
}

impl SeamContinuity {
    pub fn name(self) -> &'static str {
        match self {
            SeamContinuity::None => "none",
            SeamContinuity::G0 => "G0",
            SeamContinuity::G1 => "G1",
            SeamContinuity::G2 => "G2",
        }
    }
}

pub fn classify_seam(
    report: &SeamReport,
    position_tol: f64,
    normal_angle_tol: f64,
    curvature_jump_tol: f64,
) -> SeamContinuity {
    if report.max_position_gap > position_tol {
        SeamContinuity::None
    } else if report.max_normal_angle > normal_angle_tol {
        SeamContinuity::G0
    } else if report.max_curvature_jump > curvature_jump_tol {
        SeamContinuity::G1
    } else {
        SeamContinuity::G2
    }
}

/// (963) Результат пороговой валидации: класс + список нарушений.
#[derive(Clone, Debug)]
pub struct ClassAReport {
    pub class: SurfaceClass,
    pub continuity: SeamContinuity,
    pub violations: Vec<Defect>,
}

impl ClassAReport {
    /// Простая JSON-подобная сериализация без внешних зависимостей.
    pub fn to_json(&self) -> String {
        let mut out = String::with_capacity(128 + 96 * self.violations.len());
        out.push_str("{\"class\":\"");
        out.push_str(self.class.name());
        out.push_str("\",\"continuity\":\"");
        out.push_str(self.continuity.name());
        out.push_str("\",\"violations\":[");
        for (i, v) in self.violations.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            let kind = match v.kind {
                DefectKind::SeamPositionGap => "position_gap",
                DefectKind::SeamNormalAngle => "normal_angle",
                DefectKind::SeamCurvatureJump => "curvature_jump",
                DefectKind::HighlightKink => "highlight_kink",
                DefectKind::ReflectionJump => "reflection_jump",
                DefectKind::IsophoteUneven => "isophote_uneven",
                DefectKind::Dent => "dent",
            };
            out.push_str(&format!(
                "{{\"type\":\"{}\",\"t\":{},\"u\":{},\"v\":{},\"value\":{},\"threshold\":{}}}",
                kind,
                if v.uv[1].is_nan() {
                    format!("{}", v.uv[0])
                } else {
                    "null".to_string()
                },
                v.uv[0],
                v.uv[1],
                v.value,
                v.threshold,
            ));
        }
        out.push_str("]}");
        out
    }
}

/// (962/963) Пороговая класс-A валидация шва: класс A/B/C + нарушения.
pub fn validate_seam_class(
    report: &SeamReport,
    tolerances: &ClassATolerances,
) -> Result<ClassAReport> {
    check(
        tolerances.a_position >= 0.
            && tolerances.a_normal_angle >= 0.
            && tolerances.a_curvature_jump >= 0.
            && tolerances.b_position >= tolerances.a_position
            && tolerances.b_normal_angle >= tolerances.a_normal_angle
            && tolerances.b_curvature_jump >= tolerances.a_curvature_jump,
        "Class-A tolerances must be non-negative and B no tighter than A",
    )?;
    let mut violations = Vec::new();
    // Нарушения фиксируем относительно B-порогов (всё, что мешает классу B и выше),
    // а класс считаем отдельно; нарушение A-порога помечаем тем же типом.
    for s in &report.samples {
        let uv = [s.t, f64::NAN];
        if s.position_gap > tolerances.a_position {
            violations.push(Defect {
                kind: DefectKind::SeamPositionGap,
                uv,
                value: s.position_gap,
                threshold: tolerances.a_position,
            });
        }
        if s.normal_angle > tolerances.a_normal_angle {
            violations.push(Defect {
                kind: DefectKind::SeamNormalAngle,
                uv,
                value: s.normal_angle,
                threshold: tolerances.a_normal_angle,
            });
        }
        if s.curvature_jump > tolerances.a_curvature_jump {
            violations.push(Defect {
                kind: DefectKind::SeamCurvatureJump,
                uv,
                value: s.curvature_jump,
                threshold: tolerances.a_curvature_jump,
            });
        }
    }
    let class = if report.max_position_gap <= tolerances.a_position
        && report.max_normal_angle <= tolerances.a_normal_angle
        && report.max_curvature_jump <= tolerances.a_curvature_jump
    {
        SurfaceClass::A
    } else if report.max_position_gap <= tolerances.b_position
        && report.max_normal_angle <= tolerances.b_normal_angle
        && report.max_curvature_jump <= tolerances.b_curvature_jump
    {
        SurfaceClass::B
    } else {
        SurfaceClass::C
    };
    let continuity = classify_seam(
        report,
        tolerances.b_position,
        tolerances.b_normal_angle,
        tolerances.b_curvature_jump,
    );
    Ok(ClassAReport {
        class,
        continuity,
        violations,
    })
}

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
mod tests {
    use super::*;
    use crate::curve::Curve;
    use crate::surface::revolve;

    fn sphere() -> Surface {
        // Единичная сфера; полюса на концах профиля (сетка с inset их избегает).
        let profile = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 2., 2., 2.],
            control_points: vec![
                vec![0., 0., -1.],
                vec![1., 0., -1.],
                vec![1., 0., 0.],
                vec![1., 0., 1.],
                vec![0., 0., 1.],
            ],
            weights: vec![
                1.,
                std::f64::consts::FRAC_1_SQRT_2,
                1.,
                std::f64::consts::FRAC_1_SQRT_2,
                1.,
            ],
            periodic: false,
        };
        revolve(&profile, [0.; 3], [0., 0., 1.], 360.).unwrap()
    }

    /// Билинейный прямоугольник в плоскости z=0: (x,y) ∈ [0,1]×[0,1].
    fn flat_patch(origin: [f64; 3], ux: [f64; 3], uy: [f64; 3]) -> Surface {
        let p = |a: f64, b: f64| {
            vec![
                origin[0] + a * ux[0] + b * uy[0],
                origin[1] + a * ux[1] + b * uy[1],
                origin[2] + a * ux[2] + b * uy[2],
            ]
        };
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![p(0., 0.), p(0., 1.)],
                vec![p(1., 0.), p(1., 1.)],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }

    /// Пластина 5×5 (три спана на ось) со сдвинутой по z центральной
    /// контрольной точкой — дефект локализуется базисными функциями.
    fn dented_patch(dent: f64) -> Surface {
        let knots = vec![0., 0., 0., 4. / 3., 8. / 3., 4., 4., 4.];
        let mut cp = vec![];
        for i in 0..5 {
            let mut row = vec![];
            for j in 0..5 {
                let z = if i == 2 && j == 2 { dent } else { 0. };
                row.push(vec![i as f64, j as f64, z]);
            }
            cp.push(row);
        }
        Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: knots.clone(),
            knots_v: knots,
            control_points: cp,
            weights: vec![vec![1.; 5]; 5],
            periodic_u: false,
            periodic_v: false,
        }
    }

    fn limits() -> Limits {
        Limits::default()
    }

    #[test]
    fn sphere_highlight_lines_are_clean() {
        let s = sphere();
        let lights = vec![
            [1., 0., 1.],
            [0., 1., 1.],
            [-1., 0., 1.],
            [0., -1., 1.],
        ];
        let reports =
            highlight_lines(&s, &lights, [0., 0., 1.], 0.5, [64, 64], 0.35, &limits()).unwrap();
        assert_eq!(reports.len(), 4);
        for r in &reports {
            assert!(!r.polylines.is_empty(), "sphere should show highlight lines");
            assert!(r.defects.is_empty(), "sphere highlight defects: {:?}", r.defects);
        }
    }

    #[test]
    fn sphere_reflection_lines_have_no_jumps() {
        let s = sphere();
        let (report, jumps) =
            reflection_lines(&s, [1., 0., 1.], [0., 0., 1.], 0.5, [64, 64], 200., &limits())
                .unwrap();
        assert!(!report.polylines.is_empty());
        assert!(jumps.is_empty(), "sphere reflection jumps: {:?}", jumps);
    }

    #[test]
    fn sphere_isophotes_are_clean_and_evenly_ordered() {
        let s = sphere();
        // Уровни вблизи "экватора" освещения: там шаг почти равномерный.
        let report = isophote_analysis(
            &s,
            [0., 0., 1.],
            &[0.35, 0.45, 0.55],
            [48, 48],
            0.9,
            &limits(),
        )
        .unwrap();
        assert!(report.mean_spacing > 0.);
        assert!(
            report.spacing_unevenness < 0.9,
            "unevenness {}",
            report.spacing_unevenness
        );
        for level in &report.levels {
            assert!(level.defects.is_empty(), "sphere isophote kinks: {:?}", level.defects);
        }
    }

    #[test]
    fn dent_detector_finds_shifted_control_point_zone() {
        let flat = dented_patch(0.);
        let flat_report = dent_detection(&flat, [24, 24], 1.5, 1e-6, &limits()).unwrap();
        assert!(
            flat_report.max_residual < 1e-6,
            "flat plate must be dent-free, got {}",
            flat_report.max_residual
        );
        let dented = dented_patch(0.4);
        let report = dent_detection(&dented, [24, 24], 1.5, 0.02, &limits()).unwrap();
        assert!(!report.zones.is_empty(), "dent must be detected");
        let top = &report.zones[0];
        // Вмятина у контрольной точки (2,2) пластины [0,4]² (домен узлов).
        assert!(
            (top.uv[0] - 2.).abs() < 1. && (top.uv[1] - 2.).abs() < 1.,
            "dent zone {:?} should sit near the shifted control point",
            top
        );
    }

    /// Два полуцилиндра: A покрывает углы 0..180°, B — 180..360°.
    /// Общий шов — образующая при 180°; нормали совпадают (G1), кривизны тоже (G2).
    fn half_cylinders() -> (Surface, Surface) {
        let pa = Curve::from_polyline(vec![vec![1., 0., 0.], vec![1., 0., 1.]]).unwrap();
        let pb = Curve::from_polyline(vec![vec![-1., 0., 0.], vec![-1., 0., 1.]]).unwrap();
        (
            revolve(&pa, [0.; 3], [0., 0., 1.], 180.).unwrap(),
            revolve(&pb, [0.; 3], [0., 0., 1.], 180.).unwrap(),
        )
    }

    /// Ищем пару границ, чьи средние точки совпадают.
    fn matching_edges(a: &Surface, b: &Surface) -> (SeamEdge, SeamEdge) {
        let edges = [SeamEdge::UMin, SeamEdge::UMax, SeamEdge::VMin, SeamEdge::VMax];
        let mid = |s: &Surface, e: SeamEdge| {
            let (frac, _) = e.point(0.5);
            let (du, dv) = surface_domains(s);
            s.evaluate(
                du[0] + (du[1] - du[0]) * frac[0],
                dv[0] + (dv[1] - dv[0]) * frac[1],
            )
            .unwrap()
            .point
        };
        let mut best = (f64::INFINITY, edges[0], edges[0]);
        for ea in edges {
            for eb in edges {
                let d = norm(sub(mid(a, ea), mid(b, eb)));
                if d < best.0 {
                    best = (d, ea, eb);
                }
            }
        }
        assert!(best.0 < 1e-9, "no shared boundary found, gap {}", best.0);
        (best.1, best.2)
    }

    #[test]
    fn smooth_g1_seam_passes_thresholds() {
        let (a, b) = half_cylinders();
        let (ea, eb) = matching_edges(&a, &b);
        let report = seam_zebra(&a, ea, &b, eb, 17, &limits()).unwrap();
        assert!(report.max_position_gap < 1e-9, "gap {}", report.max_position_gap);
        assert!(
            report.max_normal_angle < 1e-6,
            "normal angle {}",
            report.max_normal_angle
        );
        let continuity = classify_seam(&report, 1e-6, 1e-3, 0.05);
        assert_eq!(continuity, SeamContinuity::G2);
        let validation = validate_seam_class(&report, &ClassATolerances::default()).unwrap();
        assert_eq!(validation.class, SurfaceClass::A);
        assert!(validation.violations.is_empty());
        let json = validation.to_json();
        assert!(json.starts_with("{\"class\":\"A\""));
        assert!(json.ends_with("]}"));
    }

    #[test]
    fn kinked_seam_is_g0_but_not_g1() {
        // Две полуплоскости с малым двугранным углом (~5.7°) вдоль y-оси.
        let angle = 0.1_f64;
        let a = flat_patch([0., 0., 0.], [1., 0., 0.], [0., 1., 0.]);
        let b = flat_patch(
            [1., 0., 0.],
            [angle.cos(), 0., angle.sin()],
            [0., 1., 0.],
        );
        let report = seam_zebra(&a, SeamEdge::UMax, &b, SeamEdge::UMin, 9, &limits()).unwrap();
        assert!(report.max_position_gap < 1e-12);
        assert!(
            (report.max_normal_angle - angle).abs() < 1e-9,
            "normal angle {} vs {}",
            report.max_normal_angle,
            angle
        );
        assert_eq!(
            classify_seam(&report, 1e-6, 1e-3, 0.05),
            SeamContinuity::G0
        );
        // B-порог по нормалям 0.05 рад < 0.1 рад → класс C.
        let validation = validate_seam_class(&report, &ClassATolerances::default()).unwrap();
        assert_eq!(validation.class, SurfaceClass::C);
        assert!(
            validation
                .violations
                .iter()
                .any(|v| v.kind == DefectKind::SeamNormalAngle)
        );
        let json = validation.to_json();
        assert!(json.contains("\"type\":\"normal_angle\""));
        assert!(json.contains("\"threshold\":0.01"));
    }

    #[test]
    fn budgets_reject_oversized_grids() {
        let s = sphere();
        let tight = Limits {
            max_grid_axis: 8,
            ..Limits::default()
        };
        let err = highlight_lines(&s, &[[0., 0., 1.]], [0., 0., 1.], 0.9, [64, 64], 0.35, &tight);
        assert!(err.is_err());
    }

    #[test]
    fn invalid_inputs_are_rejected() {
        let s = sphere();
        assert!(highlight_lines(&s, &[], [0., 0., 1.], 0.9, [8, 8], 0.35, &limits()).is_err());
        assert!(highlight_lines(&s, &[[0., 0., 0.]], [0., 0., 1.], 0.9, [8, 8], 0.35, &limits())
            .is_err());
        assert!(isophote_analysis(&s, [0., 0., 1.], &[2., 3.], [8, 8], 0.5, &limits()).is_err());
    }

    #[test]
    fn rejects_non_finite_level_and_kink_angle() {
        let s = sphere();
        // NaN-уровень изолинии.
        let err = highlight_lines(&s, &[[0., 0., 1.]], [0., 0., 1.], f64::NAN, [8, 8], 0.35, &limits())
            .unwrap_err();
        assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
        assert!(err.contains("isoline level"), "{err}");
        // Inf-порог излома.
        let err = highlight_lines(&s, &[[0., 0., 1.]], [0., 0., 1.], 0.5, [8, 8], f64::INFINITY, &limits())
            .unwrap_err();
        assert!(err.contains("kink angle"), "{err}");
    }

    #[test]
    fn marching_squares_budget_is_a_named_stage() {
        // Шахматное поле: каждая ячейка неоднозначна → 2 сегмента на ячейку.
        let (nu, nv) = (4, 4);
        let mut values = Vec::with_capacity((nu + 1) * (nv + 1));
        for j in 0..=nv {
            for i in 0..=nu {
                values.push(if (i + j) % 2 == 0 { 1. } else { -1. });
            }
        }
        let field = Field {
            nu,
            nv,
            inset: 0.,
            domain_u: [0., 1.],
            domain_v: [0., 1.],
            values,
        };
        let tight = Limits {
            max_segments: 4,
            ..Limits::default()
        };
        let err = field.marching_segments(0., &tight).unwrap_err();
        assert_eq!(err.code, crate::RESOURCE_LIMIT, "{err:?}");
        assert!(err.contains("class-a.marching-squares"), "{err}");
        // С полным бюджетом та же выборка проходит.
        let segments = field.marching_segments(0., &limits()).unwrap();
        assert_eq!(segments.len(), 2 * nu * nv);
    }

    #[test]
    fn chaining_budget_limits_polyline_growth() {
        // Цепочка из трёх коллинеарных сегментов при бюджете 2 точки.
        let segments = vec![
            ([0., 0.], [1., 0.]),
            ([1., 0.], [2., 0.]),
            ([2., 0.], [3., 0.]),
        ];
        let tight = Limits {
            max_polyline_points: 2,
            ..Limits::default()
        };
        let err = chain_segments(segments.clone(), 0.1, &tight).unwrap_err();
        assert_eq!(err.code, crate::RESOURCE_LIMIT, "{err:?}");
        let lines = chain_segments(segments, 0.1, &limits()).unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].len(), 4);
    }
}
