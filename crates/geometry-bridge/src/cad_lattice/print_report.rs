//! FDM printability screening for lattice options: orientation ranking,
//! bridge / overhang / thin-wall / cooling issues and machine-agnostic
//! slicer hints. Heuristic cell-geometry estimates, not toolpath analysis.
use super::report_util::*;
use super::*;
use value_codec::json;

pub(super) struct PrintParams {
    pub nozzle: f64,
    pub layer: f64,
    pub lines: f64,
    pub skin_layers: f64,
    pub max_bridge: f64,
    pub open_top: bool,
    pub max_overhang_deg: f64,
    pub min_feature: f64,
    pub speed: f64,
    pub fan: f64,
}

impl PrintParams {
    pub fn parse(s: &Value) -> Result<Self> {
        if !s.is_object() {
            return Err(input(PRINT_SETTINGS_MESSAGE));
        }
        let nozzle = print_setting(s, "nozzle")?;
        let layer = print_setting(s, "layer")?;
        let lines = print_setting(s, "lines")?;
        let skin_layers = print_setting(s, "skinLayers")?;
        let max_bridge = print_setting(s, "maxBridge")?;
        if ![nozzle, layer, lines, skin_layers, max_bridge]
            .iter()
            .all(|v| v.is_finite())
            || nozzle <= 0.
            || layer <= 0.
            || layer > nozzle
            || lines.fract() != 0.
            || !(1. ..=8.).contains(&lines)
            || skin_layers.fract() != 0.
            || skin_layers < 0.
            || max_bridge <= 0.
        {
            return Err(input(PRINT_SETTINGS_MESSAGE));
        }
        let p = Self {
            nozzle,
            layer,
            lines,
            skin_layers,
            max_bridge,
            open_top: opt_bool(s, "openTop")?,
            max_overhang_deg: opt_num(s, "maxOverhangDeg", 50.)?,
            min_feature: opt_num(s, "minFeatureMm", nozzle)?,
            speed: opt_num(s, "printSpeedMms", 45.)?,
            fan: opt_num(s, "fanPct", 100.)?,
        };
        if p.min_feature <= 0. || p.speed <= 0. || !(0. ..=100.).contains(&p.fan) {
            return Err(input("Check minimum feature, print speed and fan settings."));
        }
        Ok(p)
    }
    pub fn line_width(&self) -> f64 {
        (self.nozzle * 1.125 * 1e6).round() / 1e6
    }
}

pub(super) fn bridge_span(o: &LatticeParams) -> f64 {
    (o.cell - o.rib).max(0.)
}

/// Overhang risk 0..1 of the lattice relative to build Z when `channel_axis` is printed upward.
pub(super) fn overhang_risk(o: &LatticeParams, channel_axis: usize, max_overhang_deg: f64) -> f64 {
    let limit = max_overhang_deg.clamp(20., 80.);
    if o.spatial() {
        let typical = match o.pattern.as_str() {
            "bcc" => 55.,
            "octet" => 45.,
            _ if o.diagonals => 45.,
            _ => 0.,
        };
        let mut risk = ((typical - (90. - limit)).max(0.) / 45.).min(1.);
        if o.skin > 0. && !o.open_top {
            risk = risk.max(0.85);
        }
        return rnd(risk, 3);
    }
    if channel_axis == 2 {
        let mut risk: f64 = 0.05;
        if o.top > 0. {
            risk = risk.max(0.9);
        }
        if o.bottom <= 0. {
            risk = risk.max(0.2);
        }
        return risk;
    }
    0.95
}

fn orientation_note(o: &LatticeParams, axis: usize) -> Value {
    let a = axis_name(axis).to_uppercase();
    if o.spatial() {
        return text(
            format!("Печать по {a}: диагонали октет/ОЦК ~45–55° — умеренный риск свесов; кожу режьте openTop."),
            format!("Print along {a}: octet/BCC diagonals ~45–55° — moderate overhang risk; prefer openTop."),
        );
    }
    if axis == 2 {
        text(
            "Каналы вдоль Z — стенки вертикальны, лучше для FDM без поддержек.",
            "Channels along Z keep walls vertical — best for support-light FDM.",
        )
    } else {
        text(
            format!("Каналы вдоль {a} при печати Z дают горизонтальные туннели — нужны поддержки или поворот."),
            format!("Channels along {a} while printing Z make horizontal tunnels — need supports or reorient."),
        )
    }
}

