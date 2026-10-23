//! Region growing сегментация облака по согласованности нормалей и кривизне.
//!
//! Соседство — общий grid-hash из [`super::cloud_normals`]; прокси кривизны —
//! RMS локальной PCA-плоскости (`math_core::local_point_planes`),
//! нормированный на радиус окрестности. Посев — в необработанной точке с
//! минимальной кривизной; рост — очередью с проверкой угла нормалей.

use crate::analysis::cloud_normals::{GridIndex, dot3, norm3};
use crate::{Result, check, numeric, numeric_err};
use math_core::{Acceleration, local_point_planes};

/// Максимальное число точек облака.
pub const MAX_CLOUD_POINTS: usize = 65536;

/// Настройки region growing.
#[derive(Clone, Copy, Debug)]
pub struct SegmentationOptions {
    /// Порог угла между нормалями соседей, радианы (типично 15–30°).
    pub normal_angle_threshold: f64,
    /// Порог безразмерной кривизны: точки глаже него могут продолжать
    /// очередь роста (быть «внутренними»); более кривые попадают в регион,
    /// но не сеют дальше.
    pub curvature_threshold: f64,
    /// Регионы меньше этого размера отбрасываются.
    pub min_region_size: usize,
    /// Число соседей в графе роста.
    pub neighbor_count: usize,
}

impl Default for SegmentationOptions {
    fn default() -> Self {
        Self {
            normal_angle_threshold: std::f64::consts::FRAC_PI_6,
            curvature_threshold: 0.05,
            min_region_size: 16,
            neighbor_count: 12,
        }
    }
}

/// Геометрическая характеристика региона.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApproxKind {
    /// Средняя кривизна региона ниже порога.
    Planar,
    /// Регион заметно кривой, но нормали согласованы.
    Curved,
    /// Слишком мало точек для классификации.
    Unknown,
}

/// Один сегмент облака.
#[derive(Clone, Debug)]
pub struct Region {
    /// Индексы точек исходного облака.
    pub point_indices: Vec<u32>,
    /// Средняя (единичная) нормаль региона.
    pub mean_normal: [f64; 3],
    pub approx_kind: ApproxKind,
}

/// Оценка безразмерной кривизны в каждой точке: RMS локальной плоскости,
/// нормированный на максимальное соседское расстояние окрестности.
pub(crate) fn estimate_curvatures(points: &[[f64; 3]]) -> Result<Vec<f64>> {
    let planes = local_point_planes(points, points, Acceleration::Cpu)
        .map_err(|e| numeric_err(format!("local plane estimation failed: {e}")))?;
    Ok(planes
        .iter()
        .map(|lp| {
            let scale = lp.max_squared_neighbor_distance.sqrt().max(1e-12);
            lp.plane.rms_distance / scale
        })
        .collect())
}

