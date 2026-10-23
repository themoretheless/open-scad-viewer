//! Карта третьих производных поверхности и зон скачков Δκ′ (items 994),
//! плюс площадная классификация зон по знаку гауссовой кривизны
//! (часть items 993).
//!
//! Третьи производные получаются стабильными центральными конечными
//! разностями поверх *аналитических* вторых производных точного
//! дифференциального джета (`Evaluation::second_derivatives`):
//!   S_uuu ≈ (S_uu(u+h) − S_uu(u−h)) / (2h),  ошибка O(h²·∂⁵S/∂u⁵);
//!   смешанные S_uuv, S_uvv и S_vvv — аналогично.
//! Шаг h = 1e-5 от локального спана, дополнительно ограниченный четвертью
//! расстояния до ближайшего узла (чтобы не сглаживать законный скачок
//! третьей производной на узловой линии) и границами домена; у границ —
//! односторонняя разность с ошибкой O(h). Ошибка документирована, метод
//! сеточный и справочный, не сертификат.
//!
//! Скачки Δκ′ (производная нормальной кривизны по дуге вдоль изолиний
//! сетки) выше порога дают список (u,v)-зон с привязкой к ближайшим
//! контрольным точкам — индексам локальной поддержки базиса (спан ±p).
//! Классификация K: доли площади (по Якобиану |S_u×S_v|) эллиптических
//! (K > tol), гиперболических (K < −tol) и параболических зон.
use crate::{
    Result, check, resource,
    surface::{Evaluation, Surface},
};
use math_core::{cross, dot, norm};

/// Бюджеты сетки.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Максимум ячеек сетки по одной оси.
    pub max_grid_axis: usize,
    /// Максимум зон скачков Δκ′ в одном отчёте.
    pub max_zones: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_grid_axis: 256,
            max_zones: 10_000,
        }
    }
}

/// Третий джет в точке: ∂³S/∂u³, ∂³S/∂u²∂v, ∂³S/∂u∂v², ∂³S/∂v³.
#[derive(Clone, Copy, Debug)]
pub struct ThirdJet {
    pub suuu: [f64; 3],
    pub suuv: [f64; 3],
    pub suvv: [f64; 3],
    pub svvv: [f64; 3],
}

