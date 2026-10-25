use super::*;
use crate::analysis::surface_classify::classify_surface;
use crate::watertight::Closure;
use nurbs_core::foundation::min_zone_fit::min_zone_flatness;
use nurbs_core::surface::{Surface, SurfaceSampler};

const FIT_SAMPLES: usize = 9;
const BOUNDARY_WEIGHT: usize = 16;
const WATERTIGHT_CELLS: usize = 10_000;

/// Result of snapping one nearly analytic freeform face to an exact plane.
#[derive(Clone, Debug)]
pub struct SnapReport {
    pub model: Model,
    pub face: usize,
    pub snapped_faces: usize,
    pub analytic_faces_before: usize,
    pub analytic_faces_after: usize,
    /// Maximum sampled distance from the source surface to the boundary-
    /// constrained target plane.
    pub max_deviation_mm: f64,
    /// Boundary discrepancy before replacement. It must be within the model
    /// tolerance so the shared contour is not pulled away from its neighbors.
    pub max_boundary_deviation_mm: f64,
    pub watertight_before: Closure,
    pub watertight_after: Closure,
    pub audit: SolidAuditCertificate,
}

struct BoundaryFit {
    plane: PlaneEq,
    origin: [f64; 3],
    points: Vec<[f64; 3]>,
    max_deviation: f64,
}

fn analytic_face_count(model: &Model, budget: &Budget) -> Result<usize> {
    let mut count = 0;
    for face in &model.faces {
        let class = classify_surface(&face.surface, model.tolerance_mm.max(1e-7), budget)?;
        count += usize::from(class.class != SurfaceClass::Freeform);
    }
    Ok(count)
}

fn sample_grid(surface: &Surface, guard: &mut BudgetGuard) -> Result<Vec<[f64; 3]>> {
    let sampler = SurfaceSampler::new(surface)?;
    let domain_u = [
        surface.knots_u[surface.degree_u],
        surface.knots_u[surface.knots_u.len() - 1 - surface.degree_u],
    ];
    let domain_v = [
        surface.knots_v[surface.degree_v],
        surface.knots_v[surface.knots_v.len() - 1 - surface.degree_v],
    ];
    let mut points = Vec::with_capacity(FIT_SAMPLES * FIT_SAMPLES);
    for i in 0..FIT_SAMPLES {
        for j in 0..FIT_SAMPLES {
            guard.tick()?;
            let u =
                domain_u[0] + (i as f64 + 0.5) / FIT_SAMPLES as f64 * (domain_u[1] - domain_u[0]);
            let v =
                domain_v[0] + (j as f64 + 0.5) / FIT_SAMPLES as f64 * (domain_v[1] - domain_v[0]);
            let point = sampler.evaluate(u, v)?.point;
            points.push([point[0], point[1], point[2]]);
        }
    }
    Ok(points)
}

/// Fit the shared contour independently of the freeform interior. The target
/// is constrained to this plane, rather than allowing an interior-only LS fit
/// to move the contour away from the adjacent faces.
fn boundary_fit(source: &Model, face: usize, guard: &mut BudgetGuard) -> Result<BoundaryFit> {
    let boundary = &source.faces[face];
    let wire = &source.loops[boundary.outer];
    if !boundary.holes.is_empty() || wire.coedges.len() < 3 {
        return Err(error(
            SNAP_UNSUPPORTED,
            "Snap/1 needs one outer straight-edge loop with at least three edges and no holes",
        ));
    }
    let points: Vec<[f64; 3]> = loop_vertices(source, boundary.outer)?
        .into_iter()
        .map(|v| source.vertices[v].point)
        .collect();
    let mut origin = [0.; 3];
    let mut newell = [0.; 3];
    for i in 0..points.len() {
        let p = points[i];
        let q = points[(i + 1) % points.len()];
        origin = add(origin, p);
        newell[0] += (p[1] - q[1]) * (p[2] + q[2]);
        newell[1] += (p[2] - q[2]) * (p[0] + q[0]);
        newell[2] += (p[0] - q[0]) * (p[1] + q[1]);
    }
    origin = mul(origin, 1. / points.len() as f64);
    let length = norm(newell);
    if !length.is_finite() || length <= 1e-12 {
        return Err(error(SNAP_UNSUPPORTED, "Snap boundary has no stable plane"));
    }
    let normal = mul(newell, 1. / length);
    let plane = PlaneEq {
        normal,
        offset: dot(normal, origin),
    };
    let distance_to_plane = |p: [f64; 3]| dot(sub(p, origin), normal).abs();
    let mut max_deviation = points
        .iter()
        .map(|&p| distance_to_plane(p))
        .fold(0., f64::max);
    for coedge in &wire.coedges {
        guard.tick()?;
        let edge = &source.edges[coedge.edge];
        if edge.degenerate || edge.curve.degree != 1 {
            return Err(error(
                SNAP_UNSUPPORTED,
                format!(
                    "Edge {} is not a non-degenerate straight boundary edge",
                    coedge.edge
                ),
            ));
        }
        for point in &edge.curve.control_points {
            if point.len() != 3 {
                return Err(error(SNAP_UNSUPPORTED, "Snap edge curves must be 3D"));
            }
            max_deviation = max_deviation.max(distance_to_plane([point[0], point[1], point[2]]));
        }
    }
    let boundary_tolerance = source.tolerance_mm.max(1e-7);
    if max_deviation > boundary_tolerance {
        return Err(error(
            SNAP_UNSUPPORTED,
            format!(
                "Boundary plane discrepancy {max_deviation} exceeds model tolerance {boundary_tolerance}; constrained snap would move shared edges"
            ),
        ));
    }
    Ok(BoundaryFit {
        plane,
        origin,
        points,
        max_deviation,
    })
}

