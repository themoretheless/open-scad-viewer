//! Согласованная ориентация нормалей облака точек.
//!
//! Строит граф k-ближайших соседей (равномерная grid-hash структура без
//! зависимостей), минимальное остовное дерево по весу `1 − |nᵢ·nⱼ|` и
//! распространяет знак нормалей от корней деревьев. Общая часть соседства
//! ([`GridIndex`]) переиспользуется сегментацией (`cloud_segmentation`).
//!
//! Политика: без `unsafe`, без новых зависимостей, ошибки через `Result`.

use std::collections::HashMap;

use crate::{Result, check, numeric};

/// Максимальное число точек, обрабатываемое за один вызов.
pub const MAX_CLOUD_POINTS: usize = 65536;

/// Равномерная grid-hash структура для запросов ближайших соседей.
///
/// Ячейки — целочисленные тройки `floor(p / cell)`, соседние ячейки
/// перебираются кубическим окрестностным проходом. Используется совместно
/// модулями `cloud_normals` и `cloud_segmentation`.
pub(crate) struct GridIndex<'a> {
    points: &'a [[f64; 3]],
    cell: f64,
    buckets: HashMap<(i64, i64, i64), Vec<u32>>,
}

impl<'a> GridIndex<'a> {
    /// Строит индекс; `cell` — ребро ячейки (должно быть положительным и
    /// конечным). Обычно берут оценку среднего межточечного расстояния.
    pub(crate) fn build(points: &'a [[f64; 3]], cell: f64) -> Result<Self> {
        check(
            !points.is_empty() && points.len() <= MAX_CLOUD_POINTS,
            "Point count must be in 1..=65536",
        )?;
        check(
            cell.is_finite() && cell > 0.,
            "Grid cell size must be positive and finite",
        )?;
        let mut buckets: HashMap<(i64, i64, i64), Vec<u32>> = HashMap::new();
        for (i, p) in points.iter().enumerate() {
            buckets.entry(key(p, cell)).or_default().push(i as u32);
        }
        Ok(Self { points, cell, buckets })
    }

    /// Оценка ребра ячейки по ограничивающему параллелепипеду:
    /// `diag(bbox) / n^(1/3)`, снизу ограниченная для вырожденных облаков.
    pub(crate) fn suggested_cell(points: &[[f64; 3]]) -> f64 {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for p in points {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        let diag = ((hi[0] - lo[0]).powi(2) + (hi[1] - lo[1]).powi(2) + (hi[2] - lo[2]).powi(2))
            .sqrt();
        let scale = diag.max(1e-12);
        (scale / (points.len() as f64).cbrt()).max(scale * 1e-9)
    }

    /// Индексы точек в пределах `radius` от `p` (включая саму точку, если она
    /// в индексе). Кандидаты собираются из окрестности ячеек, затем фильтруются
    /// точным расстоянием.
    pub(crate) fn neighbors_within(&self, p: [f64; 3], radius: f64) -> Vec<u32> {
        let r = radius.max(0.);
        let reach = (r / self.cell).ceil() as i64;
        let base = key(&p, self.cell);
        let r2 = r * r;
        let mut out = Vec::new();
        for dx in -reach..=reach {
            for dy in -reach..=reach {
                for dz in -reach..=reach {
                    let k = (base.0 + dx, base.1 + dy, base.2 + dz);
                    if let Some(bucket) = self.buckets.get(&k) {
                        for &i in bucket {
                            let q = self.points[i as usize];
                            let d2 = (q[0] - p[0]).powi(2)
                                + (q[1] - p[1]).powi(2)
                                + (q[2] - p[2]).powi(2);
                            if d2 <= r2 {
                                out.push(i);
                            }
                        }
                    }
                }
            }
        }
        out
    }

    /// `k` ближайших соседей точки `idx` (без неё самой), по возрастанию
    /// расстояния. Радиус поиска расширяется, пока не наберётся `k`
    /// кандидатов; при изолированных точках — полный проход как запасной путь.
    pub(crate) fn k_nearest(&self, idx: usize, k: usize) -> Vec<u32> {
        let p = self.points[idx];
        let k = k.min(self.points.len().saturating_sub(1));
        if k == 0 {
            return Vec::new();
        }
        let mut radius = self.cell;
        let mut cand: Vec<u32> = Vec::new();
        for _ in 0..32 {
            cand = self.neighbors_within(p, radius);
            cand.retain(|&i| i as usize != idx);
            if cand.len() >= k {
                break;
            }
            radius *= 2.;
        }
        if cand.len() < k {
            // Запасной путь для сильно неоднородной плотности.
            cand = (0..self.points.len() as u32).filter(|&i| i as usize != idx).collect();
        }
        cand.sort_by(|&a, &b| {
            dist2(self.points[a as usize], p).total_cmp(&dist2(self.points[b as usize], p))
        });
        cand.truncate(k);
        cand
    }
}

fn key(p: &[f64; 3], cell: f64) -> (i64, i64, i64) {
    (
        (p[0] / cell).floor() as i64,
        (p[1] / cell).floor() as i64,
        (p[2] / cell).floor() as i64,
    )
}

pub(crate) fn dist2(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)
}

