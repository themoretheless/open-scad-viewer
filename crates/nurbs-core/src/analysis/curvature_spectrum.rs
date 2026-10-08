//! Спектральная детекция «волн» на поверхности по кривизне (items 991).
//!
//! Вдоль изопараметрической линии сэмплируется нормальная кривизна κ
//! (аналитически, из дифференциального джета поверхности) на сетке,
//! равномерной по длине дуги (пересэмплирование с плотной параметрической
//! сетки — метрический масштаб даёт длина дуги изокривой, а не параметр).
//! Собственный radix-2 FFT (без внешних зависимостей, см. `fft`); если
//! длина сетки не степень двойки, сигнал дополняется нулями до ближайшей
//! степени двойки, а для малых длин доступен прямой DFT-фолбэк (`dft`).
//!
//! Энергия в заданной полосе длин волн (метры, не бины) сравнивается с
//! порогом; превышение даёт дефект с локализацией по огибающей
//! полосового сигнала (обратное преобразование полосы). Отчёт содержит
//! peak-to-valley эквивалент волны в κ, доминирующую длину волны и
//! энергию полосы. Сеточная метрика, не сертификат.
use crate::{
    Result, check, numeric, numeric_err, resource,
    surface::{Evaluation, Surface},
};
use math_core::{dot, norm};

/// Бюджеты выборки и преобразования.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Максимум точек спектральной сетки (и длины FFT после дополнения).
    pub max_samples: usize,
    /// Максимум изопараметрических линий в карте.
    pub max_lines: usize,
    /// Максимум длины прямого DFT-фолбэка (O(n²)).
    pub max_dft: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_samples: 65_536,
            max_lines: 128,
            max_dft: 4_096,
        }
    }
}

/// Ось изопараметрической линии: вдоль U при фиксированном v или наоборот.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IsoAxis {
    U,
    V,
}
impl IsoAxis {
    pub fn name(self) -> &'static str {
        match self {
            IsoAxis::U => "u",
            IsoAxis::V => "v",
        }
    }
}

/// Дефект «волна»: превышение амплитудного порога в полосе.
#[derive(Clone, Debug)]
pub struct WaveDefect {
    /// (u,v) максимума огибающей полосового сигнала.
    pub uv: [f64; 2],
    /// Дуговая координата максимума вдоль линии, в единицах длины.
    pub arc_position: f64,
    /// Peak-to-valley эквивалент доминирующей волны (в единицах κ).
    pub peak_to_valley: f64,
    /// Порог, который превышен.
    pub threshold: f64,
}

/// Спектральный отчёт по одной изопараметрической линии (items 991).
#[derive(Clone, Debug)]
pub struct SpectrumReport {
    pub axis: IsoAxis,
    /// Фиксированное значение поперечного параметра.
    pub parameter: f64,
    /// Число точек спектральной сетки (до дополнения нулями).
    pub samples: usize,
    /// Полная длина дуги изокривой.
    pub arc_length: f64,
    /// Доминирующая длина волны в полосе (None — полоса пуста по энергии).
    pub dominant_wavelength: Option<f64>,
    /// Peak-to-valley эквивалент доминирующей волны (2·амплитуда в κ).
    pub peak_to_valley: f64,
    /// Энергия полосы (Σ|X_k|²/n², т.е. вклад полосы в дисперсию κ).
    pub band_energy: f64,
    /// Полная энергия спектра (без DC), та же нормировка.
    pub total_energy: f64,
    /// Доля энергии полосы в полной.
    pub band_ratio: f64,
    pub defect: Option<WaveDefect>,
}

/// (items 991) Карта по набору линий: отчёты + худшая линия.
#[derive(Clone, Debug)]
pub struct SpectrumMap {
    pub lines: Vec<SpectrumReport>,
    /// Индекс линии с максимальной долей энергии полосы.
    pub worst_line: Option<usize>,
}

/// Комплексное число (re, im) — собственная арифметика, без зависимостей.
type C = [f64; 2];

fn cadd(a: C, b: C) -> C {
    [a[0] + b[0], a[1] + b[1]]
}
fn csub(a: C, b: C) -> C {
    [a[0] - b[0], a[1] - b[1]]
}
fn cmul(a: C, b: C) -> C {
    [a[0] * b[0] - a[1] * b[1], a[0] * b[1] + a[1] * b[0]]
}

