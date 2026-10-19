use super::*;

pub(super) fn exact_axis_aligned_valence3_corner_blend(
    model: &Model,
    edges: &[usize],
    radius: f64,
) -> Result<AuditedFeatureResult> {
    audit_solid(model).map_err(|_| {
        refuse(
            "BREP_VALENCE3_CORNER_BLEND_REFUSED",
            "Valence-3 corner blend source must pass the global solid audit",
        )
    })?;
    if !(radius.is_finite() && radius > 0.) {
        return Err(refuse(
            "BREP_VALENCE3_CORNER_BLEND_REFUSED",
            "Valence-3 radius must be finite and positive",
        ));
    }
    if edges.len() != 3
        || edges
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != 3
    {
        return Err(refuse(
            "BREP_VALENCE3_CORNER_BLEND_REFUSED",
            "exact-valence3-corner-blend/1 admits exactly three distinct concurrent cuboid edges",
        ));
    }
    if !is_axis_aligned_cuboid(model) {
        return Err(refuse(
            "BREP_VALENCE3_CORNER_BLEND_REFUSED",
            "exact-valence3-corner-blend/1 admits axis-aligned planar cuboids only",
        ));
    }
    let (min, max) = model_bounds(model);
    if (0..3).any(|i| max[i] - min[i] <= 2. * radius) {
        return Err(refuse(
            "BREP_VALENCE3_CORNER_BLEND_REFUSED",
            "Cuboid extents too small for the valence-3 radius",
        ));
    }
    let mut corner_vertex = None;
    for &edge in edges {
        if edge >= model.edges.len() {
            return Err(refuse(
                "BREP_VALENCE3_CORNER_BLEND_REFUSED",
                "Edge index out of range",
            ));
        }
        if model.edges[edge].curve.degree != 1 {
            return Err(refuse(
                "BREP_VALENCE3_CORNER_BLEND_REFUSED",
                "Curved edges are outside exact-valence3-corner-blend/1",
            ));
        }
    }
    // Three distinct edges must share one cuboid corner.
    let sets: Vec<[usize; 2]> = edges.iter().map(|&e| model.edges[e].vertices).collect();
    for &v in &sets[0] {
        if sets[1].contains(&v) && sets[2].contains(&v) {
            corner_vertex = Some(v);
            break;
        }
    }
    let Some(vid) = corner_vertex else {
        return Err(refuse(
            "BREP_VALENCE3_CORNER_BLEND_REFUSED",
            "Selected edges do not meet at a single valence-3 vertex",
        ));
    };
    let p = model.vertices[vid].point;
    // Reflect each minimum coordinate to the maximum corner. This is an
    // isometry: radius and contact angles remain unchanged. affine() also
    // reverses shell uses for odd reflections and preserves topology IDs.
    let tolerance = model.tolerance_mm.max(1e-9);
    let mut placement = [[0.; 4]; 4];
    placement[3][3] = 1.;
    for i in 0..3 {
        if (p[i] - max[i]).abs() <= tolerance {
            placement[i][i] = 1.;
        } else if (p[i] - min[i]).abs() <= tolerance {
            placement[i][i] = -1.;
            placement[i][3] = min[i] + max[i];
        } else {
            return Err(refuse(
                "BREP_VALENCE3_CORNER_BLEND_REFUSED",
                "Selected vertex must be a cuboid corner",
            ));
        }
    }
    let local_source = crate::transform::affine(model, placement)?;
    let local_result =
        crate::imprint_pipeline::valence3_cuboid_max_corner(&local_source, min, max, radius)
            .map_err(|error| {
                refuse(
                    "BREP_VALENCE3_CORNER_BLEND_REFUSED",
                    &format!("Valence-3 authorship refused: {}", error.message),
                )
            })?;
    // The reflection is its own inverse. Certify the final world-space solid.
    let result = crate::transform::affine(&local_result, placement)?;
    let spheres = result
        .faces
        .iter()
        .filter(|f| f.surface.degree_u == 2 && f.surface.degree_v == 2)
        .count();
    let cylinders = result
        .faces
        .iter()
        .filter(|f| f.surface.degree_u == 2 && f.surface.degree_v == 1)
        .count();
    if spheres != 1 || cylinders < 3 {
        return Err(refuse(
            "BREP_VALENCE3_CORNER_BLEND_REFUSED",
            "Valence-3 result missing sphere octant or three cylinder faces",
        ));
    }
    certify_blend_result(
        result,
        EXACT_VALENCE3_CORNER_BLEND_CAPABILITY,
        vec![
            "exact_equal_radius_sphere_octant",
            "three_rational_cylinders",
            "plane_cylinder_sphere_network",
        ],
        radius,
        "BREP_VALENCE3_CORNER_BLEND_REFUSED",
    )
}

