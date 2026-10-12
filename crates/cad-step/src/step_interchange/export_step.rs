use super::*;

fn axial_height_from_planes(
    origin: [f64; 3],
    axis: [f64; 3],
    plane_origins: &[[f64; 3]],
) -> Option<f64> {
    if plane_origins.len() < 2 {
        return None;
    }
    let mut zs: Vec<f64> = plane_origins
        .iter()
        .map(|po| {
            (po[0] - origin[0]) * axis[0]
                + (po[1] - origin[1]) * axis[1]
                + (po[2] - origin[2]) * axis[2]
        })
        .collect();
    zs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let h = (zs[zs.len() - 1] - zs[0]).abs();
    if h.is_finite() && h > 1e-9 {
        Some(h)
    } else {
        None
    }
}

fn axial_height_from_points(
    entities: &BTreeMap<usize, (String, String)>,
    origin: [f64; 3],
    axis: [f64; 3],
) -> Option<f64> {
    let mut zs = Vec::new();
    for (ty, args) in entities.values() {
        if ty != "CARTESIAN_POINT" {
            continue;
        }
        if let Some(coords) = args.rsplit_once('(').map(|(_, c)| c.trim_end_matches(')'))
            && let Some(p) = parse_vec3(coords)
        {
            zs.push(
                (p[0] - origin[0]) * axis[0]
                    + (p[1] - origin[1]) * axis[1]
                    + (p[2] - origin[2]) * axis[2],
            );
        }
    }
    if zs.len() < 2 {
        return None;
    }
    zs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let h = (zs[zs.len() - 1] - zs[0]).abs();
    if h.is_finite() && h > 1e-9 {
        Some(h)
    } else {
        None
    }
}

pub(super) fn frame_from_axis_ref(axis: [f64; 3], ref_dir: [f64; 3]) -> [[f64; 3]; 2] {
    let y = [
        axis[1] * ref_dir[2] - axis[2] * ref_dir[1],
        axis[2] * ref_dir[0] - axis[0] * ref_dir[2],
        axis[0] * ref_dir[1] - axis[1] * ref_dir[0],
    ];
    [ref_dir, y]
}

