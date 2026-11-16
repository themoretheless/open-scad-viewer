use super::*;

const VALENCE3_REFUSED: &str = "BREP_VALENCE3_CORNER_BLEND_REFUSED";
/// Floor for the corner-recognition tolerance.
const MIN_CORNER_TOLERANCE: f64 = 1e-9;

pub(super) fn exact_axis_aligned_valence3_corner_blend(
    model: &Model,
    edges: &[usize],
    radius: f64,
) -> Result<AuditedFeatureResult> {
    let (min, max, vid) = valence3_corner_vertex(model, edges, radius)?;
    let placement = max_corner_reflection(model, model.vertices[vid].point, min, max)?;
    let local_source = crate::transform::affine(model, placement)?;
    let local_result =
        crate::imprint_pipeline::valence3_cuboid_max_corner(&local_source, min, max, radius)
            .map_err(|error| {
                refuse(
                    VALENCE3_REFUSED,
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
            VALENCE3_REFUSED,
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
        VALENCE3_REFUSED,
    )
}

/// Audits the request; returns the cuboid bounds and the shared corner vertex.
fn valence3_corner_vertex(
    model: &Model,
    edges: &[usize],
    radius: f64,
) -> Result<([f64; 3], [f64; 3], usize)> {
    audit_solid(model).map_err(|_| {
        refuse(
            VALENCE3_REFUSED,
            "Valence-3 corner blend source must pass the global solid audit",
        )
    })?;
    if !(radius.is_finite() && radius > 0.) {
        return Err(refuse(
            VALENCE3_REFUSED,
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
            VALENCE3_REFUSED,
            "exact-valence3-corner-blend/1 admits exactly three distinct concurrent cuboid edges",
        ));
    }
    if !is_axis_aligned_cuboid(model) {
        return Err(refuse(
            VALENCE3_REFUSED,
            "exact-valence3-corner-blend/1 admits axis-aligned planar cuboids only",
        ));
    }
    let (min, max) = model_bounds(model);
    if (0..3).any(|i| max[i] - min[i] <= 2. * radius) {
        return Err(refuse(
            VALENCE3_REFUSED,
            "Cuboid extents too small for the valence-3 radius",
        ));
    }
    let mut sets = Vec::with_capacity(edges.len());
    for &edge in edges {
        let Some(edge_ref) = model.edges.get(edge) else {
            return Err(refuse(VALENCE3_REFUSED, "Edge index out of range"));
        };
        if edge_ref.curve.degree != 1 {
            return Err(refuse(
                VALENCE3_REFUSED,
                "Curved edges are outside exact-valence3-corner-blend/1",
            ));
        }
        sets.push(edge_ref.vertices);
    }
    // Three distinct edges must share one cuboid corner.
    let corner_vertex = match sets.as_slice() {
        [first, second, third] => first
            .iter()
            .copied()
            .find(|v| second.contains(v) && third.contains(v)),
        _ => None,
    };
    let Some(vid) = corner_vertex else {
        return Err(refuse(
            VALENCE3_REFUSED,
            "Selected edges do not meet at a single valence-3 vertex",
        ));
    };
    Ok((min, max, vid))
}

/// Reflect each minimum coordinate to the maximum corner. This is an
/// isometry: radius and contact angles remain unchanged. affine() also
/// reverses shell uses for odd reflections and preserves topology IDs.
fn max_corner_reflection(
    model: &Model,
    p: [f64; 3],
    min: [f64; 3],
    max: [f64; 3],
) -> Result<[[f64; 4]; 4]> {
    let tolerance = model.tolerance_mm.max(MIN_CORNER_TOLERANCE);
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
                VALENCE3_REFUSED,
                "Selected vertex must be a cuboid corner",
            ));
        }
    }
    Ok(placement)
}

/// AF-01: cuboid single convex edge → cylindrical fillet via corner cutter − cylinder.
pub fn analytic_fillet(
    model: &Model,
    edge: usize,
    radius: f64,
) -> Result<(Model, FeatureCertificate)> {
    crate::admit_operation_input(cad_predicates::OperationBoundary::Fillet, model.tolerance_mm)?;
    model.validate()?;
    let input_mm = model.tolerance_mm;
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
    let mut result =
        crate::imprint_pipeline::rounded_cuboid_vertical_edges(model, min, max, rounded, radius)?;
    crate::enforce_model_tolerance_handoff(
        cad_predicates::OperationBoundary::Fillet,
        input_mm,
        0.,
        &mut result,
    )?;
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
    crate::admit_operation_input(cad_predicates::OperationBoundary::Fillet, model.tolerance_mm)?;
    model.validate()?;
    let input_mm = model.tolerance_mm;
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
    let mut result =
        crate::imprint_pipeline::rounded_cuboid_vertical_edges(model, min, max, rounded, radius)?;
    crate::enforce_model_tolerance_handoff(
        cad_predicates::OperationBoundary::Fillet,
        input_mm,
        0.,
        &mut result,
    )?;
    Ok((
        result,
        FeatureCertificate {
            capability: "analytic-fillet/1",
            complete: true,
            notes: vec!["af01_fillet_chain_remapped", "af01_exact_arc_profile"],
        },
    ))
}

