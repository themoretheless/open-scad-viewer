//! Heuristic weak-spot screening for lattices: free ends, hinge and
//! under-connected nodes, body/frame corners, acute cell corners, slender
//! struts, long load-aligned spans, horizontal bridges and thin skeletal
//! walls. Strength-of-materials ranking — not FEA hotspots.
use super::report_util::*;
use super::strength_material::Material;
use super::*;
use value_codec::json;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum LoadCase {
    Compression,
    Tension,
    Bending,
}

impl LoadCase {
    pub fn parse(s: Option<&str>) -> Result<Self> {
        match s {
            Some("compression") => Ok(Self::Compression),
            Some("tension") => Ok(Self::Tension),
            Some("bending") => Ok(Self::Bending),
            _ => Err(input("Load case must be compression, tension or bending.")),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Compression => "compression",
            Self::Tension => "tension",
            Self::Bending => "bending",
        }
    }
}

pub(super) struct Graph {
    pub nodes: Vec<[f64; 3]>,
    pub edges: Vec<[usize; 2]>,
    pub degree: Vec<usize>,
    pub mean_length: Option<f64>,
    pub connectivity: Option<f64>,
}

impl Graph {
    pub fn new(nodes: Vec<[f64; 3]>, edges: Vec<[usize; 2]>) -> Result<Self> {
        let mut degree = vec![0; nodes.len()];
        let mut sum = 0.;
        for &[a, b] in &edges {
            if a >= nodes.len() || b >= nodes.len() {
                return Err(input("Lattice graph edge references a missing node."));
            }
            degree[a] += 1;
            degree[b] += 1;
            sum += dist(nodes[a], nodes[b]);
        }
        let connectivity = (!nodes.is_empty())
            .then(|| rnd(degree.iter().sum::<usize>() as f64 / nodes.len() as f64, 3));
        let mean_length = (!edges.is_empty()).then(|| rnd(sum / edges.len() as f64, 3));
        Ok(Self { nodes, edges, degree, mean_length, connectivity })
    }
}

pub(super) fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

pub(super) fn severity(score: f64) -> &'static str {
    if score >= 0.85 {
        "critical"
    } else if score >= 0.65 {
        "high"
    } else if score >= 0.4 {
        "medium"
    } else {
        "low"
    }
}

/// Kind weight from material process, toughness and FDM layer risk.
pub(super) fn material_bias(m: &Material, kind: &str, load: LoadCase) -> f64 {
    let bend = if load == LoadCase::Bending { 1.15 } else { 1. };
    let buckle = (3500. / m.e.max(1.)).sqrt().clamp(0.45, 1.6);
    let v = match kind {
        "slender_strut" => buckle * match load {
            LoadCase::Compression => 1.2,
            LoadCase::Bending => 1.05,
            LoadCase::Tension => 0.85,
        },
        "bridge" | "thin_wall" => (0.55 + 0.7 * m.layer) * if m.fdm { 1.15 } else { 0.55 } * bend,
        "sharp_corner" | "body_corner" => m.notch * (1.25 - 0.45 * m.ductility) * bend,
        "free_end" | "hinge_node" => (0.85 + 0.35 * m.layer)
            * if load == LoadCase::Bending { 1.2 } else { 1. }
            * (1.1 - 0.2 * m.ductility),
        "underconnected" => (0.9 + 0.25 * m.layer) * bend,
        "long_span" => (0.75 + 0.35 * (1. - m.ductility) + 0.25 * m.layer) * bend,
        _ => 1.,
    };
    rnd(v, 3)
}

/// Severity palette: cooler for metals, warmer for brittle FDM.
pub(super) fn highlight_color(sev: &str, m: &Material) -> [f64; 4] {
    let i = ["critical", "high", "medium", "low"].iter().position(|s| *s == sev).unwrap_or(3);
    let table: [[f64; 4]; 4] = if !m.fdm {
        [[0.95, 0.2, 0.35, 1.], [1., 0.45, 0.15, 1.], [0.2, 0.75, 0.95, 1.], [0.35, 0.55, 0.85, 1.]]
    } else if m.ductility < 0.35 {
        [[1., 0.08, 0.12, 1.], [1., 0.35, 0.05, 1.], [1., 0.7, 0.1, 1.], [0.95, 0.85, 0.25, 1.]]
    } else {
        [[0.95, 0.15, 0.4, 1.], [1., 0.5, 0.12, 1.], [1., 0.75, 0.2, 1.], [0.55, 0.85, 0.35, 1.]]
    };
    table[i]
}

