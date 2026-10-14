//! Поле знаковых отклонений поверхности от эталона (items 992; метрика
//! для items 964) и 1D-Wasserstein расстояние между распределениями
//! кривизн двух поверхностей (сравнительная часть items 993).
//!
//! Для каждого узла равномерной (u,v)-сетки точка поверхности проецируется
//! на эталон через `distance::surface_distance::nearest_point`; знак
//! отклонения даёт нормаль эталона в проекции: sign((p − q)·n_ref).
//! Отклонения группируются по зональным маскам весов (функция веса от
//! (u,v) и положения точки — напр. передняя кромка ×10), считаются
//! RMS/max по зонам и гистограмма; сериализация `to_json` в стиле
//! `class_a`. Чисто численная сеточная метрика, не сертификат.
//!
//! `wasserstein_1d` — расстояние Вассерштейна W1 между эмпирическими
//! распределениями (например, выборками кривизны двух поверхностей):
//! сортировка → эмпирические квантильные функции → интеграл
//! |F₁⁻¹ − F₂⁻¹| по равномерной сетке квантилей (трапеции).
use crate::{
    Result, check, resource,
    distance::surface_distance::nearest_point,
    surface::Surface,
};
use math_core::{dot, norm, sub};

/// Бюджеты выборки.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Максимум ячеек сетки по одной оси.
    pub max_grid_axis: usize,
    /// Бюджет ячеек одного запроса closest-point к эталону.
    pub max_cells_per_query: usize,
    /// Максимум зон.
    pub max_zones: usize,
    /// Максимум квантилей в Wasserstein-интеграле.
    pub max_quantiles: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_grid_axis: 128,
            max_cells_per_query: 8_192,
            max_zones: 32,
            max_quantiles: 65_536,
        }
    }
}

/// Зональная маска весов: имя + вес по (u,v) и положению точки.
pub struct Zone<'a> {
    pub name: &'a str,
    /// Вес ≥ 0; зона собирает статистику по узлам с весом > 0.
    pub weight: &'a dyn Fn([f64; 2], [f64; 3]) -> f64,
}

/// Статистика одной зоны.
#[derive(Clone, Debug)]
pub struct ZoneStats {
    pub name: String,
    /// Узлов с ненулевым весом.
    pub samples: usize,
    pub weight_sum: f64,
    /// Взвешенное среднее отклонения (со знаком).
    pub mean: f64,
    /// Взвешенный RMS.
    pub rms: f64,
    /// max |deviation| в зоне.
    pub max_abs: f64,
}

/// Гистограмма знаковых отклонений.
#[derive(Clone, Debug)]
pub struct Histogram {
    pub min: f64,
    pub max: f64,
    /// Число узлов в бине, сумма = количеству успешных выборок.
    pub counts: Vec<usize>,
}

/// (items 992 / метрика 964) Отчёт о знаковом отклонении от эталона.
#[derive(Clone, Debug)]
pub struct DeviationReport {
    pub grid: [usize; 2],
    /// Успешных проекций (остальные — не сошлись и исключены).
    pub samples: usize,
    pub failed: usize,
    pub mean: f64,
    pub rms: f64,
    pub max_abs: f64,
    /// (u,v) узла с max |deviation|.
    pub worst_uv: Option<[f64; 2]>,
    pub histogram: Histogram,
    pub zones: Vec<ZoneStats>,
}

impl DeviationReport {
    /// JSON-подобная сериализация в стиле `class_a::ClassAReport::to_json`.
    pub fn to_json(&self) -> String {
        let mut out = String::with_capacity(384 + 32 * self.histogram.counts.len());
        out.push_str(&format!(
            "{{\"grid\":[{},{}],\"samples\":{},\"failed\":{},\"mean\":{},\"rms\":{},\"max_abs\":{},\"worst_uv\":",
            self.grid[0], self.grid[1], self.samples, self.failed, self.mean, self.rms, self.max_abs
        ));
        match self.worst_uv {
            Some(uv) => out.push_str(&format!("[{},{}]", uv[0], uv[1])),
            None => out.push_str("null"),
        }
        out.push_str(&format!(
            ",\"histogram\":{{\"min\":{},\"max\":{},\"counts\":[",
            self.histogram.min, self.histogram.max
        ));
        for (i, c) in self.histogram.counts.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&format!("{}", c));
        }
        out.push_str("]},\"zones\":[");
        for (i, z) in self.zones.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&format!(
                "{{\"name\":\"{}\",\"samples\":{},\"weight_sum\":{},\"mean\":{},\"rms\":{},\"max_abs\":{}}}",
                z.name, z.samples, z.weight_sum, z.mean, z.rms, z.max_abs
            ));
        }
        out.push_str("]}");
        out
    }
}

