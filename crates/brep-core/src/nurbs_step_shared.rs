//! Shared STEP writer / parse helpers for freeform NURBS interchange (A1–A4).

use nurbs_core::{Error, Result, surface::Surface};
use std::collections::BTreeMap;

pub(crate) fn refuse(message: &str) -> Error {
    Error::new("BREP_NURBS_STEP_REFUSED", message)
}

pub(crate) fn is_uniform_bicubic_positive(surface: &Surface) -> bool {
    surface.degree_u == 3
        && surface.degree_v == 3
        && surface.control_points.len() == 4
        && surface.control_points.iter().all(|row| row.len() == 4)
        && surface.knots_u.len() == 8
        && surface.knots_v.len() == 8
        && surface
            .weights
            .iter()
            .flatten()
            .all(|w| w.is_finite() && (*w - 1.).abs() <= 1e-12)
        && !surface.periodic_u
        && !surface.periodic_v
}

fn mix_pt(a: &[f64], b: &[f64], t: f64) -> Vec<f64> {
    a.iter()
        .zip(b.iter())
        .map(|(u, v)| (1. - t) * u + t * v)
        .collect()
}

/// Elevate planar bilinear 2×2 (or already-uniform bicubic) — same logic as nurbs_ss_g6.
pub(crate) fn as_uniform_bicubic(surface: &Surface) -> Option<Surface> {
    if is_uniform_bicubic_positive(surface) {
        return Some(surface.clone());
    }
    if surface.periodic_u || surface.periodic_v {
        return None;
    }
    if surface.degree_u != 1 || surface.degree_v != 1 {
        return None;
    }
    if surface.control_points.len() != 2 || surface.control_points.iter().any(|r| r.len() != 2) {
        return None;
    }
    if surface
        .weights
        .iter()
        .flatten()
        .any(|w| !w.is_finite() || (*w - 1.).abs() > 1e-12)
    {
        return None;
    }
    let p00 = &surface.control_points[0][0];
    let p10 = &surface.control_points[0][1];
    let p01 = &surface.control_points[1][0];
    let p11 = &surface.control_points[1][1];
    let row = |a: &[f64], b: &[f64]| -> Vec<Vec<f64>> {
        vec![
            a.to_vec(),
            mix_pt(a, b, 1. / 3.),
            mix_pt(a, b, 2. / 3.),
            b.to_vec(),
        ]
    };
    let r0 = row(p00, p10);
    let r1 = row(p01, p11);
    let mut control_points = Vec::with_capacity(4);
    for j in 0..4 {
        let t = j as f64 / 3.;
        control_points.push((0..4).map(|i| mix_pt(&r0[i], &r1[i], t)).collect());
    }
    Some(Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points,
        weights: vec![vec![1.; 4]; 4],
        periodic_u: false,
        periodic_v: false,
    })
}

pub(crate) fn corner_xyz(surface: &Surface, u: f64, v: f64) -> Result<[f64; 3]> {
    let p = surface.evaluate(u, v)?.point;
    if p.len() < 3 || !p.iter().all(|c| c.is_finite()) {
        return Err(refuse("Surface corner evaluate failed"));
    }
    Ok([p[0], p[1], p[2]])
}

