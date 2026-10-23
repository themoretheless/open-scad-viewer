//! Геометрический вотермаркинг NURBS-поверхности в пределах допуска (item 981).
//!
//! Payload (16–32 байта и т.п.) кодируется помехоустойчивым кодом
//! (повторный код ×3 с мажоритарным декодированием + бит чётности на байт,
//! реализован здесь же, без внешних зависимостей) в последовательность бит.
//! Биты модулируют коэффициенты низкочастотных двумерных базисных функций
//! Б-сплайна: выбираются первые `k` базисных функций по (u,v)-пирамиде
//! (слои `i + j = const`), и соответствующие контрольные точки сдвигаются
//! вдоль локальной нормали поверхности (в точке Гревиля базиса) на
//! `±a`, где амплитуда `a` строго ниже заданного допуска. Каждая
//! базисная функция несёт не более одного кодового бита, поэтому сдвиг
//! любой контрольной точки не превышает `a`, а по свойству выпуклой
//! комбинации отклонение поверхности не превышает сдвига контрольных
//! точек; тем не менее фактическое отклонение обязательно проверяется
//! функцией [`max_deviation`] по плотной сетке с запасом, и превышение
//! допуска — check-ошибка, молчаливого превышения нет.
//!
//! Ключ (seed) раскручивает детерминированный генератор SplitMix64,
//! который перемешивает назначение «бит → базисная функция» (тасование
//! Фишера–Йетса) и выбирает знак каждого бита. Извлечение —
//! корреляционный приёмник: проекция смещений контрольных точек
//! (относительно референса без вотермарка либо относительно сглаженной
//! лапласианом версии самой размеченной поверхности) на нормали в точках
//! Гревиля; знак корреляции даёт биполярное значение бита, ECC исправляет
//! одиночные ошибки. Отчёт содержит число исправленных бит, уверенности
//! корреляций и максимальное внесённое отклонение (при наличии референса).
//!
//! Стойкость: извлечение работает по контрольным точкам, а не по мешу,
//! поэтому инвариантно к ретесселяции; перед сравнением с референсом обе
//! поверхности приводятся в канонический вид PCA-каркасом контрольной сети
//! (центроид, собственные вектора ковариации, собственный поворот —
//! det = +1, детерминированные знаки осей) с компенсацией остаточного
//! поворота между каркасами — это даёт инвариантность к rigid-движениям
//! и равномерному масштабу. Документированные уязвимости: агрессивный
//! healing/фэйринг контрольной сетки разрушает низкоамплитудную модуляцию;
//! при (почти) вырожденном спектре ковариации сетки (куб, шар) выбор осей
//! PCA нестабилен и стойкость к поворотам не гарантируется.

use crate::foundation::guards::{hash_slice, require_finite_f64};
use crate::{Result, check, numeric, resource, surface::Surface};

/// Кратность повторного кода (мажоритарное декодирование).
const REPETITIONS: usize = 3;
/// Бюджет: максимальный размер payload в байтах.
pub const MAX_PAYLOAD_BYTES: usize = 64;
/// Бюджет: максимальный размер сетки проверки отклонения по одной оси.
pub const MAX_GRID: usize = 129;
/// Размер сетки проверки отклонения по умолчанию (с запасом).
pub const DEFAULT_GRID: usize = 65;
/// Число сглаживающих итераций для безреференсного извлечения.
const SMOOTHING_ROUNDS: usize = 2;

