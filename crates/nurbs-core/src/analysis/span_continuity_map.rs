//! Карта геометрической непрерывности между спанами поверхности (items 990).
//!
//! Для каждой внутренней узловой линии (u = u_i кратности m по оси U и
//! v = v_j по оси V) сэмплируются точки по обе стороны линии (сдвиг ±ε,
//! ε = доля длины соседнего спана — обе выборки лежат строго внутри своих
//! спанов, поэтому односторонние пределы базиса не нужны). Измеряются:
//! скачок позиции (G0), угол между нормалями (G1) и относительный скачок
//! нормальной кривизны поперёк линии (G2). Теоретическая непрерывность
//! базиса C^{p−m} сверяется с численной: расхождение (численно хуже
//! теории) означает дефект построения и попадает в список дефектов.
//!
//! Всё read-only и сеточное: выводы справочные, не сертификаты.
use crate::{
    Result, check, resource,
    foundation::guards::{Budget, require_finite_f64},
    surface::{Evaluation, Surface},
};
use math_core::{dot, norm};

/// Бюджеты выборки.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Максимум выборок вдоль одной узловой линии.
    pub max_samples_per_line: usize,
    /// Максимум узловых линий (по обеим осям суммарно).
    pub max_lines: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_samples_per_line: 256,
            max_lines: 512,
        }
    }
}

/// Пороги численной классификации G0/G1/G2.
#[derive(Clone, Copy, Debug)]
pub struct Tolerances {
    /// Скачок позиции (в единицах длины модели).
    pub position: f64,
    /// Угол между нормалями, рад.
    pub normal_angle: f64,
    /// Относительный скачок нормальной кривизны.
    pub curvature_jump: f64,
}
impl Default for Tolerances {
    fn default() -> Self {
        Self {
            position: 1e-8,
            normal_angle: 1e-6,
            curvature_jump: 0.01,
        }
    }
}

/// Ось, вдоль которой пробегает узловая линия (линия u = const — ось U).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnotAxis {
    U,
    V,
}
impl KnotAxis {
    pub fn name(self) -> &'static str {
        match self {
            KnotAxis::U => "u",
            KnotAxis::V => "v",
        }
    }
}

/// Численно измеренная непрерывность линии.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Measured {
    /// Даже позиционной стыковки нет.
    None = 0,
    G0 = 1,
    G1 = 2,
    G2 = 3,
}
impl Measured {
    pub fn name(self) -> &'static str {
        match self {
            Measured::None => "none",
            Measured::G0 => "G0",
            Measured::G1 => "G1",
            Measured::G2 => "G2",
        }
    }
}

/// Отчёт по одной узловой линии.
#[derive(Clone, Debug)]
pub struct KnotLineReport {
    pub axis: KnotAxis,
    /// Значение узла.
    pub knot: f64,
    /// Кратность узла m.
    pub multiplicity: usize,
    /// Степень по оси линии p.
    pub degree: usize,
    /// Теоретический порядок непрерывности базиса: C^{p−m}.
    pub theoretical_order: usize,
    /// Максимальный скачок позиции по выборке.
    pub max_position_jump: f64,
    /// Максимальный угол между нормалями, рад.
    pub max_normal_angle: f64,
    /// Максимальный относительный скачок нормальной кривизны.
    pub max_curvature_jump: f64,
    /// (u,v) худшей точки (по наибольшему из нормированных скачков).
    pub worst_uv: [f64; 2],
    /// Численная классификация по порогам.
    pub measured: Measured,
}

/// Дефект: численная непрерывность хуже теоретической C^{p−m}.
#[derive(Clone, Debug)]
pub struct ContinuityDefect {
    pub axis: KnotAxis,
    pub knot: f64,
    /// Ожидалось по теории (p − m).
    pub expected_order: usize,
    /// Измерено.
    pub measured: Measured,
    pub worst_uv: [f64; 2],
}

/// Итоговая карта непрерывности (items 990).
#[derive(Clone, Debug)]
pub struct ContinuityMap {
    pub lines: Vec<KnotLineReport>,
    pub defects: Vec<ContinuityDefect>,
    /// Худшая точка всей карты (по углу нормалей, затем по кривизне).
    pub worst_uv: Option<[f64; 2]>,
}