fn target_patch(boundary: &BoundaryFit) -> Result<Surface> {
    let (u, v) = super::rims::plane_basis(boundary.plane.normal);
    let mut min = [f64::INFINITY; 2];
    let mut max = [f64::NEG_INFINITY; 2];
    for p in &boundary.points {
        let d = sub(*p, boundary.origin);
        let uv = [dot(d, u), dot(d, v)];
        for axis in 0..2 {
            min[axis] = min[axis].min(uv[axis]);
            max[axis] = max[axis].max(uv[axis]);
        }
    }
    let span_u = max[0] - min[0];
    let span_v = max[1] - min[1];
    if !span_u.is_finite() || !span_v.is_finite() || span_u <= 1e-12 || span_v <= 1e-12 {
        return Err(error(
            SNAP_UNSUPPORTED,
            "Snap boundary has a degenerate UV extent",
        ));
    }
    let origin = add(add(boundary.origin, mul(u, min[0])), mul(v, min[1]));
    let du = mul(u, span_u);
    let dv = mul(v, span_v);
    Ok(Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![origin.to_vec(), add(origin, dv).to_vec()],
            vec![add(origin, du).to_vec(), add(add(origin, du), dv).to_vec()],
        ],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    })
}

/// Snap one nearly planar freeform face to an exact bilinear plane. The
/// existing classifier performs the analytic admissibility check; the bounded
/// min-zone fit starts from least squares over a grid with extra boundary
/// weight. The authored target is then constrained to the shared boundary
/// plane, and replacement is delegated to `replace_face` (874).
pub fn snap_face_to_analytic(
    model: &Model,
    face: usize,
    tolerance: f64,
    budget: &Budget,
) -> Result<SnapReport> {
    if !tolerance.is_finite() || tolerance <= 0. {
        return Err(error(
            SNAP_INVALID,
            "Snap tolerance must be finite and positive",
        ));
    }
    let snapshot = ModelSnapshot::new(model.clone())?;
    let source = snapshot.model();
    if face >= source.faces.len() {
        return Err(error(SNAP_INVALID, format!("Face {face} is out of range")));
    }
    let mut guard = budget.guard("snap-face-to-analytic");
    guard.check()?;
    let class = classify_surface(&source.faces[face].surface, tolerance, budget)?;
    match class.class {
        SurfaceClass::Plane => {}
        other => {
            return Err(error(
                SNAP_NO_FIT,
                format!("Face {face} fits {other:?}, not Plane within tolerance {tolerance}"),
            ));
        }
    }

    let boundary = boundary_fit(source, face, &mut guard)?;
    let grid = sample_grid(&source.faces[face].surface, &mut guard)?;
    let mut weighted = grid.clone();
    for &point in &boundary.points {
        weighted.extend(std::iter::repeat_n(point, BOUNDARY_WEIGHT));
    }
    let zone = min_zone_flatness(&weighted).map_err(|e| {
        error(
            SNAP_UNSUPPORTED,
            format!("Boundary-weighted plane fit failed: {}", e.message),
        )
    })?;
    if zone.report.zone_width > 2. * tolerance
        || 1. - dot(zone.plane_normal, boundary.plane.normal).abs() > 0.05
    {
        return Err(error(
            SNAP_NO_FIT,
            format!("Face {face} has no boundary-compatible plane within tolerance {tolerance}"),
        ));
    }
    let max_deviation = grid
        .iter()
        .map(|&p| (dot(boundary.plane.normal, p) - boundary.plane.offset).abs())
        .fold(0., f64::max);
    if max_deviation > tolerance {
        return Err(error(
            SNAP_NO_FIT,
            format!("Face {face} plane deviation {max_deviation} exceeds tolerance {tolerance}"),
        ));
    }
    let target = target_patch(&boundary)?;
    let watertight_before = crate::watertight::inspect(source, WATERTIGHT_CELLS)?;
    if watertight_before.status != Closure::WithinTolerance {
        return Err(error(
            SNAP_UNSUPPORTED,
            format!(
                "Snap requires a watertight source, got {:?}",
                watertight_before.status
            ),
        ));
    }
    let analytic_faces_before = analytic_face_count(source, budget)?;
    let replaced = replace_face(source, face, &target, budget)?;
    let staged = replaced.model;
    let watertight_after = crate::watertight::inspect(&staged, WATERTIGHT_CELLS)?;
    if watertight_after.status != Closure::WithinTolerance {
        return Err(error(
            SNAP_UNSUPPORTED,
            format!(
                "Snap would reduce watertight closure to {:?}",
                watertight_after.status
            ),
        ));
    }
    let analytic_faces_after = analytic_face_count(&staged, budget)?;
    guard.check()?;
    Ok(SnapReport {
        model: staged,
        face,
        snapped_faces: 1,
        analytic_faces_before,
        analytic_faces_after,
        max_deviation_mm: max_deviation,
        max_boundary_deviation_mm: boundary.max_deviation,
        watertight_before: watertight_before.status,
        watertight_after: watertight_after.status,
        audit: replaced.audit,
    })
}
