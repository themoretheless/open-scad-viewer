use super::*;
use crate::analysis::surface_classify::classify_surface;
use nurbs_core::surface::Surface;

/// Face-deletion strategy. `Auto` uses planar-neighbor → extend and
/// curved/freeform-neighbor → patch; `Cap` is always an explicit request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeleteFaceStrategy {
    Auto,
    Extend,
    Cap,
    Patch,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DeleteFaceOptions {
    pub strategy: DeleteFaceStrategy,
}

impl DeleteFaceOptions {
    pub fn with_strategy(strategy: DeleteFaceStrategy) -> Self {
        Self { strategy }
    }
}

/// Outcome of one successful face deletion. Solid audits apply only to extend
/// and patch; cap is validated as a standalone open sheet, never as a solid.
#[derive(Clone, Debug)]
pub struct DeleteFaceReport {
    pub model: Model,
    pub deleted_face: usize,
    pub strategy: DeleteFaceStrategy,
    pub extended_supports: Option<[usize; 2]>,
    pub audit: Option<SolidAuditCertificate>,
    pub euler_characteristic: i64,
    pub boundary_edge_count: usize,
    /// Maximum sampled edge/surface discrepancy on a planar patch fill.
    pub max_g0_gap_mm: Option<f64>,
}

impl Default for DeleteFaceStrategy {
    fn default() -> Self {
        Self::Auto
    }
}

fn neighbors_of(model: &Model, face: usize, guard: &mut BudgetGuard) -> Result<Vec<usize>> {
    if face >= model.faces.len() {
        return Err(error(
            DELETE_INVALID,
            format!("Face {face} is out of range"),
        ));
    }
    let adjacency = edge_face_adjacency(model, guard)?;
    let mut neighbors = BTreeSet::new();
    for &ring in std::iter::once(&model.faces[face].outer).chain(&model.faces[face].holes) {
        for coedge in &model.loops[ring].coedges {
            let faces = &adjacency[coedge.edge];
            if faces.len() != 2 || !faces.contains(&face) {
                return Err(error(
                    DELETE_UNSUPPORTED,
                    format!(
                        "Edge {} of face {face} is not a manifold two-face boundary",
                        coedge.edge
                    ),
                ));
            }
            neighbors.insert(if faces[0] == face { faces[1] } else { faces[0] });
        }
    }
    if neighbors.is_empty() {
        return Err(error(
            DELETE_INVALID,
            format!("Face {face} has no neighboring faces"),
        ));
    }
    Ok(neighbors.into_iter().collect())
}

/// Deterministic default strategy, exposed so callers can inspect the choice
/// before committing an edit. Cap is never inferred implicitly.
pub fn recommend_delete_face_strategy(
    model: &Model,
    face: usize,
    budget: &Budget,
) -> Result<DeleteFaceStrategy> {
    let mut guard = budget.guard("delete-face-strategy");
    guard.check()?;
    let neighbors = neighbors_of(model, face, &mut guard)?;
    let source_class = classify_surface(
        &model.faces[face].surface,
        model.tolerance_mm.max(1e-7),
        budget,
    )?;
    let mut all_planar = source_class.class == SurfaceClass::Plane;
    for neighbor in neighbors {
        guard.tick()?;
        let class = classify_surface(
            &model.faces[neighbor].surface,
            model.tolerance_mm.max(1e-7),
            budget,
        )?;
        all_planar &= class.class == SurfaceClass::Plane;
    }
    Ok(if all_planar {
        DeleteFaceStrategy::Extend
    } else {
        DeleteFaceStrategy::Patch
    })
}

