//! Shared STEP writer / parse helpers for freeform NURBS interchange (A1–A4).

use nurbs_core::{Error, Result, surface::Surface};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const MAX_STEP_PAYLOAD_BYTES: usize = 8 * 1024 * 1024;

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
    let flat = text.replace(['\n', '\r'], " ");
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
        if body.starts_with('(') {
            map.insert(id, ("COMPLEX".into(), body.to_string()));
            continue;
        }
        let Some(paren) = body.find('(') else {
            continue;
        };
        let ty = body[..paren].trim().to_string();
        let args = body[paren + 1..].trim_end_matches(')').to_string();
        map.insert(id, (ty, args));
    }
    map
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum StepGraphRoot {
    OpenShell,
    Solid,
    SolidMany,
}

pub(crate) struct LinkedFace {
    pub surface_id: usize,
    pub outer_vertex_ids: Vec<usize>,
    pub hole_vertex_ids: Vec<Vec<usize>>,
}

pub(crate) struct LinkedStepGraph {
    pub faces: Vec<LinkedFace>,
    pub body_face_ranges: Vec<std::ops::Range<usize>>,
}

fn refs(token: &str) -> Vec<usize> {
    let bytes = token.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'#' {
            i += 1;
            continue;
        }
        i += 1;
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if start == i {
            continue;
        }
        if let Ok(id) = token[start..i].parse() {
            out.push(id);
        }
    }
    out
}

fn one_ref(token: &str, message: &str) -> Result<usize> {
    let found = refs(token);
    if found.len() != 1 {
        return Err(refuse(message));
    }
    Ok(found[0])
}

fn entity<'a>(
    entities: &'a BTreeMap<usize, (String, String)>,
    id: usize,
    expected: &str,
    message: &str,
) -> Result<&'a str> {
    match entities.get(&id) {
        Some((ty, args)) if ty == expected => Ok(args),
        _ => Err(refuse(message)),
    }
}

#[derive(Default)]
struct GraphMarks {
    shells: BTreeSet<usize>,
    faces: BTreeSet<usize>,
    outer_bounds: BTreeSet<usize>,
    hole_bounds: BTreeSet<usize>,
    loops: BTreeSet<usize>,
    oriented_edges: BTreeSet<usize>,
    edge_curves: BTreeSet<usize>,
    vertices: BTreeSet<usize>,
    surfaces: BTreeSet<usize>,
    active_geometry: BTreeSet<usize>,
    geometry: BTreeSet<usize>,
}