impl ThirdJet {
    /// Максимальная норма компоненты.
    pub fn max_norm(&self) -> f64 {
        norm(self.suuu)
            .max(norm(self.suuv))
            .max(norm(self.suvv))
            .max(norm(self.svvv))
    }
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn unit(v: [f64; 3]) -> Option<[f64; 3]> {
    let n = norm(v);
    (n > 1e-14).then(|| [v[0] / n, v[1] / n, v[2] / n])
}

fn surface_domains(s: &Surface) -> ([f64; 2], [f64; 2]) {
    (
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    )
}

/// Расстояние до ближайшего *другого* узла (в обе стороны) по оси.
fn knot_clearance(knots: &[f64], t: f64) -> f64 {
    knots
        .iter()
        .filter_map(|&k| {
            let d = (k - t).abs();
            (d > 0.).then_some(d)
        })
        .fold(f64::INFINITY, f64::min)
}

/// Вторая производная по u в точке (u,v) или None.
fn suu_at(surface: &Surface, u: f64, v: f64) -> Result<Option<[f64; 3]>> {
    Ok(surface.evaluate(u, v)?.second_derivatives().map(|d| d.0))
}
fn svv_at(surface: &Surface, u: f64, v: f64) -> Result<Option<[f64; 3]>> {
    Ok(surface.evaluate(u, v)?.second_derivatives().map(|d| d.2))
}

/// Центральная разность вторых производных; у границ — односторонняя.
/// `f` извлекает нужную компоненту второго джета.
fn third_component(
    surface: &Surface,
    u: f64,
    v: f64,
    axis_is_u: bool,
    h: f64,
    f: impl Fn(&Surface, f64, f64) -> Result<Option<[f64; 3]>>,
) -> Result<Option<[f64; 3]>> {
    let at = |u: f64, v: f64| f(surface, u, v);
    let (plus, minus, center) = if axis_is_u {
        (at(u + h, v)?, at(u - h, v)?, at(u, v)?)
    } else {
        (at(u, v + h)?, at(u, v - h)?, at(u, v)?)
    };
    let scale = |a: [f64; 3], b: [f64; 3], denom: f64| {
        [
            (a[0] - b[0]) / denom,
            (a[1] - b[1]) / denom,
            (a[2] - b[2]) / denom,
        ]
    };
    Ok(match (plus, minus, center) {
        (Some(p), Some(m), _) => Some(scale(p, m, 2. * h)),
        (Some(p), None, Some(c)) => Some(scale(p, c, h)),
        (None, Some(m), Some(c)) => Some(scale(c, m, h)),
        _ => None,
    })
}

/// (items 994) Третий джет в точке. Шаг: 1e-5 от локального спана,
/// ограниченный ¼ клиренса до узлов и половиной длины домена.
pub fn third_jet(surface: &Surface, u: f64, v: f64) -> Result<ThirdJet> {
    surface.validate()?;
    let (du, dv) = surface_domains(surface);
    check(
        u.is_finite() && du[0] <= u && u <= du[1] && v.is_finite() && dv[0] <= v && v <= dv[1],
        "Third-derivative query lies outside the surface domain",
    )?;
    let span_u = (du[1] - du[0]).max(1e-300);
    let span_v = (dv[1] - dv[0]).max(1e-300);
    let h_u = (1e-5 * span_u)
        .min(0.25 * knot_clearance(&surface.knots_u, u))
        .max(1e-12 * span_u);
    let h_v = (1e-5 * span_v)
        .min(0.25 * knot_clearance(&surface.knots_v, v))
        .max(1e-12 * span_v);
    let missing = || {
        crate::numeric_err(
            "higher_derivative_map: second derivatives unavailable at the query point",
        )
    };
    Ok(ThirdJet {
        suuu: third_component(surface, u, v, true, h_u, suu_at)?.ok_or_else(missing)?,
        suuv: third_component(surface, u, v, false, h_v, suu_at)?.ok_or_else(missing)?,
        suvv: third_component(surface, u, v, true, h_u, svv_at)?.ok_or_else(missing)?,
        svvv: third_component(surface, u, v, false, h_v, svv_at)?.ok_or_else(missing)?,
    })
}

/// Зона скачка Δκ′ с привязкой к контрольным точкам (items 994).
#[derive(Clone, Debug)]
pub struct CurvatureSlopeZone {
    /// (u,v) центра скачка.
    pub uv: [f64; 2],
    /// Измеренный |Δκ/Δs| (в единицах κ на длину).
    pub value: f64,
    /// Порог, который превышен.
    pub threshold: f64,
    /// Индексы контрольных точек (строка u, столбец v) локальной
    /// поддержки базиса в точке скачка, отсортированы.
    pub control_points: Vec<(usize, usize)>,
}

/// Площадная классификация по знаку гауссовой кривизны (items 993).
#[derive(Clone, Copy, Debug)]
pub struct GaussianPartition {
    /// Доли площади: K > tol.
    pub elliptic_fraction: f64,
    /// K < −tol.
    pub hyperbolic_fraction: f64,
    /// |K| ≤ tol.
    pub parabolic_fraction: f64,
    /// Полная площадь сеточной аппроксимации (Σ|J|ΔuΔv).
    pub total_area: f64,
}

/// Итоговая карта (items 994 + часть 993).
#[derive(Clone, Debug)]
pub struct HigherDerivativeMap {
    /// Размеры сетки (ячеек по u, v).
    pub grid: [usize; 2],
    /// Максимальные нормы компонент третьего джета по сетке:
    /// (suuu, suuv, suvv, svvv).
    pub max_component_norms: [f64; 4],
    /// (u,v) максимума `ThirdJet::max_norm`.
    pub max_uv: [f64; 2],
    pub slope_zones: Vec<CurvatureSlopeZone>,
    pub partition: GaussianPartition,
}

impl HigherDerivativeMap {
    /// JSON-подобная сериализация в стиле `class_a`.
    pub fn to_json(&self) -> String {
        let mut out = String::with_capacity(256 + 192 * self.slope_zones.len());
        out.push_str(&format!(
            "{{\"grid\":[{},{}],\"max_component_norms\":[{},{},{},{}],\"max_uv\":[{},{}],\"partition\":{{\"elliptic\":{},\"hyperbolic\":{},\"parabolic\":{},\"total_area\":{}}},\"slope_zones\":[",
            self.grid[0],
            self.grid[1],
            self.max_component_norms[0],
            self.max_component_norms[1],
            self.max_component_norms[2],
            self.max_component_norms[3],
            self.max_uv[0],
            self.max_uv[1],
            self.partition.elliptic_fraction,
            self.partition.hyperbolic_fraction,
            self.partition.parabolic_fraction,
            self.partition.total_area,
        ));
        for (i, z) in self.slope_zones.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&format!(
                "{{\"u\":{},\"v\":{},\"value\":{},\"threshold\":{},\"control_points\":[",
                z.uv[0], z.uv[1], z.value, z.threshold
            ));
            for (j, &(a, b)) in z.control_points.iter().enumerate() {
                if j > 0 {
                    out.push(',');
                }
                out.push_str(&format!("[{},{}]", a, b));
            }
            out.push_str("]}");
        }
        out.push_str("]}");
        out
    }
}