fn delete_extend(source: &Model, face: usize, guard: &mut BudgetGuard) -> Result<DeleteFaceReport> {
    face_plane_fit(source, face).map_err(|_| {
        error(
            DELETE_UNSUPPORTED,
            format!("Face {face} is not planar; extend needs a planar lid"),
        )
    })?;
    let boundary = &source.faces[face];
    if !boundary.holes.is_empty() || source.loops[boundary.outer].coedges.len() != 4 {
        return Err(error(
            DELETE_UNSUPPORTED,
            "Extend/1 needs one quadrilateral outer loop and no holes",
        ));
    }
    let adjacency = edge_face_adjacency(source, guard)?;
    let neighbors: Vec<usize> = source.loops[boundary.outer]
        .coedges
        .iter()
        .map(|coedge| {
            let faces = &adjacency[coedge.edge];
            if faces.len() != 2 || !faces.contains(&face) {
                return Err(error(
                    DELETE_UNSUPPORTED,
                    format!("Edge {} is not a manifold lid boundary", coedge.edge),
                ));
            }
            Ok(if faces[0] == face { faces[1] } else { faces[0] })
        })
        .collect::<Result<_>>()?;

    // Opposite loop edges are the support candidates. The other opposite pair
    // closes the two prism ends, exactly as in fillet suppression (870).
    for (a, b) in [(0, 2), (1, 3)] {
        guard.tick()?;
        let supports = [neighbors[a], neighbors[b]];
        let p0 = face_plane_fit(source, supports[0])?;
        let p1 = face_plane_fit(source, supports[1])?;
        if intersect_planes(p0, p1, 1e-9).is_none() {
            continue;
        }
        let chain = FilletChain {
            faces: vec![face],
            edges: Vec::new(),
            kind: crate::aag_fillets::FilletKind::ConstantRadius,
            radius_min: 0.,
            radius_max: 0.,
            supports,
        };
        let (staged, _, _) = suppress_fillet_chain(source, &chain, guard).map_err(|e| {
            error(
                DELETE_UNSUPPORTED,
                format!("Extend through supports {supports:?} failed: {}", e.message),
            )
        })?;
        let validation = staged.validate()?;
        let audit = crate::solid_audit::audit_solid(&staged)?;
        let euler_characteristic = staged
            .shell_euler_characteristic(0)
            .ok_or_else(|| error(DELETE_UNSUPPORTED, "Extend result has no outer shell"))?;
        return Ok(DeleteFaceReport {
            model: staged,
            deleted_face: face,
            strategy: DeleteFaceStrategy::Extend,
            extended_supports: Some(supports),
            audit: Some(audit),
            euler_characteristic,
            boundary_edge_count: validation.boundary_edge_count,
            max_g0_gap_mm: None,
        });
    }
    Err(error(
        DELETE_DIVERGENT,
        format!("Face {face}: no opposite neighbor-plane pair intersects to close the deletion"),
    ))
}

fn delete_cap(source: &Model, face: usize, guard: &mut BudgetGuard) -> Result<DeleteFaceReport> {
    if source.bodies.len() != 1
        || source.shells.len() != 1
        || !source.bodies[0].inner_shells.is_empty()
    {
        return Err(error(
            DELETE_UNSUPPORTED,
            "Cap/1 supports one body with one outer shell and no cavities",
        ));
    }
    guard.tick()?;
    let boundary = &source.faces[face];
    let loops: BTreeSet<usize> = std::iter::once(boundary.outer)
        .chain(boundary.holes.iter().copied())
        .collect();
    // The boundary edges and vertices are still used by the retained faces.
    let mut staged = suppression::compact(
        source,
        &BTreeSet::from([face]),
        &loops,
        &BTreeSet::new(),
        &BTreeSet::new(),
    )?;
    staged.shells[0].closed = false;
    staged.bodies.clear();
    staged.1.bodies.clear();
    let validation = staged.validate()?;
    let euler_characteristic = staged
        .shell_euler_characteristic(0)
        .ok_or_else(|| error(DELETE_UNSUPPORTED, "Cap result has no outer shell"))?;
    guard.check()?;
    Ok(DeleteFaceReport {
        model: staged,
        deleted_face: face,
        strategy: DeleteFaceStrategy::Cap,
        extended_supports: None,
        audit: None,
        euler_characteristic,
        boundary_edge_count: validation.boundary_edge_count,
        max_g0_gap_mm: None,
    })
}