fn validate_curve_geometry(
    entities: &BTreeMap<usize, (String, String)>,
    geometry_id: usize,
    face_surface_id: usize,
    marks: &mut GraphMarks,
) -> Result<()> {
    if !marks.active_geometry.insert(geometry_id) {
        return Err(refuse("Cyclic EDGE_CURVE geometry references"));
    }
    marks.geometry.insert(geometry_id);
    let (ty, args) = entities
        .get(&geometry_id)
        .ok_or_else(|| refuse("EDGE_CURVE geometry reference broken"))?;
    let parts = split_top_args(args);
    match ty.as_str() {
        "LINE" => {
            if parts.len() != 3 {
                return Err(refuse("LINE argument graph incomplete"));
            }
            let point = one_ref(&parts[1], "LINE missing point reference")?;
            entity(
                entities,
                point,
                "CARTESIAN_POINT",
                "LINE point reference has wrong type",
            )?;
            let vector = one_ref(&parts[2], "LINE missing vector reference")?;
            marks.geometry.insert(vector);
            let vector_args = entity(
                entities,
                vector,
                "VECTOR",
                "LINE vector reference has wrong type",
            )?;
            let vector_parts = split_top_args(vector_args);
            if vector_parts.len() != 3 {
                return Err(refuse("VECTOR argument graph incomplete"));
            }
            let direction = one_ref(&vector_parts[1], "VECTOR missing direction reference")?;
            marks.geometry.insert(direction);
            entity(
                entities,
                direction,
                "DIRECTION",
                "VECTOR direction reference has wrong type",
            )?;
        }
        "SURFACE_CURVE" => {
            if parts.len() < 4 {
                return Err(refuse("SURFACE_CURVE argument graph incomplete"));
            }
            let line = one_ref(&parts[1], "SURFACE_CURVE missing 3D curve reference")?;
            validate_curve_geometry(entities, line, face_surface_id, marks)?;
            let pcurves = refs(&parts[2]);
            if pcurves.is_empty() {
                return Err(refuse("SURFACE_CURVE missing PCURVE reference"));
            }
            for pcurve in pcurves {
                marks.geometry.insert(pcurve);
                let pcurve_args = entity(
                    entities,
                    pcurve,
                    "PCURVE",
                    "SURFACE_CURVE PCURVE reference has wrong type",
                )?;
                let pcurve_parts = split_top_args(pcurve_args);
                if pcurve_parts.len() != 3
                    || one_ref(&pcurve_parts[1], "PCURVE missing surface reference")?
                        != face_surface_id
                {
                    return Err(refuse("PCURVE is not linked to its ADVANCED_FACE surface"));
                }
                let curve_2d = one_ref(&pcurve_parts[2], "PCURVE missing 2D curve reference")?;
                validate_curve_geometry(entities, curve_2d, face_surface_id, marks)?;
            }
        }
        _ => return Err(refuse("EDGE_CURVE geometry has unsupported or wrong type")),
    }
    marks.active_geometry.remove(&geometry_id);
    Ok(())
}

fn point_components(entities: &BTreeMap<usize, (String, String)>, id: usize) -> Result<Vec<f64>> {
    let args = entity(
        entities,
        id,
        "CARTESIAN_POINT",
        "Curve point reference has wrong type",
    )?;
    let coords = args
        .rsplit_once('(')
        .map(|(_, values)| values.trim_end_matches(')'))
        .ok_or_else(|| refuse("CARTESIAN_POINT coordinates malformed"))?;
    coords
        .split(',')
        .map(|value| {
            value
                .trim()
                .parse::<f64>()
                .map_err(|_| refuse("CARTESIAN_POINT coordinate is invalid"))
        })
        .collect()
}

fn line_origin_delta(
    entities: &BTreeMap<usize, (String, String)>,
    id: usize,
) -> Result<(Vec<f64>, Vec<f64>)> {
    let args = entity(entities, id, "LINE", "Expected LINE geometry")?;
    let parts = split_top_args(args);
    if parts.len() != 3 {
        return Err(refuse("LINE argument graph incomplete"));
    }
    let origin = point_components(
        entities,
        one_ref(&parts[1], "LINE missing point reference")?,
    )?;
    let vector_args = entity(
        entities,
        one_ref(&parts[2], "LINE missing vector reference")?,
        "VECTOR",
        "LINE vector reference has wrong type",
    )?;
    let vector_parts = split_top_args(vector_args);
    if vector_parts.len() != 3 {
        return Err(refuse("VECTOR argument graph incomplete"));
    }
    let direction_args = entity(
        entities,
        one_ref(&vector_parts[1], "VECTOR missing direction reference")?,
        "DIRECTION",
        "VECTOR direction reference has wrong type",
    )?;
    let direction = point_components_from_args(direction_args)?;
    let magnitude = vector_parts[2]
        .parse::<f64>()
        .map_err(|_| refuse("VECTOR magnitude is invalid"))?;
    if origin.len() != direction.len() || !magnitude.is_finite() {
        return Err(refuse("LINE point and direction dimensions disagree"));
    }
    Ok((
        origin,
        direction
            .into_iter()
            .map(|value| value * magnitude)
            .collect(),
    ))
}

fn point_components_from_args(args: &str) -> Result<Vec<f64>> {
    let coords = args
        .rsplit_once('(')
        .map(|(_, values)| values.trim_end_matches(')'))
        .ok_or_else(|| refuse("DIRECTION coordinates malformed"))?;
    coords
        .split(',')
        .map(|value| {
            value
                .trim()
                .parse::<f64>()
                .map_err(|_| refuse("DIRECTION coordinate is invalid"))
        })
        .collect()
}

