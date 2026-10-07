//! RANSAC-выделение аналитических примитивов из облака точек (по Шнабелю,
//! упрощённо): минимальные наборы, порог инлаеров, итерации с ранним выходом,
//! последовательное удаление инлаеров. Генератор — детерминированный
//! [`SplitMix64`], без внешних зависимостей.
//!
//! Уточнение параметров выбранного кандидата выполняется нелинейными fit'ами
//! из [`super::primitive_fit`]; для плоскости используется
//! `math_core::point_fit_plane`.

use crate::analysis::primitive_fit::{
    fit_cone, fit_cylinder, fit_sphere,
};
use crate::foundation::guards::{Budget, require_finite_f64, require_finite_point};
use crate::{Result, check, numeric, numeric_err};
use math_core::{Acceleration, point_fit_plane};

/// Максимальное число точек облака.
pub const MAX_CLOUD_POINTS: usize = 65536;

/// Детерминированный генератор SplitMix64 (публичный — удобен и в тестах
/// потребителей для воспроизводимых синтетических данных).
#[derive(Clone, Debug)]
pub struct SplitMix64(u64);

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Равномерное число в [0, 1).
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// Индекс в [0, n).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n.max(1) as u64) as usize
    }
    /// Приближённо нормальный шум (Ирвин–Холл, сумма 4 равномерных).
    pub fn gauss(&mut self) -> f64 {
        (0..4).map(|_| self.next_f64()).sum::<f64>() - 2.
    }
}

/// Тип детектированного примитива.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimitiveKind {
    Plane,
    Sphere,
    Cylinder,
    Cone,
}

/// Геометрические параметры примитива.
#[derive(Clone, Copy, Debug)]
pub enum PrimitiveParams {
    /// Плоскость `normal·x + offset = 0`, `normal` единичная.
    Plane { normal: [f64; 3], offset: f64 },
    Sphere { center: [f64; 3], radius: f64 },
    Cylinder { point_on_axis: [f64; 3], direction: [f64; 3], radius: f64 },
    Cone { vertex: [f64; 3], axis: [f64; 3], half_angle: f64 },
}

impl PrimitiveParams {
    /// Расстояние точки до поверхности (по модулю).
    fn deviation(&self, p: [f64; 3]) -> f64 {
        match *self {
            PrimitiveParams::Plane { normal, offset } => {
                (dot(p, normal) + offset).abs()
            }
            PrimitiveParams::Sphere { center, radius } => {
                (norm(sub(p, center)) - radius).abs()
            }
            PrimitiveParams::Cylinder { point_on_axis, direction, radius } => {
                let w = sub(p, point_on_axis);
                let t = dot(w, direction);
                let axial = [direction[0] * t, direction[1] * t, direction[2] * t];
                (norm(sub(w, axial)) - radius).abs()
            }
            PrimitiveParams::Cone { vertex, axis, half_angle } => {
                let w = sub(p, vertex);
                let t = dot(w, axis);
                let axial = [axis[0] * t, axis[1] * t, axis[2] * t];
                let rho = norm(sub(w, axial));
                (t * half_angle.sin() - rho * half_angle.cos()).abs()
            }
        }
    }
}

/// Детектированный примитив: вид, параметры, инлаеры и их RMS.
#[derive(Clone, Debug)]
pub struct DetectedPrimitive {
    pub kind: PrimitiveKind,
    pub params: PrimitiveParams,
    /// Индексы точек исходного облака.
    pub inlier_indices: Vec<u32>,
    pub rms: f64,
}

/// Настройки RANSAC-детектора.
#[derive(Clone, Debug)]
pub struct RansacOptions {
    /// Порог расстояния до поверхности для инлаеров.
    pub threshold: f64,
    /// Минимальное число инлаеров, чтобы принять примитив.
    pub min_inliers: usize,
    /// Число RANSAC-итераций на вид примитива за раунд.
    pub max_iterations: usize,
    /// Максимум примитивов за вызов.
    pub max_primitives: usize,
    /// Ранний выход: принять кандидата, если инлаеры ≥ доли оставшихся точек.
    pub early_exit_fraction: f64,
    /// Зерно детерминированного RNG.
    pub seed: u64,
    /// Какие виды искать.
    pub detect_plane: bool,
    pub detect_sphere: bool,
    pub detect_cylinder: bool,
    pub detect_cone: bool,
}