/// Итеративный radix-2 Cooley–Tukey FFT, in-place. Прямое преобразование
/// (знак −). Длина должна быть степенью двойки — проверяется вызывающим.
fn fft_inplace(a: &mut [C]) {
    let n = a.len();
    // Бит-реверсивная перестановка.
    let bits = n.trailing_zeros();
    for i in 0..n {
        let j = i.reverse_bits() >> (usize::BITS - bits);
        if i < j {
            a.swap(i, j as usize);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = -2. * std::f64::consts::PI / len as f64;
        let wlen: C = [ang.cos(), ang.sin()];
        let half = len / 2;
        for block in a.chunks_exact_mut(len) {
            let mut w: C = [1., 0.];
            for k in 0..half {
                let u = block[k];
                let v = cmul(block[k + half], w);
                block[k] = cadd(u, v);
                block[k + half] = csub(u, v);
                w = cmul(w, wlen);
            }
        }
        len *= 2;
    }
}

/// Прямое преобразование. Степень двойки — radix-2 FFT (~n log n), иначе
/// DFT-фолбэк O(n²) с бюджетом `limits.max_dft`.
pub fn fft(x: &[f64], limits: &Limits) -> Result<Vec<C>> {
    check(!x.is_empty(), "FFT needs a non-empty signal")?;
    if x.len() > limits.max_samples {
        return Err(resource("curvature_spectrum signal length exceeds the budget"));
    }
    check(x.iter().all(|v| v.is_finite()), "FFT signal must be finite")?;
    let n = if x.len().is_power_of_two() {
        x.len()
    } else {
        if x.len() > limits.max_dft {
            return Err(resource(
                "curvature_spectrum non-power-of-two signal exceeds the DFT budget",
            ));
        }
        return Ok(dft(x));
    };
    let mut buf: Vec<C> = x.iter().map(|&v| [v, 0.]).collect();
    debug_assert_eq!(buf.len(), n);
    fft_inplace(&mut buf);
    Ok(buf)
}

/// Прямой DFT O(n²) — фолбэк и эталон для тестов.
pub fn dft(x: &[f64]) -> Vec<C> {
    let n = x.len();
    let mut out = vec![[0.; 2]; n];
    for (k, slot) in out.iter_mut().enumerate() {
        let mut acc: C = [0., 0.];
        for (t, &v) in x.iter().enumerate() {
            let ang = -2. * std::f64::consts::PI * k as f64 * t as f64 / n as f64;
            acc = cadd(acc, [v * ang.cos(), v * ang.sin()]);
        }
        *slot = acc;
    }
    out
}

/// Обратное преобразование: conjugate → forward → conjugate, /n.
fn ifft(a: &mut [C]) {
    for v in a.iter_mut() {
        v[1] = -v[1];
    }
    fft_inplace(a);
    let n = a.len() as f64;
    for v in a.iter_mut() {
        *v = [v[0] / n, -v[1] / n];
    }
}

fn unit(v: [f64; 3]) -> Option<[f64; 3]> {
    let n = norm(v);
    (n > 1e-14).then(|| [v[0] / n, v[1] / n, v[2] / n])
}

/// Нормальная кривизна в единичном мировом направлении d.
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

fn surface_domains(s: &Surface) -> ([f64; 2], [f64; 2]) {
    (
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    )
}

/// (items 991) Спектр кривизны вдоль одной изопараметрической линии.
///
/// `samples` ≥ 64; не степень двойки — сигнал дополняется нулями до
/// ближайшей сверху степени двойки (частотная сетка в этом случае
/// пересчитывается через шаг по дуге). `band` — [min,max] длин волн в
/// единицах длины модели; 0 < min < max. `amplitude_threshold` — порог
/// peak-to-valley эквивалента в κ для регистрации дефекта.
pub fn iso_spectrum(
    surface: &Surface,
    axis: IsoAxis,
    parameter: f64,
    samples: usize,
    band: [f64; 2],
    amplitude_threshold: f64,
    limits: &Limits,
) -> Result<SpectrumReport> {
    surface.validate()?;
    check(samples >= 64, "Curvature spectrum needs a grid of at least 64 points")?;
    check(
        band[0] > 0. && band[0] < band[1],
        "Wavelength band must satisfy 0 < min < max",
    )?;
    check(
        amplitude_threshold > 0.,
        "Spectrum amplitude threshold must be positive",
    )?;
    if samples > limits.max_samples {
        return Err(resource(
            "curvature_spectrum grid exceeds the sampling budget",
        ));
    }
    let (du, dv) = surface_domains(surface);
    let (along, across) = match axis {
        IsoAxis::U => (du, dv),
        IsoAxis::V => (dv, du),
    };
    check(
        parameter.is_finite() && across[0] <= parameter && parameter <= across[1],
        "Spectrum line parameter lies outside the surface domain",
    )?;
    // Плотная параметрическая выборка (4× передискретизация) для длины
    // дуги и κ(s); конец интервала исключён (периодичность FFT).
    let dense = 4 * samples;
    let mut points: Vec<[f64; 3]> = Vec::with_capacity(dense);
    let mut kappas: Vec<f64> = Vec::with_capacity(dense);
    let mut params: Vec<f64> = Vec::with_capacity(dense);
    for i in 0..dense {
        let t = along[0] + (along[1] - along[0]) * i as f64 / dense as f64;
        let (u, v) = match axis {
            IsoAxis::U => (t, parameter),
            IsoAxis::V => (parameter, t),
        };
        let ev = surface.evaluate(u, v)?;
        let dir = match axis {
            IsoAxis::U => ev.first_derivatives().and_then(|(su, _)| unit(su)),
            IsoAxis::V => ev.first_derivatives().and_then(|(_, sv)| unit(sv)),
        };
        let Some(d) = dir else {
            return Err(numeric_err(
                "curvature_spectrum hit a degenerate iso-curve direction",
            ));
        };
        let Some(kappa) = normal_curvature(&ev, d) else {
            return Err(numeric_err(
                "curvature_spectrum hit a singular surface jet on the iso line",
            ));
        };
        points.push(ev.point);
        kappas.push(kappa);
        params.push(t);
    }
    // Кумулятивная длина дуги.
    let mut arc: Vec<f64> = Vec::with_capacity(dense + 1);
    arc.push(0.);
    for w in points.windows(2) {
        arc.push(arc[arc.len() - 1] + norm([w[1][0] - w[0][0], w[1][1] - w[0][1], w[1][2] - w[0][2]]));
    }
    let arc_length = arc[arc.len() - 1];
    numeric(arc_length > 0., "Curvature spectrum iso-curve has zero arc length")?;
    // Пересэмплирование κ на равномерную по дуге сетку из `samples` точек
    // (линейная интерполяция, детерминированный двоичный проход).
    let ds = arc_length / samples as f64;
    let mut signal = Vec::with_capacity(samples);
    let mut seg = 0usize;
    for k in 0..samples {
        let s = k as f64 * ds;
        while seg + 1 < dense && arc[seg + 1] < s {
            seg += 1;
        }
        let (s0, s1) = (arc[seg], arc[(seg + 1).min(dense)]);
        let w = if s1 > s0 { (s - s0) / (s1 - s0) } else { 0. };
        signal.push(kappas[seg] * (1. - w) + kappas[(seg + 1).min(dense - 1)] * w);
    }
    // Центрирование: убираем DC (среднюю кривизну линии).
    let mean = signal.iter().sum::<f64>() / samples as f64;
    for v in &mut signal {
        *v -= mean;
    }
    // Дополнение нулями до степени двойки.
    let m = samples.next_power_of_two();
    if m > limits.max_samples {
        return Err(resource(
            "curvature_spectrum padded FFT length exceeds the budget",
        ));
    }
    let mut buf: Vec<C> = signal.iter().map(|&v| [v, 0.]).collect();
    buf.resize(m, [0., 0.]);
    fft_inplace(&mut buf);
    // Частотная сетка: шаг по дуге ds, бин k = k/(m·ds) циклов/длину,
    // длина волны λ_k = m·ds/k. Амплитуда синусоиды = 2|X_k|/samples.
    let n_f = samples as f64;
    let wavelength = |k: usize| m as f64 * ds / k as f64;
    let cabs = |c: C| c[0].hypot(c[1]);
    let amplitude = |k: usize| 2. * cabs(buf[k]) / n_f;
    let k_band = |lambda: f64| (m as f64 * ds / lambda).round() as usize;
    let k_lo = k_band(band[1]).max(1);
    let k_hi = k_band(band[0]).min(m / 2 - 1);
    let mut band_energy = 0.;
    let mut total_energy = 0.;
    let mut dominant = None;
    for k in 1..m / 2 {
        let e = cabs(buf[k]).powi(2) / (n_f * n_f);
        total_energy += 2. * e;
        if k_lo <= k && k <= k_hi {
            band_energy += 2. * e;
            if dominant.map_or(true, |(_, a): (usize, f64)| amplitude(k) > a) {
                dominant = Some((k, amplitude(k)));
            }
        }
    }
    let (dominant_wavelength, peak_to_valley) = match dominant {
        Some((k, a)) if a > 0. => (Some(wavelength(k)), 2. * a),
        _ => (None, 0.),
    };
    let band_ratio = if total_energy > 0. {
        band_energy / total_energy
    } else {
        0.
    };
    // Локализация дефекта: полосовое обратное преобразование, максимум
    // огибающей → дуговая координата → параметр линейной интерполяцией
    // по таблице (arc, params).
    let mut defect = None;
    if peak_to_valley > amplitude_threshold {
        let mut filtered = vec![[0.; 2]; m];
        for k in k_lo..=k_hi {
            filtered[k] = buf[k];
            filtered[m - k] = buf[m - k];
        }
        ifft(&mut filtered);
        let mut best = (0usize, 0.);
        for (k, v) in filtered.iter().enumerate().take(samples) {
            if v[0].abs() > best.1 {
                best = (k, v[0].abs());
            }
        }
        let s_star = best.0 as f64 * ds;
        let mut seg = 0usize;
        while seg + 1 < dense && arc[seg + 1] < s_star {
            seg += 1;
        }
        let (s0, s1) = (arc[seg], arc[(seg + 1).min(dense)]);
        let w = if s1 > s0 {
            ((s_star - s0) / (s1 - s0)).clamp(0., 1.)
        } else {
            0.
        };
        let t = params[seg] * (1. - w) + params[(seg + 1).min(dense - 1)] * w;
        let uv = match axis {
            IsoAxis::U => [t, parameter],
            IsoAxis::V => [parameter, t],
        };
        defect = Some(WaveDefect {
            uv,
            arc_position: s_star,
            peak_to_valley,
            threshold: amplitude_threshold,
        });
    }
    Ok(SpectrumReport {
        axis,
        parameter,
        samples,
        arc_length,
        dominant_wavelength,
        peak_to_valley,
        band_energy,
        total_energy,
        band_ratio,
        defect,
    })
}

/// (items 991) Карта спектров по `lines` равномерно расположенным
/// изопараметрическим линиям (поперечный параметр — доли домена, без
/// концевых интрузий вырождений: inset 2%).
#[allow(clippy::too_many_arguments)]
pub fn spectrum_map(
    surface: &Surface,
    axis: IsoAxis,
    lines: usize,
    samples: usize,
    band: [f64; 2],
    amplitude_threshold: f64,
    limits: &Limits,
) -> Result<SpectrumMap> {
    check(lines >= 1, "Spectrum map needs at least one iso line")?;
    if lines > limits.max_lines {
        return Err(resource("curvature_spectrum line count exceeds the budget"));
    }
    let (du, dv) = surface_domains(surface);
    let across = match axis {
        IsoAxis::U => dv,
        IsoAxis::V => du,
    };
    let inset = 0.02;
    let mut map = SpectrumMap {
        lines: Vec::with_capacity(lines),
        worst_line: None,
    };
    let mut worst = -1.;
    for i in 0..lines {
        let t = if lines == 1 {
            0.5
        } else {
            inset + (1. - 2. * inset) * i as f64 / (lines - 1) as f64
        };
        let parameter = across[0] + (across[1] - across[0]) * t;
        let report = iso_spectrum(
            surface,
            axis,
            parameter,
            samples,
            band,
            amplitude_threshold,
            limits,
        )?;
        if report.band_ratio > worst {
            worst = report.band_ratio;
            map.worst_line = Some(i);
        }
        map.lines.push(report);
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> Limits {
        Limits::default()
    }

    /// Плоский прямоугольник [0,L]×[0,1] в плоскости z=0.
    fn flat_patch(l: f64) -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![l, 0., 0.], vec![l, 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }

    /// Волнистая пластина z = A·sin(2πx/λ): кубическая B-сплайн
    /// квази-интерполяция синуса (32 контрольные точки — предел крейта,
    /// домен u = x).
    fn wavy_patch(amplitude: f64, wavelength: f64, length: f64) -> Surface {
        let n = 32;
        // Clamped knots: 4 + (n−p−1 = 29) + 4 = n + p + 1 = 37.
        let knots_u: Vec<f64> = {
            let mut k = vec![0.; 4];
            k.extend((1..=(n - 4)).map(|i| length * i as f64 / (n - 3) as f64));
            k.extend(vec![length; 4]);
            k
        };
        let mut cp = vec![];
        for i in 0..n {
            let x = length * i as f64 / (n - 1) as f64;
            let z = amplitude * (2. * std::f64::consts::PI * x / wavelength).sin();
            cp.push(vec![vec![x, 0., z], vec![x, 1., z]]);
        }
        Surface {
            degree_u: 3,
            degree_v: 1,
            knots_u,
            knots_v: vec![0., 0., 1., 1.],
            control_points: cp,
            weights: vec![vec![1.; 2]; n],
            periodic_u: false,
            periodic_v: false,
        }
    }

    #[test]
    fn fft_recovers_known_cosine_bin() {
        let n = 64;
        let x: Vec<f64> = (0..n)
            .map(|i| (2. * std::f64::consts::PI * 3. * i as f64 / n as f64).cos())
            .collect();
        let spec = fft(&x, &limits()).unwrap();
        let cabs = |c: C| c[0].hypot(c[1]);
        assert!((cabs(spec[3]) - n as f64 / 2.).abs() < 1e-9, "|X3| {}", cabs(spec[3]));
        for k in 1..n / 2 {
            if k != 3 {
                assert!(cabs(spec[k]) < 1e-9, "bin {} {}", k, cabs(spec[k]));
            }
        }
    }

    #[test]
    fn dft_fallback_matches_fft() {
        let x: Vec<f64> = (0..24).map(|i| (i as f64 * 0.7).sin()).collect();
        let direct = dft(&x);
        let via_fft = fft(&x, &limits()).unwrap();
        for (a, b) in direct.iter().zip(via_fft.iter()) {
            assert!((a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9);
        }
    }

    #[test]
    fn flat_patch_has_no_wave() {
        let s = flat_patch(8.);
        let report = iso_spectrum(&s, IsoAxis::U, 0.5, 128, [0.2, 4.], 1e-6, &limits()).unwrap();
        assert!(report.peak_to_valley < 1e-9, "p2v {}", report.peak_to_valley);
        assert!(report.defect.is_none());
        assert!(report.band_ratio < 1e-12, "ratio {}", report.band_ratio);
    }

    #[test]
    fn sine_wave_is_detected_at_right_wavelength() {
        // 4 волны на 32 КТ: 8 точек на волну, затухание квази-интерполяции
        // умеренное (допуск 30 % по амплитуде).
        let (a, lambda, length) = (0.02, 2.0, 8.0);
        let s = wavy_patch(a, lambda, length);
        let report = iso_spectrum(&s, IsoAxis::U, 0.5, 256, [0.6, 6.0], 0.1, &limits()).unwrap();
        let dom = report.dominant_wavelength.expect("dominant wavelength");
        assert!(
            (dom - lambda).abs() < 0.05 * lambda,
            "dominant {} vs {}",
            dom,
            lambda
        );
        // Амплитуда кривизны синуса ≈ A·(2π/λ)²; квази-интерполяция её
        // слегка занижает, допускаем 30 %.
        let expected = a * (2. * std::f64::consts::PI / lambda).powi(2);
        assert!(
            (report.peak_to_valley - 2. * expected).abs() < 0.6 * expected,
            "p2v {} vs {}",
            report.peak_to_valley,
            2. * expected
        );
        let defect = report.defect.expect("wave defect");
        // Локализация внутри домена линии, arc-координата в пределах длины.
        assert!(defect.arc_position >= 0. && defect.arc_position <= report.arc_length);
        assert!((defect.uv[1] - 0.5).abs() < 1e-9);
        assert!(report.band_ratio > 0.9, "ratio {}", report.band_ratio);
    }

    #[test]
    fn non_power_of_two_grid_is_zero_padded() {
        let (a, lambda, length) = (0.02, 1.0, 8.0);
        let s = wavy_patch(a, lambda, length);
        let report = iso_spectrum(&s, IsoAxis::U, 0.5, 100, [0.3, 3.0], 0.05, &limits()).unwrap();
        let dom = report.dominant_wavelength.expect("dominant wavelength");
        assert!((dom - lambda).abs() < 0.06 * lambda, "dominant {}", dom);
    }

    #[test]
    fn map_picks_the_wavy_lines_and_checks_budgets() {
        let s = wavy_patch(0.02, 1.0, 8.0);
        let map = spectrum_map(&s, IsoAxis::U, 3, 128, [0.3, 3.0], 0.05, &limits()).unwrap();
        assert_eq!(map.lines.len(), 3);
        assert!(map.lines.iter().all(|l| l.defect.is_some()));
        assert!(map.worst_line.is_some());
        assert!(iso_spectrum(&s, IsoAxis::U, 0.5, 32, [0.3, 3.0], 0.05, &limits()).is_err());
        assert!(iso_spectrum(&s, IsoAxis::U, 0.5, 64, [3.0, 0.3], 0.05, &limits()).is_err());
        assert!(iso_spectrum(&s, IsoAxis::U, 5.0, 64, [0.3, 3.0], 0.05, &limits()).is_err());
    }
}