fn validate_curve_pcurve_correspondence(
    entities: &BTreeMap<usize, (String, String)>,
    geometry_id: usize,
    surface_id: usize,
    vertex_ids: [usize; 2],
) -> Result<()> {
    let Some((ty, args)) = entities.get(&geometry_id) else {
        return Err(refuse("EDGE_CURVE geometry reference broken"));
    };
    if ty != "SURFACE_CURVE" {
        return Ok(());
    }
    let parts = split_top_args(args);
    let line_id = one_ref(&parts[1], "SURFACE_CURVE missing 3D curve reference")?;
    let (line_origin, line_delta) = line_origin_delta(entities, line_id)?;
    if line_origin.len() != 3 {
        return Err(refuse("SURFACE_CURVE 3D curve is not three-dimensional"));
    }
    let surface_args = &entities
        .get(&surface_id)
        .ok_or_else(|| refuse("PCURVE surface reference broken"))?
        .1;
    let surface = surface_from_b_spline_args(entities, surface_args)?;
    let mut vertices = Vec::new();
    for vertex_id in vertex_ids {
        let vertex_args = entity(
            entities,
            vertex_id,
            "VERTEX_POINT",
            "EDGE_CURVE vertex reference has wrong type",
        )?;
        let point_id = one_ref(
            &split_top_args(vertex_args)[1],
            "VERTEX_POINT missing point reference",
        )?;
        vertices.push(point_components(entities, point_id)?);
    }
    for pcurve_id in refs(&parts[2]) {
        let pcurve_args = entity(
            entities,
            pcurve_id,
            "PCURVE",
            "SURFACE_CURVE PCURVE reference has wrong type",
        )?;
        let pcurve_parts = split_top_args(pcurve_args);
        let uv_line = one_ref(&pcurve_parts[2], "PCURVE missing 2D curve reference")?;
        let (uv0, duv) = line_origin_delta(entities, uv_line)?;
        if uv0.len() != 2 {
            return Err(refuse("PCURVE curve is not two-dimensional"));
        }
        for t in [0., 0.5, 1.] {
            let uv = [uv0[0] + t * duv[0], uv0[1] + t * duv[1]];
            let mapped = surface.evaluate(uv[0], uv[1])?.point;
            let on_3d = [
                line_origin[0] + t * line_delta[0],
                line_origin[1] + t * line_delta[1],
                line_origin[2] + t * line_delta[2],
            ];
            let error = (mapped[0] - on_3d[0])
                .hypot(mapped[1] - on_3d[1])
                .hypot(mapped[2] - on_3d[2]);
            if !error.is_finite() || error > 1e-5 {
                return Err(refuse(
                    "Independent 3D curve and PCURVE correspondence check failed",
                ));
            }
        }
        let endpoint_error =
            |a: &[f64], b: &[f64]| (a[0] - b[0]).hypot(a[1] - b[1]).hypot(a[2] - b[2]);
        let end = surface.evaluate(uv0[0] + duv[0], uv0[1] + duv[1])?.point;
        let start = surface.evaluate(uv0[0], uv0[1])?.point;
        let forward = endpoint_error(&start, &vertices[0]) + endpoint_error(&end, &vertices[1]);
        let reverse = endpoint_error(&start, &vertices[1]) + endpoint_error(&end, &vertices[0]);
        if forward.min(reverse) > 2e-5 {
            return Err(refuse("PCURVE endpoints do not match EDGE_CURVE vertices"));
        }
    }
    Ok(())
}