/// Parallel load paths across the section normal to the lattice axis.
pub(super) fn parallel_paths(o: &LatticeParams, size: [f64; 3], edge_count: usize) -> f64 {
    let cell = o.cell.max(o.rib * 2.);
    let cross = (size[(o.axis + 1) % 3] / cell).ceil().max(1.) * (size[(o.axis + 2) % 3] / cell).ceil().max(1.);
    match o.pattern.as_str() {
        "octet" => (cross * 2.).round().max(1.),
        "bcc" => (cross * 1.5).round().max(1.),
        _ if o.spatial() => ((edge_count.max(1) as f64).sqrt() / 2.).round().max(1.),
        _ => cross.max(1.),
    }
}

struct Spot {
    kind: &'static str,
    score: f64,
    at: [f64; 3],
    ru: String,
    en: String,
    detail: Vec<(&'static str, f64)>,
}

fn polygon_angles(poly: &[[f64; 2]]) -> Vec<([f64; 2], f64)> {
    let n = poly.len();
    let mut out = Vec::new();
    for i in 0..n {
        let (a, b, c) = (poly[(i + n - 1) % n], poly[i], poly[(i + 1) % n]);
        let ab = [a[0] - b[0], a[1] - b[1]];
        let cb = [c[0] - b[0], c[1] - b[1]];
        let (lab, lcb) = (ab[0].hypot(ab[1]), cb[0].hypot(cb[1]));
        if lab < 1e-9 || lcb < 1e-9 {
            continue;
        }
        let cos = ((ab[0] * cb[0] + ab[1] * cb[1]) / (lab * lcb)).clamp(-1., 1.);
        out.push((b, rnd(cos.acos().to_degrees(), 1)));
    }
    out
}

pub(super) struct WeakInput<'a> {
    pub o: &'a LatticeParams,
    pub options: &'a Value,
    pub min: [f64; 3],
    pub max: [f64; 3],
    pub graph: Option<&'a Graph>,
    pub material: &'a Material,
    pub load: LoadCase,
    pub force_n: f64,
}