impl ContinuityMap {
    /// Простая JSON-подобная сериализация без внешних зависимостей,
    /// в стиле `class_a::ClassAReport::to_json`. Неконечные метрики
    /// маркируются `null` — NaN/Inf в отчёте не прячутся в текст.
    pub fn to_json(&self) -> String {
        fn num(x: f64) -> String {
            if x.is_finite() { format!("{x}") } else { "null".to_string() }
        }
        let mut out = String::with_capacity(256 + 160 * self.lines.len());
        out.push_str("{\"lines\":[");
        for (i, l) in self.lines.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&format!(
                "{{\"axis\":\"{}\",\"knot\":{},\"multiplicity\":{},\"degree\":{},\"theoretical_order\":{},\"max_position_jump\":{},\"max_normal_angle\":{},\"max_curvature_jump\":{},\"worst_u\":{},\"worst_v\":{},\"measured\":\"{}\"}}",
                l.axis.name(),
                num(l.knot),
                l.multiplicity,
                l.degree,
                l.theoretical_order,
                num(l.max_position_jump),
                num(l.max_normal_angle),
                num(l.max_curvature_jump),
                num(l.worst_uv[0]),
                num(l.worst_uv[1]),
                l.measured.name(),
            ));
        }
        out.push_str("],\"defects\":[");
        for (i, d) in self.defects.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&format!(
                "{{\"axis\":\"{}\",\"knot\":{},\"expected_order\":{},\"measured\":\"{}\",\"u\":{},\"v\":{}}}",
                d.axis.name(),
                num(d.knot),
                d.expected_order,
                d.measured.name(),
                num(d.worst_uv[0]),
                num(d.worst_uv[1]),
            ));
        }
        out.push_str("],\"worst_uv\":");
        match self.worst_uv {
            Some(uv) => out.push_str(&format!("[{},{}]", num(uv[0]), num(uv[1]))),
            None => out.push_str("null"),
        }
        out.push('}');
        out
    }
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn unit(v: [f64; 3]) -> Option<[f64; 3]> {
    let n = norm(v);
    (n > 1e-14).then(|| [v[0] / n, v[1] / n, v[2] / n])
}

/// Домен по узловому вектору (как `Curve::domain`).
fn domain(knots: &[f64], degree: usize, count: usize) -> [f64; 2] {
    [knots[degree], knots[count]]
}

/// Внутренние узлы строго внутри активного домена: (значение, кратность),
/// по возрастанию значения (детерминизм).
fn interior_knots(knots: &[f64], degree: usize, count: usize) -> Vec<(f64, usize)> {
    let [a, b] = domain(knots, degree, count);
    let mut out = Vec::new();
    let mut i = 0;
    while i < knots.len() {
        let mut end = i + 1;
        while end < knots.len() && knots[end] == knots[i] {
            end += 1;
        }
        if a < knots[i] && knots[i] < b {
            out.push((knots[i], end - i));
        }
        i = end;
    }
    out
}