/// Rank print axes for support-light FDM (higher score first; ties keep Z, Y, X order).
pub(super) fn orientations(o: &LatticeParams, p: &PrintParams) -> Vec<(usize, f64, f64, Value)> {
    let bridge = bridge_span(o);
    let mut ranked: Vec<_> = [2usize, 1, 0]
        .into_iter()
        .map(|axis| {
            let oh = overhang_risk(o, axis, p.max_overhang_deg);
            let mut score = 100. - bridge * 3. - oh * 40.;
            if !o.spatial() {
                if axis == 2 {
                    score += 30.;
                }
                if o.axis == axis {
                    score += 5.;
                }
            } else if axis == 2 {
                score += 5.;
            }
            if o.skin > 0. && !o.open_top && axis == 2 {
                score -= 15.;
            }
            if o.top > 0. && !o.open_top && axis == 2 && !o.spatial() {
                score -= 10.;
            }
            (axis, rnd(score, 1), oh, orientation_note(o, axis))
        })
        .collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
    ranked
}

fn issue(kind: &str, severity: &str, ru: String, en: String) -> Value {
    json!({"kind": kind, "severity": severity, "ru": ru, "en": en})
}

pub(super) fn report(o: &LatticeParams, p: &PrintParams) -> Value {
    let width = p.line_width();
    let min_wall = width * p.lines;
    let ranked = orientations(o, p);
    let preferred = ranked[0].0;
    let bridge = bridge_span(o);
    let oh = overhang_risk(o, if o.spatial() { preferred } else { o.axis }, p.max_overhang_deg);
    let mut issues = Vec::new();
    let mut tips = Vec::new();
    if p.layer > p.nozzle * 0.8 {
        issues.push(issue("layer", "warn",
            format!("Слой {} mm высок для сопла {} mm — хуже свесы и щели в решётке.", p.layer, p.nozzle),
            format!("Layer {} mm is high for a {} mm nozzle — worse overhangs and lattice gaps.", p.layer, p.nozzle)));
    }
    if o.rib + 1e-9 < min_wall {
        issues.push(issue("thin_wall", "critical",
            format!("Ребро {} mm < {}×линии ({} mm).", o.rib, p.lines, rnd(min_wall, 2)),
            format!("Rib {} mm < {}×line width ({} mm).", o.rib, p.lines, rnd(min_wall, 2))));
    }
    if o.rib < p.min_feature * 1.5 {
        issues.push(issue("tiny_feature", "warn",
            format!("Тонкий элемент {} mm близок к минимуму сопла (~{} mm).", o.rib, p.min_feature),
            format!("Feature {} mm is near the nozzle minimum (~{} mm).", o.rib, p.min_feature)));
    }
    if bridge > p.max_bridge {
        let severity = if bridge > p.max_bridge * 1.5 { "critical" } else { "warn" };
        issues.push(issue("bridge", severity,
            format!("Мост ≈{} mm > лимита {} mm.", rnd(bridge, 1), p.max_bridge),
            format!("Bridge ≈{} mm > limit {} mm.", rnd(bridge, 1), p.max_bridge)));
        tips.push(text(
            "Уменьшите ячейку, увеличьте ребро, откройте верх или поверните каналы вдоль Z.",
            "Reduce cell, thicken rib, open the top, or align channels with Z."));
    }
    if oh >= 0.6 {
        let severity = if oh >= 0.85 { "critical" } else { "warn" };
        let pct = (oh * 100.).round();
        issues.push(issue("overhang", severity,
            format!("Риск свесов высокий (≈{pct}% при лимите {}°).", p.max_overhang_deg),
            format!("Overhang risk high (≈{pct}% at {}° limit).", p.max_overhang_deg)));
        tips.push(text(
            "Выберите ориентацию с вертикальными стенками; для октета держите openTop и тонкую/нулевую кожу.",
            "Pick an orientation with vertical walls; for octet keep openTop and thin/zero skin."));
    }
    let layer_time = (o.cell * 4.).max(o.rib * 8.) / p.speed.max(1.);
    if layer_time < 4. && o.cell < 10. {
        issues.push(issue("cooling", "warn",
            format!("Мелкая ячейка: оценка времени слоя ~{} s при {} mm/s — риск перегрева.", rnd(layer_time, 1), p.speed),
            format!("Fine cell: ~{} s layer-time proxy at {} mm/s — heat-soak risk.", rnd(layer_time, 1), p.speed)));
        tips.push(if p.fan < 80. {
            text("Включите обдув ≥80% или печатайте пачками / снизьте скорость периметров.",
                "Enable fan ≥80% or print in batches / slow perimeters.")
        } else {
            text("Снизьте скорость периметров или увеличьте ячейку, чтобы слой успевал остыть.",
                "Slow perimeters or enlarge the cell so layers can cool.")
        });
    }
    if !o.spatial() && o.axis != 2 {
        let a = axis_name(o.axis).to_uppercase();
        issues.push(issue("orientation", "warn",
            format!("Каналы по {a} при печати Z — горизонтальные туннели."),
            format!("Channels along {a} while printing Z make horizontal tunnels.")));
    }
    if o.skin > 0. && !o.open_top && o.spatial() {
        issues.push(issue("skin", "info",
            "Сплошная верхняя кожа на скелете — мосты между стержнями.".into(),
            "Closed top skin on a skeletal lattice bridges between struts.".into()));
    }
    let pa = axis_name(preferred).to_uppercase();
    tips.push(text(
        format!("Предпочтительная ось печати: {pa} (оценка {}).", ranked[0].1),
        format!("Preferred print axis: {pa} (score {}).", ranked[0].1)));
    tips.push(text(
        "Швы: на скрытой грани; для решёток удобен случайный/aligned шов у стойки.",
        "Seams: hide on a back face; for lattices prefer random/aligned seams at a post."));
    let orientations: Vec<Value> = ranked
        .into_iter()
        .map(|(axis, score, oh, notes)| json!({"axis": axis_name(axis), "score": score,
            "bridgeMm": rnd(bridge, 2), "overhangRisk": oh, "notes": notes}))
        .collect();
    let hints = json!({
        "layer_height": p.layer,
        "line_width": width,
        "wall_loops": p.lines,
        "top_shell_layers": if p.open_top { 0. } else { p.skin_layers },
        "bottom_shell_layers": p.skin_layers,
        "bridge_flow_ratio": if bridge > p.max_bridge { 0.85 } else { 0.95 },
        "bridge_speed_mms": p.speed.min(25.),
        "fan_max_pct": if layer_time < 4. { 100f64.max(p.fan) } else { p.fan },
        "detect_thin_walls": true,
        "fill_gaps": true,
        "support_enable": oh >= 0.7 || bridge > p.max_bridge,
        "support_overhang_deg": p.max_overhang_deg,
        "seam_position": "rear",
        "recommended_print_axis": axis_name(preferred),
        "notes": "Hints only — verify in PrusaSlicer / Orca / Cura; not a vendor profile file."
    });
    json!({
        "lineWidthMm": width,
        "minWallMm": rnd(min_wall, 3),
        "bridgeSpanMm": rnd(bridge, 2),
        "overhangRisk": oh,
        "preferredAxis": axis_name(preferred),
        "orientations": orientations,
        "issues": issues,
        "tips": tips,
        "slicerHints": hints
    })
}