fn surface_domains(s: &Surface) -> ([f64; 2], [f64; 2]) {
    (
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    )
}

/// (items 992, метрика для 964) Поле знаковых отклонений к эталону.
///
/// `grid` — ячеек по осям; `bins` — столбцов гистограммы (≥ 4);
/// `tolerance` — точность closest-point запросов к эталону.
pub fn signed_deviation(
    surface: &Surface,
    reference: &Surface,
    grid: [usize; 2],
    bins: usize,
    zones: &[Zone],
    tolerance: f64,
    limits: &Limits,
) -> Result<DeviationReport> {
    surface.validate()?;
    reference.validate()?;
    check(
        grid[0] >= 2 && grid[1] >= 2,
        "Deviation field needs at least 2 cells per axis",
    )?;
    check(bins >= 4, "Deviation histogram needs at least 4 bins")?;
    check(tolerance > 0., "Deviation closest-point tolerance must be positive")?;
    check(
        zones.iter().all(|z| !z.name.is_empty()),
        "Deviation zones need non-empty names",
    )?;
    if grid[0] > limits.max_grid_axis || grid[1] > limits.max_grid_axis {
        return Err(resource(
            "reference_deviation grid axis exceeds the sampling budget",
        ));
    }
    if zones.len() > limits.max_zones {
        return Err(resource("reference_deviation zone count exceeds the budget"));
    }
    let (nu, nv) = (grid[0], grid[1]);
    let (du, dv) = surface_domains(surface);
    let inset = 0.02;
    // Выборка знаковых отклонений.
    let mut values: Vec<(f64, [f64; 2])> = Vec::with_capacity((nu + 1) * (nv + 1));
    let mut failed = 0usize;
    for j in 0..=nv {
        let v = dv[0] + (dv[1] - dv[0]) * (inset + (1. - 2. * inset) * j as f64 / nv as f64);
        for i in 0..=nu {
            let u = du[0] + (du[1] - du[0]) * (inset + (1. - 2. * inset) * i as f64 / nu as f64);
            let p = surface.evaluate(u, v)?.point;
            let nearest = nearest_point(reference, p, tolerance, limits.max_cells_per_query)?;
            if !nearest.converged {
                failed += 1;
                continue;
            }
            let q = nearest.parameters;
            let ev = reference.evaluate(q[0], q[1])?;
            let Some(n) = ev.unit_normal() else {
                failed += 1;
                continue;
            };
            let d = norm(sub(p, ev.point));
            let signed = if dot(sub(p, ev.point), n) >= 0. { d } else { -d };
            values.push((signed, [u, v]));
        }
    }
    check(
        !values.is_empty(),
        "Deviation field: no closest-point query converged",
    )?;
    // Глобальная статистика (детерминированный порядок обхода).
    let samples = values.len();
    let mean = values.iter().map(|v| v.0).sum::<f64>() / samples as f64;
    let rms = (values.iter().map(|v| v.0 * v.0).sum::<f64>() / samples as f64).sqrt();
    let mut max_abs = 0.;
    let mut worst_uv = None;
    for &(d, uv) in &values {
        if d.abs() >= max_abs {
            max_abs = d.abs();
            worst_uv = Some(uv);
        }
    }
    // Гистограмма на [min, max] выборки; крайние значения в крайние бины.
    let min = values.iter().map(|v| v.0).fold(f64::INFINITY, f64::min);
    let max = values.iter().map(|v| v.0).fold(f64::NEG_INFINITY, f64::max);
    let width = (max - min).max(1e-300);
    let mut counts = vec![0usize; bins];
    for &(d, _) in &values {
        let k = (((d - min) / width) * bins as f64) as usize;
        counts[k.min(bins - 1)] += 1;
    }
    // Зоны: взвешенная статистика по маскам.
    let mut zone_stats = Vec::with_capacity(zones.len());
    for zone in zones {
        let mut z_samples = 0usize;
        let mut w_sum = 0.;
        let mut d_sum = 0.;
        let mut d2_sum = 0.;
        let mut z_max: f64 = 0.;
        for &(d, uv) in &values {
            let p = surface.evaluate(uv[0], uv[1])?.point;
            let w = (zone.weight)(uv, p);
            check(w >= 0. && w.is_finite(), "Deviation zone weights must be finite and >= 0")?;
            if w > 0. {
                z_samples += 1;
                w_sum += w;
                d_sum += w * d;
                d2_sum += w * d * d;
                z_max = z_max.max(d.abs());
            }
        }
        let (z_mean, z_rms) = if w_sum > 0. {
            (d_sum / w_sum, (d2_sum / w_sum).sqrt())
        } else {
            (0., 0.)
        };
        zone_stats.push(ZoneStats {
            name: zone.name.to_string(),
            samples: z_samples,
            weight_sum: w_sum,
            mean: z_mean,
            rms: z_rms,
            max_abs: z_max,
        });
    }
    Ok(DeviationReport {
        grid,
        samples,
        failed,
        mean,
        rms,
        max_abs,
        worst_uv,
        histogram: Histogram { min, max, counts },
        zones: zone_stats,
    })
}

