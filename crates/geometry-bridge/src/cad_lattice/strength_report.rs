//! Report assembly for `cad_lattice_strength` (see `strength.rs`).
use super::report_util::*;
use super::strength::{Section, advice, estimate_relative_density, panel_buckling};
use super::strength_material::*;
use super::weak_spots::{Graph, LoadCase, parallel_paths};
use super::*;
use std::f64::consts::PI;
use value_codec::json;

pub(super) struct Input<'a> {
    pub o: &'a LatticeParams,
    pub min: [f64; 3],
    pub max: [f64; 3],
    pub graph: Option<&'a Graph>,
    pub material: &'a Material,
    pub case: LoadCase,
    pub force_n: f64,
    pub safety: f64,
    pub direction: [f64; 3],
    pub service: &'a Service,
    pub section: &'a Section,
    pub volumes: Option<(f64, f64)>,
}

pub(super) fn analyze(i: &Input) -> Value {
    let (o, m, sec) = (i.o, i.material, i.section);
    let size: [f64; 3] = std::array::from_fn(|k| i.max[k] - i.min[k]);
    let eff = effective(m, i.service, i.direction);
    let (_, load_class, _, _) = structure_hint(&o.pattern);
    let rho = match i.volumes {
        Some((before, after)) => rnd((after / before).max(1e-4), 4),
        None => estimate_relative_density(o, size),
    };
    let n = ashby_exponent(&o.pattern, o.diagonals);
    let (c_e, c_s) = match load_class {
        "stretch" => (0.5, 0.35),
        "bending" => (0.8, 0.5),
        _ => (0.6, 0.4),
    };
    let rel_stiffness = rnd(c_e * rho.powf(n), 4);
    let strength_exp = if i.case == LoadCase::Bending { n } else { (n * 0.85).max(1.) };
    let rel_strength = rnd(c_s * rho.powf(strength_exp), 4);
    let mut warnings: Vec<Value> = Vec::new();
    let sigma_allow = eff.sigma_allow.min(m.sigma_allow * m.fatigue_ratio);
    let allow = (sigma_allow / i.safety).max(1e-6);
    let e_use = eff.e.max(1.);
    let kt = m.kt;
    let (area, stress, utilization);
    let (mut euler, mut buckling, mut panel) = (None, None, None);
    if o.spatial() {
        let g = i.graph;
        let l = g.and_then(|g| g.mean_length).unwrap_or(o.cell).max(o.rib);
        let paths = parallel_paths(o, size, g.map_or(0, |g| g.edges.len()));
        let per_strut = i.force_n / paths;
        let bend = if sec.eccentricity > 0. && sec.modulus > 0. { per_strut * sec.eccentricity / sec.modulus } else { 0. };
        let s = rnd(kt * (per_strut / sec.area + bend), 4);
        let pcr = rnd(PI * PI * e_use * sec.inertia / (l * l), 4);
        let ratio = rnd(per_strut / pcr.max(1e-9), 3);
        let u = rnd(s / allow, 3);
        let wall = (o.rib * 0.35).max(if o.wall_depth > 0. { o.wall_depth * 0.25 } else { 0.4 });
        panel = Some(rnd(panel_buckling(e_use, wall, o.cell), 4));
        if let Some(z) = g.and_then(|g| g.connectivity) && z < 6. && o.pattern != "bone" {
            warnings.push(text(
                format!("Средняя связность узла Z≈{z} < 6 — каркас ближе к изгибному (Maxwell/Deshpande)."),
                format!("Mean node connectivity Z≈{z} < 6 — closer to bending-dominated (Maxwell/Deshpande).")));
        }
        if ratio > 0.7 {
            warnings.push(text(
                format!("Запас по Эйлеру низкий (P/Pcr≈{ratio}). Утолщите стержни или уменьшите ячейку."),
                format!("Euler buckling margin is low (P/Pcr≈{ratio}). Increase strut diameter or reduce cell size.")));
        }
        if u > 1. {
            warnings.push(text(
                format!("Расчётные напряжения (сжатие+изгиб, Kt={kt}) выше допуска с запасом (η≈{u})."),
                format!("Combined compression+bending (Kt={kt}) exceeds allowable with safety (η≈{u}).")));
        }
        if o.wall_depth > 0. && !o.keep_core {
            warnings.push(text(
                "Пустая сердцевина: оценка — для мембранной/оболочечной работы стенки, не для объёмного сжатия целиком.",
                "Hollow core: estimate targets wall membrane/shell action, not bulk solid compression."));
        }
        area = Some(rnd(sec.area, 4));
        stress = Some(s);
        euler = Some(pcr);
        buckling = Some(ratio);
        utilization = u;
    } else {
        if matches!(o.pattern.as_str(), "grid" | "web") {
            warnings.push(text(
                "Канальная сетка изгибно-доминантна: при той же плотности жёсткость ниже, чем у изогрида/октета.",
                "Channel grid is bending-dominated: at the same density, stiffness trails isogrid/octet."));
        }
        if o.top > 0. {
            warnings.push(text(
                "Верхняя кожа образует мосты при печати; локальная прочность кожи не моделируется.",
                "Top skin introduces print bridges; local skin strength is not modelled."));
        }
        let paths = parallel_paths(o, size, 0);
        area = Some(rnd(o.rib * (size[o.axis] - o.bottom - o.top).max(o.rib), 3));
        let s = rnd(i.force_n / (paths * (o.rib * o.rib).max(1e-6)), 4);
        stress = Some(s);
        utilization = rnd(s / allow, 3);
    }
    if i.case == LoadCase::Bending && rho < 0.25 {
        warnings.push(text(
            "При изгибе тонкого облегчённого тела важны положение материала и моменты инерции — Ashby даёт только порядок величины.",
            "For bending of a light body, material placement and section moments dominate — Ashby only gives order-of-magnitude."));
    }
    let sv = i.service;
    if m.fdm {
        let align = (eff.layer_alignment * 100.).round();
        let detail = format!("T={}°C, RH={}%, infill={}%, peri={}, print {}", sv.temperature_c, sv.humidity_pct,
            (sv.infill * 100.).round(), sv.perimeters, axis_name(sv.print_axis).to_uppercase());
        warnings.push(text(
            format!("FDM ортотропия: E_eff={} MPa из E∥={} / E⊥={} (выравнивание нагрузки с осью печати {align}%), σ_inter≈{} MPa ({detail}).",
                eff.e.round(), eff.e_parallel.round(), eff.e_perp.round(), rnd(eff.sigma_interlayer, 1)),
            format!("FDM orthotropy: E_eff={} MPa from E∥={} / E⊥={} (load↔print-axis alignment {align}%), σ_inter≈{} MPa ({detail}).",
                eff.e.round(), eff.e_parallel.round(), eff.e_perp.round(), rnd(eff.sigma_interlayer, 1))));
        if eff.layer_alignment >= 0.7 {
            warnings.push(text(
                "Нагрузка почти вдоль оси печати — работает межслойная прочность (E⊥ / σ_inter). Поверните деталь или усильте периметры.",
                "Load nearly along print axis — interlayer strength governs (E⊥ / σ_inter). Reorient the part or add perimeters."));
        }
        if m.id == "nylon" && sv.humidity_pct >= 40. {
            warnings.push(text(
                "Nylon: высокая влажность заметно снижает E и прочность — сушите филамент и учитывайте кондиционирование детали.",
                "Nylon: high humidity notably lowers E and strength — dry filament and account for part conditioning."));
        }
    } else {
        warnings.push(text(
            format!("Металл ({}): подсветка смещена к потере устойчивости и концентраторам; слоистая анизотропия почти не учитывается.", m.ru),
            format!("Metal ({}): highlights bias toward buckling and concentrators; layer anisotropy is nearly ignored.", m.en)));
    }
    if let Some(s) = stress {
        let yield_limit = eff.sigma_yield / i.safety;
        if eff.sigma_yield > 0. && s > yield_limit {
            warnings.push(text(
                format!("Экран пластичности: σ≈{s} MPa выше σy/n ({} MPa).", rnd(yield_limit, 1)),
                format!("Plasticity screen: σ≈{s} MPa above σy/n ({} MPa).", rnd(yield_limit, 1))));
        }
        if let Some(p) = panel && s > p {
            warnings.push(text(
                format!("Локальная потеря устойчивости панели/стенки: σ > σ_panel≈{p} MPa."),
                format!("Local panel/wall buckling: σ > σ_panel≈{p} MPa.")));
        }
    }
    let tips = advice(o, utilization, buckling.unwrap_or(0.), sv, n);
    let g = i.graph;
    json!({
        "pattern": o.pattern.clone(),
        "loadClass": load_class,
        "loadCase": i.case.name(),
        "forceN": i.force_n,
        "safety": i.safety,
        "relativeDensity": rho,
        "volumeReductionPct": rnd((1. - rho) * 100., 1),
        "ashbyExponent": n,
        "relativeStiffness": rel_stiffness,
        "relativeStrength": rel_strength,
        "specificStiffness": rnd(rel_stiffness / rho.max(1e-6), 3),
        "meanStrutLengthMm": g.and_then(|g| g.mean_length),
        "connectivity": g.and_then(|g| g.connectivity),
        "strutAreaMm2": area,
        "strutStressMPa": stress,
        "eulerBucklingN": euler,
        "bucklingRatio": buckling,
        "utilization": utilization,
        "panelBucklingMPa": panel,
        "sectionLabel": sec.label.clone(),
        "material": {"id": m.id.clone(), "ru": m.ru.clone(), "en": m.en.clone()},
        "materialEffective": {
            "E": rnd(eff.e, 1), "EParallel": rnd(eff.e_parallel, 1), "EPerp": rnd(eff.e_perp, 1),
            "sigmaAllow": rnd(eff.sigma_allow, 2), "sigmaInterlayer": rnd(eff.sigma_interlayer, 2),
            "sigmaYield": rnd(eff.sigma_yield, 2), "envFactor": rnd(eff.env, 3),
            "layerAlignment": rnd(eff.layer_alignment, 3)
        },
        "homogenized": {"EStar": rnd(e_use * rel_stiffness, 2), "sigmaStar": rnd(sigma_allow * rel_strength, 2),
            "relativeDensity": rho},
        "advice": tips,
        "warnings": warnings,
        "disclaimer": text(
            "Сопромат-оценка (Gibson–Ashby, Эйлер, ортотропия FDM). Не сертификат и не FEA; для опор и нагрузок используйте расчёт фермы.",
            "Strength-of-materials estimate (Gibson–Ashby, Euler, FDM orthotropy). Not certification or FEA; use the truss analysis for explicit supports and loads.")
    })
}

/// Attach ranked weak spots and their summary warning.
pub(super) fn finish(report: &mut Value, spots: Vec<Value>) {
    let top = spots.first().map(|s| (s["severity"].as_str() == Some("critical"), s["kind"].as_str().unwrap_or("").to_string()));
    let high = spots.iter().any(|s| s["severity"].as_str() == Some("high"));
    if let Some(warnings) = report["warnings"].as_array_mut() {
        match top {
            Some((true, kind)) => warnings.push(text(
                format!("Найдены критические слабые места (топ: {kind}) — см. список координат ниже."),
                format!("Critical weak spots found (top: {kind}) — see coordinate list below."))),
            _ if high => warnings.push(text(
                "Обнаружены зоны повышенного риска (углы/тонкие стержни/мосты) — проверьте список слабых мест.",
                "Elevated-risk zones detected (corners/slender struts/bridges) — review the weak-spot list.")),
            _ => {}
        }
    }
    report["weakSpots"] = Value::Array(spots);
}