/// Audited successor wrapper around the exact multi-edge fillet author, run
/// under the Fillet contract (pre- and postconditions in `operation_contracts`).
/// The result is published only after context-bound correspondence evidence,
/// global solid audit, and persistent naming/ChangeSet evidence agree.
pub fn audited_multi_edge_fillet(
    model: &Model,
    edges: &[usize],
    radius: f64,
) -> Result<AuditedFeatureResult> {
    let (result, mut feature, _contract) =
        crate::operation_contracts::contracted_analytic_fillet_chain(model, edges, radius)?;
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
const CHAMFER_REFUSED: &str = "BREP_ANALYTIC_CHAMFER_REFUSED";
/// Absolute tolerance for recognizing a vertical edge.
const CHAMFER_VERTICAL_TOLERANCE: f64 = 1e-12;
/// Absolute tolerance for placing the edge on a cuboid corner.
const CHAMFER_CORNER_TOLERANCE: f64 = 1e-9;

pub fn analytic_chamfer(
    model: &Model,
    edge: usize,
    distance: f64,
) -> Result<(Model, FeatureCertificate)> {
    let (a, b, len, min, max) = vertical_chamfer_edge(model, edge, distance)?;
    let z0 = a[2].min(b[2]);
    let height = len;
    let x = (a[0] + b[0]) * 0.5;
    let y = (a[1] + b[1]) * 0.5;
    let sx = cuboid_corner_side(x, min[0], max[0])?;
    let sy = cuboid_corner_side(y, min[1], max[1])?;
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
            CHAMFER_REFUSED,
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

/// +1 at the maximum face, -1 at the minimum face, refused otherwise.
fn cuboid_corner_side(coordinate: f64, min: f64, max: f64) -> Result<f64> {
    if (coordinate - max).abs() <= CHAMFER_CORNER_TOLERANCE {
        Ok(1.)
    } else if (coordinate - min).abs() <= CHAMFER_CORNER_TOLERANCE {
        Ok(-1.)
    } else {
        Err(refuse(
            CHAMFER_REFUSED,
            "Selected edge is not on a cuboid corner",
        ))
    }
}

/// Audits the request; returns the edge endpoints, its length and the cuboid bounds.
#[allow(clippy::type_complexity)]
fn vertical_chamfer_edge(
    model: &Model,
    edge: usize,
    distance: f64,
) -> Result<([f64; 3], [f64; 3], f64, [f64; 3], [f64; 3])> {
    model.validate()?;
    if !(distance.is_finite() && distance > 0.) {
        return Err(refuse(
            CHAMFER_REFUSED,
            "Chamfer distance must be finite and positive",
        ));
    }
    if !is_axis_aligned_cuboid(model) {
        return Err(refuse(
            CHAMFER_REFUSED,
            "analytic-chamfer/1 admits axis-aligned planar cuboids only",
        ));
    }
    let Some(edge_ref) = model.edges.get(edge) else {
        return Err(refuse(CHAMFER_REFUSED, "Edge index out of range"));
    };
    if edge_ref.curve.degree != 1 {
        return Err(refuse(
            CHAMFER_REFUSED,
            "Curved edges are outside analytic-chamfer/1",
        ));
    }
    let a = model.vertices[edge_ref.vertices[0]].point;
    let b = model.vertices[edge_ref.vertices[1]].point;
    let dir = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let len = dir[0].hypot(dir[1]).hypot(dir[2]);
    if !(len.is_finite() && len > distance * 2.) {
        return Err(refuse(
            CHAMFER_REFUSED,
            "Edge too short for the requested chamfer distance",
        ));
    }
    let (min, max) = model_bounds(model);
    let span = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    if span.iter().any(|s| *s <= distance * 2.) {
        return Err(refuse(
            CHAMFER_REFUSED,
            "Cuboid extents too small for chamfer distance",
        ));
    }
    let vertical = dir[0].abs() <= CHAMFER_VERTICAL_TOLERANCE
        && dir[1].abs() <= CHAMFER_VERTICAL_TOLERANCE
        && dir[2].abs() > CHAMFER_VERTICAL_TOLERANCE;
    if !vertical {
        return Err(refuse(
            CHAMFER_REFUSED,
            "AF-01 walking slice admits vertical (+Z) cuboid edges only",
        ));
    }
    Ok((a, b, len, min, max))
}