/// (сравнительная часть items 993) 1D-Wasserstein W1 между эмпирическими
/// распределениями двух выборок (например, гистограмм кривизны на сетке).
/// Эмпирические квантильные функции интерполируются линейно, интеграл
/// |F₁⁻¹ − F₂⁻¹| — трапециями по `quantiles` равномерным уровням.
pub fn wasserstein_1d(a: &[f64], b: &[f64], quantiles: usize, limits: &Limits) -> Result<f64> {
    check(!a.is_empty() && !b.is_empty(), "Wasserstein needs two non-empty samples")?;
    check(
        a.iter().chain(b.iter()).all(|x| x.is_finite()),
        "Wasserstein samples must be finite",
    )?;
    check(quantiles >= 8, "Wasserstein needs at least 8 quantile levels")?;
    if quantiles > limits.max_quantiles {
        return Err(resource(
            "reference_deviation quantile count exceeds the budget",
        ));
    }
    let mut sa = a.to_vec();
    let mut sb = b.to_vec();
    sa.sort_by(f64::total_cmp);
    sb.sort_by(f64::total_cmp);
    // Квантильная функция выборки: линейная интерполяция по позициям
    // i/(n−1) (гидрологический тип, детерминированный).
    let quantile = |s: &[f64], q: f64| {
        if s.len() == 1 {
            return s[0];
        }
        let pos = q * (s.len() - 1) as f64;
        let i = (pos as usize).min(s.len() - 2);
        let w = pos - i as f64;
        s[i] * (1. - w) + s[i + 1] * w
    };
    let mut acc = 0.;
    let mut prev = (quantile(&sa, 0.) - quantile(&sb, 0.)).abs();
    for k in 1..quantiles {
        let q = k as f64 / (quantiles - 1) as f64;
        let d = (quantile(&sa, q) - quantile(&sb, q)).abs();
        acc += 0.5 * (prev + d);
        prev = d;
    }
    Ok(acc / (quantiles - 1) as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::Curve;
    use crate::surface::revolve;

    /// Сфера радиуса r (профиль-полуокружность, revolve на 360°).
    fn sphere(r: f64) -> Surface {
        let profile = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 2., 2., 2.],
            control_points: vec![
                vec![0., 0., -r],
                vec![r, 0., -r],
                vec![r, 0., 0.],
                vec![r, 0., r],
                vec![0., 0., r],
            ],
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1., std::f64::consts::FRAC_1_SQRT_2, 1.],
            periodic: false,
        };
        revolve(&profile, [0.; 3], [0., 0., 1.], 360.).unwrap()
    }

    /// Плоскость z = h на [0,1]².
    fn plane(h: f64) -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., h], vec![0., 1., h]],
                vec![vec![1., 0., h], vec![1., 1., h]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }

    #[test]
    fn two_spheres_deviate_by_radius_difference() {
        let outer = sphere(1.05);
        let reference = sphere(1.0);
        let budget = Limits {
            max_cells_per_query: 65_536,
            ..Limits::default()
        };
        let report =
            signed_deviation(&outer, &reference, [12, 12], 16, &[], 1e-7, &budget).unwrap();
        // Единичный узел у полюса может не сойтись — допускаем малую долю.
        assert!(report.failed <= 2, "failed projections {}", report.failed);
        // Нормаль эталона (revolve-профиль) смотрит внутрь: внешняя
        // сфера даёт отклонение −0.05 со знаком нормали эталона.
        assert!((report.mean + 0.05).abs() < 1e-3, "mean {}", report.mean);
        assert!((report.rms - 0.05).abs() < 1e-3, "rms {}", report.rms);
        assert!((report.max_abs - 0.05).abs() < 1e-3, "max {}", report.max_abs);
        // Все отклонения отрицательные: гистограмма занимает левый край.
        assert!(report.histogram.max < 0.);
        assert_eq!(
            report.histogram.counts.iter().sum::<usize>(),
            report.samples
        );
    }

    #[test]
    fn plane_above_plane_has_constant_signed_deviation() {
        let report = signed_deviation(
            &plane(0.2),
            &plane(0.0),
            [8, 8],
            8,
            &[],
            1e-9,
            &Limits::default(),
        )
        .unwrap();
        assert!((report.mean - 0.2).abs() < 1e-6, "mean {}", report.mean);
        assert!((report.max_abs - 0.2).abs() < 1e-6);
        // Знаковость: эталон выше поверхности → отрицательное поле.
        let flipped = signed_deviation(
            &plane(0.0),
            &plane(0.2),
            [8, 8],
            8,
            &[],
            1e-9,
            &Limits::default(),
        )
        .unwrap();
        assert!((flipped.mean + 0.2).abs() < 1e-6, "mean {}", flipped.mean);
    }

    #[test]
    fn weighted_zones_rescale_rms() {
        // Поверхность-плоскость с линейным по x отклонением: эталон —
        // наклонная плоскость z = 0.1·x (аппроксимация разностью высот).
        let a = plane(0.0);
        let mut tilted = plane(0.0);
        tilted.control_points[1][0][2] = 0.1;
        tilted.control_points[1][1][2] = 0.1;
        let front = Zone {
            name: "leading_edge",
            weight: &|uv, _| if uv[0] < 0.5 { 10. } else { 0. },
        };
        let rest = Zone {
            name: "rest",
            weight: &|uv, _| if uv[0] >= 0.5 { 1. } else { 0. },
        };
        let report = signed_deviation(
            &a,
            &tilted,
            [16, 16],
            16,
            &[front, rest],
            1e-9,
            &Limits::default(),
        )
        .unwrap();
        assert_eq!(report.zones.len(), 2);
        // Передняя кромка (u < 0.5): малые отклонения; хвост — до 0.1.
        let (lead, rest) = (&report.zones[0], &report.zones[1]);
        assert!(lead.samples > 0 && rest.samples > 0);
        assert!(lead.rms < rest.rms, "lead {} rest {}", lead.rms, rest.rms);
        assert!(rest.max_abs > 0.08, "rest max {}", rest.max_abs);
        let json = report.to_json();
        assert!(json.contains("\"name\":\"leading_edge\""));
        assert!(json.contains("\"histogram\":{"));
    }

    #[test]
    fn wasserstein_of_shifted_samples_is_the_shift() {
        let base: Vec<f64> = (0..101).map(|i| i as f64 * 0.01).collect();
        let shifted: Vec<f64> = base.iter().map(|x| x + 0.37).collect();
        let limits = Limits::default();
        let d = wasserstein_1d(&base, &shifted, 257, &limits).unwrap();
        assert!((d - 0.37).abs() < 1e-12, "distance {}", d);
        assert_eq!(wasserstein_1d(&base, &base, 257, &limits).unwrap(), 0.);
        // Разные длины выборок: W1 между U[0,1] и точкой 0.5 ≈ 0.25.
        let point = [0.5];
        let d = wasserstein_1d(&base, &point, 257, &limits).unwrap();
        assert!((d - 0.2501).abs() < 1e-3, "distance {}", d);
        assert!(wasserstein_1d(&[], &base, 64, &limits).is_err());
        assert!(wasserstein_1d(&base, &base, 4, &limits).is_err());
    }
}
