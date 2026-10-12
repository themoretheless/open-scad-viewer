use super::*;

/// Parallel cylinder wall imprint from Complete generator-line events.
///
/// After a Complete imprint plan is certified, author the retained circular
/// spans, shared crossing vertices, side patches, and cap loops exactly.
/// Tangent, axial mismatch, or unsupported frame cases refuse.
#[derive(Clone, Copy)]
pub struct CylinderEnvelope {
    pub origin: [f64; 3],
    pub axis: [f64; 3],
    pub radius: f64,
    pub height: [f64; 2],
}

pub fn parallel_cylinder_wall_imprint(
    a: &Model,
    b: &Model,
    operation: &str,
    events: &[ImprintEvent],
    a_cylinder: CylinderEnvelope,
    b_cylinder: CylinderEnvelope,
) -> Result<Model> {
    let CylinderEnvelope {
        origin: a_origin,
        axis: a_axis,
        radius: a_radius,
        height: a_height,
    } = a_cylinder;
    let CylinderEnvelope {
        origin: b_origin,
        axis: b_axis,
        radius: b_radius,
        height: b_height,
    } = b_cylinder;
    a.validate()?;
    b.validate()?;
    if !matches!(operation, "union" | "difference" | "intersection") {
        return Err(refuse(
            "Wall imprint admits union, difference, or intersection only",
        ));
    }
    let max_span = (a_height[1] - a_height[0]).abs() + (b_height[1] - b_height[0]).abs() + 1.;
    let plan = build_imprint_plan(events, max_span)?;
    if !plan.complete || plan.segments.len() < 2 {
        return Err(refuse(
            "Wall imprint requires a Complete multi-segment plan",
        ));
    }
    let tol = a.tolerance_mm.max(b.tolerance_mm);
    if (a_axis[0] - b_axis[0]).abs() > 1e-12
        || (a_axis[1] - b_axis[1]).abs() > 1e-12
        || (a_axis[2] - b_axis[2]).abs() > 1e-12
        || a_axis[0].abs() > 1e-12
        || a_axis[1].abs() > 1e-12
        || (a_axis[2] - 1.).abs() > 1e-12
    {
        return Err(refuse(
            "Cylinder wall imprint currently admits common +Z axes only",
        ));
    }
    if (a_height[0] - b_height[0]).abs() > tol || (a_height[1] - b_height[1]).abs() > tol {
        return Err(refuse(
            "Cylinder wall imprint requires coincident cap planes",
        ));
    }
    let arcs = circle_boolean_boundary(
        [a_origin[0], a_origin[1]],
        a_radius,
        [b_origin[0], b_origin[1]],
        b_radius,
        operation,
    )?;
    extrude_arc_loop(&arcs, a_height[0], a_height[1], tol, &[a, b])
}

/// Regularized empty-algebra for disjoint / identical / containment corpora.
/// Does not call prismatic_boolean or mesh paths.
pub fn regularized_empty_algebra(
    a: &Model,
    b: &Model,
    operation: &str,
    relation: SpatialRelation,
) -> Result<Model> {
    a.validate()?;
    b.validate()?;
    if !matches!(operation, "union" | "difference" | "intersection" | "xor") {
        return Err(refuse(
            "Boolean operation must be union, difference, intersection, or xor",
        ));
    }
    let tolerance = a.tolerance_mm.max(b.tolerance_mm);
    let out = match (relation, operation) {
        (SpatialRelation::Disjoint, "union") => crate::boolean_support::separated_union(a, b)?,
        (SpatialRelation::Disjoint, "intersection") => Model::empty(tolerance)?,
        (SpatialRelation::Disjoint, "difference") => a.clone(),
        (SpatialRelation::Disjoint, "xor") => crate::boolean_support::separated_union(a, b)?,
        (SpatialRelation::Identical, "union" | "intersection") => a.clone(),
        (SpatialRelation::Identical, "difference" | "xor") => Model::empty(tolerance)?,
        (SpatialRelation::AContainsB, "union") => a.clone(),
        (SpatialRelation::AContainsB, "intersection") => b.clone(),
        (SpatialRelation::AContainsB, "difference") => cavity(a, b, tolerance)?,
        (SpatialRelation::AContainsB, "xor") => cavity(a, b, tolerance)?,
        (SpatialRelation::BContainsA, "union") => b.clone(),
        (SpatialRelation::BContainsA, "intersection") => a.clone(),
        (SpatialRelation::BContainsA, "difference") => Model::empty(tolerance)?,
        (SpatialRelation::BContainsA, "xor") => cavity(b, a, tolerance)?,
        (SpatialRelation::WallIntersect, _) => {
            return Err(refuse(
                "WallIntersect requires parallel_cylinder_wall_imprint, not empty algebra",
            ));
        }
        _ => {
            return Err(refuse(
                "Boolean operation not admitted for regularized empty algebra",
            ));
        }
    };
    out.validate()?;
    Ok(out)
}

/// Outer body with inner shell inverted (cavity). Shared by sphere and cylinder.
pub fn cavity(outer: &Model, inner: &Model, tolerance: f64) -> Result<Model> {
    let mut result = outer.clone();
    result.tolerance_mm = tolerance;
    let (v, e, l, f, s) = (
        outer.vertices.len(),
        outer.edges.len(),
        outer.loops.len(),
        outer.faces.len(),
        outer.shells.len(),
    );
    result.vertices.extend(inner.vertices.iter().cloned());
    result
        .edges
        .extend(inner.edges.iter().cloned().map(|mut edge| {
            edge.vertices.iter_mut().for_each(|id| *id += v);
            edge
        }));
    result
        .loops
        .extend(inner.loops.iter().cloned().map(|mut wire| {
            wire.coedges.iter_mut().for_each(|use_| use_.edge += e);
            wire
        }));
    result
        .faces
        .extend(inner.faces.iter().cloned().map(|mut face| {
            face.outer += l;
            face.holes.iter_mut().for_each(|id| *id += l);
            face
        }));
    if inner.shells.is_empty() || result.bodies.is_empty() {
        return Err(refuse(
            "Cavity requires nonempty outer body and inner shell",
        ));
    }
    result.shells.push(Shell {
        faces: inner.shells[0]
            .faces
            .iter()
            .map(|use_| FaceUse {
                face: use_.face + f,
                reversed: !use_.reversed,
            })
            .collect(),
        closed: true,
    });
    result.bodies[0].inner_shells.push(s);
    let _ = Body {
        outer_shell: 0,
        inner_shells: vec![],
    };
    result.inherit_topology_ids(&[outer, inner]);
    result.validate()?;
    Ok(result)
}