fn kind_from_surfaces(entities: &BTreeMap<usize, (String, String)>) -> Option<AnalyticKind> {
    let mut spherical = None;
    let mut toroidal = None;
    let mut cylindrical = Vec::new();
    let mut conical = None;
    let mut planes = 0usize;
    for (ty, args) in entities.values() {
        match ty.as_str() {
            "SPHERICAL_SURFACE" => {
                let parts: Vec<&str> = args.split(',').collect();
                if parts.len() >= 2 {
                    let axis_id: usize = parts[parts.len() - 2]
                        .trim()
                        .trim_start_matches('#')
                        .parse()
                        .ok()?;
                    let radius: f64 = parts[parts.len() - 1].trim().parse().ok()?;
                    let (o, _, _) = resolve_axis2(entities, axis_id)?;
                    spherical = Some(AnalyticKind::Sphere { center: o, radius });
                }
            }
            "TOROIDAL_SURFACE" => {
                let parts: Vec<&str> = args.split(',').collect();
                if parts.len() >= 3 {
                    let axis_id: usize = parts[parts.len() - 3]
                        .trim()
                        .trim_start_matches('#')
                        .parse()
                        .ok()?;
                    let major: f64 = parts[parts.len() - 2].trim().parse().ok()?;
                    let minor: f64 = parts[parts.len() - 1].trim().parse().ok()?;
                    let (o, a, r) = resolve_axis2(entities, axis_id)?;
                    toroidal = Some(AnalyticKind::Torus {
                        major,
                        minor,
                        center: o,
                        axis: a,
                        frame: frame_from_axis_ref(a, r),
                    });
                }
            }
            "CYLINDRICAL_SURFACE" => {
                let parts: Vec<&str> = args.split(',').collect();
                if parts.len() >= 2 {
                    let axis_id: usize = parts[parts.len() - 2]
                        .trim()
                        .trim_start_matches('#')
                        .parse()
                        .ok()?;
                    let radius: f64 = parts[parts.len() - 1].trim().parse().ok()?;
                    if let Some((o, a, r)) = resolve_axis2(entities, axis_id) {
                        cylindrical.push((radius, o, a, r));
                    }
                }
            }
            "CONICAL_SURFACE" => {
                let parts: Vec<&str> = args.split(',').collect();
                if parts.len() >= 3 {
                    let axis_id: usize = parts[parts.len() - 3]
                        .trim()
                        .trim_start_matches('#')
                        .parse()
                        .ok()?;
                    let r_bottom: f64 = parts[parts.len() - 2].trim().parse().ok()?;
                    let semi: f64 = parts[parts.len() - 1].trim().parse().ok()?;
                    let (o, a, r) = resolve_axis2(entities, axis_id)?;
                    conical = Some((r_bottom, o, a, r, semi));
                }
            }
            "PLANE" => planes += 1,
            _ => {}
        }
    }
    let plane_origins = plane_axis_origins(entities);

    if let Some(s) = spherical {
        return Some(s);
    }
    if let Some(t) = toroidal {
        return Some(t);
    }
    if let Some((rb, o, a, r, semi)) = conical {
        let height = if plane_origins.len() >= 2 {
            axial_height_from_planes(o, a, &plane_origins)?
        } else {
            // Apex / single-cap only: one radius must be zero after recovery.
            let height = axial_height_from_points(entities, o, a)?;
            let r_top = rb + height * semi.tan();
            if !(rb.abs() <= 1e-12 || r_top.abs() <= 1e-12) {
                return None;
            }
            height
        };
        // `tan` differs across libm implementations; a true apex must recover to exactly 0.
        let snap_apex = |r: f64| if r.abs() <= 1e-9 { 0. } else { r };
        let r_top = snap_apex(rb + height * semi.tan());
        return Some(AnalyticKind::Frustum {
            r_bottom: snap_apex(rb),
            r_top,
            height,
            origin: o,
            axis: a,
            frame: frame_from_axis_ref(a, r),
        });
    }
    if cylindrical.len() == 1 {
        let (radius, o, a, r) = cylindrical[0];
        let height = axial_height_from_planes(o, a, &plane_origins)?;
        return Some(AnalyticKind::Cylinder {
            radius,
            height,
            origin: o,
            axis: a,
            frame: frame_from_axis_ref(a, r),
        });
    }
    if cylindrical.len() == 2 {
        let mut pair = cylindrical.clone();
        pair.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap_or(Ordering::Equal));
        let (inner, outer) = (pair[0].0, pair[1].0);
        let (_, o, a, r) = pair[0];
        let height = axial_height_from_planes(o, a, &plane_origins)?;
        return Some(AnalyticKind::Tube {
            outer,
            inner,
            height,
            origin: o,
            axis: a,
            frame: frame_from_axis_ref(a, r),
        });
    }
    if planes >= 6
        && spherical.is_none()
        && toroidal.is_none()
        && cylindrical.is_empty()
        && conical.is_none()
    {
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        let mut n = 0usize;
        for (ty, args) in entities.values() {
            if ty == "CARTESIAN_POINT"
                && let Some(coords) = args.rsplit_once('(').map(|(_, c)| c.trim_end_matches(')'))
                && let Some(p) = parse_vec3(coords)
            {
                for i in 0..3 {
                    min[i] = min[i].min(p[i]);
                    max[i] = max[i].max(p[i]);
                }
                n += 1;
            }
        }
        if n >= 4 && min.iter().all(|v| v.is_finite()) && max.iter().all(|v| v.is_finite()) {
            return Some(AnalyticKind::Cuboid { min, max });
        }
    }
    None
}

fn refuse_mesh_payloads(text: &str) -> Result<()> {
    if text.len() > 8 * 1024 * 1024 {
        return Err(refuse("STEP payload exceeds 8 MiB resource limit"));
    }
    if !text.contains("ISO-10303-21") {
        return Err(refuse("Not an ISO-10303-21 STEP exchange"));
    }
    if text.contains("FACETED_BREP") && !text.contains("ADVANCED_FACE") {
        return Err(refuse("Faceted STEP is not analytic B-rep interchange"));
    }
    let upper = text.to_ascii_uppercase();
    if upper.contains("SOLID ASCII")
        || upper.contains("ENDSOLID")
        || upper
            .lines()
            .any(|l| l.trim_start().starts_with("FACET NORMAL"))
    {
        return Err(refuse("STL mesh payload refused as analytic STEP"));
    }
    if (upper.contains("MTLLIB") || upper.lines().any(|l| l.trim_start().starts_with("F ")))
        && !text.contains("ISO-10303-21")
    {
        return Err(refuse("OBJ mesh payload refused as analytic STEP"));
    }
    if !(text.contains("ADVANCED_FACE") || text.contains("MANIFOLD_SOLID_BREP")) {
        return Err(refuse("STEP missing ADVANCED_FACE / MANIFOLD_SOLID_BREP"));
    }
    if !text.contains("MANIFOLD_SOLID_BREP") {
        return Err(refuse("STEP missing MANIFOLD_SOLID_BREP"));
    }
    Ok(())
}