/// Индекс спана: i ∈ [p, n−1] с k[i] ≤ t < k[i+1]; на правом конце — n−1.
fn span_index(knots: &[f64], degree: usize, count: usize, t: f64) -> usize {
    let (p, n) = (degree, count);
    if t >= knots[n] {
        return n - 1;
    }
    (p..n)
        .find(|&i| knots[i] <= t && t < knots[i + 1])
        .unwrap_or(n - 1)
}

/// Индексы контрольных точек локальной поддержки базиса в (u,v).
fn influencing_control_points(surface: &Surface, u: f64, v: f64) -> Vec<(usize, usize)> {
    let iu = span_index(
        &surface.knots_u,
        surface.degree_u,
        surface.control_points.len(),
        u,
    );
    let iv = span_index(
        &surface.knots_v,
        surface.degree_v,
        surface.control_points[0].len(),
        v,
    );
    let mut out = Vec::with_capacity((surface.degree_u + 1) * (surface.degree_v + 1));
    for i in (iu - surface.degree_u)..=iu {
        for j in (iv - surface.degree_v)..=iv {
            out.push((i, j));
        }
    }
    out
}

/// Нормальная кривизна в направлении du (для изолиний u) или dv.
fn directional_curvature(ev: &Evaluation, along_u: bool) -> Option<f64> {
    let (su, sv) = ev.first_derivatives()?;
    let (suu, _suv, svv) = ev.second_derivatives()?;
    let n = ev.unit_normal()?;
    if along_u {
        let e = dot(su, su);
        if e <= 1e-28 {
            return None;
        }
        Some(dot(n, suu) / e)
    } else {
        let g = dot(sv, sv);
        if g <= 1e-28 {
            return None;
        }
        Some(dot(n, svv) / g)
    }
}