fn spatial_spots(w: &WeakInput, g: &Graph, spots: &mut Vec<Spot>) {
    let (o, m, size) = (w.o, w.material, std::array::from_fn::<f64, 3, _>(|k| w.max[k] - w.min[k]));
    let r = o.rib / 2.;
    let gyration = r / 2.;
    let inertia = std::f64::consts::PI * r.powi(4) / 4.;
    let axis: [f64; 3] = if w.load == LoadCase::Bending {
        [1., 0., 0.]
    } else {
        std::array::from_fn(|k| if k == o.axis { 1. } else { 0. })
    };
    let diag = dist(w.min, w.max).max(1e-12);
    for (i, &p) in g.nodes.iter().enumerate() {
        let d = g.degree[i];
        let df = d as f64;
        if d <= 1 {
            spots.push(Spot { kind: "free_end", score: 0.92, at: p,
                ru: format!("Свободный конец стержня (Z={d}) — механизм/консоль, высокий изгиб."),
                en: format!("Free strut end (Z={d}) — mechanism/cantilever with high bending."),
                detail: vec![("degree", df)] });
        } else if d == 2 {
            spots.push(Spot { kind: "hinge_node", score: 0.78, at: p,
                ru: "Шарнирный узел (Z=2) — изгибная «цепочка», слабая на поперечную нагрузку.".into(),
                en: "Hinge node (Z=2) — bending chain, weak under transverse load.".into(),
                detail: vec![("degree", df)] });
        } else if d < 4 {
            spots.push(Spot { kind: "underconnected", score: 0.55, at: p,
                ru: format!("Слабо связанный узел (Z={d}) — локально ближе к изгибному режиму."),
                en: format!("Under-connected node (Z={d}) — locally closer to bending-dominated behaviour."),
                detail: vec![("degree", df)] });
        }
        let mut corner = None;
        for x in [w.min[0], w.max[0]] {
            for y in [w.min[1], w.max[1]] {
                for z in [w.min[2], w.max[2]] {
                    if dist(p, [x, y, z]) <= diag * 0.12 {
                        corner = Some([x, y, z]);
                    }
                }
            }
        }
        if let Some(c) = corner && d < 6 {
            spots.push(Spot { kind: "body_corner", score: rnd(0.5 + (6. - df) * 0.08, 3), at: p,
                ru: format!("Узел у угла тела (Z={d}) — типичный концентратор при сжатии/изгибе корпуса."),
                en: format!("Node at body corner (Z={d}) — typical concentrator under shell compression/bending."),
                detail: vec![("degree", df), ("cx", c[0]), ("cy", c[1]), ("cz", c[2])] });
        }
    }
    let paths = parallel_paths(o, size, g.edges.len());
    let force = w.force_n / paths;
    for &[a, b] in &g.edges {
        let (pa, pb) = (g.nodes[a], g.nodes[b]);
        let l = dist(pa, pb);
        if l < 1e-9 {
            continue;
        }
        let mid = std::array::from_fn(|k| (pa[k] + pb[k]) / 2.);
        let dir: [f64; 3] = std::array::from_fn(|k| (pb[k] - pa[k]) / l);
        let slender = l / gyration.max(1e-6);
        let align = (dir[0] * axis[0] + dir[1] * axis[1] + dir[2] * axis[2]).abs();
        let pcr = std::f64::consts::PI.powi(2) * m.e * inertia / (l * l);
        let buckling = force / pcr.max(1e-9);
        if slender > 40. || buckling > 0.5 {
            spots.push(Spot { kind: "slender_strut", score: rnd((slender / 80.).max(buckling).min(1.), 3), at: mid,
                ru: format!("Тонкий/длинный стержень λ≈{}, P/Pcr≈{} — риск потери устойчивости.", rnd(slender, 1), rnd(buckling, 2)),
                en: format!("Slender strut λ≈{}, P/Pcr≈{} — buckling risk.", rnd(slender, 1), rnd(buckling, 2)),
                detail: vec![("length", rnd(l, 2)), ("slenderness", rnd(slender, 1)), ("buckling", rnd(buckling, 3))] });
        }
        let horizontal = 1. - dir[2].abs();
        if horizontal > 0.85 && l > o.rib * 4. {
            spots.push(Spot { kind: "bridge", score: rnd((0.45 + l / (o.cell * 4.)).min(1.), 3), at: mid,
                ru: format!("Горизонтальный мост L≈{} mm — изгиб + риск провисания при печати.", rnd(l, 1)),
                en: format!("Horizontal bridge L≈{} mm — bending and print-sag risk.", rnd(l, 1)),
                detail: vec![("length", rnd(l, 2)), ("horizontal", rnd(horizontal, 2))] });
        }
        if l > g.mean_length.unwrap_or(o.cell) * 1.35 && align > 0.7 {
            spots.push(Spot { kind: "long_span", score: rnd((0.4 + align * 0.3).min(1.), 3), at: mid,
                ru: format!("Длинный пролёт вдоль нагрузки L≈{} mm — повышенные σ и прогиб.", rnd(l, 1)),
                en: format!("Long span aligned with load L≈{} mm — elevated stress and deflection.", rnd(l, 1)),
                detail: vec![("length", rnd(l, 2)), ("align", rnd(align, 2))] });
        }
    }
    if o.wall_depth > 0. && o.wall_depth < o.rib * 2. {
        let c = std::array::from_fn(|k| (w.min[k] + w.max[k]) / 2.);
        spots.push(Spot { kind: "thin_wall", score: 0.7, at: c,
            ru: format!("Скелетная стенка тоньше 2·диаметра стержня ({} < {} mm) — слабые стыки слоёв.", o.wall_depth, rnd(o.rib * 2., 1)),
            en: format!("Skeletal wall thinner than 2× strut diameter ({} < {} mm) — weak layer junctions.", o.wall_depth, rnd(o.rib * 2., 1)),
            detail: vec![("wallDepth", o.wall_depth), ("rib", o.rib)] });
    }
}