fn entity_refs(args: &str) -> Vec<usize> {
    let bytes = args.as_bytes();
    let mut refs = Vec::new();
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
        if start < i
            && let Ok(id) = args[start..i].parse()
        {
            refs.push(id);
        }
    }
    refs
}

fn require_entity_type(
    entities: &BTreeMap<usize, (String, String)>,
    id: usize,
    allowed: &[&str],
    owner: &str,
) -> Result<()> {
    let Some((ty, _)) = entities.get(&id) else {
        return Err(refuse(&format!("{owner} reference #{id} is broken")));
    };
    if !allowed.contains(&ty.as_str()) {
        return Err(refuse(&format!(
            "{owner} reference #{id} has type {ty}, expected {}",
            allowed.join("|")
        )));
    }
    Ok(())
}

/// Validate the exact topology chain reachable from MANIFOLD_SOLID_BREP and
/// return its dependency closure. Orphan entities are deliberately excluded
/// from surface recognition and the curved CIRCLE gate.
fn validate_solid_graph(entities: &BTreeMap<usize, (String, String)>) -> Result<BTreeSet<usize>> {
    let solids: Vec<usize> = entities
        .iter()
        .filter_map(|(id, (ty, _))| (ty == "MANIFOLD_SOLID_BREP").then_some(*id))
        .collect();
    if solids.len() != 1 {
        return Err(refuse(
            "Analytic STEP requires exactly one MANIFOLD_SOLID_BREP",
        ));
    }
    let mut closure = BTreeSet::new();
    let solid = solids[0];
    closure.insert(solid);
    let solid_refs = entity_refs(&entities[&solid].1);
    if solid_refs.len() != 1 {
        return Err(refuse(
            "MANIFOLD_SOLID_BREP must reference exactly one CLOSED_SHELL",
        ));
    }
    let shell = solid_refs[0];
    require_entity_type(entities, shell, &["CLOSED_SHELL"], "MANIFOLD_SOLID_BREP")?;
    closure.insert(shell);
    let faces = entity_refs(&entities[&shell].1);
    if faces.is_empty() {
        return Err(refuse("CLOSED_SHELL must reference ADVANCED_FACE entities"));
    }
    for face in faces {
        require_entity_type(entities, face, &["ADVANCED_FACE"], "CLOSED_SHELL")?;
        closure.insert(face);
        let refs = entity_refs(&entities[&face].1);
        if refs.len() < 2 {
            return Err(refuse(
                "ADVANCED_FACE requires at least one bound and one surface",
            ));
        }
        let surface = *refs.last().unwrap();
        require_entity_type(
            entities,
            surface,
            &[
                "PLANE",
                "CYLINDRICAL_SURFACE",
                "CONICAL_SURFACE",
                "SPHERICAL_SURFACE",
                "TOROIDAL_SURFACE",
            ],
            "ADVANCED_FACE",
        )?;
        closure.insert(surface);
        for bound in &refs[..refs.len() - 1] {
            require_entity_type(
                entities,
                *bound,
                &["FACE_OUTER_BOUND", "FACE_BOUND"],
                "ADVANCED_FACE",
            )?;
            closure.insert(*bound);
            let loops = entity_refs(&entities[bound].1);
            if loops.len() != 1 {
                return Err(refuse("FACE bound must reference exactly one EDGE_LOOP"));
            }
            let loop_id = loops[0];
            require_entity_type(entities, loop_id, &["EDGE_LOOP"], "FACE bound")?;
            closure.insert(loop_id);
            let oriented_edges = entity_refs(&entities[&loop_id].1);
            if oriented_edges.is_empty() {
                return Err(refuse("EDGE_LOOP must reference ORIENTED_EDGE entities"));
            }
            for oriented in oriented_edges {
                require_entity_type(entities, oriented, &["ORIENTED_EDGE"], "EDGE_LOOP")?;
                closure.insert(oriented);
                let edge_refs = entity_refs(&entities[&oriented].1);
                if edge_refs.len() != 1 {
                    return Err(refuse(
                        "ORIENTED_EDGE must reference exactly one EDGE_CURVE",
                    ));
                }
                let edge = edge_refs[0];
                require_entity_type(entities, edge, &["EDGE_CURVE"], "ORIENTED_EDGE")?;
                closure.insert(edge);
                let geometry_refs = entity_refs(&entities[&edge].1);
                if geometry_refs.len() != 3 {
                    return Err(refuse(
                        "EDGE_CURVE must reference two vertices and one curve",
                    ));
                }
                for vertex in &geometry_refs[..2] {
                    require_entity_type(entities, *vertex, &["VERTEX_POINT"], "EDGE_CURVE")?;
                    closure.insert(*vertex);
                    let point_refs = entity_refs(&entities[vertex].1);
                    if point_refs.len() != 1 {
                        return Err(refuse(
                            "VERTEX_POINT must reference exactly one CARTESIAN_POINT",
                        ));
                    }
                    require_entity_type(
                        entities,
                        point_refs[0],
                        &["CARTESIAN_POINT"],
                        "VERTEX_POINT",
                    )?;
                    closure.insert(point_refs[0]);
                }
                let curve = geometry_refs[2];
                require_entity_type(entities, curve, &["LINE", "CIRCLE"], "EDGE_CURVE")?;
                closure.insert(curve);
            }
        }
    }

    // Include geometric placement dependencies, but only from already-linked
    // topology/surface/curve entities.
    let mut frontier: Vec<usize> = closure.iter().copied().collect();
    while let Some(id) = frontier.pop() {
        for reference in entity_refs(&entities[&id].1) {
            if !entities.contains_key(&reference) {
                return Err(refuse(&format!(
                    "Linked STEP entity #{id} has broken dependency #{reference}"
                )));
            }
            if closure.insert(reference) {
                frontier.push(reference);
            }
        }
    }
    Ok(closure)
}