fn walk_loop(
    entities: &BTreeMap<usize, (String, String)>,
    loop_id: usize,
    face_surface_id: usize,
    marks: &mut GraphMarks,
) -> Result<Vec<usize>> {
    if !marks.loops.insert(loop_id) {
        return Err(refuse("EDGE_LOOP is linked more than once"));
    }
    let loop_args = entity(
        entities,
        loop_id,
        "EDGE_LOOP",
        "FACE bound EDGE_LOOP reference broken or wrong type",
    )?;
    let loop_parts = split_top_args(loop_args);
    if loop_parts.len() != 2 {
        return Err(refuse("EDGE_LOOP argument graph incomplete"));
    }
    let oriented_ids = refs(&loop_parts[1]);
    if oriented_ids.is_empty() {
        return Err(refuse("EDGE_LOOP has no ORIENTED_EDGE references"));
    }
    let mut vertex_ids = Vec::new();
    for oriented_id in oriented_ids {
        if !marks.oriented_edges.insert(oriented_id) {
            return Err(refuse("ORIENTED_EDGE is linked more than once"));
        }
        let oriented_args = entity(
            entities,
            oriented_id,
            "ORIENTED_EDGE",
            "EDGE_LOOP reference is not an ORIENTED_EDGE",
        )?;
        let oriented_parts = split_top_args(oriented_args);
        if oriented_parts.len() != 5 {
            return Err(refuse("ORIENTED_EDGE argument graph incomplete"));
        }
        let edge_id = one_ref(
            &oriented_parts[3],
            "ORIENTED_EDGE missing EDGE_CURVE reference",
        )?;
        marks.edge_curves.insert(edge_id);
        let edge_args = entity(
            entities,
            edge_id,
            "EDGE_CURVE",
            "ORIENTED_EDGE reference is not an EDGE_CURVE",
        )?;
        let edge_parts = split_top_args(edge_args);
        if edge_parts.len() != 5 {
            return Err(refuse("EDGE_CURVE argument graph incomplete"));
        }
        let mut edge_vertices = [0usize; 2];
        for (position, token) in [&edge_parts[1], &edge_parts[2]].into_iter().enumerate() {
            let vertex_id = one_ref(token, "EDGE_CURVE missing vertex reference")?;
            let vertex_args = entity(
                entities,
                vertex_id,
                "VERTEX_POINT",
                "EDGE_CURVE vertex reference has wrong type",
            )?;
            let vertex_parts = split_top_args(vertex_args);
            if vertex_parts.len() != 2 {
                return Err(refuse("VERTEX_POINT argument graph incomplete"));
            }
            let point_id = one_ref(&vertex_parts[1], "VERTEX_POINT missing point reference")?;
            entity(
                entities,
                point_id,
                "CARTESIAN_POINT",
                "VERTEX_POINT point reference has wrong type",
            )?;
            marks.vertices.insert(vertex_id);
            vertex_ids.push(vertex_id);
            edge_vertices[position] = vertex_id;
        }
        let geometry_id = one_ref(&edge_parts[3], "EDGE_CURVE missing geometry reference")?;
        validate_curve_geometry(entities, geometry_id, face_surface_id, marks)?;
        validate_curve_pcurve_correspondence(
            entities,
            geometry_id,
            face_surface_id,
            edge_vertices,
        )?;
    }
    vertex_ids.sort_unstable();
    vertex_ids.dedup();
    Ok(vertex_ids)
}

