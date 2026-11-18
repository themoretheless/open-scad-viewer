use super::*;
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