fn curved_requires_circle(kind: &AnalyticKind) -> bool {
    match kind {
        AnalyticKind::Cuboid { .. } => false,
        AnalyticKind::Frustum {
            r_bottom, r_top, ..
        } => r_bottom.abs() > 1e-12 || r_top.abs() > 1e-12,
        _ => true,
    }
}

/// Export constructor-corpus Model as linked AP214/AP242 Part 21.
pub fn export_step(model: &Model) -> Result<(String, FeatureCertificate)> {
    let kind = classify(model)?;
    export_kind(&kind)
}

fn mat_mul(a: [[f64; 4]; 4], b: [[f64; 4]; 4]) -> [[f64; 4]; 4] {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..4).map(|k| a[i][k] * b[k][j]).sum()))
}

fn rigid_axis_matrix(
    origin: [f64; 3],
    axis: [f64; 3],
    reference: [f64; 3],
) -> Result<[[f64; 4]; 4]> {
    let normalize = |v: [f64; 3]| -> Result<[f64; 3]> {
        let n = v[0].hypot(v[1]).hypot(v[2]);
        if !n.is_finite() || n <= 1e-12 {
            return Err(refuse("STEP rigid placement has a degenerate direction"));
        }
        Ok([v[0] / n, v[1] / n, v[2] / n])
    };
    let z = normalize(axis)?;
    let mut x = normalize(reference)?;
    let projection = x[0] * z[0] + x[1] * z[1] + x[2] * z[2];
    x = normalize([
        x[0] - projection * z[0],
        x[1] - projection * z[1],
        x[2] - projection * z[2],
    ])?;
    let y = [
        z[1] * x[2] - z[2] * x[1],
        z[2] * x[0] - z[0] * x[2],
        z[0] * x[1] - z[1] * x[0],
    ];
    Ok([
        [x[0], y[0], z[0], origin[0]],
        [x[1], y[1], z[1], origin[1]],
        [x[2], y[2], z[2], origin[2]],
        [0., 0., 0., 1.],
    ])
}