pub(crate) fn is_planar_surface(surface: &Surface) -> bool {
    let pts: Vec<[f64; 3]> = surface
        .control_points
        .iter()
        .flatten()
        .filter_map(|p| {
            if p.len() >= 3 {
                Some([p[0], p[1], p[2]])
            } else {
                None
            }
        })
        .collect();
    if pts.len() < 3 {
        return false;
    }
    let o = pts[0];
    let mut n = [0., 0., 0.];
    for i in 1..pts.len() {
        for j in (i + 1)..pts.len() {
            let a = [pts[i][0] - o[0], pts[i][1] - o[1], pts[i][2] - o[2]];
            let b = [pts[j][0] - o[0], pts[j][1] - o[1], pts[j][2] - o[2]];
            let c = [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ];
            let len = (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt();
            if len > 1e-12 {
                n = [c[0] / len, c[1] / len, c[2] / len];
                break;
            }
        }
        if n[0] != 0. || n[1] != 0. || n[2] != 0. {
            break;
        }
    }
    if n[0] == 0. && n[1] == 0. && n[2] == 0. {
        return true; // degenerate colinear → treat as planar
    }
    pts.iter().all(|p| {
        let d = (p[0] - o[0]) * n[0] + (p[1] - o[1]) * n[1] + (p[2] - o[2]) * n[2];
        d.abs() <= 1e-8
    })
}

pub(crate) struct StepWriter {
    pub next: usize,
    pub lines: Vec<String>,
}

impl StepWriter {
    pub fn new() -> Self {
        Self {
            next: 1,
            lines: Vec::new(),
        }
    }

    pub fn emit(&mut self, body: String) -> usize {
        let id = self.next;
        self.next += 1;
        self.lines.push(format!("#{id}={body};"));
        id
    }

    pub fn cartesian(&mut self, p: [f64; 3]) -> usize {
        self.emit(format!(
            "CARTESIAN_POINT('',({:.15},{:.15},{:.15}))",
            p[0], p[1], p[2]
        ))
    }

    pub fn cartesian2(&mut self, p: [f64; 2]) -> usize {
        self.emit(format!("CARTESIAN_POINT('',({:.15},{:.15}))", p[0], p[1]))
    }

    pub fn direction(&mut self, d: [f64; 3]) -> usize {
        self.emit(format!(
            "DIRECTION('',({:.15},{:.15},{:.15}))",
            d[0], d[1], d[2]
        ))
    }

    pub fn direction2(&mut self, d: [f64; 2]) -> usize {
        self.emit(format!("DIRECTION('',({:.15},{:.15}))", d[0], d[1]))
    }

    pub fn vertex_point(&mut self, point_id: usize) -> usize {
        self.emit(format!("VERTEX_POINT('',#{point_id})"))
    }

    pub fn line_edge(&mut self, va: usize, vb: usize, pa: usize, dir: [f64; 3]) -> usize {
        let d = self.direction(dir);
        let vec = self.emit(format!("VECTOR('',#{d},1.)"));
        let line = self.emit(format!("LINE('',#{pa},#{vec})"));
        self.emit(format!("EDGE_CURVE('',#{va},#{vb},#{line},.T.)"))
    }

    /// Emit an EDGE_CURVE whose 3D LINE is explicitly associated with its
    /// surface-space LINE through PCURVE/SURFACE_CURVE.
    pub fn pcurve_line_edge(
        &mut self,
        va: usize,
        vb: usize,
        pa: usize,
        dir: [f64; 3],
        surface: usize,
        uv_a: [f64; 2],
        uv_b: [f64; 2],
    ) -> usize {
        let d3 = self.direction(dir);
        let vec3 = self.emit(format!("VECTOR('',#{d3},1.)"));
        let line3 = self.emit(format!("LINE('',#{pa},#{vec3})"));
        let uvp = self.cartesian2(uv_a);
        let uvd = self.direction2([uv_b[0] - uv_a[0], uv_b[1] - uv_a[1]]);
        let uvvec = self.emit(format!("VECTOR('',#{uvd},1.)"));
        let uvline = self.emit(format!("LINE('',#{uvp},#{uvvec})"));
        let pcurve = self.emit(format!("PCURVE('',#{surface},#{uvline})"));
        let surface_curve = self.emit(format!(
            "SURFACE_CURVE('',#{line3},(#{pcurve}),.PCURVE_S1.)"
        ));
        self.emit(format!("EDGE_CURVE('',#{va},#{vb},#{surface_curve},.T.)"))
    }
}

pub(crate) fn fmt_list_f64(vals: &[f64]) -> String {
    vals.iter()
        .map(|v| format!("{:.15}", v))
        .collect::<Vec<_>>()
        .join(",")
}

pub(crate) fn fmt_refs(ids: &[usize]) -> String {
    ids.iter()
        .map(|id| format!("#{id}"))
        .collect::<Vec<_>>()
        .join(",")
}

pub(crate) fn emit_b_spline_surface(w: &mut StepWriter, surface: &Surface) -> usize {
    let mut cp_ids = Vec::with_capacity(surface.control_points.len());
    for row in &surface.control_points {
        let mut row_ids = Vec::with_capacity(row.len());
        for p in row {
            row_ids.push(w.cartesian([p[0], p[1], p[2]]));
        }
        cp_ids.push(row_ids);
    }
    let control_grid = cp_ids
        .iter()
        .map(|row| format!("({})", fmt_refs(row)))
        .collect::<Vec<_>>()
        .join(",");
    let weights_flat: Vec<f64> = surface.weights.iter().flatten().copied().collect();
    let u_mults = [surface.degree_u + 1, surface.degree_u + 1];
    let v_mults = [surface.degree_v + 1, surface.degree_v + 1];
    let u_knots = [0.0_f64, 1.0];
    let v_knots = [0.0_f64, 1.0];
    w.emit(format!(
        "B_SPLINE_SURFACE_WITH_KNOTS('',{du},{dv},({control_grid}),.UNSPECIFIED.,.UNSPECIFIED.,.F.,.F.,.F.,({weights}),({u_m}),({v_m}),({u_k}),({v_k}),.UNSPECIFIED.)",
        du = surface.degree_u,
        dv = surface.degree_v,
        weights = fmt_list_f64(&weights_flat),
        u_m = u_mults
            .iter()
            .map(|m| m.to_string())
            .collect::<Vec<_>>()
            .join(","),
        v_m = v_mults
            .iter()
            .map(|m| m.to_string())
            .collect::<Vec<_>>()
            .join(","),
        u_k = fmt_list_f64(&u_knots),
        v_k = fmt_list_f64(&v_knots),
    ))
}

pub(crate) fn parse_entities(text: &str) -> BTreeMap<usize, (String, String)> {
    let mut map = BTreeMap::new();
    let flat = text.replace('\n', " ").replace('\r', " ");
    for chunk in flat.split(';') {
        let chunk = chunk.trim();
        if !chunk.starts_with('#') {
            continue;
        }
        let Some((id_s, body)) = chunk.split_once('=') else {
            continue;
        };
        let Ok(id) = id_s.trim().trim_start_matches('#').parse::<usize>() else {
            continue;
        };
        let body = body.trim();
        let Some(paren) = body.find('(') else {
            continue;
        };
        let ty = body[..paren].trim().to_string();
        let args = body[paren + 1..].trim_end_matches(')').to_string();
        map.insert(id, (ty, args));
    }
    map
}

pub(crate) fn resolve_cartesian(
    entities: &BTreeMap<usize, (String, String)>,
    id: usize,
) -> Option<[f64; 3]> {
    let (ty, args) = entities.get(&id)?;
    if ty != "CARTESIAN_POINT" {
        return None;
    }
    let coords = args.rsplit_once('(')?.1.trim_end_matches(')');
    let mut vals = Vec::new();
    for part in coords.split(',') {
        vals.push(part.trim().parse::<f64>().ok()?);
    }
    if vals.len() < 3 {
        return None;
    }
    Some([vals[0], vals[1], vals[2]])
}

pub(crate) fn parse_f64_list(s: &str) -> Option<Vec<f64>> {
    let s = s.trim().trim_start_matches('(').trim_end_matches(')');
    if s.is_empty() {
        return Some(Vec::new());
    }
    s.split(',').map(|t| t.trim().parse::<f64>().ok()).collect()
}

pub(crate) fn parse_usize_list(s: &str) -> Option<Vec<usize>> {
    let s = s.trim().trim_start_matches('(').trim_end_matches(')');
    if s.is_empty() {
        return Some(Vec::new());
    }
    s.split(',')
        .map(|t| t.trim().parse::<usize>().ok())
        .collect()
}

/// Split top-level comma-separated STEP args respecting parentheses.
pub(crate) fn split_top_args(args: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    for ch in args.chars() {
        match ch {
            '(' => {
                depth += 1;
                cur.push(ch);
            }
            ')' => {
                depth -= 1;
                cur.push(ch);
            }
            ',' if depth == 0 => {
                out.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(ch),
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

pub(crate) fn parse_control_grid(
    entities: &BTreeMap<usize, (String, String)>,
    grid_arg: &str,
) -> Option<Vec<Vec<Vec<f64>>>> {
    let inner = grid_arg.trim();
    if !inner.starts_with('(') {
        return None;
    }
    let rows = split_top_args(&inner[1..inner.len().saturating_sub(1)]);
    let mut control_points = Vec::new();
    for row in rows {
        let refs = parse_usize_list(&row.replace('#', ""))?;
        let mut pts = Vec::new();
        for id in refs {
            let p = resolve_cartesian(entities, id)?;
            pts.push(p.to_vec());
        }
        control_points.push(pts);
    }
    Some(control_points)
}

pub(crate) fn surface_from_b_spline_args(
    entities: &BTreeMap<usize, (String, String)>,
    args: &str,
) -> Result<Surface> {
    let parts = split_top_args(args);
    if parts.len() < 14 {
        return Err(refuse("B_SPLINE_SURFACE_WITH_KNOTS arg count incomplete"));
    }
    let degree_u: usize = parts[1].parse().map_err(|_| refuse("Bad degree_u"))?;
    let degree_v: usize = parts[2].parse().map_err(|_| refuse("Bad degree_v"))?;
    let control_points = parse_control_grid(entities, &parts[3])
        .ok_or_else(|| refuse("Failed to parse control point grid"))?;
    let weights_flat = parse_f64_list(&parts[9]).ok_or_else(|| refuse("Bad weights"))?;
    let u_mults = parse_usize_list(&parts[10]).ok_or_else(|| refuse("Bad u multiplicities"))?;
    let v_mults = parse_usize_list(&parts[11]).ok_or_else(|| refuse("Bad v multiplicities"))?;
    let u_knot_vals = parse_f64_list(&parts[12]).ok_or_else(|| refuse("Bad u knots"))?;
    let v_knot_vals = parse_f64_list(&parts[13]).ok_or_else(|| refuse("Bad v knots"))?;

    let expand_knots = |mults: &[usize], vals: &[f64]| -> Result<Vec<f64>> {
        if mults.len() != vals.len() {
            return Err(refuse("Knot multiplicity length mismatch"));
        }
        let mut out = Vec::new();
        for (m, v) in mults.iter().zip(vals.iter()) {
            for _ in 0..*m {
                out.push(*v);
            }
        }
        Ok(out)
    };
    let knots_u = expand_knots(&u_mults, &u_knot_vals)?;
    let knots_v = expand_knots(&v_mults, &v_knot_vals)?;
    let nu = control_points.len();
    let nv = control_points.first().map(|r| r.len()).unwrap_or(0);
    if nu == 0 || nv == 0 || control_points.iter().any(|r| r.len() != nv) {
        return Err(refuse("Control net shape invalid"));
    }
    if weights_flat.len() != nu * nv {
        return Err(refuse("Weight count mismatch"));
    }
    let mut weights = Vec::with_capacity(nu);
    for i in 0..nu {
        weights.push(weights_flat[i * nv..(i + 1) * nv].to_vec());
    }
    Ok(Surface {
        degree_u,
        degree_v,
        knots_u,
        knots_v,
        control_points,
        weights,
        periodic_u: false,
        periodic_v: false,
    })
}

pub(crate) fn parse_b_spline_surfaces(
    entities: &BTreeMap<usize, (String, String)>,
) -> Result<Vec<(usize, Surface)>> {
    let mut out = Vec::new();
    for (id, (ty, args)) in entities {
        if ty == "B_SPLINE_SURFACE_WITH_KNOTS" {
            out.push((*id, surface_from_b_spline_args(entities, args)?));
        }
    }
    if out.is_empty() {
        return Err(refuse("Missing B_SPLINE_SURFACE_WITH_KNOTS"));
    }
    Ok(out)
}

pub(crate) fn parse_b_spline_surface(
    entities: &BTreeMap<usize, (String, String)>,
) -> Result<Surface> {
    let surfs = parse_b_spline_surfaces(entities)?;
    if surfs.len() != 1 {
        return Err(refuse("Expected exactly one B_SPLINE_SURFACE_WITH_KNOTS"));
    }
    let surface = surfs.into_iter().next().unwrap().1;
    if !is_uniform_bicubic_positive(&surface) {
        return Err(refuse(
            "Imported surface outside freeform NURBS STEP (bicubic w≡1)",
        ));
    }
    Ok(surface)
}

pub(crate) fn refuse_mesh_payloads_common(text: &str) -> Result<()> {
    if !text.contains("ISO-10303-21") {
        return Err(refuse("Not an ISO-10303-21 STEP exchange"));
    }
    if text.contains("FACETED_BREP") && !text.contains("ADVANCED_FACE") {
        return Err(refuse(
            "Faceted STEP is not freeform NURBS face interchange",
        ));
    }
    let upper = text.to_ascii_uppercase();
    if upper.contains("SOLID ASCII")
        || upper.contains("ENDSOLID")
        || upper
            .lines()
            .any(|l| l.trim_start().starts_with("FACET NORMAL"))
    {
        return Err(refuse("STL mesh payload refused as NURBS STEP"));
    }
    if (upper.contains("MTLLIB") || upper.lines().any(|l| l.trim_start().starts_with("F ")))
        && !text.contains("ISO-10303-21")
    {
        return Err(refuse("OBJ mesh payload refused as NURBS STEP"));
    }
    if !text.contains("B_SPLINE_SURFACE_WITH_KNOTS") {
        return Err(refuse("Missing B_SPLINE_SURFACE_WITH_KNOTS"));
    }
    if text.contains("OSCAD_SOLID") {
        return Err(refuse("OSCAD_SOLID refused"));
    }
    Ok(())
}

pub(crate) fn invert_uv(surface: &Surface, point: [f64; 3]) -> Result<[f64; 2]> {
    let mut best = [0.5, 0.5];
    let mut best_d = f64::INFINITY;
    for i in 0..=48 {
        for j in 0..=48 {
            let u = i as f64 / 48.;
            let v = j as f64 / 48.;
            let p = surface.evaluate(u, v)?.point;
            let d =
                (p[0] - point[0]).powi(2) + (p[1] - point[1]).powi(2) + (p[2] - point[2]).powi(2);
            if d < best_d {
                best_d = d;
                best = [u, v];
            }
        }
    }
    let mut u = best[0];
    let mut v = best[1];
    let mut step = 0.04;
    for _ in 0..16 {
        let mut improved = false;
        for (du, dv) in [
            (-step, 0.),
            (step, 0.),
            (0., -step),
            (0., step),
            (-step, -step),
            (step, step),
            (-step, step),
            (step, -step),
        ] {
            let nu = (u + du).clamp(0., 1.);
            let nv = (v + dv).clamp(0., 1.);
            let p = surface.evaluate(nu, nv)?.point;
            let d =
                (p[0] - point[0]).powi(2) + (p[1] - point[1]).powi(2) + (p[2] - point[2]).powi(2);
            if d + 1e-18 < best_d {
                best_d = d;
                u = nu;
                v = nv;
                improved = true;
            }
        }
        if !improved {
            step *= 0.5;
        }
    }
    if best_d > 1e-2 {
        return Err(refuse("Failed to invert surface UV for hole vertex"));
    }
    Ok([u, v])
}

pub(crate) fn surface_aabb(surface: &Surface) -> ([f64; 3], [f64; 3]) {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for p in surface.control_points.iter().flatten() {
        for i in 0..3 {
            min[i] = min[i].min(p[i]);
            max[i] = max[i].max(p[i]);
        }
    }
    (min, max)
}

pub(crate) fn step_header(filename: &str, description: &str) -> Vec<String> {
    vec![
        "ISO-10303-21;".into(),
        "HEADER;".into(),
        format!("FILE_DESCRIPTION(('{description}'),'2;1');"),
        format!(
            "FILE_NAME('{filename}','2026-09-16',('open-scad-viewer'),(''),'nurbs-step','','');"
        ),
        "FILE_SCHEMA(('AUTOMOTIVE_DESIGN','AP242_MANAGED_MODEL_BASED_3D_ENGINEERING'));".into(),
        "ENDSEC;".into(),
        "DATA;".into(),
    ]
}