/// `cad_lattice_print_report`: `{options, settings}` → printability report.
pub fn print_report(v: Value) -> Result<Value> {
    let o = LatticeParams::parse(&v["options"])?;
    let p = PrintParams::parse(&v["settings"])?;
    Ok(report(&o, &p))
}

/// `cad_lattice_print_optimize`: reorient channels, open spatial tops, fit to
/// lines/layers and shrink the cell (or thicken the rib) until the bridge fits.
pub fn print_optimize(v: Value) -> Result<Value> {
    let reorient = !matches!(v["reorient"].as_bool(), Some(false));
    let shrink = !matches!(v["shrinkBridges"].as_bool(), Some(false));
    let settings = v["settings"].clone();
    let p = PrintParams::parse(&settings)?;
    let mut next = v["options"].clone();
    let mut changed: Vec<String> = Vec::new();
    let params = LatticeParams::parse(&next)?;
    if reorient && !params.spatial() {
        let best = orientations(&params, &p)[0].0;
        if params.axis != best {
            next["axis"] = Value::from(axis_name(best));
            changed.push(format!("reorient_channels_{}", axis_name(best)));
        }
    }
    if params.spatial() && p.open_top && !params.open_top {
        next["openTop"] = Value::from(true);
        changed.push("open_top".into());
    }
    let fit = |options: Value| print_fit(json!({"options": options, "settings": settings.clone()}));
    next = fit(next)?;
    changed.push("fit_lines_layers".into());
    if shrink {
        let fitted = LatticeParams::parse(&next)?;
        if bridge_span(&fitted) > p.max_bridge {
            let width = p.line_width();
            let target = (fitted.rib * 2. + width).max(p.max_bridge + fitted.rib);
            if target < fitted.cell {
                next["cell"] = Value::from(rnd(target, 3));
                changed.push("shrink_cell_for_bridge".into());
            } else {
                let rib = round_to_step(fitted.rib.max(fitted.cell - p.max_bridge), width);
                if rib > fitted.rib {
                    next["rib"] = Value::from(rib);
                    next["cell"] = Value::from(fitted.cell.max(rib * 2. + width));
                    changed.push("thicken_rib_for_bridge".into());
                }
            }
            next = fit(next)?;
        }
    }
    let report = report(&LatticeParams::parse(&next)?, &p);
    Ok(json!({"options": next, "report": report, "changed": changed}))
}