fn rigid_inverse(m: [[f64; 4]; 4]) -> [[f64; 4]; 4] {
    let mut out = [[0.; 4]; 4];
    for i in 0..3 {
        for j in 0..3 {
            out[i][j] = m[j][i];
        }
        out[i][3] = -(0..3).map(|j| out[i][j] * m[j][3]).sum::<f64>();
    }
    out[3][3] = 1.;
    out
}

/// Admits a finite nested chain of standard ITEM_DEFINED_TRANSFORMATION rows.
/// Each row maps its first AXIS2 frame into its second frame.
pub(crate) fn v2_nested_placement(text: &str) -> Result<Option<[[f64; 4]; 4]>> {
    let entities = parse_entities(text);
    let mut total = [
        [1., 0., 0., 0.],
        [0., 1., 0., 0.],
        [0., 0., 1., 0.],
        [0., 0., 0., 1.],
    ];
    let mut count = 0usize;
    for (ty, args) in entities.values() {
        if ty != "ITEM_DEFINED_TRANSFORMATION" {
            continue;
        }
        let refs = entity_refs(args);
        if refs.len() != 2 {
            return Err(refuse(
                "STEP /2 ITEM_DEFINED_TRANSFORMATION requires two AXIS2 frames",
            ));
        }
        let (ao, az, ax) = resolve_axis2(&entities, refs[0])
            .ok_or_else(|| refuse("STEP /2 transformation source frame is invalid"))?;
        let (bo, bz, bx) = resolve_axis2(&entities, refs[1])
            .ok_or_else(|| refuse("STEP /2 transformation target frame is invalid"))?;
        let from = rigid_axis_matrix(ao, az, ax)?;
        let to = rigid_axis_matrix(bo, bz, bx)?;
        total = mat_mul(mat_mul(to, rigid_inverse(from)), total);
        count += 1;
        if count > 8 {
            return Err(refuse("STEP /2 nested placement depth exceeds eight"));
        }
    }
    Ok((count != 0).then_some(total))
}

/// Successor export. `/1` remains byte/API compatible.
pub fn export_step_v2(model: &Model) -> Result<(String, FeatureCertificate, StepIdentityReport)> {
    let (text, _) = export_step(model)?;
    let text = attach_v2_identity(&add_v2_context(&text), model)?;
    let count = topology_count(model);
    Ok((
        text,
        FeatureCertificate {
            capability: STEP_INTERCHANGE_V2_CAPABILITY,
            complete: true,
            notes: vec![
                "si_unit_context",
                "topology_id_metadata",
                "identity_preserved",
                "strict_finite_subset",
            ],
        },
        StepIdentityReport {
            preserved: true,
            source: "internal-metadata",
            preserved_count: count,
            created_count: 0,
            lost_count: 0,
        },
    ))
}

/// Import analytic STEP from surfaces + AXIS2 only → constructors.
pub fn import_step(text: &str) -> Result<(Model, FeatureCertificate)> {
    refuse_mesh_payloads(text)?;
    let entities = parse_entities(text);
    let closure = validate_solid_graph(&entities)?;
    let linked_entities: BTreeMap<usize, (String, String)> = entities
        .iter()
        .filter(|(id, _)| closure.contains(id))
        .map(|(id, entity)| (*id, entity.clone()))
        .collect();
    let kind = kind_from_surfaces(&linked_entities).ok_or_else(|| {
        refuse(
            "Incomplete analytic STEP graph; no constructor solid recognized (graph-only; no OSCAD_SOLID; AABB removed)",
        )
    })?;
    let linked_circle = linked_entities.values().any(|(ty, _)| ty == "CIRCLE");
    if curved_requires_circle(&kind) && !linked_circle {
        return Err(refuse(
            "Curved analytic STEP requires CIRCLE ring edges (graph honesty)",
        ));
    }
    let model = rebuild(&kind)?;
    model.validate()?;
    Ok((
        model,
        FeatureCertificate {
            capability: "step-interchange/1",
            complete: true,
            notes: vec![
                "step_topology_roundtrip_constructors",
                "step_ap214_ap242_entity_graph",
                "step_graph_only_import",
                "step_circle_import_gate",
                "aabb_import_removed",
                "oscad_solid_removed",
            ],
        },
    ))
}