impl Default for RansacOptions {
    fn default() -> Self {
        Self {
            threshold: 0.01,
            min_inliers: 100,
            max_iterations: 200,
            max_primitives: 8,
            early_exit_fraction: 0.85,
            seed: 0x5EED,
            detect_plane: true,
            detect_sphere: true,
            detect_cylinder: true,
            detect_cone: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Малая векторная арифметика.
// ---------------------------------------------------------------------------

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

fn unit(a: [f64; 3]) -> Option<[f64; 3]> {
    let l = norm(a);
    (l > 1e-12).then(|| [a[0] / l, a[1] / l, a[2] / l])
}

// ---------------------------------------------------------------------------
// Минимальные модели.
// ---------------------------------------------------------------------------

/// Плоскость по 3 неколлинеарным точкам.
fn plane_from3(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> Option<PrimitiveParams> {
    let n = unit(cross(sub(b, a), sub(c, a)))?;
    Some(PrimitiveParams::Plane { normal: n, offset: -dot(n, a) })
}

/// Сфера по 4 некомпланарным точкам (пересечение радикальных плоскостей).
fn sphere_from4(pts: &[[f64; 3]; 4]) -> Option<PrimitiveParams> {
    let p0 = pts[0];
    let mut m = [[0.; 3]; 3];
    let mut rhs = [0.; 3];
    for i in 0..3 {
        let pi = pts[i + 1];
        for k in 0..3 {
            m[i][k] = 2. * (pi[k] - p0[k]);
        }
        rhs[i] = dot(pi, pi) - dot(p0, p0);
    }
    let c = solve3(m, rhs)?;
    let r = norm(sub(p0, c));
    (r > 1e-12).then_some(PrimitiveParams::Sphere { center: c, radius: r })
}

/// Цилиндр по 2 точкам с нормалями (Шнабель): ось ⊥ обеим нормалям,
/// точка оси — пересечение нормальных прямых в проекции на плоскость ⊥ оси.
fn cylinder_from2n(
    p1: [f64; 3],
    n1: [f64; 3],
    p2: [f64; 3],
    n2: [f64; 3],
) -> Option<PrimitiveParams> {
    let dir = unit(cross(n1, n2))?;
    // Проецируем на плоскость ⊥ dir, решаем 2x2 пересечение прямых
    // p1 + t·n1 = p2 + s·n2 в базисе {u, v}.
    let seed = if dir[0].abs() < 0.9 { [1., 0., 0.] } else { [0., 1., 0.] };
    let u = unit(cross(dir, seed))?;
    let v = cross(dir, u);
    let a11 = dot(n1, u);
    let a12 = -dot(n2, u);
    let a21 = dot(n1, v);
    let a22 = -dot(n2, v);
    let b1 = dot(sub(p2, p1), u);
    let b2 = dot(sub(p2, p1), v);
    let det = a11 * a22 - a12 * a21;
    if det.abs() < 1e-12 {
        return None;
    }
    let t = (b1 * a22 - b2 * a12) / det;
    let s = (a11 * b2 - a21 * b1) / det;
    let q1 = [p1[0] + t * n1[0], p1[1] + t * n1[1], p1[2] + t * n1[2]];
    let q2 = [p2[0] + s * n2[0], p2[1] + s * n2[1], p2[2] + s * n2[2]];
    let q = [(q1[0] + q2[0]) / 2., (q1[1] + q2[1]) / 2., (q1[2] + q2[2]) / 2.];
    let r = (t.abs() + s.abs()) / 2.;
    (r > 1e-12).then_some(PrimitiveParams::Cylinder {
        point_on_axis: q,
        direction: dir,
        radius: r,
    })
}

/// Решение 3x3 Гауссом с частичным выбором ведущего.
fn solve3(mut m: [[f64; 3]; 3], mut rhs: [f64; 3]) -> Option<[f64; 3]> {
    for col in 0..3 {
        let mut piv = col;
        for r in col + 1..3 {
            if m[r][col].abs() > m[piv][col].abs() {
                piv = r;
            }
        }
        if m[piv][col].abs() < 1e-12 {
            return None;
        }
        m.swap(piv, col);
        rhs.swap(piv, col);
        for r in col + 1..3 {
            let f = m[r][col] / m[col][col];
            for c in col..3 {
                m[r][c] -= f * m[col][c];
            }
            rhs[r] -= f * rhs[col];
        }
    }
    let mut x = [0.; 3];
    for i in (0..3).rev() {
        let mut s = rhs[i];
        for c in i + 1..3 {
            s -= m[i][c] * x[c];
        }
        x[i] = s / m[i][i];
    }
    x.iter().all(|v| v.is_finite()).then_some(x)
}

// ---------------------------------------------------------------------------
// RANSAC.
// ---------------------------------------------------------------------------

/// Выбор `k` различных индексов из `n`.
fn sample(rng: &mut SplitMix64, n: usize, k: usize) -> Vec<usize> {
    let mut out = Vec::with_capacity(k);
    while out.len() < k {
        let i = rng.below(n);
        if !out.contains(&i) {
            out.push(i);
        }
    }
    out
}

/// Индексы инлаеров кандидата внутри `pool`.
fn inliers_of(
    points: &[[f64; 3]],
    pool: &[u32],
    params: &PrimitiveParams,
    threshold: f64,
) -> Vec<u32> {
    pool.iter()
        .copied()
        .filter(|&i| params.deviation(points[i as usize]) <= threshold)
        .collect()
}

/// RANSAC-раунд для одного вида примитива: лучший кандидат по числу инлаеров.
/// Итерации идут под единым [`Budget`] (пункт 1065), подкрепляющим
/// `options.max_iterations` именованной стадией.
fn ransac_kind(
    kind: PrimitiveKind,
    points: &[[f64; 3]],
    normals: Option<&[[f64; 3]]>,
    pool: &[u32],
    options: &RansacOptions,
    rng: &mut SplitMix64,
) -> Result<Option<(PrimitiveParams, Vec<u32>)>> {
    let n = pool.len();
    let min_sample = match kind {
        PrimitiveKind::Plane => 3,
        PrimitiveKind::Sphere => 4,
        PrimitiveKind::Cylinder => {
            if normals.is_some() { 2 } else { 6 }
        }
        PrimitiveKind::Cone => 7,
    };
    if n < min_sample.max(options.min_inliers.min(n)) {
        return Ok(None);
    }
    let stage = match kind {
        PrimitiveKind::Plane => "ransac.plane",
        PrimitiveKind::Sphere => "ransac.sphere",
        PrimitiveKind::Cylinder => "ransac.cylinder",
        PrimitiveKind::Cone => "ransac.cone",
    };
    let mut guard = Budget::with_iterations(options.max_iterations)?.guard(stage);
    let mut best: Option<(PrimitiveParams, Vec<u32>)> = None;
    for _ in 0..options.max_iterations {
        guard.tick()?;
        let idx = sample(rng, n, min_sample);
        let params = match kind {
            PrimitiveKind::Plane => plane_from3(
                points[pool[idx[0]] as usize],
                points[pool[idx[1]] as usize],
                points[pool[idx[2]] as usize],
            ),
            PrimitiveKind::Sphere => sphere_from4(&[
                points[pool[idx[0]] as usize],
                points[pool[idx[1]] as usize],
                points[pool[idx[2]] as usize],
                points[pool[idx[3]] as usize],
            ]),
            PrimitiveKind::Cylinder => {
                if let Some(nrm) = normals {
                    cylinder_from2n(
                        points[pool[idx[0]] as usize],
                        nrm[pool[idx[0]] as usize],
                        points[pool[idx[1]] as usize],
                        nrm[pool[idx[1]] as usize],
                    )
                } else {
                    let mini: Vec<[f64; 3]> =
                        idx.iter().map(|&i| points[pool[i] as usize]).collect();
                    fit_cylinder(&mini).ok().map(|f| PrimitiveParams::Cylinder {
                        point_on_axis: f.point_on_axis,
                        direction: f.direction,
                        radius: f.radius,
                    })
                }
            }
            PrimitiveKind::Cone => {
                let mini: Vec<[f64; 3]> =
                    idx.iter().map(|&i| points[pool[i] as usize]).collect();
                fit_cone(&mini).ok().map(|f| PrimitiveParams::Cone {
                    vertex: f.vertex,
                    axis: f.axis,
                    half_angle: f.half_angle,
                })
            }
        };
        let Some(params) = params else { continue };
        let inl = inliers_of(points, pool, &params, options.threshold);
        let better = match &best {
            None => true,
            Some((_, b)) => inl.len() > b.len(),
        };
        if better {
            best = Some((params, inl));
        }
        if let Some((_, b)) = &best {
            if b.len() as f64 >= options.early_exit_fraction * n as f64 {
                break;
            }
        }
    }
    guard.check()?;
    Ok(best)
}

/// Нелинейное уточнение кандидата по его инлаерам.
fn refine(
    kind: PrimitiveKind,
    points: &[[f64; 3]],
    inl: &[u32],
) -> Result<PrimitiveParams> {
    let pts: Vec<[f64; 3]> = inl.iter().map(|&i| points[i as usize]).collect();
    match kind {
        PrimitiveKind::Plane => {
            let plane = point_fit_plane(&pts, Acceleration::Cpu)
                .map_err(|e| numeric_err(format!("plane refinement failed: {e}")))?;
            Ok(PrimitiveParams::Plane { normal: plane.normal, offset: plane.offset })
        }
        PrimitiveKind::Sphere => fit_sphere(&pts)
            .map(|f| PrimitiveParams::Sphere { center: f.center, radius: f.radius }),
        PrimitiveKind::Cylinder => fit_cylinder(&pts).map(|f| PrimitiveParams::Cylinder {
            point_on_axis: f.point_on_axis,
            direction: f.direction,
            radius: f.radius,
        }),
        PrimitiveKind::Cone => fit_cone(&pts).map(|f| PrimitiveParams::Cone {
            vertex: f.vertex,
            axis: f.axis,
            half_angle: f.half_angle,
        }),
    }
}

/// Последовательное RANSAC-выделение примитивов из облака.
///
/// - `points` — 3D-точки (≤ 65536, конечные);
/// - `normals` — необязательные нормали (ускоряют и стабилизируют цилиндр;
///   без них цилиндр оценивается мини-fit'ом по 6 точкам);
/// - `options` — см. [`RansacOptions`].
///
/// Каждый раунд перебирает включённые виды, выбирает кандидата с наибольшим
/// числом инлаеров, уточняет его параметры МНК по инлаерам, пересчитывает
/// инлаеры и удаляет их из пула. Остановка — когда ни один вид не набрал
/// `min_inliers` или достигнут `max_primitives`.
pub fn detect_primitives(
    points: &[[f64; 3]],
    normals: Option<&[[f64; 3]]>,
    options: &RansacOptions,
) -> Result<Vec<DetectedPrimitive>> {
    check(
        !points.is_empty() && points.len() <= MAX_CLOUD_POINTS,
        "Point count must be in 1..=65536",
    )?;
    for p in points {
        require_finite_point(p, "points")?;
    }
    if let Some(nrm) = normals {
        check(
            nrm.len() == points.len(),
            "Normals must match the point count",
        )?;
        for n in nrm {
            require_finite_point(n, "normals")?;
        }
    }
    require_finite_f64(options.threshold, "threshold")?;
    check(options.threshold > 0., "Threshold must be positive")?;
    require_finite_f64(options.early_exit_fraction, "early_exit_fraction")?;
    check(
        (0. ..=1.).contains(&options.early_exit_fraction),
        "Early exit fraction must lie in [0, 1]",
    )?;
    check(
        options.detect_plane
            || options.detect_sphere
            || options.detect_cylinder
            || options.detect_cone,
        "At least one primitive kind must be enabled",
    )?;
    let mut rng = SplitMix64::new(options.seed);
    let mut pool: Vec<u32> = (0..points.len() as u32).collect();
    let kinds = [
        (PrimitiveKind::Plane, options.detect_plane),
        (PrimitiveKind::Sphere, options.detect_sphere),
        (PrimitiveKind::Cylinder, options.detect_cylinder),
        (PrimitiveKind::Cone, options.detect_cone),
    ];
    let mut out = Vec::new();
    // Единый бюджет раундов последовательного удаления инлаеров.
    let mut round_guard = Budget::with_iterations(options.max_primitives)?
        .guard("ransac.rounds");
    while pool.len() >= options.min_inliers && out.len() < options.max_primitives {
        round_guard.tick()?;
        let mut best: Option<(PrimitiveKind, PrimitiveParams, Vec<u32>)> = None;
        for &(kind, enabled) in &kinds {
            if !enabled {
                continue;
            }
            if let Some((params, inl)) = ransac_kind(kind, points, normals, &pool, options, &mut rng)?
            {
                let better = match &best {
                    None => true,
                    Some((_, _, b)) => inl.len() > b.len(),
                };
                if better {
                    best = Some((kind, params, inl));
                }
            }
        }
        let Some((kind, rough_params, inl)) = best else { break };
        if inl.len() < options.min_inliers {
            break;
        }
        // Нелинейное уточнение по инлаерам; при неудаче — грубые параметры.
        let refined = refine(kind, points, &inl).unwrap_or(rough_params);
        let inl_refined = inliers_of(points, &pool, &refined, options.threshold);
        if inl_refined.len() < options.min_inliers {
            // Уточнение ухудшило кандидата: удаляем грубые инлаеры и
            // продолжаем, чтобы не зациклиться.
            let raw: std::collections::HashSet<u32> = inl.iter().copied().collect();
            pool.retain(|i| !raw.contains(i));
            continue;
        }
        let inl = inl_refined;
        let rms = (inl
            .iter()
            .map(|&i| refined.deviation(points[i as usize]).powi(2))
            .sum::<f64>()
            / inl.len() as f64)
            .sqrt();
        // Метрика отчёта обязана быть конечной: NaN в отчёте — баг.
        numeric(rms.is_finite(), "RANSAC produced a non-finite inlier RMS")?;
        let in_pool: std::collections::HashSet<u32> = inl.iter().copied().collect();
        pool.retain(|i| !in_pool.contains(i));
        out.push(DetectedPrimitive { kind, params: refined, inlier_indices: inl, rms });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plane_points(n: usize, noise: f64, rng: &mut SplitMix64) -> Vec<[f64; 3]> {
        (0..n)
            .map(|_| {
                [
                    rng.next_f64() * 6. - 3.,
                    rng.next_f64() * 6. - 3.,
                    0.5 + noise * rng.gauss(),
                ]
            })
            .collect()
    }

    fn sphere_points(
        center: [f64; 3],
        radius: f64,
        n: usize,
        noise: f64,
        rng: &mut SplitMix64,
    ) -> Vec<[f64; 3]> {
        (0..n)
            .map(|_| {
                let u = 2. * rng.next_f64() - 1.;
                let phi = 2. * std::f64::consts::PI * rng.next_f64();
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

    #[test]
    fn detects_plane_and_sphere_in_mixture() {
        let mut rng = SplitMix64::new(101);
        let mut pts = plane_points(2500, 0.002, &mut rng);
        let sph_center = [10., 0., 0.];
        pts.extend(sphere_points(sph_center, 1.5, 1500, 0.002, &mut rng));
        let options = RansacOptions {
            threshold: 0.02,
            min_inliers: 500,
            max_iterations: 300,
            detect_cylinder: false,
            detect_cone: false,
            ..RansacOptions::default()
        };
        let found = detect_primitives(&pts, None, &options).unwrap();
        assert_eq!(found.len(), 2, "found: {:?}", found.iter().map(|d| d.kind).collect::<Vec<_>>());
        let kinds: Vec<PrimitiveKind> = found.iter().map(|d| d.kind).collect();
        assert!(kinds.contains(&PrimitiveKind::Plane));
        assert!(kinds.contains(&PrimitiveKind::Sphere));
        let sphere = found.iter().find(|d| d.kind == PrimitiveKind::Sphere).unwrap();
        if let PrimitiveParams::Sphere { center, radius } = sphere.params {
            for k in 0..3 {
                assert!((center[k] - sph_center[k]).abs() < 0.05);
            }
            assert!((radius - 1.5).abs() < 0.05);
        } else {
            panic!("wrong params");
        }
        // Индексы инлаеров валидны и не пересекаются.
        let mut seen = std::collections::HashSet::new();
        for d in &found {
            assert!(d.rms < 0.05);
            for &i in &d.inlier_indices {
                assert!((i as usize) < pts.len());
                assert!(seen.insert(i), "инлаеры примитивов пересекаются");
            }
        }
    }

    #[test]
    fn detects_plane_with_normals_cylinder_mixture() {
        // Плоскость + цилиндр; нормали заданы аналитически.
        let mut rng = SplitMix64::new(77);
        let mut pts = Vec::new();
        let mut nrm = Vec::new();
        for _ in 0..2000 {
            pts.push([
                rng.next_f64() * 6. - 3.,
                rng.next_f64() * 6. - 3.,
                0.002 * rng.gauss(),
            ]);
            nrm.push([0., 0., 1.]);
        }
        let axis = [0., 0., 1.];
        let q = [0., 0., -2.];
        for _ in 0..1800 {
            let phi = 2. * std::f64::consts::PI * rng.next_f64();
            let t = rng.next_f64() * 3.;
            let r = 1. + 0.002 * rng.gauss();
            let radial = [phi.cos(), phi.sin(), 0.];
            pts.push([
                q[0] + r * radial[0] + axis[0] * t,
                q[1] + r * radial[1] + axis[1] * t,
                q[2] + r * radial[2] + axis[2] * t,
            ]);
            nrm.push(radial);
        }
        let options = RansacOptions {
            threshold: 0.02,
            min_inliers: 500,
            max_iterations: 400,
            detect_sphere: false,
            detect_cone: false,
            ..RansacOptions::default()
        };
        let found = detect_primitives(&pts, Some(&nrm), &options).unwrap();
        assert_eq!(found.len(), 2, "found: {:?}", found.iter().map(|d| d.kind).collect::<Vec<_>>());
        let cyl = found
            .iter()
            .find(|d| d.kind == PrimitiveKind::Cylinder)
            .expect("цилиндр не найден");
        if let PrimitiveParams::Cylinder { direction, radius, .. } = cyl.params {
            assert!(dot(direction, axis).abs() > 0.999);
            assert!((radius - 1.).abs() < 0.05);
        } else {
            panic!("wrong params");
        }
    }

    #[test]
    fn validates_options() {
        let pts = plane_points(50, 0., &mut SplitMix64::new(1));
        let mut options = RansacOptions::default();
        options.threshold = 0.;
        assert!(detect_primitives(&pts, None, &options).is_err());
        let options = RansacOptions {
            detect_plane: false,
            detect_sphere: false,
            detect_cylinder: false,
            detect_cone: false,
            ..RansacOptions::default()
        };
        assert!(detect_primitives(&pts, None, &options).is_err());
    }

    #[test]
    fn rejects_non_finite_boundaries_with_param_names() {
        let pts = plane_points(50, 0., &mut SplitMix64::new(1));
        // NaN-нормаль.
        let mut nrm = vec![[0., 0., 1.]; 50];
        nrm[7][1] = f64::NAN;
        let err = detect_primitives(&pts, Some(&nrm), &RansacOptions::default()).unwrap_err();
        assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
        assert!(err.contains("normals"), "{err}");
        // NaN-порог и некорректная доля раннего выхода.
        let mut options = RansacOptions::default();
        options.threshold = f64::NAN;
        let err = detect_primitives(&pts, None, &options).unwrap_err();
        assert!(err.contains("threshold"), "{err}");
        let mut options = RansacOptions::default();
        options.early_exit_fraction = 1.5;
        assert!(detect_primitives(&pts, None, &options).is_err());
    }

    #[test]
    fn ransac_budget_stages_are_named_and_round_guard_holds() {
        // Истощение итерационного бюджета вида — типизированный resource()
        // с именем стадии.
        let budget = Budget::with_iterations(2).unwrap();
        let mut guard = budget.guard("ransac.sphere");
        guard.tick().unwrap();
        guard.tick().unwrap();
        let err = guard.tick().unwrap_err();
        assert_eq!(err.code, crate::RESOURCE_LIMIT, "{err:?}");
        assert!(err.contains("ransac.sphere"), "{err}");
        // Бюджет раундов: max_primitives = 1 — не больше одного примитива.
        let mut rng = SplitMix64::new(101);
        let pts = plane_points(1500, 0.002, &mut rng);
        let options = RansacOptions {
            threshold: 0.02,
            min_inliers: 500,
            max_primitives: 1,
            detect_sphere: false,
            detect_cylinder: false,
            detect_cone: false,
            ..RansacOptions::default()
        };
        let found = detect_primitives(&pts, None, &options).unwrap();
        assert_eq!(found.len(), 1);
        assert!(found[0].rms.is_finite());
    }

    #[test]
    fn minimal_models_are_exact() {
        let plane = plane_from3([0., 0., 2.], [1., 0., 2.], [0., 1., 2.]).unwrap();
        assert!(plane.deviation([5., 5., 2.]) < 1e-12);
        assert!((plane.deviation([5., 5., 3.]) - 1.).abs() < 1e-12);
        let s = sphere_from4(&[[1., 0., 0.], [-1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]).unwrap();
        if let PrimitiveParams::Sphere { center, radius } = s {
            assert!(norm(center) < 1e-12);
            assert!((radius - 1.).abs() < 1e-12);
        } else {
            panic!("wrong params");
        }
        let cyl = cylinder_from2n([1., 0., 0.], [1., 0., 0.], [0., 1., 5.], [0., 1., 0.]).unwrap();
        if let PrimitiveParams::Cylinder { point_on_axis, direction, radius } = cyl {
            assert!(dot(direction, [0., 0., 1.]).abs() > 1e-9);
            assert!((radius - 1.).abs() < 1e-9);
            assert!(norm([point_on_axis[0], point_on_axis[1], 0.]) < 1e-9);
        } else {
            panic!("wrong params");
        }
    }
}