fn channel_spots(w: &WeakInput, spots: &mut Vec<Spot>) {
    let o = w.o;
    let (ax, u, v) = (o.axis, (o.axis + 1) % 3, (o.axis + 2) % 3);
    let mid_axis = (w.min[ax] + w.max[ax]) / 2.;
    let place = |pu: f64, pv: f64, pa: f64| {
        let mut at = [0.; 3];
        at[u] = pu;
        at[v] = pv;
        at[ax] = pa;
        at
    };
    let seed = w.options["seed"].as_f64().unwrap_or(0.);
    let jitter = w.options["jitter"].as_f64().unwrap_or(0.);
    // Generation limits must not block the rest of the screen.
    if let Ok(cells) = graph::generate_lightening_cells(
        [w.min[u] + o.rim, w.min[v] + o.rim],
        [w.max[u] - o.rim, w.max[v] - o.rim],
        &o.pattern, o.cell, o.rib, seed, jitter,
    ) {
        for cell in &cells {
            for (at, deg) in polygon_angles(cell) {
                if deg < 55. {
                    spots.push(Spot { kind: "sharp_corner", score: rnd(((55. - deg) / 55. + 0.45).min(1.), 3),
                        at: place(at[0], at[1], mid_axis),
                        ru: format!("Острый угол ячейки ≈{deg}° — концентратор напряжений в перемычке."),
                        en: format!("Acute cell corner ≈{deg}° — stress concentrator in the rib."),
                        detail: vec![("angleDeg", deg)] });
                }
            }
        }
    }
    let frame = if matches!(o.pattern.as_str(), "grid" | "web") { 0.62 } else { 0.48 };
    for uu in [w.min[u] + o.rim, w.max[u] - o.rim] {
        for vv in [w.min[v] + o.rim, w.max[v] - o.rim] {
            spots.push(Spot { kind: "body_corner", score: frame, at: place(uu, vv, mid_axis),
                ru: "Угол рамки канала — типичное место излома при изгибе панели.".into(),
                en: "Channel frame corner — typical break location under panel bending.".into(),
                detail: vec![] });
        }
    }
    let centre: [f64; 3] = std::array::from_fn(|k| (w.min[k] + w.max[k]) / 2.);
    let opening = o.cell - o.rib;
    if opening > o.rib * 5. {
        spots.push(Spot { kind: "long_span", score: rnd((opening / (o.rib * 8.)).min(1.), 3), at: centre,
            ru: format!("Широкий проём {} mm при перемычке {} mm — изгиб перемычек.", rnd(opening, 1), o.rib),
            en: format!("Wide opening {} mm with rib {} mm — rib bending.", rnd(opening, 1), o.rib),
            detail: vec![("opening", rnd(opening, 2)), ("rib", o.rib)] });
    }
    if o.top > 0. && o.axis == 2 {
        spots.push(Spot { kind: "bridge", score: 0.58, at: [centre[0], centre[1], w.max[2] - o.top / 2.],
            ru: "Верхняя кожа над каналами — мост при печати и изгибная пластинка.".into(),
            en: "Top skin over channels — print bridge and bending plate.".into(),
            detail: vec![] });
    }
}

/// Ranked, material-weighted weak spots with viewport colours. Keeps kind
/// diversity (≤3 per kind in the first pass) and spatial separation.
pub(super) fn find(w: &WeakInput, limit: usize) -> Vec<Value> {
    let mut spots = Vec::new();
    match w.graph {
        Some(g) if w.o.spatial() => spatial_spots(w, g, &mut spots),
        _ if !w.o.spatial() => channel_spots(w, &mut spots),
        _ => {}
    }
    let m = w.material;
    let mut scored: Vec<(Spot, f64, f64)> = spots
        .into_iter()
        .map(|s| {
            let bias = material_bias(m, s.kind, w.load);
            let score = rnd((s.score * bias).min(1.), 3);
            (s, bias, score)
        })
        .collect();
    scored.sort_by(|a, b| b.2.total_cmp(&a.2).then_with(|| a.0.kind.cmp(b.0.kind)));
    let sep = w.o.cell * 0.35;
    let mut kept: Vec<usize> = Vec::new();
    let mut per_kind: BTreeMap<&str, usize> = BTreeMap::new();
    for (i, (s, _, _)) in scored.iter().enumerate() {
        let count = per_kind.get(s.kind).copied().unwrap_or(0);
        if count >= 3 || kept.iter().any(|&k| scored[k].0.kind == s.kind && dist(scored[k].0.at, s.at) < sep) {
            continue;
        }
        kept.push(i);
        per_kind.insert(s.kind, count + 1);
        if kept.len() >= limit {
            break;
        }
    }
    if kept.len() < limit {
        for (i, (s, _, _)) in scored.iter().enumerate() {
            if kept.contains(&i) || kept.iter().any(|&k| dist(scored[k].0.at, s.at) < sep) {
                continue;
            }
            kept.push(i);
            if kept.len() >= limit {
                break;
            }
        }
    }
    kept.sort_by(|a, b| scored[*b].2.total_cmp(&scored[*a].2).then(a.cmp(b)));
    kept.into_iter()
        .map(|i| {
            let (s, bias, score) = &scored[i];
            let sev = severity(*score);
            let mut detail = value_codec::Map::new();
            for (k, v) in &s.detail {
                detail.insert((*k).into(), json!(*v));
            }
            detail.insert("materialBias".into(), json!(*bias));
            let (ru, en) = if *bias != 1. {
                (format!("{} [{}: ×{bias}]", s.ru, m.ru), format!("{} [{}: ×{bias}]", s.en, m.en))
            } else {
                (s.ru.clone(), s.en.clone())
            };
            json!({"kind": s.kind, "severity": sev, "score": *score, "at": s.at.to_vec(),
                "color": highlight_color(sev, m).to_vec(), "materialBias": *bias,
                "ru": ru, "en": en, "detail": Value::Object(detail)})
        })
        .collect()
}
