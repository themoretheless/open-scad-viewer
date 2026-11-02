use super::*;
/// Bounded regularized CSG over retained boundary geometry. Empty results have
/// no placeholder topology. Unsupported curved intersections remain explicit.
pub fn boolean(a: &Model, b: &Model, operation: &str) -> Result<Model> {
    boolean_impl(a, b, operation, true)
}
pub(super) fn boolean_impl(
    a: &Model,
    b: &Model,
    operation: &str,
    retry_seams: bool,
) -> Result<Model> {
    a.validate()?;
    b.validate()?;
    if !matches!(operation, "union" | "difference" | "intersection" | "xor") {
        return Err(Error::new(
            "BREP_INVALID_OPERATION",
            "Boolean operation must be union, difference, intersection, or xor",
        ));
    }
    if let Some(result) = crate::boolean_support::simplify(a, b, operation)? {
        return Ok(result);
    }
    let has_curved_geometry = a
        .edges
        .iter()
        .chain(&b.edges)
        .any(|edge| edge.curve.degree > 1)
        || a.faces
            .iter()
            .chain(&b.faces)
            .any(|face| face.surface.degree_u > 1 || face.surface.degree_v > 1);
    // Curved operands are fail-closed: a rejected analytic certificate must
    // never fall through to prism recognition, polygonal CSG, or Manifold.
    if has_curved_geometry {
        return match curved_boolean_exact(a, b, operation) {
            Ok(model) => Ok(model),
            Err(error) if proof_refusal(error.code) => {
                if retry_seams {
                    if let Some(model) =
                        super::seam_boolean::try_seam_normalized_boolean(a, b, operation)?
                    {
                        return Ok(model);
                    }
                }
                if let Some(model) =
                    crate::minkowski3d::curved::try_graph_box_boolean(a, b, operation)?
                {
                    return Ok(model);
                }
                if operation == "intersection" {
                    if let Some(model) =
                        crate::minkowski3d::curved::try_graph_box_boolean(b, a, operation)?
                    {
                        return Ok(model);
                    }
                }
                if let Some(model) =
                    crate::minkowski3d::curved::try_nonintersecting_nurbs_boolean(a, b, operation)?
                {
                    return Ok(model);
                }
                if let Some(model) =
                    crate::minkowski3d::curved::try_nurbs_boundary_only_boolean(a, b, operation)?
                {
                    return Ok(model);
                }
                if let Some(model) =
                    crate::minkowski3d::curved::try_retained_nurbs_boolean(a, b, operation)?
                {
                    return Ok(model);
                }
                Err(Error::new(
                    "BREP_NURBS_BOOLEAN_UNPROVED",
                    format!(
                        "Complete NURBS intersection coverage and retained-material classification are unproved: {}",
                        error.message
                    ),
                ))
            }
            Err(error) => Err(error),
        };
    }
    // Convex planar CSG is built directly from clipped boundary polygons,
    // independent of face orientation in world axes.
    if operation != "xor"
        && a.bodies.len() == 1
        && b.bodies.len() == 1
        && a.bodies[0].inner_shells.is_empty()
        && b.bodies[0].inner_shells.is_empty()
        && (orthogonal(a).is_err() || orthogonal(b).is_err())
        && convex_planes(a).is_ok()
        && convex_planes(b).is_ok()
    {
        return convex_boolean(a, b, operation).map_err(|e| {
            if e.code == OPERATION_FAILED {
                unsupported("Boolean result is empty or dimensionally collapsed")
            } else {
                e
            }
        });
    }
    if orthogonal(a).is_err() || orthogonal(b).is_err() {
        return planar_boolean(a, b, operation);
    }
    orthogonal(a)?;
    orthogonal(b)?;
    let mut coordinates: [Vec<f64>; 3] = std::array::from_fn(|_| vec![]);
    for vertex in a.vertices.iter().chain(&b.vertices) {
        for axis in 0..3 {
            coordinates[axis].push(vertex.point[axis]);
        }
    }
    for values in &mut coordinates {
        values.sort_by(f64::total_cmp);
        values.dedup_by(|x, y| (*x - *y).abs() <= a.tolerance_mm.max(b.tolerance_mm) * 4.);
        if values.len() > 32 {
            return Err(Error::new(
                "BREP_RESOURCE_LIMIT",
                "Boolean coordinate grid exceeds 31 cells per axis",
            ));
        }
    }
    let shape = [
        coordinates[0].len() - 1,
        coordinates[1].len() - 1,
        coordinates[2].len() - 1,
    ];
    let mut occupied = BTreeSet::<[usize; 3]>::new();
    for x in 0..shape[0] {
        for y in 0..shape[1] {
            for z in 0..shape[2] {
                let p = [
                    (coordinates[0][x] + coordinates[0][x + 1]) / 2.,
                    (coordinates[1][y] + coordinates[1][y + 1]) / 2.,
                    (coordinates[2][z] + coordinates[2][z + 1]) / 2.,
                ];
                let inside_a = contains(a, p)?;
                let inside_b = contains(b, p)?;
                let keep = match operation {
                    "union" => inside_a || inside_b,
                    "difference" => inside_a && !inside_b,
                    "xor" => inside_a != inside_b,
                    _ => inside_a && inside_b,
                };
                if keep {
                    occupied.insert([x, y, z]);
                }
            }
        }
    }
    if occupied.is_empty() {
        return Model::empty(a.tolerance_mm.max(b.tolerance_mm));
    }
    let polygons = super::orthogonal_boundary::polygons(&coordinates, &occupied)?;
    let mut result = model_from_polygons(polygons, a.tolerance_mm.max(b.tolerance_mm))?;
    result.inherit_topology_ids(&[a, b]);
    result.validate()?;
    Ok(result)
}