/// AF-01: cuboid single convex edge → cylindrical fillet via corner cutter − cylinder.
pub fn analytic_fillet(
    model: &Model,
    edge: usize,
    radius: f64,
) -> Result<(Model, FeatureCertificate)> {
    model.validate()?;
    if !(radius.is_finite() && radius > 0.) {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Fillet radius must be finite and positive",
        ));
    }
    if !is_axis_aligned_cuboid(model) {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "analytic-fillet/1 admits axis-aligned planar cuboids only (AF-N1)",
        ));
    }
    if edge >= model.edges.len() {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Edge index out of range",
        ));
    }
    let edge_ref = &model.edges[edge];
    if edge_ref.curve.degree != 1 {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Curved edges are outside analytic-fillet/1 (AF-N1)",
        ));
    }
    let a = model.vertices[edge_ref.vertices[0]].point;
    let b = model.vertices[edge_ref.vertices[1]].point;
    let dir = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let len = dir[0].hypot(dir[1]).hypot(dir[2]);
    if !(len.is_finite() && len > radius * 2.) {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Edge too short for the requested fillet radius",
        ));
    }
    // Prefer a vertical (+Z) edge on the +X/+Y corner of an axis-aligned box.
    let (min, max) = model_bounds(model);
    let span = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    if span.iter().any(|s| *s <= radius * 2.) {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Cuboid extents too small for fillet radius",
        ));
    }
    let vertical = dir[0].abs() <= 1e-12 && dir[1].abs() <= 1e-12 && dir[2].abs() > 1e-12;
    if !vertical {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "AF-01 walking slice admits vertical (+Z) cuboid edges only",
        ));
    }
    let x = (a[0] + b[0]) * 0.5;
    let y = (a[1] + b[1]) * 0.5;
    let at_max_x = (x - max[0]).abs() <= (x - min[0]).abs();
    let at_max_y = (y - max[1]).abs() <= (y - min[1]).abs();
    let corner = match (at_max_x, at_max_y) {
        (false, false) => 0,
        (true, false) => 1,
        (true, true) => 2,
        (false, true) => 3,
    };
    let mut rounded = [false; 4];
    rounded[corner] = true;
    let result =
        crate::imprint_pipeline::rounded_cuboid_vertical_edges(model, min, max, rounded, radius)?;
    result.validate()?;
    let has_cyl = result
        .faces
        .iter()
        .any(|f| f.surface.degree_u > 1 || f.surface.degree_v > 1);
    if !has_cyl {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Fillet result missing cylindrical face; refuse faceted claim",
        ));
    }
    Ok((
        result,
        FeatureCertificate {
            capability: "analytic-fillet/1",
            complete: true,
            notes: vec!["af01_cylindrical_edge"],
        },
    ))
}

pub(super) fn vertical_edge_xy(model: &Model, edge: usize) -> Option<[f64; 2]> {
    let e = model.edges.get(edge)?;
    let a = model.vertices.get(e.vertices[0])?.point;
    let b = model.vertices.get(e.vertices[1])?.point;
    if (a[0] - b[0]).abs() > 1e-9 || (a[1] - b[1]).abs() > 1e-9 {
        return None;
    }
    Some([a[0], a[1]])
}

pub(super) fn remap_vertical_edge(model: &Model, xy: [f64; 2], tol: f64) -> Result<usize> {
    model
        .edges
        .iter()
        .enumerate()
        .find_map(|(i, e)| {
            let a = model.vertices[e.vertices[0]].point;
            let b = model.vertices[e.vertices[1]].point;
            if (a[0] - b[0]).abs() > 1e-9 || (a[1] - b[1]).abs() > 1e-9 {
                return None;
            }
            let mx = 0.5 * (a[0] + b[0]);
            let my = 0.5 * (a[1] + b[1]);
            if (mx - xy[0]).hypot(my - xy[1]) <= tol {
                Some(i)
            } else {
                None
            }
        })
        .ok_or_else(|| {
            refuse(
                "BREP_ANALYTIC_FILLET_REFUSED",
                "Fillet chain remapping lost a vertical edge; refuse silent nearest-edge",
            )
        })
}