/// Детерминированный генератор SplitMix64 (локальный, как в
/// `analysis/ransac_primitives.rs`; без внешних зависимостей).
#[derive(Clone, Debug)]
struct SplitMix64(u64);

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self(seed)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn next_bit(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

/// Параметры внедрения вотермарка.
#[derive(Clone, Copy, Debug)]
pub struct WatermarkOptions {
    /// Явная амплитуда сдвига контрольных точек. Должна быть строго ниже
    /// допуска, иначе — check-ошибка. По умолчанию `0.1 * tolerance`.
    pub amplitude: Option<f64>,
    /// Размер сетки проверки отклонения по одной оси (9..=MAX_GRID).
    pub grid: usize,
}

impl Default for WatermarkOptions {
    fn default() -> Self {
        Self {
            amplitude: None,
            grid: DEFAULT_GRID,
        }
    }
}

/// Отчёт о внедрении.
#[derive(Clone, Debug)]
pub struct EmbedReport {
    /// Размеченная поверхность.
    pub surface: Surface,
    /// Применённая амплитуда сдвига контрольных точек.
    pub amplitude: f64,
    /// Фактическое максимальное отклонение поверхности (по сетке).
    pub max_deviation: f64,
    /// Число кодовых бит после ECC.
    pub coded_bits: usize,
    /// Число задействованных базисных функций.
    pub basis_used: usize,
}

/// Отчёт об извлечении.
#[derive(Clone, Debug)]
pub struct ExtractReport {
    /// Восстановленный payload.
    pub payload: Vec<u8>,
    /// Число кодовых бит, исправленных ECC (неодногласные тройки).
    pub corrected_bits: usize,
    /// Число байт, у которых не сошёлся бит чётности после исправления.
    pub parity_failures: usize,
    /// Средняя уверенность (нормированная корреляция) по битам payload.
    pub mean_confidence: f64,
    /// Минимальная уверенность по битам payload.
    pub min_confidence: f64,
    /// Максимальное отклонение от референса, если референс задан.
    pub max_deviation: Option<f64>,
}

/// ECC: payload-байты → 9 бит на байт (8 данных + чётность), каждый бит
/// повторяется REPETITIONS раз. ~50 строк, без внешних зависимостей.
fn ecc_encode(payload: &[u8]) -> Vec<bool> {
    let mut out = Vec::with_capacity(payload.len() * 9 * REPETITIONS);
    for &byte in payload {
        let mut bits: Vec<bool> = (0..8).map(|b| byte >> b & 1 == 1).collect();
        bits.push(bits.iter().fold(false, |acc, &b| acc ^ b));
        for bit in bits {
            out.extend(std::iter::repeat(bit).take(REPETITIONS));
        }
    }
    out
}

/// ECC-декодирование: мажоритарное голосование по тройкам, затем проверка
/// чётности каждого байта. Возвращает (payload, исправленные тройки,
/// байты с несошедшейся чётностью).
fn ecc_decode(coded: &[bool], payload_len: usize) -> (Vec<u8>, usize, usize) {
    let mut corrected = 0;
    let mut payload = Vec::with_capacity(payload_len);
    let mut parity_failures = 0;
    for byte_index in 0..payload_len {
        let base = byte_index * 9 * REPETITIONS;
        let mut bits = [false; 9];
        for (i, bit) in bits.iter_mut().enumerate() {
            let triple = &coded[base + i * REPETITIONS..base + (i + 1) * REPETITIONS];
            let ones = triple.iter().filter(|&&b| b).count();
            *bit = ones * 2 > REPETITIONS;
            if ones != 0 && ones != REPETITIONS {
                corrected += 1;
            }
        }
        let byte: u8 = bits[..8]
            .iter()
            .enumerate()
            .map(|(b, &bit)| (bit as u8) << b)
            .sum();
        if bits[..8].iter().fold(false, |acc, &b| acc ^ b) != bits[8] {
            parity_failures += 1;
        }
        payload.push(byte);
    }
    (payload, corrected, parity_failures)
}

/// Первые `k` двумерных базисных функций по (u,v)-пирамиде: слои
/// `i + j = 0, 1, 2, ...`, внутри слоя — по возрастанию `j`.
/// Возвращает индексы (i, j) контрольной сетки nu × nv.
fn pyramid_basis(nu: usize, nv: usize, k: usize) -> Vec<(usize, usize)> {
    let mut order: Vec<(usize, usize)> = (0..nu)
        .flat_map(|i| (0..nv).map(move |j| (i, j)))
        .collect();
    order.sort_by_key(|&(i, j)| (i + j, j));
    order.truncate(k);
    order
}

/// Тасование Фишера–Йетса и ключевые знаки: назначение кодовых бит на
/// базисные функции и знак модуляции каждого бита.
fn key_schedule(key: u64, k: usize) -> (Vec<usize>, Vec<bool>) {
    let mut rng = SplitMix64::new(key);
    let mut permutation: Vec<usize> = (0..k).collect();
    for i in (1..k).rev() {
        let j = (rng.next_u64() % (i as u64 + 1)) as usize;
        permutation.swap(i, j);
    }
    let signs: Vec<bool> = (0..k).map(|_| rng.next_bit()).collect();
    (permutation, signs)
}

/// Нормаль поверхности в точке Гревиля базисной функции (i, j).
fn greville_normal(surface: &Surface, i: usize, j: usize) -> Result<[f64; 3]> {
    let greville = |degree: usize, knots: &[f64], index: usize| -> f64 {
        knots[index + 1..index + 1 + degree].iter().sum::<f64>() / degree as f64
    };
    let u = greville(surface.degree_u, &surface.knots_u, i);
    let v = greville(surface.degree_v, &surface.knots_v, j);
    let evaluation = surface.evaluate(u, v)?;
    numeric(
        evaluation.unit_normal().is_some(),
        "Watermark requires a regular surface normal at the basis Greville point.",
    )?;
    Ok(evaluation.unit_normal().unwrap())
}

/// Детерминированный дайджест контрольной сети и узловых векторов
/// поверхности: FNV-1a над каноническими битами координат
/// (guards::hash_slice, item 1094). −0.0 и +0.0 дают одинаковый дайджест;
/// порядок обхода фиксирован (u-строки, затем v), поэтому дайджест
/// воспроизводим между запусками и платформами. Используется для проверок
/// «разметка не изменила топологию/привязку» и регрессионного пиннинга.
pub fn digest(surface: &Surface) -> Result<u64> {
    surface.validate()?;
    let mut values: Vec<f64> = Vec::with_capacity(
        surface.knots_u.len()
            + surface.knots_v.len()
            + surface.control_points.len() * surface.control_points[0].len() * 3
            + 2,
    );
    values.push(surface.degree_u as f64);
    values.push(surface.degree_v as f64);
    values.extend(surface.knots_u.iter().copied());
    values.extend(surface.knots_v.iter().copied());
    for point in surface.control_points.iter().flatten() {
        values.extend(point.iter().copied());
    }
    Ok(hash_slice(&values))
}

/// Максимальное отклонение между двумя поверхностями с одинаковой
/// параметризацией по сетке grid × grid (обязательный шаг embed'а).
pub fn max_deviation(before: &Surface, after: &Surface, grid: usize) -> Result<f64> {
    before.validate()?;
    after.validate()?;
    check(
        grid >= 9,
        "Watermark deviation grid must have at least 9 samples per axis.",
    )?;
    if grid > MAX_GRID {
        return Err(resource("Watermark deviation grid exceeds the sampling budget."));
    }
    let (domain_u, domain_v) = before.evaluate_validated(
        before.knots_u[before.degree_u],
        before.knots_v[before.degree_v],
    )?.domains();
    let mut max: f64 = 0.;
    for iu in 0..grid {
        let u = domain_u[0] + (domain_u[1] - domain_u[0]) * iu as f64 / (grid - 1) as f64;
        for iv in 0..grid {
            let v = domain_v[0] + (domain_v[1] - domain_v[0]) * iv as f64 / (grid - 1) as f64;
            let a = before.evaluate_validated(u, v)?.point;
            let b = after.evaluate_validated(u, v)?.point;
            let d = ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
            numeric(d.is_finite(), "Watermark deviation sampling hit a non-finite point.")?;
            max = max.max(d);
        }
    }
    Ok(max)
}

/// Собственные вектора симметричной 3×3 матрицы: детерминированные
/// вращения Якоби (фиксированный бюджет проходов), столбцы — ось + значение.
fn jacobi_eigen3(mut a: [[f64; 3]; 3]) -> ([[f64; 3]; 3], [f64; 3]) {
    let mut v = [[0.; 3]; 3];
    for (d, row) in v.iter_mut().enumerate() {
        row[d] = 1.;
    }
    for _ in 0..32 {
        // Наибольший внедиагональный элемент.
        let (mut p, mut q, mut largest) = (0, 1, 0.);
        for i in 0..3 {
            for j in i + 1..3 {
                if a[i][j].abs() > largest {
                    largest = a[i][j].abs();
                    p = i;
                    q = j;
                }
            }
        }
        if largest < 1e-15 {
            break;
        }
        let theta = (a[q][q] - a[p][p]) / (2. * a[p][q]);
        let t = theta.signum() / (theta.abs() + (theta * theta + 1.).sqrt());
        let (cos, sin) = {
            let c = 1. / (t * t + 1.).sqrt();
            (c, t * c)
        };
        for k in 0..3 {
            let (akp, akq) = (a[k][p], a[k][q]);
            a[k][p] = cos * akp - sin * akq;
            a[k][q] = sin * akp + cos * akq;
        }
        for k in 0..3 {
            let (apk, aqk) = (a[p][k], a[q][k]);
            a[p][k] = cos * apk - sin * aqk;
            a[q][k] = sin * apk + cos * aqk;
        }
        for k in 0..3 {
            let (vkp, vkq) = (v[k][p], v[k][q]);
            v[k][p] = cos * vkp - sin * vkq;
            v[k][q] = sin * vkp + cos * vkq;
        }
    }
    (v, [a[0][0], a[1][1], a[2][2]])
}

/// Жёсткий каркас нормировки контрольной сети: центроид, ортонормированный
/// PCA-базис ковариации (строки `rotation` — оси с детерминированным
/// порядком и знаком), масштаб — диагональ AABB в выровненном базисе.
/// Извлечение вычисляет каркас один раз по РАЗМЕЧЕННОЙ поверхности и
/// применяет его же к референсу: независимые каркасы разошлись бы на
/// величину смещений и заняли бы корреляции членами ε·‖p‖.
/// При вырожденных собственных значениях (симметричная сетка) стойкость
/// к поворотам не гарантируется — документированное ограничение.
#[derive(Clone, Copy, Debug)]
struct RigidFrame {
    centroid: [f64; 3],
    rotation: [[f64; 3]; 3],
    scale: f64,
}

fn rigid_frame(surface: &Surface) -> Result<RigidFrame> {
    let points: Vec<&Vec<f64>> = surface.control_points.iter().flatten().collect();
    let n = points.len() as f64;
    let centroid = std::array::from_fn::<_, 3, _>(|d| {
        points.iter().map(|p| p[d]).sum::<f64>() / n
    });
    let mut covariance = [[0.; 3]; 3];
    for p in &points {
        let r = std::array::from_fn::<_, 3, _>(|d| p[d] - centroid[d]);
        for i in 0..3 {
            for j in 0..3 {
                covariance[i][j] += r[i] * r[j];
            }
        }
    }
    let (axes, values) = jacobi_eigen3(covariance);
    numeric(
        values.iter().all(|v| v.is_finite()),
        "Watermark rigid normalization hit a non-finite covariance spectrum.",
    )?;
    // Порядок осей по убыванию собственного значения.
    let mut order = [0, 1, 2];
    order.sort_by(|&a, &b| values[b].total_cmp(&values[a]));
    let centered: Vec<[f64; 3]> = points
        .iter()
        .map(|p| std::array::from_fn(|d| p[d] - centroid[d]))
        .collect();
    let aligned: Vec<[f64; 3]> = centered
        .iter()
        .map(|r| {
            std::array::from_fn(|d| {
                let axis = order[d];
                (0..3).map(|k| r[k] * axes[k][axis]).sum::<f64>()
            })
        })
        .collect();
    // Знак оси: сумма кубов координат положительна; при нуле — знак
    // координаты с максимальным модулем (детерминированный tie-break).
    let mut sign = [1.; 3];
    for d in 0..3 {
        let skew: f64 = aligned.iter().map(|p| p[d].powi(3)).sum();
        let flip = if skew != 0. {
            skew < 0.
        } else {
            let pivot = aligned
                .iter()
                .map(|p| p[d])
                .max_by(|a, b| a.abs().total_cmp(&b.abs()))
                .unwrap_or(0.);
            pivot < 0.
        };
        if flip {
            sign[d] = -1.;
        }
    }
    let mut rotation = [[0.; 3]; 3];
    for d in 0..3 {
        for k in 0..3 {
            rotation[d][k] = sign[d] * axes[k][order[d]];
        }
    }
    // Каркас обязан быть собственным поворотом: при отражении
    // (det = −1) нормали размеченной поверхности инвертируются относительно
    // референса и приёмник читает все биты наоборот.
    let [r0, r1, r2] = rotation;
    let det = r0[0] * (r1[1] * r2[2] - r1[2] * r2[1])
        - r0[1] * (r1[0] * r2[2] - r1[2] * r2[0])
        + r0[2] * (r1[0] * r2[1] - r1[1] * r2[0]);
    if det < 0. {
        for k in 0..3 {
            rotation[2][k] = -rotation[2][k];
        }
        sign[2] = -sign[2];
    }
    let (mut min, mut max) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
    for p in &aligned {
        for d in 0..3 {
            let v = sign[d] * p[d];
            min[d] = min[d].min(v);
            max[d] = max[d].max(v);
        }
    }
    let scale = (0..3).map(|d| (max[d] - min[d]).powi(2)).sum::<f64>().sqrt();
    numeric(
        scale.is_finite() && scale > 0.,
        "Watermark rigid normalization requires a non-degenerate control net.",
    )?;
    Ok(RigidFrame {
        centroid,
        rotation,
        scale,
    })
}

/// Применение каркаса: p ↦ rotation·(p − centroid)/scale.
fn apply_frame(surface: &Surface, frame: &RigidFrame) -> Surface {
    let mut out = surface.clone();
    for point in out.control_points.iter_mut().flatten() {
        let r = std::array::from_fn::<_, 3, _>(|d| point[d] - frame.centroid[d]);
        let aligned = std::array::from_fn::<_, 3, _>(|d| {
            (0..3).map(|k| frame.rotation[d][k] * r[k]).sum::<f64>() / frame.scale
        });
        for d in 0..3 {
            point[d] = aligned[d];
        }
    }
    out
}

/// Ортогональный остаточный поворот между двумя нормированными сетками
/// (полярный множитель кросс-ковариации, Кабш без SVD — через Якоби
/// собственное разложение HᵀH). Независимые PCA-каркасы размеченной
/// поверхности и референса расходятся на величину порядка внесённых
/// смещений; без компенсации член ε·‖p‖ сравним с сигналом и портит
/// корреляции. Поворот оценивается уже в канонических координатах, поэтому
/// инвариантность к rigid/scale сохраняется.
fn relative_rotation(marked: &[[f64; 3]], reference: &[[f64; 3]]) -> Result<[[f64; 3]; 3]> {
    let mut h = [[0.; 3]; 3];
    for (m, r) in marked.iter().zip(reference) {
        for d in 0..3 {
            for k in 0..3 {
                h[d][k] += m[d] * r[k];
            }
        }
    }
    let mut hth = [[0.; 3]; 3];
    for k in 0..3 {
        for l in 0..3 {
            hth[k][l] = (0..3).map(|d| h[d][k] * h[d][l]).sum();
        }
    }
    let (v, lambda) = jacobi_eigen3(hth);
    numeric(
        lambda.iter().all(|l| l.is_finite() && *l > 1e-30),
        "Watermark residual rotation requires a non-degenerate control net covariance.",
    )?;
    // (HᵀH)^{-1/2} = V diag(λ^{-1/2}) Vᵀ.
    let mut inv_sqrt = [[0.; 3]; 3];
    for k in 0..3 {
        for l in 0..3 {
            inv_sqrt[k][l] = (0..3)
                .map(|e| v[k][e] * v[l][e] / lambda[e].sqrt())
                .sum();
        }
    }
    let mut w = [[0.; 3]; 3];
    for d in 0..3 {
        for l in 0..3 {
            w[d][l] = (0..3).map(|k| h[d][k] * inv_sqrt[k][l]).sum();
        }
    }
    Ok(w)
}

/// Безреференсный референс: лапласиановское сглаживание контрольной сети
/// (несколько итераций Жакоби); граничные точки усредняются по
/// существующим соседям, иначе их смещения зануляли бы корреляцию.
fn smoothed(surface: &Surface) -> Surface {
    let nu = surface.control_points.len();
    let nv = surface.control_points[0].len();
    let mut current = surface.control_points.clone();
    for _ in 0..SMOOTHING_ROUNDS {
        let mut next = current.clone();
        for i in 0..nu {
            for j in 0..nv {
                let mut count = 0.;
                let mut sum = [0.; 3];
                for (di, dj) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                    let (ii, jj) = (i as i32 + di, j as i32 + dj);
                    if ii < 0 || jj < 0 || ii >= nu as i32 || jj >= nv as i32 {
                        continue;
                    }
                    count += 1.;
                    for d in 0..3 {
                        sum[d] += current[ii as usize][jj as usize][d];
                    }
                }
                for d in 0..3 {
                    let average = sum[d] / count;
                    next[i][j][d] = (current[i][j][d] + average) / 2.;
                }
            }
        }
        current = next;
    }
    Surface {
        control_points: current,
        ..surface.clone()
    }
}