// Unsupported sufficient proofs can be followed by another complete proof.
// Invalid data and resource-limit codes keep their original failure semantics.
pub(super) fn proof_refusal(code: &str) -> bool {
    code == "BREP_UNSUPPORTED_OPERATION"
        || code == "BREP_UNSUPPORTED_PLANAR_TRIM"
        || code.ends_with("_REFUSED")
        || code.ends_with("_UNPROVED")
        || code.ends_with("_UNRESOLVED")
}
fn curved_boolean_exact(a: &Model, b: &Model, operation: &str) -> Result<Model> {
    if let Some(model) = crate::minkowski3d::curved::try_common_quadratic_boolean(a, b, operation)?
    {
        return Ok(model);
    }
    if crate::nurbs_ss_g6::is_general_nurbs_boolean_candidate(a, b) {
        return crate::nurbs_ss_g6::author_general_nurbs_boolean(a, b, operation)
            .map(|(model, _certificate)| model);
    }
    if crate::nurbs_ss_g6::is_nurbs_boolean_v5_candidate(a, b) {
        return crate::nurbs_ss_g6::nurbs_boolean_rational_graph_patch_v5(a, b, operation)
            .map(|(model, _certificate)| model);
    }
    if crate::nurbs_ss_g6::is_nurbs_boolean_v3_candidate(a, b) {
        if crate::nurbs_ss_g6::canonical_graph_is_first(a)
            && let Ok((model, _certificate)) =
                crate::nurbs_ss_g6::nurbs_boolean_graph_containment_v4(a, b, operation)
        {
            return Ok(model);
        }
        if let Ok((model, _certificate)) =
            crate::nurbs_ss_g6::nurbs_boolean_graph_patch_unequal_v4(a, b, operation)
        {
            return Ok(model);
        }
        return crate::nurbs_ss_g6::nurbs_boolean_graph_patch_v3(a, b, operation)
            .map(|(model, _certificate)| model);
    }
    if crate::nurbs_ss_g6::is_nurbs_boolean_candidate(a, b) {
        return crate::nurbs_ss_g6::nurbs_boolean_imprint_solids(a, b, operation)
            .map(|(model, _certificate)| model);
    }
    match crate::profile_imprint::boolean(a, b, operation) {
        Ok(Some(result)) => {
            crate::solid_audit::audit_solid(&result)?;
            return Ok(result);
        }
        Ok(None) => {}
        // A profile family that cannot read an operand (free-form
        // trims, fitted flanks) is not a verdict on the pair: the
        // analytic matrix still gets to prove the pair.
        Err(e) if e.code == "BREP_UNSUPPORTED_PLANAR_TRIM" => {}
        Err(e) => return Err(e),
    }
    match crate::analytic_boolean::analytic_boolean(a, b, operation) {
        Ok((result, cert)) => {
            if !cert.permits_topology_change() || !cert.no_prism_authorship {
                return Err(Error::new(
                    "BREP_ANALYTIC_BOOLEAN_REFUSED",
                    "Curved Boolean certificate does not permit topology change",
                ));
            }
            return Ok(result);
        }
        Err(error) => Err(error),
    }
}
