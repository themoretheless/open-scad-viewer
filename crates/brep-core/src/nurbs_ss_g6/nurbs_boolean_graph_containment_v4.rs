use super::*;

/// Finite `/4` containment cell: an affine cutter lies strictly inside the
/// canonical graph volume and below the convex-hull lower roof bound.
pub fn nurbs_boolean_graph_containment_v4(
    graph: &Model,
    cutter: &Model,
    operation: &str,
) -> Result<(Model, ContainedGraphBooleanCertificate)> {
    graph.validate()?;
    cutter.validate()?;
    if !matches!(operation, "union" | "intersection" | "difference") {
        return Err(refuse(
            "V4 containment admits union, intersection, or difference",
        ));
    }
    let (_, top, origin, u, v, floor_offset) = canonical_graph_frame(graph)?;
    if cutter.bodies.len() != 1
        || cutter.shells.len() != 1
        || !cutter.bodies[0].inner_shells.is_empty()
        || cutter
            .faces
            .iter()
            .any(|face| planar_support(&face.surface, cutter.tolerance_mm.max(1e-9) * 8.).is_none())
    {
        return Err(refuse(
            "V4 containment cutter must be one affine-planar body",
        ));
    }
    let normal =
        normalize(cross(u, v)).ok_or_else(|| refuse("V4 containment graph frame is singular"))?;
    let normal = if dot(floor_offset, normal) < 0. {
        normal
    } else {
        normal.map(|value| -value)
    };
    let floor_height = dot(floor_offset, normal);
    let roof_lower_bound = top
        .control_points
        .iter()
        .flatten()
        .map(|point| dot(sub([point[0], point[1], point[2]], origin), normal))
        .fold(f64::INFINITY, f64::min);
    let uu = dot(u, u);
    let uv = dot(u, v);
    let vv = dot(v, v);
    let determinant = uu * vv - uv * uv;
    if determinant <= 1e-18 {
        return Err(refuse("V4 containment graph projection is singular"));
    }
    let clear = graph
        .tolerance_context()
        .map_err(|_| refuse("V4 containment tolerance context is invalid"))?
        .spatial_bounds()
        .clear_mm;
    let mut uv_margin = f64::INFINITY;
    let mut floor_clearance = f64::INFINITY;
    let mut roof_clearance = f64::INFINITY;
    for vertex in &cutter.vertices {
        let rhs = sub(vertex.point, origin);
        let ru = dot(rhs, u);
        let rv = dot(rhs, v);
        let su = (ru * vv - rv * uv) / determinant;
        let sv = (rv * uu - ru * uv) / determinant;
        let height = dot(rhs, normal);
        uv_margin = uv_margin.min(su).min(1. - su).min(sv).min(1. - sv);
        floor_clearance = floor_clearance.min(height - floor_height);
        roof_clearance = roof_clearance.min(roof_lower_bound - height);
    }
    if uv_margin <= clear || floor_clearance <= clear || roof_clearance <= clear {
        return Err(refuse(
            "V4 strict containment, floor, or convex-hull roof separation proof failed",
        ));
    }
    let tolerance = graph.tolerance_mm.max(cutter.tolerance_mm);
    let mut result = match operation {
        "union" => graph.clone(),
        "intersection" => cutter.clone(),
        "difference" => crate::imprint_pipeline::cavity(graph, cutter, tolerance)?,
        _ => unreachable!(),
    };
    result.inherit_topology_ids(&[graph, cutter]);
    let current_ids: std::collections::BTreeSet<_> = result
        .identity_groups()
        .into_iter()
        .flat_map(|(_, ids)| ids.iter().copied())
        .collect();
    for change in &mut result.1.change_set.changes {
        if change.children.iter().any(|id| current_ids.contains(id)) {
            change.provenance.operation = NURBS_BOOLEAN_CAPABILITY_V4.into();
            change.role = format!("v4-{operation}-{}", change.topo_kind.as_str());
        }
    }
    result.1.change_set.validate()?;
    let audited = LocallyValidatedModel::new(result)?.audit()?;
    let mut audit = audited.certificate().clone();
    audit.notes.push("v4_strict_containment_convex_bounds_ok");
    if operation == "difference" {
        audit.notes.push("v4_inverted_cavity_shell_ok");
    }
    let result = audited.into_model();
    let cert = ContainedGraphBooleanCertificate {
        capability: NURBS_BOOLEAN_CAPABILITY_V4,
        status: "Complete",
        operation: operation.into(),
        relation: "graph-strictly-contains-affine-cutter",
        strict_uv_margin: uv_margin,
        floor_clearance,
        roof_clearance,
        cavity_proof: operation != "difference" || result.bodies[0].inner_shells.len() == 1,
        separation_proof: true,
        audit,
        change_set: result.1.change_set.clone(),
        naming_complete: result.persistent_naming_complete(),
        no_fallback: true,
    };
    if !cert.permits_topology_change() {
        return Err(refuse("V4 containment aggregate certificate failed closed"));
    }
    Ok((result, cert))
}
