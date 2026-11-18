use super::*;

/// Gram determinant at or below which the graph frame projection is singular.
pub(super) const SINGULAR_PROJECTION_DETERMINANT: f64 = 1e-18;
/// Smallest tolerance used when admitting a cutter face as planar.
const MIN_PLANAR_TOLERANCE_MM: f64 = 1e-9;
/// Multiplier applied to the cutter tolerance for planar admission.
const PLANAR_TOLERANCE_FACTOR: f64 = 8.;

/// Tolerance used to admit a cutter face as affine-planar.
pub(super) fn planar_tolerance(cutter: &Model) -> f64 {
    cutter.tolerance_mm.max(MIN_PLANAR_TOLERANCE_MM) * PLANAR_TOLERANCE_FACTOR
}

/// One body, one shell, no cavity, and only affine-planar faces.
pub(super) fn is_single_affine_body(cutter: &Model) -> bool {
    let tolerance = planar_tolerance(cutter);
    cutter.bodies.len() == 1
        && cutter.shells.len() == 1
        && cutter.bodies[0].inner_shells.is_empty()
        && cutter
            .faces
            .iter()
            .all(|face| planar_support(&face.surface, tolerance).is_some())
}

/// Why graph-frame clearances could not be measured.
pub(super) enum ClearanceFault {
    /// `u x v` has no direction.
    SingularFrame,
    /// The `(u, v)` Gram matrix is singular.
    SingularProjection,
}

/// Minimum `[uv margin, floor clearance, convex-hull roof clearance]` of the
/// cutter vertices in the canonical graph frame.
pub(super) fn graph_frame_clearances(
    (_, top, origin, u, v, floor_offset): GraphFrame<'_>,
    cutter: &Model,
) -> std::result::Result<[f64; 3], ClearanceFault> {
    let normal = normalize(cross(u, v)).ok_or(ClearanceFault::SingularFrame)?;
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
    if determinant <= SINGULAR_PROJECTION_DETERMINANT {
        return Err(ClearanceFault::SingularProjection);
    }
    let mut bounds = [f64::INFINITY; 3];
    for vertex in &cutter.vertices {
        let rhs = sub(vertex.point, origin);
        let ru = dot(rhs, u);
        let rv = dot(rhs, v);
        let su = (ru * vv - rv * uv) / determinant;
        let sv = (rv * uu - ru * uv) / determinant;
        let height = dot(rhs, normal);
        bounds[0] = bounds[0].min(su).min(1. - su).min(sv).min(1. - sv);
        bounds[1] = bounds[1].min(height - floor_height);
        bounds[2] = bounds[2].min(roof_lower_bound - height);
    }
    Ok(bounds)
}