fn walk_face(
    entities: &BTreeMap<usize, (String, String)>,
    face_id: usize,
    marks: &mut GraphMarks,
) -> Result<LinkedFace> {
    if !marks.faces.insert(face_id) {
        return Err(refuse("ADVANCED_FACE is linked more than once"));
    }
    let face_args = entity(
        entities,
        face_id,
        "ADVANCED_FACE",
        "Shell face reference is not an ADVANCED_FACE",
    )?;
    let face_parts = split_top_args(face_args);
    if face_parts.len() != 4 {
        return Err(refuse("ADVANCED_FACE argument graph incomplete"));
    }
    let surface_id = one_ref(&face_parts[2], "ADVANCED_FACE missing surface reference")?;
    entity(
        entities,
        surface_id,
        "B_SPLINE_SURFACE_WITH_KNOTS",
        "ADVANCED_FACE surface reference has wrong type",
    )?;
    if !marks.surfaces.insert(surface_id) {
        return Err(refuse(
            "B_SPLINE_SURFACE_WITH_KNOTS is linked by multiple faces",
        ));
    }

    let bound_ids = refs(&face_parts[1]);
    if bound_ids.is_empty() {
        return Err(refuse("ADVANCED_FACE has no FACE bounds"));
    }
    let mut outer = None;
    let mut holes = Vec::new();
    for bound_id in bound_ids {
        let (bound_ty, bound_args) = entities
            .get(&bound_id)
            .ok_or_else(|| refuse("ADVANCED_FACE bound reference broken"))?;
        if bound_ty != "FACE_OUTER_BOUND" && bound_ty != "FACE_BOUND" {
            return Err(refuse("ADVANCED_FACE bound reference has wrong type"));
        }
        let bound_parts = split_top_args(bound_args);
        if bound_parts.len() != 3 {
            return Err(refuse("FACE bound argument graph incomplete"));
        }
        let loop_id = one_ref(&bound_parts[1], "FACE bound missing EDGE_LOOP reference")?;
        let vertices = walk_loop(entities, loop_id, surface_id, marks)?;
        if bound_ty == "FACE_OUTER_BOUND" {
            if !marks.outer_bounds.insert(bound_id) {
                return Err(refuse("FACE_OUTER_BOUND is linked more than once"));
            }
            if outer.replace(vertices).is_some() {
                return Err(refuse("ADVANCED_FACE has multiple outer bounds"));
            }
        } else {
            if !marks.hole_bounds.insert(bound_id) {
                return Err(refuse("FACE_BOUND is linked more than once"));
            }
            holes.push(vertices);
        }
    }
    Ok(LinkedFace {
        surface_id,
        outer_vertex_ids: outer.ok_or_else(|| refuse("ADVANCED_FACE missing FACE_OUTER_BOUND"))?,
        hole_vertex_ids: holes,
    })
}

fn require_all_linked(
    entities: &BTreeMap<usize, (String, String)>,
    ty: &str,
    linked: &BTreeSet<usize>,
) -> Result<()> {
    let all: BTreeSet<_> = entities
        .iter()
        .filter_map(|(id, (entity_ty, _))| (entity_ty == ty).then_some(*id))
        .collect();
    let linked_of_type: BTreeSet<_> = linked
        .iter()
        .filter_map(|id| {
            entities
                .get(id)
                .and_then(|(entity_ty, _)| (entity_ty == ty).then_some(*id))
        })
        .collect();
    if all != linked_of_type {
        return Err(refuse("STEP graph contains orphan or unlinked entities"));
    }
    Ok(())
}

