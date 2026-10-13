use super::*;

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

pub(crate) struct PcurveLineEdge {
    pub va: usize,
    pub vb: usize,
    pub point: usize,
    pub direction: [f64; 3],
    pub surface: usize,
    pub uv_a: [f64; 2],
    pub uv_b: [f64; 2],
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
    pub fn pcurve_line_edge(&mut self, edge: PcurveLineEdge) -> usize {
        let PcurveLineEdge {
            va,
            vb,
            point,
            direction,
            surface,
            uv_a,
            uv_b,
        } = edge;
        let d3 = self.direction(direction);
        let vec3 = self.emit(format!("VECTOR('',#{d3},1.)"));
        let line3 = self.emit(format!("LINE('',#{point},#{vec3})"));
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

pub(super) fn refs(token: &str) -> Vec<usize> {
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

pub(super) fn one_ref(token: &str, message: &str) -> Result<usize> {
    let found = refs(token);
    if found.len() != 1 {
        return Err(refuse(message));
    }
    Ok(found[0])
}

pub(super) fn entity<'a>(
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
pub(super) struct GraphMarks {
    pub(super) shells: BTreeSet<usize>,
    pub(super) faces: BTreeSet<usize>,
    pub(super) outer_bounds: BTreeSet<usize>,
    pub(super) hole_bounds: BTreeSet<usize>,
    pub(super) loops: BTreeSet<usize>,
    pub(super) oriented_edges: BTreeSet<usize>,
    pub(super) edge_curves: BTreeSet<usize>,
    pub(super) vertices: BTreeSet<usize>,
    pub(super) surfaces: BTreeSet<usize>,
    pub(super) active_geometry: BTreeSet<usize>,
    pub(super) geometry: BTreeSet<usize>,
}

pub(super) fn validate_curve_geometry(
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

pub(super) fn point_components(entities: &BTreeMap<usize, (String, String)>, id: usize) -> Result<Vec<f64>> {
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

pub(super) fn line_origin_delta(
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
