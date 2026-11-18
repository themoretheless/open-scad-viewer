use super::*;

/// Exact material-left rounded profile, shared by extrusion and revolution.
pub(crate) fn rounded_profile_curves(
    source: &Model,
    profile: &[[f64; 2]],
    rounded: &[bool],
    radius: f64,
    z0: f64,
) -> Result<Vec<Curve>> {
    if profile.len() < 3
        || profile.len() != rounded.len()
        || !(radius.is_finite() && radius > 0. && z0.is_finite())
    {
        return Err(refuse("Rounded convex prism parameters are invalid"));
    }
    let n = profile.len();
    let mut entry = profile.to_vec();
    let mut exit = profile.to_vec();
    let mut arcs = vec![None; n];
    let mut tangent = vec![0.; n];
    for i in 0..n {
        if !rounded[i] {
            continue;
        }
        let p = profile[i];
        let prev = profile[(i + n - 1) % n];
        let next = profile[(i + 1) % n];
        let unit = |d: [f64; 2]| {
            let length = d[0].hypot(d[1]);
            (length > 1e-12).then_some([d[0] / length, d[1] / length])
        };
        let u = unit([prev[0] - p[0], prev[1] - p[1]])
            .ok_or_else(|| refuse("Rounded profile has a collapsed incoming edge"))?;
        let v = unit([next[0] - p[0], next[1] - p[1]])
            .ok_or_else(|| refuse("Rounded profile has a collapsed outgoing edge"))?;
        if u[0] * v[1] - u[1] * v[0] >= -1e-12 {
            return Err(refuse("Selected profile corner must be convex"));
        }
        let theta = (u[0] * v[0] + u[1] * v[1]).clamp(-1., 1.).acos();
        let turn = std::f64::consts::PI - theta;
        if !(theta > 1e-6 && turn > 1e-6) {
            return Err(refuse("Rounded profile requires strict convexity"));
        }
        let distance = radius / (theta * 0.5).tan();
        let bisector_length = (u[0] + v[0]).hypot(u[1] + v[1]);
        let center_distance = radius / (theta * 0.5).sin();
        let center = [
            p[0] + center_distance * (u[0] + v[0]) / bisector_length,
            p[1] + center_distance * (u[1] + v[1]) / bisector_length,
        ];
        entry[i] = [p[0] + distance * u[0], p[1] + distance * u[1]];
        exit[i] = [p[0] + distance * v[0], p[1] + distance * v[1]];
        tangent[i] = distance;
        let start = (entry[i][1] - center[1]).atan2(entry[i][0] - center[0]);
        arcs[i] = Some(CircleArc {
            center,
            radius,
            start,
            sweep: turn,
        });
    }
    for i in 0..n {
        let length = (profile[(i + 1) % n][0] - profile[i][0])
            .hypot(profile[(i + 1) % n][1] - profile[i][1]);
        if tangent[i] + tangent[(i + 1) % n] >= length - source.tolerance_mm * 8. {
            return Err(refuse("Fillet radius collides on an adjacent profile edge"));
        }
    }
    let mut curves = Vec::with_capacity(n * 2);
    for i in 0..n {
        if let Some(arc) = arcs[i] {
            curves.push(arc_curve(arc, z0));
        }
        let next = (i + 1) % n;
        curves.push(crate::line(
            vec![exit[i][0], exit[i][1], z0],
            vec![entry[next][0], entry[next][1], z0],
        ));
    }
    // Nonadjacent boundary spans must be proved disjoint. A valid adjacent
    // tangent budget alone does not exclude a collision across a concave notch.
    let mut spec = cad_predicates::ToleranceContext::default_valid()
        .specification()
        .clone();
    spec.linear_abs = source.tolerance_mm;
    let bounds: Vec<_> = curves
        .iter()
        .map(|curve| {
            let mut min = [f64::INFINITY; 2];
            let mut max = [f64::NEG_INFINITY; 2];
            for point in &curve.control_points {
                for axis in 0..2 {
                    min[axis] = min[axis].min(point[axis]);
                    max[axis] = max[axis].max(point[axis]);
                }
            }
            (min, max)
        })
        .collect();
    for i in 0..curves.len() {
        for j in i + 2..curves.len() {
            if i == 0 && j + 1 == curves.len() {
                continue;
            }
            let (a_min, a_max) = bounds[i];
            let (b_min, b_max) = bounds[j];
            if (0..2).any(|axis| {
                a_max[axis] + source.tolerance_mm < b_min[axis]
                    || b_max[axis] + source.tolerance_mm < a_min[axis]
            }) {
                continue;
            }
            let report = nurbs_core::intersection::intersect_curve_curve_report(
                &curves[i],
                &curves[j],
                Some(
                    cad_predicates::ToleranceContext::new(spec.clone())
                        .map_err(|_| refuse("Invalid fillet collision tolerance"))?,
                ),
            )?;
            if !report.report.unresolved.is_empty() || !report.report.components.is_empty() {
                return Err(refuse(
                    "Fillet boundary collides with a nonadjacent span or separation is unresolved",
                ));
            }
        }
    }
    Ok(curves)
}

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