pub(crate) fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub(crate) fn norm3(a: [f64; 3]) -> f64 {
    dot3(a, a).sqrt()
}

/// Обертка над `f64` с тотальным порядком для бинарной кучи.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Ordered(f64);
impl Eq for Ordered {}
impl PartialOrd for Ordered {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Ordered {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

/// Ребро в куче Прима; минимальный вес извлекается первым (инверсия).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Edge {
    weight: Ordered,
    from: u32,
    to: u32,
}
impl Ord for Edge {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .weight
            .cmp(&self.weight)
            .then_with(|| self.to.cmp(&other.to))
    }
}
impl PartialOrd for Edge {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Отчёт об ориентации: согласованные нормали и диагностика.
#[derive(Clone, Debug)]
pub struct OrientReport {
    /// Согласованно ориентированные нормали (едичные), в порядке входа.
    pub normals: Vec<[f64; 3]>,
    /// Сколько нормалей было перевёрнуто.
    pub flipped: usize,
    /// Число компонент связности графа соседства.
    pub components: usize,
}

/// Согласованная ориентация нормалей облака (Hoppe'92, упрощённо).
///
/// - `points` — 3D-точки, `normals` — единичные (приблизительно) нормали,
///   возможно с разнонаправленными знаками; обе длины совпадают.
/// - Строится граф взаимного k-соседства (k = 12), вес ребра `1 − |nᵢ·nⱼ|`,
///   лес минимальных остовных деревьев (Prim от каждой необработанной
///   вершины), знак распространяется от корня: сосед переворачивается, если
///   `n_parent · n_child < 0`. Остов согласует знаки лишь с точностью до
///   общего флипа компоненты, поэтому каждая компонента доворачивается
///   «наружу»: знак выбирается так, чтобы `Σ nᵢ·(pᵢ − центроид)` ≥ 0.
///
/// Возвращает [`OrientReport`]; ошибка — при несовпадении длин, пустом входе,
/// нулевых/неконечных нормалях или превышении бюджета точек.
pub fn orient_normals_consistent(
    points: &[[f64; 3]],
    normals: &[[f64; 3]],
) -> Result<OrientReport> {
    check(
        points.len() == normals.len() && !points.is_empty(),
        "Points and normals must be non-empty and of equal length",
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
    let n = points.len();
    // Нормировка входных нормалей.
    let mut unit: Vec<[f64; 3]> = Vec::with_capacity(n);
    for v in normals {
        let l = norm3(*v);
        unit.push([v[0] / l, v[1] / l, v[2] / l]);
    }
    let cell = GridIndex::suggested_cell(points);
    let grid = GridIndex::build(points, cell)?;
    let k = 12.min(n - 1);
    // Список рёбер взаимного k-соседства: (i, j, weight).
    let mut adj: Vec<Vec<(u32, f64)>> = vec![Vec::new(); n];
    for i in 0..n {
        for j in grid.k_nearest(i, k) {
            let ju = j as usize;
            let w = 1. - dot3(unit[i], unit[ju]).abs();
            adj[i].push((j, w));
            adj[ju].push((i as u32, w));
        }
    }
    // Лес прима: parent[u] = откуда пришла вершина; корни имеют parent = u32::MAX.
    let mut parent = vec![u32::MAX; n];
    let mut visited = vec![false; n];
    let mut components = 0usize;
    for root in 0..n {
        if visited[root] {
            continue;
        }
        components += 1;
        visited[root] = true;
        // Prim с бинарной кучей (ленивое удаление устаревших записей).
        let mut heap = std::collections::BinaryHeap::new();
        for &(j, w) in &adj[root] {
            heap.push(Edge { weight: Ordered(w), from: root as u32, to: j });
        }
        while let Some(Edge { weight, from, to }) = heap.pop() {
            let tu = to as usize;
            if visited[tu] {
                continue;
            }
            visited[tu] = true;
            parent[tu] = from;
            let _ = weight;
            for &(j, w) in &adj[tu] {
                if !visited[j as usize] {
                    heap.push(Edge { weight: Ordered(w), from: to, to: j });
                }
            }
        }
    }
    // Распространение знака от корней по дереву (порядок вершин согласован с
    // parent-ссылками: обрабатываем в порядке обхода через очередь).
    let mut sign = vec![1.0f64; n];
    let mut comp = vec![usize::MAX; n];
    // Топологический порядок: простая очередь от корней.
    let mut queue: Vec<usize> = (0..n).filter(|&i| parent[i] == u32::MAX).collect();
    for &r in &queue {
        comp[r] = r;
    }
    let mut head = 0;
    // Дети: строим списки для обхода.
    let mut children: Vec<Vec<u32>> = vec![Vec::new(); n];
    for (i, &p) in parent.iter().enumerate() {
        if p != u32::MAX {
            children[p as usize].push(i as u32);
        }
    }
    while head < queue.len() {
        let u = queue[head];
        head += 1;
        for &c in &children[u] {
            let cu = c as usize;
            sign[cu] = if dot3(unit[u], unit[cu]) < 0. { -sign[u] } else { sign[u] };
            comp[cu] = comp[u];
            queue.push(cu);
        }
    }
    // Глобальный знак компоненты: MST согласует нормали лишь с точностью до
    // общего флипа, поэтому каждую компоненту доворачиваем «наружу» — так,
    // чтобы Σ nᵢ·(pᵢ − центроид компоненты) был неотрицательным.
    let mut comp_sum: HashMap<usize, f64> = HashMap::new();
    let mut comp_centroid: HashMap<usize, [f64; 3]> = HashMap::new();
    let mut comp_count: HashMap<usize, f64> = HashMap::new();
    for i in 0..n {
        let c = comp[i];
        let e = comp_centroid.entry(c).or_insert([0.; 3]);
        for k in 0..3 {
            e[k] += points[i][k];
        }
        *comp_count.entry(c).or_insert(0.) += 1.;
    }
    for (c, e) in comp_centroid.iter_mut() {
        let cnt = comp_count[c];
        for k in 0..3 {
            e[k] /= cnt;
        }
    }
    for i in 0..n {
        let c = comp[i];
        let ctr = comp_centroid[&c];
        let d = [
            points[i][0] - ctr[0],
            points[i][1] - ctr[1],
            points[i][2] - ctr[2],
        ];
        let signed = [unit[i][0] * sign[i], unit[i][1] * sign[i], unit[i][2] * sign[i]];
        *comp_sum.entry(c).or_insert(0.) += dot3(signed, d);
    }
    let mut flipped = 0usize;
    for i in 0..n {
        if comp_sum[&comp[i]] < 0. {
            sign[i] = -sign[i];
        }
        if sign[i] < 0. {
            flipped += 1;
        }
    }
    let out: Vec<[f64; 3]> = unit
        .iter()
        .zip(&sign)
        .map(|(v, s)| [v[0] * s, v[1] * s, v[2] * s])
        .collect();
    Ok(OrientReport { normals: out, flipped, components })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Детерминированный SplitMix64 для тестов.
    struct Rng(u64);
    impl Rng {
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
    }

    fn sphere_cloud(samples: usize, seed: u64) -> (Vec<[f64; 3]>, Vec<[f64; 3]>) {
        let mut rng = Rng(seed);
        let mut pts = Vec::with_capacity(samples);
        let mut nrm = Vec::with_capacity(samples);
        for i in 0..samples {
            // Равномерные направления по сфере (Марсалья).
            let u = 2. * rng.f64() - 1.;
            let phi = 2. * std::f64::consts::PI * rng.f64();
            let s = (1. - u * u).sqrt();
            let d = [s * phi.cos(), s * phi.sin(), u];
            pts.push(d);
            // Случайный знак: часть нормалей направлена внутрь.
            let sign = if i % 3 == 0 { -1. } else { 1. };
            nrm.push([d[0] * sign, d[1] * sign, d[2] * sign]);
        }
        (pts, nrm)
    }

    #[test]
    fn orients_sphere_normals_outward() {
        let (pts, nrm) = sphere_cloud(1500, 42);
        let report = orient_normals_consistent(&pts, &nrm).unwrap();
        assert_eq!(report.normals.len(), pts.len());
        assert_eq!(report.components, 1);
        // Все нормали согласованы с радиальным направлением.
        for (p, n) in pts.iter().zip(&report.normals) {
            assert!(dot3(*p, *n) > 0.999, "dot = {}", dot3(*p, *n));
        }
        assert!(report.flipped > 0, "ожидались перевороты исходных знаков");
    }

    #[test]
    fn rejects_mismatched_lengths() {
        let pts = vec![[0.; 3], [1., 0., 0.]];
        let nrm = vec![[0., 0., 1.]];
        assert!(orient_normals_consistent(&pts, &nrm).is_err());
    }

    #[test]
    fn rejects_zero_normal() {
        let pts = vec![[0.; 3], [1., 0., 0.]];
        let nrm = vec![[0., 0., 1.], [0.; 3]];
        assert!(orient_normals_consistent(&pts, &nrm).is_err());
    }

    #[test]
    fn grid_finds_k_nearest() {
        // Решётка точек на прямой: ближайшие соседи точки 0 — 1 и 2.
        let pts: Vec<[f64; 3]> = (0..100).map(|i| [i as f64, 0., 0.]).collect();
        let grid = GridIndex::build(&pts, 0.5).unwrap();
        let nn = grid.k_nearest(0, 2);
        assert_eq!(nn, vec![1, 2]);
        let nn = grid.k_nearest(50, 2);
        assert_eq!(nn, vec![49, 51]);
    }
}
