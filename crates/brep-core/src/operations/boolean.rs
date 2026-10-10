use super::*;
/// Bounded regularized CSG over retained boundary geometry. Empty results have
/// no placeholder topology. Unsupported curved intersections remain explicit.
pub fn boolean(a: &Model, b: &Model, operation: &str) -> Result<Model> {
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
            // analytic matrix and the tolerant fallback still get to try.
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
            Err(analytic_error) => {
                // The exact matrix refused this pair. A numerical
                // surface/surface trace with a stated tolerance is the
                // fallback; its result carries `tolerance_mm` so callers can
                // tell it from an exact one. Hard errors (invalid input,
                // resource limits) are not retried.
                if !matches!(
                    analytic_error.code,
                    "BREP_UNSUPPORTED_OPERATION"
                        | "BREP_ANALYTIC_BOOLEAN_REFUSED"
                        | "BREP_SOLID_AUDIT_REFUSED"
                ) || operation == "xor"
                {
                    return Err(analytic_error);
                }
                return crate::tolerant_boolean::boolean(a, b, operation).map_err(|tolerant| {
                    Error::new(
                        analytic_error.code,
                        format!(
                            "{}; tolerant fallback: {}",
                            analytic_error.message, tolerant.message
                        ),
                    )
                });
            }
        }
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
    let mut polygons = vec![];
    let directions = [
        ([-1, 0, 0], 0usize),
        ([1, 0, 0], 0),
        ([0, -1, 0], 1),
        ([0, 1, 0], 1),
        ([0, 0, -1], 2),
        ([0, 0, 1], 2),
    ];
    for &[x, y, z] in &occupied {
        let cell = [x, y, z];
        let x0 = coordinates[0][x];
        let x1 = coordinates[0][x + 1];
        let y0 = coordinates[1][y];
        let y1 = coordinates[1][y + 1];
        let z0 = coordinates[2][z];
        let z1 = coordinates[2][z + 1];
        for (side, &(delta, axis)) in directions.iter().enumerate() {
            let adjacent = cell[axis] as isize + delta[axis] as isize;
            let exposed = adjacent < 0 || adjacent >= shape[axis] as isize || {
                let mut neighbor = cell;
                neighbor[axis] = adjacent as usize;
                !occupied.contains(&neighbor)
            };
            if !exposed {
                continue;
            }
            polygons.push(match side {
                0 => vec![[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]],
                1 => vec![[x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [x1, y0, z1]],
                2 => vec![[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
                3 => vec![[x0, y1, z0], [x0, y1, z1], [x1, y1, z1], [x1, y1, z0]],
                4 => vec![[x0, y0, z0], [x0, y1, z0], [x1, y1, z0], [x1, y0, z0]],
                _ => vec![[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
            });
        }
    }
    let mut result = model_from_polygons(polygons, a.tolerance_mm.max(b.tolerance_mm))?;
    result.inherit_topology_ids(&[a, b]);
    result.validate()?;
    Ok(result)
}