/// AF-01 fillet across a chain of vertical cuboid edges.
/// Cutters are authored from the original solid (durable XY), then applied as one
/// compound difference so intermediate non-cuboid solids never re-enter AF-01.
pub fn analytic_fillet_chain(
    model: &Model,
    edges: &[usize],
    radius: f64,
) -> Result<(Model, FeatureCertificate)> {
    if edges.is_empty() {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Fillet chain requires at least one edge",
        ));
    }
    if edges.len() > 8 {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Fillet chain exceeds 8-edge budget",
        ));
    }
    if !(radius.is_finite() && radius > 0.) {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Fillet radius must be finite and positive",
        ));
    }
    if !is_axis_aligned_cuboid(model) {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "analytic-fillet/1 admits axis-aligned planar cuboids only (AF-N1)",
        ));
    }
    let (min, max) = model_bounds(model);
    if max[0] - min[0] <= 2. * radius || max[1] - min[1] <= 2. * radius {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Cuboid extents are too small for the fillet chain radius",
        ));
    }
    let mut rounded = [false; 4];
    let mut seen = std::collections::BTreeSet::new();
    for &e in edges {
        let xy = vertical_edge_xy(model, e).ok_or_else(|| {
            refuse(
                "BREP_ANALYTIC_FILLET_REFUSED",
                "Fillet chain admits vertical cuboid edges only",
            )
        })?;
        let key = ((xy[0] * 1e9).round() as i64, (xy[1] * 1e9).round() as i64);
        if !seen.insert(key) {
            continue;
        }
        let idx = remap_vertical_edge(model, xy, radius.max(1e-6))?;
        let edge_ref = &model.edges[idx];
        let a = model.vertices[edge_ref.vertices[0]].point;
        let b = model.vertices[edge_ref.vertices[1]].point;
        let height = (a[2] - b[2]).abs();
        if !(height.is_finite() && height > radius * 2.) {
            return Err(refuse(
                "BREP_ANALYTIC_FILLET_REFUSED",
                "Edge too short for the requested fillet radius",
            ));
        }
        let x = (a[0] + b[0]) * 0.5;
        let y = (a[1] + b[1]) * 0.5;
        let at_max_x = (x - max[0]).abs() <= (x - min[0]).abs();
        let at_max_y = (y - max[1]).abs() <= (y - min[1]).abs();
        let corner = match (at_max_x, at_max_y) {
            (false, false) => 0,
            (true, false) => 1,
            (true, true) => 2,
            (false, true) => 3,
        };
        rounded[corner] = true;
    }
    if !rounded.iter().any(|rounded| *rounded) {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Fillet chain selected no distinct vertical corners",
        ));
    }
    let result =
        crate::imprint_pipeline::rounded_cuboid_vertical_edges(model, min, max, rounded, radius)?;
    result.validate()?;
    Ok((
        result,
        FeatureCertificate {
            capability: "analytic-fillet/1",
            complete: true,
            notes: vec!["af01_fillet_chain_remapped", "af01_exact_arc_profile"],
        },
    ))
}