fn delete_patch(source: &Model, face: usize, guard: &mut BudgetGuard) -> Result<DeleteFaceReport> {
    let boundary = &source.faces[face];
    let wire = &source.loops[boundary.outer];
    if !boundary.holes.is_empty() || wire.coedges.len() < 3 {
        return Err(error(
            DELETE_UNSUPPORTED,
            "Patch/1 needs one outer loop with at least three edges and no holes",
        ));
    }

    // Fit only the boundary: the source face itself may be freeform. Newell's
    // formula remains deterministic for non-rectangular planar polygons.
    let vertex_ids = loop_vertices(source, boundary.outer)?;
    let points: Vec<[f64; 3]> = vertex_ids
        .iter()
        .map(|&v| source.vertices[v].point)
        .collect();
    let mut newell = [0.; 3];
    let mut origin = [0.; 3];
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
        return Err(error(
            DELETE_UNSUPPORTED,
            "Patch boundary has no stable plane",
        ));
    }
    let normal = mul(newell, 1. / length);
    let tolerance = source.tolerance_mm.max(1e-7) * 8.;
    for coedge in &wire.coedges {
        guard.tick()?;
        let edge = &source.edges[coedge.edge];
        if edge.degenerate || edge.curve.degree != 1 {
            return Err(error(
                DELETE_UNSUPPORTED,
                format!(
                    "Edge {} is not a non-degenerate straight boundary edge",
                    coedge.edge
                ),
            ));
        }
        for point in &edge.curve.control_points {
            if point.len() != 3 {
                return Err(error(DELETE_UNSUPPORTED, "Patch edge curves must be 3D"));
            }
            let p = [point[0], point[1], point[2]];
            if dot(sub(p, origin), normal).abs() > tolerance {
                return Err(error(
                    DELETE_UNSUPPORTED,
                    format!("Edge {} leaves the fitted patch plane", coedge.edge),
                ));
            }
        }
    }

    let (u, v) = super::rims::plane_basis(normal);
    let mut min = [f64::INFINITY; 2];
    let mut max = [f64::NEG_INFINITY; 2];
    for p in &points {
        let d = sub(*p, origin);
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
            DELETE_UNSUPPORTED,
            "Patch boundary has a degenerate UV extent",
        ));
    }
    let patch_origin = add(add(origin, mul(u, min[0])), mul(v, min[1]));
    let du = mul(u, span_u);
    let dv = mul(v, span_v);
    let patch = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![patch_origin.to_vec(), add(patch_origin, dv).to_vec()],
            vec![
                add(patch_origin, du).to_vec(),
                add(add(patch_origin, du), dv).to_vec(),
            ],
        ],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    };
    patch.validate()?;

    // Geometry-only replacement: all topology and persistent ids stay intact.
    let mut staged = source.clone();
    staged.faces[face].surface = patch.clone();
    let mut max_g0_gap_mm = 0f64;
    for (ci, coedge) in wire.coedges.iter().enumerate() {
        guard.tick()?;
        let mut curve = source.edges[coedge.edge].curve.clone();
        if coedge.reversed {
            curve = curve.reverse()?;
        }
        let pcurve = super::rims::curve_to_uv(&curve, patch_origin, u, v, span_u, span_v)?;
        let domain = curve.domain();
        for i in 0..=16 {
            let t = domain[0] + i as f64 / 16. * (domain[1] - domain[0]);
            let uv = pcurve.evaluate(t)?.point;
            let on_patch = patch.evaluate(uv[0], uv[1])?.point;
            let on_edge = curve.evaluate(t)?.point;
            max_g0_gap_mm = max_g0_gap_mm.max(dist(
                [on_patch[0], on_patch[1], on_patch[2]],
                [on_edge[0], on_edge[1], on_edge[2]],
            ));
        }
        let edited = &mut staged.loops[boundary.outer].coedges[ci];
        *edited = brep_topology::Coedge::new(edited.edge, edited.reversed, pcurve);
    }
    if max_g0_gap_mm > tolerance {
        return Err(error(
            DELETE_UNSUPPORTED,
            format!("Patch G0 gap {max_g0_gap_mm} exceeds tolerance {tolerance}"),
        ));
    }
    let validation = staged.validate()?;
    let audit = crate::solid_audit::audit_solid(&staged)?;
    let euler_characteristic = staged
        .shell_euler_characteristic(0)
        .ok_or_else(|| error(DELETE_UNSUPPORTED, "Patch result has no outer shell"))?;
    guard.check()?;
    Ok(DeleteFaceReport {
        model: staged,
        deleted_face: face,
        strategy: DeleteFaceStrategy::Patch,
        extended_supports: None,
        audit: Some(audit),
        euler_characteristic,
        boundary_edge_count: validation.boundary_edge_count,
        max_g0_gap_mm: Some(max_g0_gap_mm),
    })
}

/// Delete one face with the selected sewing strategy. The source is
/// snapshotted and every result is staged beside it; any error discards the
/// staged model, leaving the caller's model unchanged.
pub fn delete_face(
    model: &Model,
    face: usize,
    options: DeleteFaceOptions,
    budget: &Budget,
) -> Result<DeleteFaceReport> {
    let snapshot = ModelSnapshot::new(model.clone())?;
    let source = snapshot.model();
    if face >= source.faces.len() {
        return Err(error(
            DELETE_INVALID,
            format!("Face {face} is out of range"),
        ));
    }
    let mut guard = budget.guard("delete-face");
    guard.check()?;
    let strategy = if options.strategy == DeleteFaceStrategy::Auto {
        recommend_delete_face_strategy(source, face, budget)?
    } else {
        options.strategy
    };
    match strategy {
        DeleteFaceStrategy::Extend => delete_extend(source, face, &mut guard),
        DeleteFaceStrategy::Cap => delete_cap(source, face, &mut guard),
        DeleteFaceStrategy::Patch => delete_patch(source, face, &mut guard),
        DeleteFaceStrategy::Auto => unreachable!("auto strategy resolved above"),
    }
}