/// Внедрение вотермарка: payload модулирует низкочастотные базисные
/// функции сдвигом контрольных точек вдоль нормалей на ±a, a < tolerance.
pub fn embed(
    surface: &Surface,
    payload: &[u8],
    key: u64,
    tolerance: f64,
    options: WatermarkOptions,
) -> Result<EmbedReport> {
    surface.validate()?;
    check(
        !payload.is_empty() && payload.len() <= MAX_PAYLOAD_BYTES,
        "Watermark payload must be 1..=64 bytes.",
    )?;
    require_finite_f64(tolerance, "watermark_tolerance")?;
    check(
        tolerance > 0.,
        "Watermark tolerance must be finite and positive.",
    )?;
    let amplitude = match options.amplitude {
        Some(a) => {
            require_finite_f64(a, "watermark_amplitude")?;
            check(
                a > 0.,
                "Watermark amplitude must be finite and positive.",
            )?;
            check(
                a < tolerance,
                "Watermark amplitude must be strictly below the tolerance.",
            )?;
            a
        }
        None => tolerance / 10.,
    };
    let coded = ecc_encode(payload);
    let nu = surface.control_points.len();
    let nv = surface.control_points[0].len();
    check(
        coded.len() <= nu * nv,
        "Watermark payload does not fit the control net basis pyramid.",
    )?;
    let basis_order = pyramid_basis(nu, nv, coded.len());
    let (permutation, signs) = key_schedule(key, coded.len());
    let mut marked = surface.clone();
    for (bit_index, &bit) in coded.iter().enumerate() {
        let (i, j) = basis_order[permutation[bit_index]];
        let normal = greville_normal(surface, i, j)?;
        let bipolar = if bit { 1. } else { -1. };
        let keyed = if signs[bit_index] { 1. } else { -1. };
        let shift = amplitude * bipolar * keyed;
        for d in 0..3 {
            marked.control_points[i][j][d] += shift * normal[d];
        }
    }
    marked.validate()?;
    let deviation = max_deviation(surface, &marked, options.grid)?;
    check(
        deviation <= tolerance,
        "Watermark embedding exceeded the surface tolerance; reduce the amplitude.",
    )?;
    Ok(EmbedReport {
        surface: marked,
        amplitude,
        max_deviation: deviation,
        coded_bits: coded.len(),
        basis_used: coded.len(),
    })
}