/// (items 994 + часть 993) Карта третьих производных, зон Δκ′ и
/// классификация по знаку K на равномерной (u,v)-сетке.
///
/// `slope_threshold` — порог |Δκ/Δs| вдоль изолиний сетки;
/// `parabolic_tol` — полоса |K| для параболической классификации.
pub fn higher_derivative_map(
    surface: &Surface,
    grid: [usize; 2],
    slope_threshold: f64,
    parabolic_tol: f64,
    limits: &Limits,
) -> Result<HigherDerivativeMap> {
    surface.validate()?;
    check(
        grid[0] >= 2 && grid[1] >= 2,
        "Higher-derivative map needs at least 2 cells per axis",
    )?;
    check(
        slope_threshold > 0. && parabolic_tol >= 0.,
        "Slope threshold must be positive and parabolic tolerance non-negative",
    )?;
    if grid[0] > limits.max_grid_axis || grid[1] > limits.max_grid_axis {
        return Err(resource(
            "higher_derivative_map grid axis exceeds the sampling budget",
        ));
    }
    let (nu, nv) = (grid[0], grid[1]);
    let (du, dv) = surface_domains(surface);
    let inset = 0.02;
    let uv_at = |i: usize, j: usize| {
        let fu = inset + (1. - 2. * inset) * i as f64 / nu as f64;
        let fv = inset + (1. - 2. * inset) * j as f64 / nv as f64;
        [
            du[0] + (du[1] - du[0]) * fu,
            dv[0] + (dv[1] - dv[0]) * fv,
        ]
    };
    // Джеты, кривизны и Якобианы на узлах сетки.
    let mut jets: Vec<Option<ThirdJet>> = Vec::with_capacity((nu + 1) * (nv + 1));
    let mut kappa_u = vec![f64::NAN; (nu + 1) * (nv + 1)];
    let mut kappa_v = vec![f64::NAN; (nu + 1) * (nv + 1)];
    let mut gaussian = vec![f64::NAN; (nu + 1) * (nv + 1)];
    let mut jacobian = vec![f64::NAN; (nu + 1) * (nv + 1)];
    let mut points = vec![[f64::NAN; 3]; (nu + 1) * (nv + 1)];
    let idx = |i: usize, j: usize| j * (nu + 1) + i;
    let mut max_norms: [f64; 4] = [0.; 4];
    let mut max_uv = uv_at(0, 0);
    let mut max_total = -1.;
    for j in 0..=nv {
        for i in 0..=nu {
            let [u, v] = uv_at(i, j);
            let k = idx(i, j);
            if let Ok(ev) = surface.evaluate(u, v) {
                points[k] = ev.point;
                kappa_u[k] = directional_curvature(&ev, true).unwrap_or(f64::NAN);
                kappa_v[k] = directional_curvature(&ev, false).unwrap_or(f64::NAN);
                if let Some((gk, _)) = ev.curvatures() {
                    gaussian[k] = gk;
                }
                if let Some((su, sv)) = ev.first_derivatives() {
                    jacobian[k] = norm(cross(su, sv));
                }
            }
            let jet = third_jet(surface, u, v).ok();
            if let Some(t) = jet {
                let norms = [
                    norm(t.suuu),
                    norm(t.suuv),
                    norm(t.suvv),
                    norm(t.svvv),
                ];
                for c in 0..4 {
                    max_norms[c] = max_norms[c].max(norms[c]);
                }
                let total = norms[0].max(norms[1]).max(norms[2]).max(norms[3]);
                if total > max_total {
                    max_total = total;
                    max_uv = [u, v];
                }
            }
            jets.push(jet);
        }
    }
    // Скачки Δκ′ вдоль изолиний сетки (обе оси).
    let mut slope_zones = Vec::new();
    let mut scan = |kappa: &[f64],
                    at: &dyn Fn(usize, usize) -> usize,
                    axis_len: usize,
                    cross_len: usize|
     -> Result<()> {
        for c in 0..=cross_len {
            for s in 0..axis_len {
                let (k0, k1) = (kappa[at(s, c)], kappa[at(s + 1, c)]);
                if !k0.is_finite() || !k1.is_finite() {
                    continue;
                }
                let (p0, p1) = (points[at(s, c)], points[at(s + 1, c)]);
                let ds = norm(sub(p1, p0));
                if ds <= 1e-14 {
                    continue;
                }
                let slope = ((k1 - k0) / ds).abs();
                if slope > slope_threshold {
                    if slope_zones.len() >= limits.max_zones {
                        return Err(resource(
                            "higher_derivative_map slope zone budget exceeded",
                        ));
                    }
                    let [u0, v0] = uv_at(s, c);
                    let [u1, v1] = uv_at(s + 1, c);
                    let (um, vm) = (0.5 * (u0 + u1), 0.5 * (v0 + v1));
                    slope_zones.push(CurvatureSlopeZone {
                        uv: [um, vm],
                        value: slope,
                        threshold: slope_threshold,
                        control_points: influencing_control_points(surface, um, vm),
                    });
                }
            }
        }
        Ok(())
    };
    scan(&kappa_u, &|s, c| idx(s, c), nu, nv)?;
    scan(&kappa_v, &|s, c| idx(c, s), nv, nu)?;
    // Площадная классификация по знаку K (площадь ячейки = средний
    // Якобиан углов · ΔuΔv; ячейки с NaN пропускаются).
    let cell = (du[1] - du[0]) / nu as f64 * (dv[1] - dv[0]) / nv as f64;
    let mut area = [0.; 3];
    for j in 0..nv {
        for i in 0..nu {
            let corners = [idx(i, j), idx(i + 1, j), idx(i + 1, j + 1), idx(i, j + 1)];
            if corners
                .iter()
                .any(|&c| !gaussian[c].is_finite() || !jacobian[c].is_finite())
            {
                continue;
            }
            let k_mean = corners.iter().map(|&c| gaussian[c]).sum::<f64>() / 4.;
            let j_mean = corners.iter().map(|&c| jacobian[c]).sum::<f64>() / 4.;
            let a = j_mean * cell;
            if k_mean > parabolic_tol {
                area[0] += a;
            } else if k_mean < -parabolic_tol {
                area[1] += a;
            } else {
                area[2] += a;
            }
        }
    }
    let total_area = area[0] + area[1] + area[2];
    let fraction = |a: f64| {
        if total_area > 0. {
            a / total_area
        } else {
            0.
        }
    };
    Ok(HigherDerivativeMap {
        grid,
        max_component_norms: max_norms,
        max_uv,
        slope_zones,
        partition: GaussianPartition {
            elliptic_fraction: fraction(area[0]),
            hyperbolic_fraction: fraction(area[1]),
            parabolic_fraction: fraction(area[2]),
            total_area,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::Curve;
    use crate::surface::revolve;

    /// Плоская пластина [0,4]².
    fn flat_patch() -> Surface {
        let knots = vec![0., 0., 0., 4. / 3., 8. / 3., 4., 4., 4.];
        let mut cp = vec![];
        for i in 0..5 {
            let mut row = vec![];
            for j in 0..5 {
                row.push(vec![i as f64, j as f64, 0.]);
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

    /// Кубический Bézier-патч z = u³ на [0,1]²: ∂³S/∂u³ = (0,0,6) точно,
    /// остальные третьи производные нулевые.
    fn cubic_patch() -> Surface {
        let c = |i: usize, j: usize| vec![i as f64 / 3., j as f64, if i == 3 { 1. } else { 0. }];
        Surface {
            degree_u: 3,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..4).map(|i| (0..2).map(|j| c(i, j)).collect()).collect(),
            weights: vec![vec![1.; 2]; 4],
            periodic_u: false,
            periodic_v: false,
        }
    }

    /// Единичная сфера (как в тестах class_a).
    fn sphere() -> Surface {
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
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1., std::f64::consts::FRAC_1_SQRT_2, 1.],
            periodic: false,
        };
        revolve(&profile, [0.; 3], [0., 0., 1.], 360.).unwrap()
    }

    /// Пластина со сдвинутой центральной КТ (как dented_patch в class_a).
    fn dented_patch(dent: f64) -> Surface {
        let mut s = flat_patch();
        s.control_points[2][2][2] = dent;
        s
    }

    #[test]
    fn flat_patch_has_zero_third_derivatives_and_is_parabolic() {
        let s = flat_patch();
        let map = higher_derivative_map(&s, [12, 12], 1e-3, 1e-9, &Limits::default()).unwrap();
        for (c, &m) in map.max_component_norms.iter().enumerate() {
            assert!(m < 1e-6, "component {} norm {}", c, m);
        }
        assert!(map.slope_zones.is_empty(), "zones {:?}", map.slope_zones);
        assert!(
            (map.partition.parabolic_fraction - 1.).abs() < 1e-12,
            "parabolic {}",
            map.partition.parabolic_fraction
        );
        // Квадратурная оценка площади: параметризация искажена (КТ не в
        // абсциссах Гревилля), поэтому допуск грубый.
        assert!(
            (map.partition.total_area - 16.).abs() < 1.,
            "area {}",
            map.partition.total_area
        );
    }

    #[test]
    fn cubic_patch_recovers_exact_uuu() {
        let s = cubic_patch();
        for &(u, v) in &[(0.3, 0.4), (0.7, 0.5), (0.5, 0.1)] {
            let jet = third_jet(&s, u, v).unwrap();
            assert!(
                (jet.suuu[2] - 6.).abs() < 1e-3,
                "suuu.z {} at ({},{})",
                jet.suuu[2],
                u,
                v
            );
            assert!(jet.suuu[0].abs() < 1e-9 && jet.suuu[1].abs() < 1e-9);
            assert!(norm(jet.svvv) < 1e-6, "svvv {}", norm(jet.svvv));
            assert!(norm(jet.suuv) < 1e-3, "suuv {}", norm(jet.suuv));
        }
    }

    #[test]
    fn sphere_is_fully_elliptic() {
        let s = sphere();
        let map = higher_derivative_map(&s, [24, 24], 1e9, 1e-6, &Limits::default()).unwrap();
        assert!(
            map.partition.elliptic_fraction > 0.99,
            "elliptic {}",
            map.partition.elliptic_fraction
        );
        // Площадь единичной сферы = 4π (сеточная оценка грубая у полюсов).
        assert!(
            (map.partition.total_area - 4. * std::f64::consts::PI).abs()
                < 0.2 * 4. * std::f64::consts::PI,
            "area {}",
            map.partition.total_area
        );
    }

    #[test]
    fn dented_patch_localizes_slope_zones_at_shifted_control_point() {
        let s = dented_patch(0.4);
        let map = higher_derivative_map(&s, [24, 24], 0.05, 1e-9, &Limits::default()).unwrap();
        assert!(!map.slope_zones.is_empty(), "no slope zones found");
        // Худшая зона рядом со сдвинутой КТ (2,2) пластины [0,4]².
        let worst = map
            .slope_zones
            .iter()
            .max_by(|a, b| a.value.total_cmp(&b.value))
            .unwrap();
        assert!(
            (worst.uv[0] - 2.).abs() < 1.5 && (worst.uv[1] - 2.).abs() < 1.5,
            "worst zone {:?}",
            worst.uv
        );
        // Локальная поддержка: индекс (2,2) присутствует хотя бы в одной зоне.
        assert!(
            map.slope_zones
                .iter()
                .any(|z| z.control_points.contains(&(2, 2))),
            "no zone references control point (2,2)"
        );
        // Эллиптические и гиперболические зоны обе ненулевые у вмятины.
        assert!(map.partition.elliptic_fraction > 0.);
        assert!(map.partition.hyperbolic_fraction > 0.01);
        let json = map.to_json();
        assert!(json.contains("\"slope_zones\":[{"));
        assert!(json.contains("\"parabolic\":"));
    }

    #[test]
    fn influencing_points_cover_local_support() {
        let s = flat_patch();
        // Середина домена (u=2): спан [4/3, 8/3) → i=3, поддержка строки 1..=3.
        let cps = influencing_control_points(&s, 2., 2.);
        assert_eq!(cps.len(), 9);
        assert!(cps.contains(&(2, 2)));
        assert!(cps.contains(&(1, 1)) && cps.contains(&(3, 3)));
        assert!(higher_derivative_map(&s, [300, 12], 1., 0., &Limits::default()).is_err());
        assert!(higher_derivative_map(&s, [12, 12], 0., 0., &Limits::default()).is_err());
    }
}