/// Strictly traverse the admitted STEP topology and reject every unlinked
/// topological/surface entity, broken reference, and wrong reference type.
pub(crate) fn validate_linked_step_graph(
    entities: &BTreeMap<usize, (String, String)>,
    root: StepGraphRoot,
) -> Result<LinkedStepGraph> {
    let mut marks = GraphMarks::default();
    let mut face_ids = Vec::new();
    let mut body_face_ranges = Vec::new();
    match root {
        StepGraphRoot::OpenShell => {
            let roots: Vec<_> = entities
                .iter()
                .filter_map(|(id, (ty, _))| (ty == "OPEN_SHELL").then_some(*id))
                .collect();
            if roots.len() != 1 {
                return Err(refuse("Expected exactly one OPEN_SHELL root"));
            }
            let root_id = roots[0];
            marks.shells.insert(root_id);
            let args = entity(entities, root_id, "OPEN_SHELL", "OPEN_SHELL root broken")?;
            let parts = split_top_args(args);
            if parts.len() != 2 {
                return Err(refuse("OPEN_SHELL argument graph incomplete"));
            }
            face_ids = refs(&parts[1]);
        }
        StepGraphRoot::Solid | StepGraphRoot::SolidMany => {
            let bodies: Vec<_> = entities
                .iter()
                .filter(|(_, (ty, _))| ty == "MANIFOLD_SOLID_BREP" || ty == "BREP_WITH_VOIDS")
                .collect();
            if (root == StepGraphRoot::Solid && bodies.len() != 1)
                || (root == StepGraphRoot::SolidMany && bodies.is_empty())
            {
                return Err(refuse("Expected exactly one solid BREP root"));
            }
            for (body_id, (body_ty, body_args)) in bodies {
                let start = face_ids.len();
                let parts = split_top_args(body_args);
                let shell_ids = if body_ty == "MANIFOLD_SOLID_BREP" {
                    if parts.len() != 2 {
                        return Err(refuse("MANIFOLD_SOLID_BREP argument graph incomplete"));
                    }
                    vec![one_ref(
                        &parts[1],
                        "Solid root missing CLOSED_SHELL reference",
                    )?]
                } else {
                    if parts.len() != 3 {
                        return Err(refuse("BREP_WITH_VOIDS argument graph incomplete"));
                    }
                    let mut ids = vec![one_ref(
                        &parts[1],
                        "BREP_WITH_VOIDS missing outer CLOSED_SHELL",
                    )?];
                    let voids = refs(&parts[2]);
                    if voids.len() != 1 {
                        return Err(refuse(
                            "STEP /2 admits exactly one cavity shell per BREP_WITH_VOIDS",
                        ));
                    }
                    ids.extend(voids);
                    ids
                };
                if shell_ids.is_empty() || *body_id == 0 {
                    return Err(refuse("Solid BREP has no CLOSED_SHELL"));
                }
                for shell_id in shell_ids {
                    if !marks.shells.insert(shell_id) {
                        return Err(refuse("CLOSED_SHELL is linked by multiple solid bodies"));
                    }
                    let args = entity(
                        entities,
                        shell_id,
                        "CLOSED_SHELL",
                        "Solid root shell reference has wrong type",
                    )?;
                    let shell_parts = split_top_args(args);
                    if shell_parts.len() != 2 {
                        return Err(refuse("CLOSED_SHELL argument graph incomplete"));
                    }
                    face_ids.extend(refs(&shell_parts[1]));
                }
                body_face_ranges.push(start..face_ids.len());
            }
        }
    }
    if face_ids.is_empty() {
        return Err(refuse("STEP root has no ADVANCED_FACE references"));
    }
    let mut faces = Vec::with_capacity(face_ids.len());
    for face_id in face_ids {
        faces.push(walk_face(entities, face_id, &mut marks)?);
    }

    require_all_linked(
        entities,
        if root == StepGraphRoot::OpenShell {
            "OPEN_SHELL"
        } else {
            "CLOSED_SHELL"
        },
        &marks.shells,
    )?;
    for (ty, linked) in [
        ("ADVANCED_FACE", &marks.faces),
        ("FACE_OUTER_BOUND", &marks.outer_bounds),
        ("FACE_BOUND", &marks.hole_bounds),
        ("EDGE_LOOP", &marks.loops),
        ("ORIENTED_EDGE", &marks.oriented_edges),
        ("EDGE_CURVE", &marks.edge_curves),
        ("VERTEX_POINT", &marks.vertices),
        ("B_SPLINE_SURFACE_WITH_KNOTS", &marks.surfaces),
    ] {
        require_all_linked(entities, ty, linked)?;
    }
    for ty in ["LINE", "SURFACE_CURVE", "PCURVE", "VECTOR", "DIRECTION"] {
        require_all_linked(entities, ty, &marks.geometry)?;
    }
    Ok(LinkedStepGraph {
        faces,
        body_face_ranges,
    })
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

pub(crate) fn refuse_mesh_payloads_common(text: &str) -> Result<()> {
    if text.len() > MAX_STEP_PAYLOAD_BYTES {
        return Err(refuse("STEP payload exceeds 8 MiB limit"));
    }
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