/// Извлечение вотермарка корреляционным приёмником: проекция смещений
/// контрольных точек относительно референса (приведённого PCA-каркасом)
/// либо
/// относительно сглаженной версии на нормали в точках Гревиля.
pub fn extract(
    marked: &Surface,
    key: u64,
    payload_len: usize,
    reference: Option<&Surface>,
) -> Result<ExtractReport> {
    marked.validate()?;
    check(
        payload_len >= 1 && payload_len <= MAX_PAYLOAD_BYTES,
        "Watermark payload length must be 1..=64 bytes.",
    )?;
    let nu = marked.control_points.len();
    let nv = marked.control_points[0].len();
    let coded_len = payload_len * 9 * REPETITIONS;
    check(
        coded_len <= nu * nv,
        "Watermark payload length does not fit the control net basis pyramid.",
    )?;
    let (marked_norm, reference_norm, deviation) = match reference {
        Some(reference) => {
            reference.validate()?;
            check(
                reference.control_points.len() == nu
                    && reference.control_points[0].len() == nv
                    && reference.degree_u == marked.degree_u
                    && reference.degree_v == marked.degree_v,
                "Watermark reference must share the marked surface topology.",
            )?;
            let marked_norm = apply_frame(marked, &rigid_frame(marked)?);
            let mut normalized = apply_frame(reference, &rigid_frame(reference)?);
            // Компенсация остаточного поворота между независимыми каркасами.
            let marked_points: Vec<[f64; 3]> = marked_norm
                .control_points
                .iter()
                .flatten()
                .map(|p| [p[0], p[1], p[2]])
                .collect();
            let reference_points: Vec<[f64; 3]> = normalized
                .control_points
                .iter()
                .flatten()
                .map(|p| [p[0], p[1], p[2]])
                .collect();
            let warp = relative_rotation(&marked_points, &reference_points)?;
            for point in normalized.control_points.iter_mut().flatten() {
                let rotated = std::array::from_fn::<_, 3, _>(|d| {
                    (0..3).map(|k| warp[d][k] * point[k]).sum::<f64>()
                });
                for d in 0..3 {
                    point[d] = rotated[d];
                }
            }
            let dev = max_deviation(&normalized, &marked_norm, DEFAULT_GRID)?;
            (marked_norm, normalized, Some(dev))
        }
        None => (marked.clone(), smoothed(marked), None),
    };
    let basis_order = pyramid_basis(nu, nv, coded_len);
    let (permutation, signs) = key_schedule(key, coded_len);
    let mut coded = vec![false; coded_len];
    let mut correlations = vec![0.; coded_len];
    for bit_index in 0..coded_len {
        let (i, j) = basis_order[permutation[bit_index]];
        let normal = greville_normal(&marked_norm, i, j)?;
        let a = &marked_norm.control_points[i][j];
        let b = &reference_norm.control_points[i][j];
        let correlation =
            (0..3).map(|d| (a[d] - b[d]) * normal[d]).sum::<f64>();
        numeric(
            correlation.is_finite(),
            "Watermark correlation receiver hit a non-finite displacement.",
        )?;
        let keyed = if signs[bit_index] { 1. } else { -1. };
        correlations[bit_index] = correlation * keyed;
        coded[bit_index] = correlations[bit_index] > 0.;
    }
    let (payload, corrected_bits, parity_failures) = ecc_decode(&coded, payload_len);
    let scale = correlations.iter().map(|c| c.abs()).sum::<f64>() / coded_len as f64;
    let confidence = |index: usize| {
        if scale > 0. {
            (correlations[index].abs() / scale).min(1.)
        } else {
            0.
        }
    };
    let payload_confidences: Vec<f64> = (0..payload_len * 9)
        .map(|m| {
            (0..REPETITIONS)
                .map(|r| confidence(m * REPETITIONS + r))
                .sum::<f64>()
                / REPETITIONS as f64
        })
        .collect();
    Ok(ExtractReport {
        payload,
        corrected_bits,
        parity_failures,
        mean_confidence: payload_confidences.iter().sum::<f64>()
            / payload_confidences.len() as f64,
        min_confidence: payload_confidences.iter().copied().fold(1., f64::min),
        max_deviation: deviation,
    })
}

#[cfg(test)]
#[path = "tests/watermark.rs"]
mod tests;