/// Audited successor wrapper around the exact multi-edge fillet author.
/// The result is published only after context-bound correspondence evidence,
/// global solid audit, and persistent naming/ChangeSet evidence agree.
pub fn audited_multi_edge_fillet(
    model: &Model,
    edges: &[usize],
    radius: f64,
) -> Result<AuditedFeatureResult> {
    let (result, mut feature) = analytic_fillet_chain(model, edges, radius)?;
    feature.capability = AUDITED_MULTI_EDGE_FILLET_CAPABILITY;
    let context = result.tolerance_context()?;
    let evidence = compose_predicate_evidence(
        &context,
        [
            PredicateEvidence::correspondence(&context, 0., radius)?,
            PredicateEvidence::topology_preservation(
                &context,
                "multi-edge fillet authored shared-edge incidence",
                result.persistent_naming_complete() && !result.1.faces.is_empty(),
            )?,
        ],
    )?;
    let audit = audit_solid(&result)?;
    let naming_complete = result.persistent_naming_complete()
        && result.1.faces.len() == result.faces.len()
        && result.1.edges.len() == result.edges.len();
    if !naming_complete {
        return Err(refuse(
            "BREP_ANALYTIC_FILLET_REFUSED",
            "Audited multi-edge fillet lacks complete ChangeSet/naming evidence",
        ));
    }
    Ok(AuditedFeatureResult {
        change_set: result.1.change_set.clone(),
        model: result,
        feature,
        context: context.spec_identity(),
        evidence,
        audit,
        naming_complete,
    })
}
pub fn analytic_chamfer(
    model: &Model,
    edge: usize,
    distance: f64,
) -> Result<(Model, FeatureCertificate)> {
    model.validate()?;
    if !(distance.is_finite() && distance > 0.) {
        return Err(refuse(
            "BREP_ANALYTIC_CHAMFER_REFUSED",
            "Chamfer distance must be finite and positive",
        ));
    }
    if !is_axis_aligned_cuboid(model) {
        return Err(refuse(
            "BREP_ANALYTIC_CHAMFER_REFUSED",
            "analytic-chamfer/1 admits axis-aligned planar cuboids only",
        ));
    }
    if edge >= model.edges.len() {
        return Err(refuse(
            "BREP_ANALYTIC_CHAMFER_REFUSED",
            "Edge index out of range",
        ));
    }
    let edge_ref = &model.edges[edge];
    if edge_ref.curve.degree != 1 {
        return Err(refuse(
            "BREP_ANALYTIC_CHAMFER_REFUSED",
            "Curved edges are outside analytic-chamfer/1",
        ));
    }
    let a = model.vertices[edge_ref.vertices[0]].point;
    let b = model.vertices[edge_ref.vertices[1]].point;
    let dir = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let len = dir[0].hypot(dir[1]).hypot(dir[2]);
    if !(len.is_finite() && len > distance * 2.) {
        return Err(refuse(
            "BREP_ANALYTIC_CHAMFER_REFUSED",
            "Edge too short for the requested chamfer distance",
        ));
    }
    let (min, max) = model_bounds(model);
    let span = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    if span.iter().any(|s| *s <= distance * 2.) {
        return Err(refuse(
            "BREP_ANALYTIC_CHAMFER_REFUSED",
            "Cuboid extents too small for chamfer distance",
        ));
    }
    let vertical = dir[0].abs() <= 1e-12 && dir[1].abs() <= 1e-12 && dir[2].abs() > 1e-12;
    if !vertical {
        return Err(refuse(
            "BREP_ANALYTIC_CHAMFER_REFUSED",
            "AF-01 walking slice admits vertical (+Z) cuboid edges only",
        ));
    }
    let z0 = a[2].min(b[2]);
    let height = len;
    let x = (a[0] + b[0]) * 0.5;
    let y = (a[1] + b[1]) * 0.5;
    let sx = if (x - max[0]).abs() <= 1e-9 {
        1.
    } else if (x - min[0]).abs() <= 1e-9 {
        -1.
    } else {
        return Err(refuse(
            "BREP_ANALYTIC_CHAMFER_REFUSED",
            "Selected edge is not on a cuboid corner",
        ));
    };
    let sy = if (y - max[1]).abs() <= 1e-9 {
        1.
    } else if (y - min[1]).abs() <= 1e-9 {
        -1.
    } else {
        return Err(refuse(
            "BREP_ANALYTIC_CHAMFER_REFUSED",
            "Selected edge is not on a cuboid corner",
        ));
    };
    // Right-triangular cutter at the selected corner. Keep its profile CCW.
    let mut profile = [[x - sx * distance, y], [x, y - sy * distance], [x, y]];
    let area2 = (profile[1][0] - profile[0][0]) * (profile[2][1] - profile[0][1])
        - (profile[1][1] - profile[0][1]) * (profile[2][0] - profile[0][0]);
    if area2 < 0. {
        profile.swap(0, 1);
    }
    let cutter = extrude_polygon(&profile, z0, z0 + height)?;
    let result = boolean(model, &cutter, "difference")?;
    result.validate()?;
    // Chamfer faces remain planar (degree 1). Faceted mesh bevels must not be claimed here.
    if result
        .faces
        .iter()
        .any(|f| f.surface.degree_u > 1 || f.surface.degree_v > 1)
    {
        return Err(refuse(
            "BREP_ANALYTIC_CHAMFER_REFUSED",
            "Unexpected curved face in planar chamfer result",
        ));
    }
    Ok((
        result,
        FeatureCertificate {
            capability: "analytic-chamfer/1",
            complete: true,
            notes: vec!["af01_planar_chamfer_edge", "not_mesh_bevel"],
        },
    ))
}