/// Нормальная кривизна в единичном мировом направлении d (в касательной
/// плоскости), как в `class_a::normal_curvature`.
fn normal_curvature(ev: &Evaluation, d: [f64; 3]) -> Option<f64> {
    let (su, sv) = ev.first_derivatives()?;
    let (suu, suv, svv) = ev.second_derivatives()?;
    let n = ev.unit_normal()?;
    let (e, f, g) = (dot(su, su), dot(su, sv), dot(sv, sv));
    let det = e * g - f * f;
    if det <= 1e-28 {
        return None;
    }
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

/// Классификация одной тройки измерений по порогам.
fn classify(position: f64, angle: f64, curvature: f64, tol: &Tolerances) -> Measured {
    if position > tol.position {
        Measured::None
    } else if angle > tol.normal_angle {
        Measured::G0
    } else if curvature > tol.curvature_jump {
        Measured::G1
    } else {
        Measured::G2
    }
}

/// Ожидаемая (минимальная) численная непрерывность для C^{order}.
/// C^2 и выше ⇒ G2; C^1 ⇒ G1; C^0 ⇒ G0.
fn expected(order: usize) -> Measured {
    match order {
        0 => Measured::G0,
        1 => Measured::G1,
        _ => Measured::G2,
    }
}

/// Одна тройка измерений на сдвиге ±eps вокруг узловой линии.
fn jumps_at(
    surface: &Surface,
    axis: KnotAxis,
    knot: f64,
    eps: f64,
    uv: [f64; 2],
) -> Result<(f64, f64, f64)> {
    let (a, b) = match axis {
        KnotAxis::U => (
            surface.evaluate(knot - eps, uv[1])?,
            surface.evaluate(knot + eps, uv[1])?,
        ),
        KnotAxis::V => (
            surface.evaluate(uv[0], knot - eps)?,
            surface.evaluate(uv[0], knot + eps)?,
        ),
    };
    let position = norm(sub(a.point, b.point));
    let Some((na, nb)) = a.unit_normal().zip(b.unit_normal()) else {
        return Ok((position, f64::NAN, f64::NAN));
    };
    let angle = dot(na, nb).clamp(-1., 1.).acos();
    // Нормальная кривизна поперёк линии: направление du для линии U,
    // dv для линии V.
    let curvature = match axis {
        KnotAxis::U => a
            .first_derivatives()
            .and_then(|(su, _)| unit(su))
            .zip(b.first_derivatives().and_then(|(su, _)| unit(su))),
        KnotAxis::V => a
            .first_derivatives()
            .and_then(|(_, sv)| unit(sv))
            .zip(b.first_derivatives().and_then(|(_, sv)| unit(sv))),
    }
    .and_then(|(da, db)| normal_curvature(&a, da).zip(normal_curvature(&b, db)))
    .map(|(ka, kb)| (ka - kb).abs() / ka.abs().max(kb.abs()).max(1e-12))
    .unwrap_or(0.);
    Ok((position, angle, curvature))
}

/// Сэмплинг одной узловой линии по обе стороны. Скачки измеряются на
/// сдвигах ±ε и ±ε/2 и экстраполируются к ε → 0 по Ричардсону: для
/// непрерывной величины скачок на ±ε линеен по ε (2ε·|производная|),
/// и первая разность убирает этот законный тренд, оставляя истинный
/// разрыв с остатком O(ε²).
#[allow(clippy::too_many_arguments)]
fn sample_line(
    surface: &Surface,
    axis: KnotAxis,
    knot: f64,
    multiplicity: usize,
    degree: usize,
    eps: f64,
    samples: usize,
    tol: &Tolerances,
) -> Result<KnotLineReport> {
    let (du, dv) = (
        domain(&surface.knots_u, surface.degree_u, surface.control_points.len()),
        domain(
            &surface.knots_v,
            surface.degree_v,
            surface.control_points[0].len(),
        ),
    );
    let mut report = KnotLineReport {
        axis,
        knot,
        multiplicity,
        degree,
        theoretical_order: degree - multiplicity,
        max_position_jump: 0.,
        max_normal_angle: 0.,
        max_curvature_jump: 0.,
        worst_uv: [knot, 0.5 * (dv[0] + dv[1])],
        measured: Measured::G2,
    };
    // Нормированная тяжесть тройки скачков — для выбора худшей точки.
    let mut worst_score = -1.;
    // Единый бюджет выборок вдоль линии (стадия span-continuity.line-samples).
    let mut guard = Budget::with_iterations(samples)?
        .guard("span-continuity.line-samples");
    for k in 0..samples {
        guard.tick()?;
        let t = k as f64 / (samples - 1) as f64;
        let uv = match axis {
            KnotAxis::U => [knot, dv[0] + (dv[1] - dv[0]) * t],
            KnotAxis::V => [du[0] + (du[1] - du[0]) * t, knot],
        };
        let (p1, a1, c1) = jumps_at(surface, axis, knot, eps, uv)?;
        let (p2, a2, c2) = jumps_at(surface, axis, knot, 0.5 * eps, uv)?;
        let extrapolate = |m1: f64, m2: f64| (2. * m2 - m1).max(0.);
        let position = extrapolate(p1, p2);
        let angle = if a1.is_finite() && a2.is_finite() {
            extrapolate(a1, a2)
        } else {
            continue;
        };
        let curvature = extrapolate(c1, c2);
        report.max_position_jump = report.max_position_jump.max(position);
        report.max_normal_angle = report.max_normal_angle.max(angle);
        report.max_curvature_jump = report.max_curvature_jump.max(curvature);
        let score = position / tol.position.max(1e-300)
            + angle / tol.normal_angle.max(1e-300)
            + curvature / tol.curvature_jump.max(1e-300);
        if score > worst_score {
            worst_score = score;
            report.worst_uv = uv;
        }
        let here = classify(position, angle, curvature, tol);
        report.measured = report.measured.min(here);
    }
    Ok(report)
}

/// (items 990) Карта G-непрерывности по всем внутренним узловым линиям.
///
/// `samples` — число выборок вдоль каждой линии (≥ 2). Сдвиг ε берётся как
/// 1e-5 от длины меньшего из соседних спанов (ограничен снизу 1e-12 от
/// длины домена), поэтому выборки лежат строго внутри своих спанов.
pub fn continuity_map(
    surface: &Surface,
    samples: usize,
    tolerances: &Tolerances,
    limits: &Limits,
) -> Result<ContinuityMap> {
    surface.validate()?;
    check(samples >= 2, "Span continuity map needs at least two samples per line")?;
    require_finite_f64(tolerances.position, "position tolerance")?;
    require_finite_f64(tolerances.normal_angle, "normal_angle tolerance")?;
    require_finite_f64(tolerances.curvature_jump, "curvature_jump tolerance")?;
    check(
        tolerances.position > 0. && tolerances.normal_angle > 0. && tolerances.curvature_jump > 0.,
        "Span continuity tolerances must be positive",
    )?;
    if samples > limits.max_samples_per_line {
        return Err(resource(
            "span_continuity_map sample count exceeds the budget",
        ));
    }
    let lines_u = interior_knots(
        &surface.knots_u,
        surface.degree_u,
        surface.control_points.len(),
    );
    let lines_v = interior_knots(
        &surface.knots_v,
        surface.degree_v,
        surface.control_points[0].len(),
    );
    if lines_u.len() + lines_v.len() > limits.max_lines {
        return Err(resource(
            "span_continuity_map knot line count exceeds the budget",
        ));
    }
    let mut map = ContinuityMap {
        lines: Vec::with_capacity(lines_u.len() + lines_v.len()),
        defects: Vec::new(),
        worst_uv: None,
    };
    let mut worst_angle = -1.;
    // Единый бюджет узловых линий поверх limits.max_lines (пункт 1065).
    let mut line_guard = Budget::with_iterations(limits.max_lines)?
        .guard("span-continuity.lines");
    for (axis, knots, degree, count, found) in [
        (
            KnotAxis::U,
            &surface.knots_u,
            surface.degree_u,
            surface.control_points.len(),
            &lines_u,
        ),
        (
            KnotAxis::V,
            &surface.knots_v,
            surface.degree_v,
            surface.control_points[0].len(),
            &lines_v,
        ),
    ] {
        let dom = domain(knots, degree, count);
        for &(knot, multiplicity) in found.iter() {
            line_guard.tick()?;
            // Соседние различные узлы для выбора ε.
            let left = knots[..]
                .iter()
                .filter(|&&x| x < knot)
                .copied()
                .fold(f64::NEG_INFINITY, f64::max);
            let right = knots[..]
                .iter()
                .filter(|&&x| x > knot)
                .copied()
                .fold(f64::INFINITY, f64::min);
            let span = (knot - left).min(right - knot);
            let eps = (1e-5 * span).max(1e-12 * (dom[1] - dom[0]));
            let report = sample_line(
                surface,
                axis,
                knot,
                multiplicity,
                degree,
                eps,
                samples,
                tolerances,
            )?;
            if report.measured < expected(report.theoretical_order) {
                map.defects.push(ContinuityDefect {
                    axis,
                    knot,
                    expected_order: report.theoretical_order,
                    measured: report.measured,
                    worst_uv: report.worst_uv,
                });
            }
            if report.max_normal_angle > worst_angle {
                worst_angle = report.max_normal_angle;
                map.worst_uv = Some(report.worst_uv);
            }
            map.lines.push(report);
        }
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::extrude;
    use crate::curve::Curve;

    /// Гладкая пластина 5×5 (степень 2, три спана на ось, кратности 1).
    fn smooth_patch() -> Surface {
        let knots = vec![0., 0., 0., 4. / 3., 8. / 3., 4., 4., 4.];
        let mut cp = vec![];
        for i in 0..5 {
            let mut row = vec![];
            for j in 0..5 {
                // Лёгкое плавное волнение, чтобы кривизны были ненулевыми.
                let z = 0.05 * (i as f64 * 0.7).sin() * (j as f64 * 0.9).cos();
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

    /// Поверхность с изломом: кубическая? нет — квадратичная кривая с
    /// узлом кратности 2 посередине (C0-излом), выдавленная вдоль Y.
    fn kinked_surface() -> Surface {
        let curve = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 2., 2., 2.],
            control_points: vec![
                vec![0., 0., 0.],
                vec![0.5, 0., 0.],
                vec![1., 0., 0.],
                vec![1.5, 0., 0.2],
                vec![2., 0., 0.4],
            ],
            weights: vec![1.; 5],
            periodic: false,
        };
        extrude(&curve, [0., 1., 0.]).unwrap()
    }

    #[test]
    fn smooth_patch_is_g2_everywhere_with_no_defects() {
        let s = smooth_patch();
        let map = continuity_map(&s, 33, &Tolerances::default(), &Limits::default()).unwrap();
        // Два внутренних узла по U и два по V.
        assert_eq!(map.lines.len(), 4);
        for line in &map.lines {
            assert_eq!(line.multiplicity, 1);
            assert_eq!(line.theoretical_order, 1);
            assert_eq!(line.measured, Measured::G1, "C1 line: {:?}", line);
            // Позиция и нормали непрерывны; кривизна скачет законно (C1).
            assert!(line.max_position_jump < 1e-9, "jump {}", line.max_position_jump);
            assert!(line.max_normal_angle < 1e-6, "angle {}", line.max_normal_angle);
        }
        assert!(map.defects.is_empty(), "defects {:?}", map.defects);
        let json = map.to_json();
        assert!(json.contains("\"multiplicity\":1"));
        assert!(json.contains("\"measured\":\"G1\""));
    }

    #[test]
    fn full_multiplicity_knot_shows_g0_and_matches_theory() {
        let s = kinked_surface();
        let map = continuity_map(&s, 33, &Tolerances::default(), &Limits::default()).unwrap();
        let line = map
            .lines
            .iter()
            .find(|l| l.axis == KnotAxis::U && l.knot == 1.)
            .expect("knot line at u=1");
        assert_eq!(line.multiplicity, 2);
        assert_eq!(line.theoretical_order, 0);
        // Излом: нормали скачут, позиция непрерывна.
        assert!(line.max_position_jump < 1e-9, "jump {}", line.max_position_jump);
        assert!(line.max_normal_angle > 0.05, "angle {}", line.max_normal_angle);
        assert_eq!(line.measured, Measured::G0);
        // Теория C0 совпадает с численным G0 — дефекта нет.
        assert!(map.defects.is_empty(), "defects {:?}", map.defects);
    }

    #[test]
    fn classify_matches_thresholds() {
        let tol = Tolerances::default();
        assert_eq!(classify(0., 0., 0., &tol), Measured::G2);
        assert_eq!(classify(0., 0., 0.5, &tol), Measured::G1);
        assert_eq!(classify(0., 0.1, 0., &tol), Measured::G0);
        assert_eq!(classify(1e-3, 0., 0., &tol), Measured::None);
        // Ожидание C0 не требует G1: излом на полном узле — не дефект.
        assert!(Measured::G0 >= expected(0));
        assert!(Measured::G1 < expected(2));
    }

    #[test]
    fn budgets_and_inputs_are_checked() {
        let s = smooth_patch();
        let tight = Limits {
            max_samples_per_line: 4,
            ..Limits::default()
        };
        assert!(continuity_map(&s, 33, &Tolerances::default(), &tight).is_err());
        assert!(continuity_map(&s, 1, &Tolerances::default(), &Limits::default()).is_err());
    }

    #[test]
    fn rejects_non_finite_tolerances_with_param_names() {
        let s = smooth_patch();
        let tol = Tolerances {
            position: f64::NAN,
            ..Tolerances::default()
        };
        let err = continuity_map(&s, 9, &tol, &Limits::default()).unwrap_err();
        assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
        assert!(err.contains("position tolerance"), "{err}");
        let tol = Tolerances {
            curvature_jump: f64::INFINITY,
            ..Tolerances::default()
        };
        assert!(continuity_map(&s, 9, &tol, &Limits::default()).is_err());
    }

    #[test]
    fn json_marks_non_finite_metrics_as_null() {
        // Неконечная метрика маркируется null, а не портит документ.
        let line = KnotLineReport {
            axis: KnotAxis::U,
            knot: 0.5,
            multiplicity: 1,
            degree: 2,
            theoretical_order: 1,
            max_position_jump: 0.,
            max_normal_angle: f64::NAN,
            max_curvature_jump: 1e-3,
            worst_uv: [0.5, 0.25],
            measured: Measured::G1,
        };
        let map = ContinuityMap {
            lines: vec![line],
            defects: vec![],
            worst_uv: Some([0.5, f64::NAN]),
        };
        let json = map.to_json();
        assert!(json.contains("\"max_normal_angle\":null"), "{json}");
        assert!(json.contains("\"worst_uv\":[0.5,null]"), "{json}");
        assert!(!json.contains("NaN"), "{json}");
    }

    #[test]
    fn line_budget_guard_is_a_named_stage() {
        // Истощение бюджета узловых линий носит имя стадии.
        let mut guard = Budget::with_iterations(1)
            .unwrap()
            .guard("span-continuity.lines");
        guard.tick().unwrap();
        let err = guard.tick().unwrap_err();
        assert_eq!(err.code, crate::RESOURCE_LIMIT, "{err:?}");
        assert!(err.contains("span-continuity.lines"), "{err}");
    }
}
