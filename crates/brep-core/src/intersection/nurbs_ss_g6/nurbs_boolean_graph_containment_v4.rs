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
    let frame = canonical_graph_frame(graph)?;
    if !is_single_affine_body(cutter) {
        return Err(refuse(
            "V4 containment cutter must be one affine-planar body",
        ));
    }
    let [uv_margin, floor_clearance, roof_clearance] = graph_frame_clearances(frame, cutter)
        .map_err(|fault| match fault {
            ClearanceFault::SingularFrame => refuse("V4 containment graph frame is singular"),
            ClearanceFault::SingularProjection => {
                refuse("V4 containment graph projection is singular")
            }
        })?;
    let clear = graph
        .tolerance_context()
        .map_err(|_| refuse("V4 containment tolerance context is invalid"))?
        .spatial_bounds()
        .clear_mm;
    if uv_margin <= clear || floor_clearance <= clear || roof_clearance <= clear {
        return Err(refuse(
            "V4 strict containment, floor, or convex-hull roof separation proof failed",
        ));
    }
    let result = contained_operation_result(graph, cutter, operation)?;
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
        cavity_proof: operation != "difference"
            || result
                .bodies
                .first()
                .is_some_and(|body| body.inner_shells.len() == 1),
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

/// Result model of a strict containment with V4 change-set provenance.
fn contained_operation_result(graph: &Model, cutter: &Model, operation: &str) -> Result<Model> {
    let tolerance = graph.tolerance_mm.max(cutter.tolerance_mm);
    let mut result = match operation {
        "union" => graph.clone(),
        "intersection" => cutter.clone(),
        "difference" => crate::imprint_pipeline::cavity(graph, cutter, tolerance)?,
        _ => {
            return Err(refuse(
                "V4 containment admits union, intersection, or difference",
            ));
        }
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
    Ok(result)
}