/// Region growing по согласованности нормалей и кривизне.
///
/// - `points`, `normals` — равные по длине массивы (≤ 65536); нормали
///   ненулевые и конечные (согласовать знаки заранее можно через
///   [`super::cloud_normals::orient_normals_consistent`]).
/// - Возвращает регионы размером ≥ `options.min_region_size`, отсортированные
///   по убыванию размера.
pub fn segment_cloud(
    points: &[[f64; 3]],
    normals: &[[f64; 3]],
    options: SegmentationOptions,
) -> Result<Vec<Region>> {
    check(
        points.len() == normals.len() && points.len() >= 4,
        "Points and normals must match and contain at least 4 points",
    )?;
    check(
        points.len() <= MAX_CLOUD_POINTS,
        "Point budget of 65536 exceeded",
    )?;
    check(
        points.iter().flatten().all(|v| v.is_finite()),
        "Point coordinates must be finite",
    )?;
    for n in normals {
        numeric(
            n.iter().all(|v| v.is_finite()) && norm3(*n) > 1e-12,
            "Normals must be finite and non-zero",
        )?;
    }
    check(
        options.normal_angle_threshold > 0.
            && options.normal_angle_threshold < std::f64::consts::PI,
        "Normal angle threshold must be in (0, pi)",
    )?;
    check(
        options.curvature_threshold.is_finite() && options.curvature_threshold > 0.,
        "Curvature threshold must be positive and finite",
    )?;
    check(
        options.neighbor_count >= 3 && options.min_region_size >= 1,
        "Need at least 3 neighbors and a positive minimum region size",
    )?;
    let n = points.len();
    let unit: Vec<[f64; 3]> = normals
        .iter()
        .map(|v| {
            let l = norm3(*v);
            [v[0] / l, v[1] / l, v[2] / l]
        })
        .collect();
    let curvature = estimate_curvatures(points)?;
    let cell = GridIndex::suggested_cell(points);
    let grid = GridIndex::build(points, cell)?;
    let cos_threshold = options.normal_angle_threshold.cos();

    // Порядок посевов: по возрастанию кривизны.
    let mut seeds: Vec<usize> = (0..n).collect();
    seeds.sort_by(|&a, &b| curvature[a].total_cmp(&curvature[b]));

    let mut assigned = vec![false; n];
    let mut regions = Vec::new();
    for &seed in &seeds {
        if assigned[seed] {
            continue;
        }
        // Рост очередью: сосед принимается, если угол нормалей с текущей
        // точкой ниже порога; в очередь он ставится, только если сам гладкий.
        let mut members: Vec<u32> = vec![seed as u32];
        assigned[seed] = true;
        let mut queue: Vec<u32> = vec![seed as u32];
        let mut head = 0;
        let mut normal_sum = [0.; 3];
        while head < queue.len() {
            let cur = queue[head] as usize;
            head += 1;
            for k in 0..3 {
                normal_sum[k] += unit[cur][k];
            }
            for j in grid.k_nearest(cur, options.neighbor_count) {
                let ju = j as usize;
                if assigned[ju] {
                    continue;
                }
                if dot3(unit[cur], unit[ju]) < cos_threshold {
                    continue;
                }
                assigned[ju] = true;
                members.push(j);
                if curvature[ju] < options.curvature_threshold {
                    queue.push(j);
                }
            }
        }
        if members.len() < options.min_region_size {
            continue;
        }
        let count = members.len() as f64;
        let mean = [normal_sum[0] / count, normal_sum[1] / count, normal_sum[2] / count];
        let mean_len = norm3(mean);
        let mean_normal = if mean_len > 1e-12 {
            [mean[0] / mean_len, mean[1] / mean_len, mean[2] / mean_len]
        } else {
            [0., 0., 0.]
        };
        // Классификация: плоскость по всему региону (локальная кривизна
        // патча зависит от масштаба окрестности и ненадёжна); Planar, если
        // RMS плоскости мал относительно размера региона.
        let approx_kind = if members.len() < 9 {
            ApproxKind::Unknown
        } else {
            let region_pts: Vec<[f64; 3]> =
                members.iter().map(|&i| points[i as usize]).collect();
            let plane = math_core::point_fit_plane(&region_pts, Acceleration::Cpu)
                .map_err(|e| numeric_err(format!("region plane fit failed: {e}")))?;
            let mut lo = [f64::INFINITY; 3];
            let mut hi = [f64::NEG_INFINITY; 3];
            for p in &region_pts {
                for k in 0..3 {
                    lo[k] = lo[k].min(p[k]);
                    hi[k] = hi[k].max(p[k]);
                }
            }
            let extent = ((hi[0] - lo[0]).powi(2)
                + (hi[1] - lo[1]).powi(2)
                + (hi[2] - lo[2]).powi(2))
            .sqrt()
            .max(1e-12);
            if plane.rms_distance / extent < 0.01 {
                ApproxKind::Planar
            } else {
                ApproxKind::Curved
            }
        };
        regions.push(Region { point_indices: members, mean_normal, approx_kind });
    }
    regions.sort_by(|a, b| b.point_indices.len().cmp(&a.point_indices.len()));
    Ok(regions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::ransac_primitives::SplitMix64;

    /// Плоскость z=0 (левая половина) + вертикальный цилиндр (правая
    /// половина), нормали аналитические; на стыке нормали различны.
    fn plane_plus_cylinder() -> (Vec<[f64; 3]>, Vec<[f64; 3]>) {
        let mut rng = SplitMix64::new(5);
        let mut pts = Vec::new();
        let mut nrm = Vec::new();
        // Плоскость.
        for _ in 0..2500 {
            pts.push([
                -6. + rng.next_f64() * 5.,
                rng.next_f64() * 8. - 4.,
                0.001 * rng.gauss(),
            ]);
            nrm.push([0., 0., 1.]);
        }
        // Цилиндр радиуса 1 вокруг (3, 0), высота 4.
        for _ in 0..2500 {
            let phi = 2. * std::f64::consts::PI * rng.next_f64();
            let z = rng.next_f64() * 4. - 2.;
            let r = 1. + 0.001 * rng.gauss();
            pts.push([3. + r * phi.cos(), r * phi.sin(), z]);
            nrm.push([phi.cos(), phi.sin(), 0.]);
        }
        (pts, nrm)
    }

    #[test]
    fn separates_plane_and_cylinder() {
        let (pts, nrm) = plane_plus_cylinder();
        let regions = segment_cloud(&pts, &nrm, SegmentationOptions::default()).unwrap();
        assert!(regions.len() >= 2, "regions: {}", regions.len());
        // Два крупнейших региона: планарный и кривой.
        let kinds: Vec<ApproxKind> =
            regions.iter().take(2).map(|r| r.approx_kind).collect();
        assert!(kinds.contains(&ApproxKind::Planar), "kinds = {kinds:?}");
        assert!(kinds.contains(&ApproxKind::Curved), "kinds = {kinds:?}");
        let planar = regions
            .iter()
            .find(|r| r.approx_kind == ApproxKind::Planar)
            .unwrap();
        // Средняя нормаль планарного региона — +z.
        assert!(planar.mean_normal[2] > 0.99, "mean = {:?}", planar.mean_normal);
        // Все точки планарного региона — с плоскости (x < 0).
        for &i in &planar.point_indices {
            assert!(pts[i as usize][0] < 0.5, "точка цилиндра попала в плоскость");
        }
        // Регионы не пересекаются.
        let mut seen = std::collections::HashSet::new();
        for r in &regions {
            for &i in &r.point_indices {
                assert!(seen.insert(i));
            }
        }
    }

    #[test]
    fn rejects_bad_input() {
        let pts = vec![[0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
        let nrm = vec![[0., 0., 1.]; 4];
        // Несовпадение длин.
        assert!(segment_cloud(&pts, &nrm[..3], SegmentationOptions::default()).is_err());
        // Нулевая нормаль.
        let mut bad = nrm.clone();
        bad[0] = [0.; 3];
        assert!(segment_cloud(&pts, &bad, SegmentationOptions::default()).is_err());
        // Плохой порог угла.
        let mut options = SegmentationOptions::default();
        options.normal_angle_threshold = 0.;
        assert!(segment_cloud(&pts, &nrm, options).is_err());
    }
}
