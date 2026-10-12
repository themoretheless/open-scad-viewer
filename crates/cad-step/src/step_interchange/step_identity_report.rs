use super::*;

pub(super) fn refuse(message: &str) -> Error {
    Error::new("BREP_STEP_REFUSED", message)
}

pub const STEP_INTERCHANGE_V2_CAPABILITY: &str = "step-interchange/2";

#[derive(Clone, Debug)]
pub struct StepIdentityReport {
    pub preserved: bool,
    pub source: &'static str,
    pub preserved_count: usize,
    pub created_count: usize,
    pub lost_count: usize,
}

pub(super) fn topology_count(model: &Model) -> usize {
    model.vertices.len()
        + model.edges.len()
        + model.loops.len()
        + model.faces.len()
        + model.shells.len()
        + model.bodies.len()
}

fn metadata_rows(model: &Model) -> [(&'static str, TopoKind, &[TopoId]); 6] {
    [
        ("VERTEX_POINT", TopoKind::Vertex, &model.1.vertices),
        ("EDGE_CURVE", TopoKind::Edge, &model.1.edges),
        ("EDGE_LOOP", TopoKind::Loop, &model.1.loops),
        ("ADVANCED_FACE", TopoKind::Face, &model.1.faces),
        ("CLOSED_SHELL", TopoKind::Shell, &model.1.shells),
        ("SOLID_BREP", TopoKind::Body, &model.1.bodies),
    ]
}

/// Internal AP242 name metadata. The vector index and opaque 128-bit TopoId,
/// never the Part 21 instance number, are authoritative.
pub(crate) fn attach_v2_identity(text: &str, model: &Model) -> Result<String> {
    let mut next = [0usize; 6];
    let mut out = Vec::new();
    for line in text.lines() {
        let mut rewritten = line.to_string();
        for (slot, (entity_ty, kind, ids)) in metadata_rows(model).iter().enumerate() {
            let is_body = *entity_ty == "SOLID_BREP"
                && (line.contains("=MANIFOLD_SOLID_BREP(") || line.contains("=BREP_WITH_VOIDS("));
            if (!is_body && !line.contains(&format!("={entity_ty}("))) || next[slot] >= ids.len() {
                continue;
            }
            let marker = format!(
                "OSCAD_TOPO/2|{}|{}|{}",
                kind.as_str(),
                next[slot],
                ids[next[slot]]
            );
            let open = line
                .find('(')
                .ok_or_else(|| refuse("STEP topology entity has no argument list"))?;
            let rest = &line[open + 1..];
            let quote_end = rest
                .find('\'')
                .and_then(|first| rest[first + 1..].find('\'').map(|last| first + 1 + last));
            let Some(quote_end) = quote_end else {
                return Err(refuse("STEP topology entity has no name field"));
            };
            let first_quote = open + 1 + rest.find('\'').unwrap();
            let last_quote = open + 1 + quote_end;
            rewritten = format!(
                "{}'{}'{}",
                &line[..first_quote],
                marker,
                &line[last_quote + 1..]
            );
            next[slot] += 1;
            break;
        }
        out.push(rewritten);
    }
    for (slot, (_, _, ids)) in metadata_rows(model).iter().enumerate() {
        if next[slot] != ids.len() {
            return Err(refuse(
                "STEP topology graph does not bijectively cover model identities",
            ));
        }
    }
    Ok(out.join("\n") + "\n")
}

fn metadata_name(args: &str) -> Option<&str> {
    let start = args.find('\'')? + 1;
    let end = args[start..].find('\'')? + start;
    Some(&args[start..end])
}

pub(crate) fn restore_v2_identity(text: &str, model: &mut Model) -> Result<StepIdentityReport> {
    let entities = parse_entities(text);
    let mut found = BTreeMap::<(TopoKind, usize), TopoId>::new();
    let mut seen_ids = BTreeSet::new();
    for (ty, args) in entities.values() {
        let Some(name) = metadata_name(args) else {
            continue;
        };
        let Some(payload) = name.strip_prefix("OSCAD_TOPO/2|") else {
            continue;
        };
        let mut parts = payload.split('|');
        let kind = TopoKind::parse(parts.next().unwrap_or(""))
            .ok_or_else(|| refuse("STEP identity metadata has unknown topology kind"))?;
        let index = parts
            .next()
            .ok_or_else(|| refuse("STEP identity metadata is missing index"))?
            .parse::<usize>()
            .map_err(|_| refuse("STEP identity metadata index is invalid"))?;
        let id = TopoId::parse(
            parts
                .next()
                .ok_or_else(|| refuse("STEP identity metadata is missing TopoId"))?,
        )
        .map_err(refuse)?;
        if parts.next().is_some() || id.kind() != kind {
            return Err(refuse("STEP identity metadata kind mismatch"));
        }
        let expected_ty = match kind {
            TopoKind::Vertex => ty == "VERTEX_POINT",
            TopoKind::Edge => ty == "EDGE_CURVE",
            TopoKind::Loop => ty == "EDGE_LOOP",
            TopoKind::Face => ty == "ADVANCED_FACE",
            TopoKind::Shell => ty == "CLOSED_SHELL",
            TopoKind::Body => ty == "MANIFOLD_SOLID_BREP" || ty == "BREP_WITH_VOIDS",
            TopoKind::ControlPoint => false,
        };
        if !expected_ty || found.insert((kind, index), id).is_some() || !seen_ids.insert(id) {
            return Err(refuse("Duplicate or misplaced STEP identity metadata"));
        }
    }
    if found.is_empty() {
        let count = topology_count(model);
        return Ok(StepIdentityReport {
            preserved: false,
            source: "external-step",
            preserved_count: 0,
            created_count: count,
            lost_count: count,
        });
    }
    let expected = topology_count(model);
    if found.len() != expected {
        return Err(refuse("Partial STEP identity metadata is not admitted"));
    }
    let mut remap = BTreeMap::<TopoId, TopoId>::new();
    let assign = |kind: TopoKind,
                  target: &mut [TopoId],
                  remap: &mut BTreeMap<TopoId, TopoId>|
     -> Result<()> {
        for (index, value) in target.iter_mut().enumerate() {
            let replacement = *found
                .get(&(kind, index))
                .ok_or_else(|| refuse("STEP identity metadata is not topology-bijective"))?;
            remap.insert(*value, replacement);
            *value = replacement;
        }
        Ok(())
    };
    assign(TopoKind::Vertex, &mut model.1.vertices, &mut remap)?;
    assign(TopoKind::Edge, &mut model.1.edges, &mut remap)?;
    assign(TopoKind::Loop, &mut model.1.loops, &mut remap)?;
    assign(TopoKind::Face, &mut model.1.faces, &mut remap)?;
    assign(TopoKind::Shell, &mut model.1.shells, &mut remap)?;
    assign(TopoKind::Body, &mut model.1.bodies, &mut remap)?;
    model.1.change_set.nodes = model
        .1
        .change_set
        .nodes
        .iter()
        .map(|(id, kind)| (remap.get(id).copied().unwrap_or(*id), *kind))
        .collect();
    for change in &mut model.1.change_set.changes {
        for id in change.parents.iter_mut().chain(&mut change.children) {
            if let Some(replacement) = remap.get(id) {
                *id = *replacement;
            }
        }
    }
    for relation in &mut model.1.lineage {
        for id in relation.parents.iter_mut().chain(&mut relation.children) {
            if let Some(replacement) = remap.get(id) {
                *id = *replacement;
            }
        }
    }
    for (kind, ids) in [
        (TopoKind::Vertex, model.1.vertices.as_slice()),
        (TopoKind::Edge, model.1.edges.as_slice()),
        (TopoKind::Loop, model.1.loops.as_slice()),
        (TopoKind::Face, model.1.faces.as_slice()),
        (TopoKind::Shell, model.1.shells.as_slice()),
        (TopoKind::Body, model.1.bodies.as_slice()),
    ] {
        model
            .1
            .change_set
            .nodes
            .extend(ids.iter().copied().map(|id| (id, kind)));
    }
    model
        .1
        .change_set
        .validate()
        .map_err(|_| refuse("STEP identity metadata produces invalid lineage"))?;
    Ok(StepIdentityReport {
        preserved: true,
        source: "internal-metadata",
        preserved_count: expected,
        created_count: 0,
        lost_count: 0,
    })
}

pub(crate) fn add_v2_context(text: &str) -> String {
    let addition = "#900000000=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));\n\
#900000001=(NAMED_UNIT(*)PLANE_ANGLE_UNIT()SI_UNIT($,.RADIAN.));\n\
#900000002=(NAMED_UNIT(*)SI_UNIT($,.STERADIAN.)SOLID_ANGLE_UNIT());\n\
#900000003=(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNIT_ASSIGNED_CONTEXT((#900000000,#900000001,#900000002))REPRESENTATION_CONTEXT('','3D'));\n";
    text.replacen(
        "ENDSEC;\nEND-ISO-10303-21;",
        &format!("{addition}ENDSEC;\nEND-ISO-10303-21;"),
        1,
    )
}

pub(crate) fn v2_length_scale(text: &str) -> Result<f64> {
    let upper = text.to_ascii_uppercase().replace(' ', "");
    let mut scales: Vec<f64> = Vec::new();
    let mut rest = upper.as_str();
    while let Some(at) = rest.find("SI_UNIT(") {
        rest = &rest[at + "SI_UNIT(".len()..];
        let Some(end) = rest.find(')') else { break };
        let args = &rest[..end];
        if args.ends_with(",.METRE.") {
            let prefix = args.split(',').next().unwrap_or("");
            scales.push(match prefix {
                ".MILLI." => 1.,
                ".CENTI." => 10.,
                "$" => 1000.,
                _ => {
                    return Err(refuse(
                        "STEP /2 length SI prefix is outside the finite subset",
                    ));
                }
            });
        }
        rest = &rest[end + 1..];
    }
    if scales.is_empty() || !upper.contains("GLOBAL_UNIT_ASSIGNED_CONTEXT") {
        return Err(refuse(
            "STEP /2 requires an SI length unit and global unit context",
        ));
    }
    if scales
        .iter()
        .any(|scale| (*scale - scales[0]).abs() > f64::EPSILON)
    {
        return Err(refuse("STEP /2 contains conflicting length units"));
    }
    Ok(scales[0])
}

pub(crate) fn validate_v2_finite_subset(text: &str) -> Result<()> {
    let mut instance_ids = BTreeSet::new();
    let flat = text.replace(['\n', '\r'], " ");
    for chunk in flat
        .split(';')
        .map(str::trim)
        .filter(|row| row.starts_with('#'))
    {
        let (id, _) = chunk
            .split_once('=')
            .ok_or_else(|| refuse("STEP /2 entity row is malformed"))?;
        let id = id
            .trim_start_matches('#')
            .parse::<usize>()
            .map_err(|_| refuse("STEP /2 entity number is malformed"))?;
        if !instance_ids.insert(id) {
            return Err(refuse(
                "STEP /2 contains a duplicate Part 21 instance number",
            ));
        }
    }
    let entities = parse_entities(text);
    let allowed = [
        "APPLICATION_CONTEXT",
        "ADVANCED_FACE",
        "AXIS2_PLACEMENT_3D",
        "B_SPLINE_SURFACE_WITH_KNOTS",
        "BREP_WITH_VOIDS",
        "CARTESIAN_POINT",
        "CIRCLE",
        "CLOSED_SHELL",
        "COMPLEX",
        "CONICAL_SURFACE",
        "CYLINDRICAL_SURFACE",
        "DIRECTION",
        "EDGE_CURVE",
        "EDGE_LOOP",
        "FACE_BOUND",
        "FACE_OUTER_BOUND",
        "ITEM_DEFINED_TRANSFORMATION",
        "LINE",
        "MANIFOLD_SOLID_BREP",
        "OPEN_SHELL",
        "ORIENTED_EDGE",
        "PCURVE",
        "PLANE",
        "SHELL_BASED_SURFACE_MODEL",
        "SPHERICAL_SURFACE",
        "SURFACE_CURVE",
        "TOROIDAL_SURFACE",
        "VECTOR",
        "VERTEX_POINT",
    ];
    for (ty, args) in entities.values() {
        if !allowed.contains(&ty.as_str()) {
            return Err(refuse(&format!(
                "STEP /2 entity type {ty} is outside the finite subset"
            )));
        }
        if ty == "COMPLEX"
            && !(args.contains("SI_UNIT") || args.contains("GEOMETRIC_REPRESENTATION_CONTEXT"))
        {
            return Err(refuse(
                "STEP /2 complex entity is outside the unit/context subset",
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub(super) enum AnalyticKind {
    Cuboid {
        min: [f64; 3],
        max: [f64; 3],
    },
    Cylinder {
        radius: f64,
        height: f64,
        origin: [f64; 3],
        axis: [f64; 3],
        frame: [[f64; 3]; 2],
    },
    Frustum {
        r_bottom: f64,
        r_top: f64,
        height: f64,
        origin: [f64; 3],
        axis: [f64; 3],
        frame: [[f64; 3]; 2],
    },
    Sphere {
        center: [f64; 3],
        radius: f64,
    },
    Torus {
        major: f64,
        minor: f64,
        center: [f64; 3],
        axis: [f64; 3],
        frame: [[f64; 3]; 2],
    },
    Tube {
        outer: f64,
        inner: f64,
        height: f64,
        origin: [f64; 3],
        axis: [f64; 3],
        frame: [[f64; 3]; 2],
    },
}

fn is_axis_aligned_cuboid(model: &Model) -> bool {
    model.validate().is_ok()
        && model.faces.len() == 6
        && model.vertices.len() == 8
        && model
            .faces
            .iter()
            .all(|f| f.surface.degree_u == 1 && f.surface.degree_v == 1 && f.holes.is_empty())
        && model.edges.iter().all(|e| e.curve.degree == 1)
}

pub(super) fn model_bounds(model: &Model) -> ([f64; 3], [f64; 3]) {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for v in &model.vertices {
        for i in 0..3 {
            min[i] = min[i].min(v.point[i]);
            max[i] = max[i].max(v.point[i]);
        }
    }
    (min, max)
}

fn recognize_tube(model: &Model) -> Option<AnalyticKind> {
    if model.validate().is_err()
        || model.bodies.len() != 1
        || model.shells.len() != 1
        || model.faces.len() != 10
        || model.vertices.len() != 16
    {
        return None;
    }
    let caps: Vec<_> = model
        .faces
        .iter()
        .filter(|f| f.surface.degree_u == 1 && f.surface.degree_v == 1 && !f.holes.is_empty())
        .collect();
    let sides = model
        .faces
        .iter()
        .filter(|f| f.surface.degree_u == 2 && f.surface.degree_v == 1)
        .count();
    if caps.len() != 2 || sides != 8 {
        return None;
    }
    let plane_normal = |face: &brep_core::Face| -> Option<[f64; 3]> {
        let cps = &face.surface.control_points;
        if cps.len() < 2 || cps[0].len() < 2 || cps[0][0].len() != 3 {
            return None;
        }
        let a = &cps[0][0];
        let b = &cps[1][0];
        let c = &cps[0][1];
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let len = n[0].hypot(n[1]).hypot(n[2]);
        if !(len.is_finite() && len > 1e-12) {
            return None;
        }
        Some([n[0] / len, n[1] / len, n[2] / len])
    };
    let mut axis = plane_normal(caps[0])?;
    if let Some(n1) = plane_normal(caps[1])
        && axis[0] * n1[0] + axis[1] * n1[1] + axis[2] * n1[2] < 0.
    {
        axis = [-axis[0], -axis[1], -axis[2]];
    }
    let mut centroid = [0.; 3];
    for v in &model.vertices {
        for k in 0..3 {
            centroid[k] += v.point[k] / model.vertices.len() as f64;
        }
    }
    let mut zs = Vec::new();
    let mut radii = Vec::new();
    let mut outer_pt = None;
    for v in &model.vertices {
        let d = [
            v.point[0] - centroid[0],
            v.point[1] - centroid[1],
            v.point[2] - centroid[2],
        ];
        let z = d[0] * axis[0] + d[1] * axis[1] + d[2] * axis[2];
        let radial = [d[0] - z * axis[0], d[1] - z * axis[1], d[2] - z * axis[2]];
        let r = radial[0].hypot(radial[1]).hypot(radial[2]);
        zs.push(z);
        radii.push(r);
        if outer_pt
            .map(|(_, or): ([f64; 3], f64)| r > or)
            .unwrap_or(true)
        {
            outer_pt = Some((radial, r));
        }
    }
    zs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    radii.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let z0 = zs[0];
    let z1 = zs[zs.len() - 1];
    let height = z1 - z0;
    if !(height.is_finite() && height > 1e-9) {
        return None;
    }
    let inner = radii[0];
    let outer = radii[radii.len() - 1];
    if !(outer - inner >= 1e-5 && inner > 0.) {
        return None;
    }
    let (radial, _) = outer_pt?;
    let rlen = radial[0].hypot(radial[1]).hypot(radial[2]);
    if !(rlen.is_finite() && rlen > 1e-9) {
        return None;
    }
    let ref_dir = [radial[0] / rlen, radial[1] / rlen, radial[2] / rlen];
    let frame = frame_from_axis_ref(axis, ref_dir);
    let origin = [
        centroid[0] + axis[0] * z0,
        centroid[1] + axis[1] * z0,
        centroid[2] + axis[2] * z0,
    ];
    Some(AnalyticKind::Tube {
        outer,
        inner,
        height,
        origin,
        axis,
        frame,
    })
}

pub(super) fn classify(model: &Model) -> Result<AnalyticKind> {
    model.validate()?;
    if let Some(s) = recognize_sphere(model)? {
        return Ok(AnalyticKind::Sphere {
            center: s.center,
            radius: s.radius,
        });
    }
    if let Some(t) = recognize_torus(model)? {
        return Ok(AnalyticKind::Torus {
            major: t.major,
            minor: t.minor,
            center: t.center,
            axis: t.axis,
            frame: t.frame,
        });
    }
    if let Some(c) = recognize_cone(model)? {
        return Ok(AnalyticKind::Frustum {
            r_bottom: c.r_bottom,
            r_top: c.r_top,
            height: c.height,
            origin: c.bottom,
            axis: c.axis,
            frame: c.frame,
        });
    }
    if let Some(c) = recognize_cylinder(model)? {
        let height = 2. * c.half_height;
        let origin = [
            c.center[0] - c.axis[0] * c.half_height,
            c.center[1] - c.axis[1] * c.half_height,
            c.center[2] - c.axis[2] * c.half_height,
        ];
        return Ok(AnalyticKind::Cylinder {
            radius: c.radius,
            height,
            origin,
            axis: c.axis,
            frame: c.frame,
        });
    }
    if let Some(t) = recognize_tube(model) {
        return Ok(t);
    }
    if is_axis_aligned_cuboid(model) {
        let (min, max) = model_bounds(model);
        return Ok(AnalyticKind::Cuboid { min, max });
    }
    Err(refuse(
        "STEP export admits constructor corpus only (cuboid/cylinder/frustum/sphere/torus/tube)",
    ))
}

pub(super) struct StepWriter {
    pub(super) next: usize,
    pub(super) lines: Vec<String>,
}
