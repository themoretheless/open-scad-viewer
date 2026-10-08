//! Feature suppression over the AAG (checklist 870): delete a recognized
//! feature (fillet chain, hole, closed pocket) and sew the neighborhood shut.
//!
//! Two honest mechanisms, both behind [`suppress_feature`]:
//!
//! * **Fillet chain** — the two support faces are extended (their underlying
//!   planes) until they intersect; the sharp edge replaces the blend. Caps
//!   closing the chain ends must be planar; the chain is cut out and the
//!   whole planar neighborhood is rebuilt from trimmed polygons, so extending
//!   a support never stretches a B-spline outside its domain — surfaces are
//!   re-authored over the extended region (the parametrization pitfall from
//!   the spec is sidestepped by re-parametrization, not extrapolation).
//!   Plane ∩ plane is a single line, so the multi-branch pitfall does not
//!   arise here; freeform or cone/cylinder supports, whose extensions can
//!   branch, are refused with a typed error rather than guessed.
//! * **Hole / closed pocket** — the feature faces, their loops, edges and
//!   now-orphan vertices are removed. Two rim representations are sewn:
//!   a rim held in a *hole* loop of a neighbor face is simply dropped (no
//!   extension is needed, so curved surroundings like a bore through a tube
//!   wall work); a rim fragmented across the *outer* loops of coplanar
//!   boolean shards is closed by merging the shard component into one
//!   re-authored planar face (seam edges cancel pairwise, boundary edges are
//!   re-chained, pcurves re-mapped under the rigid plane map — exact for
//!   lines and circles). Open pockets, shared rims and shard components
//!   spanning several shells are refused with a typed error.
//!
//! Transaction contour follows `transactions.rs`: the source model is
//! snapshotted, the staged replacement is built beside it, validated
//! (`Model::validate`) and audited ([`crate::solid_audit::audit_solid`]);
//! any failure discards the staged model — the input is never mutated, so a
//! refused suppression is a genuine rollback. Diverging extensions return
//! `BREP_SUPPRESS_DIVERGENT` naming the faces that cannot meet.
//!
//! Documented limits of this iteration: fillet suppression requires planar
//! supports, planar caps, exactly two cap faces, and an all-planar body
//! outside the chain (every non-chain face must be planar); multi-patch
//! chains whose split vertices do not sit on a cap edge are refused.
//! Extension of freeform surfaces and multi-branch intersection selection
//! are future work and fail closed, never silently.

use crate::aag::DihedralClass;
use crate::aag_features::{HoleFeature, PocketFeature, PocketKind};
use crate::analysis::surface_classify::{Axis, SurfaceClass};
use crate::aag_fillets::FilletChain;
use crate::operations::{PlanarBoundary, loop_vertices, model_from_trimmed_polygons};
use crate::solid_audit::SolidAuditCertificate;
use crate::transactions::ModelSnapshot;
use crate::{Error, Model, Result};
use nurbs_core::foundation::guards::{Budget, BudgetGuard};
use std::collections::{BTreeMap, BTreeSet};

/// Error code when feature surroundings fall outside the supported analytic
/// cases (freeform surface, open pocket, rim on an outer loop, ...).
pub const SUPPRESS_UNSUPPORTED: &str = "BREP_SUPPRESS_UNSUPPORTED";
/// Error code when support extensions cannot meet (parallel/diverging).
pub const SUPPRESS_DIVERGENT: &str = "BREP_SUPPRESS_DIVERGENT";
/// Error code when the feature reference itself is malformed.
pub const SUPPRESS_INVALID: &str = "BREP_SUPPRESS_INVALID";

fn error(code: &'static str, message: impl Into<String>) -> Error {
    Error::new(code, message)
}
fn unsupported(message: impl Into<String>) -> Error {
    error(SUPPRESS_UNSUPPORTED, message)
}

/// A recognized feature to suppress, by value (indices into the *input*
/// model). Obtain from [`crate::aag_fillets::find_fillet_chains`],
/// [`crate::aag_features::find_holes`] or [`crate::aag_features::find_pockets`]
/// on the same model; passing a feature recognized on another model is an
/// error, not silent corruption.
#[derive(Clone, Debug, PartialEq)]
pub enum FeatureRef {
    FilletChain(FilletChain),
    Hole(HoleFeature),
    Pocket(PocketFeature),
}

/// Outcome of one successful suppression.
#[derive(Clone, Debug)]
pub struct SuppressReport {
    /// The staged, validated and audited result model.
    pub model: Model,
    /// "fillet-chain" | "hole" | "pocket".
    pub kind: &'static str,
    /// Input-model face indices that were removed.
    pub suppressed_faces: Vec<usize>,
    /// How many input edges disappeared with the feature.
    pub removed_edges: usize,
    /// Sharp edges introduced by extension intersection (0 or 1 today).
    pub new_sharp_edges: usize,
    pub volume_before_mm3: f64,
    pub volume_after_mm3: f64,
    /// Declared analytic estimate of the volume delta (signed: after − before).
    pub delta_volume_estimate_mm3: f64,
    /// Measured volume delta from `mass_properties` (after − before).
    pub delta_volume_actual_mm3: f64,
    /// Post-operation solid audit certificate (acceptance gate).
    pub audit: SolidAuditCertificate,
}

// ---------------------------------------------------------------------------
// Small vector helpers (same convention as operations.rs).
// ---------------------------------------------------------------------------

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn mul(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}
fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    norm(sub(a, b))
}

#[derive(Clone, Copy, Debug)]
struct PlaneEq {
    normal: [f64; 3],
    offset: f64,
}

/// Best-fit plane of a face from its boundary vertices, validating that both
/// the boundary (all loops) and the surface control cage are coplanar. Works
/// for faces with curved boundary edges (e.g. the cap of a rounded cuboid),
/// unlike `operations::face_plane` which requires straight edges.
fn face_plane_fit(model: &Model, face: usize) -> Result<PlaneEq> {
    let f = model
        .faces
        .get(face)
        .ok_or_else(|| error(SUPPRESS_INVALID, format!("Face {face} is out of range")))?;
    let mut points = Vec::new();
    for &ring in std::iter::once(&f.outer).chain(&f.holes) {
        for v in loop_vertices(model, ring)? {
            points.push(model.vertices[v].point);
        }
    }
    if points.len() < 3 {
        return Err(unsupported(format!(
            "Face {face} has fewer than three boundary vertices"
        )));
    }
    let origin = points[0];
    let area = (0..points.len()).fold([0.; 3], |sum, i| {
        // Newell over the outer loop only would miss holes; vertices from
        // holes keep their own winding but contribute little area — safe for
        // a normal estimate, then verified by the flatness check below.
        add(
            sum,
            cross(sub(points[i], origin), sub(points[(i + 1) % points.len()], origin)),
        )
    });
    let length = norm(area);
    if !(length.is_finite() && length > 0.) {
        return Err(unsupported(format!(
            "Face {face} boundary is degenerate (zero area vector)"
        )));
    }
    let normal = mul(area, 1. / length);
    let offset = dot(normal, origin);
    let tolerance = model.tolerance_mm * 8.;
    let on_plane = |p: [f64; 3]| (dot(normal, p) - offset).abs() <= tolerance;
    if !points.iter().copied().all(&on_plane)
        || !f.surface
            .control_points
            .iter()
            .flatten()
            .all(|p| p.len() >= 3 && on_plane([p[0], p[1], p[2]]))
    {
        return Err(unsupported(format!(
            "Face {face} is not planar; only analytic plane extension is supported"
        )));
    }
    Ok(PlaneEq { normal, offset })
}

/// Plane ∩ plane → (point, unit direction). `None` when the planes are
/// parallel within `tolerance` — the extension cannot close (diverging case).
fn intersect_planes(a: PlaneEq, b: PlaneEq, tolerance: f64) -> Option<([f64; 3], [f64; 3])> {
    let d = cross(a.normal, b.normal);
    let len2 = dot(d, d);
    if !len2.is_finite() || len2 <= tolerance * tolerance {
        return None;
    }
    let point = mul(
        add(
            mul(cross(b.normal, d), a.offset),
            mul(cross(d, a.normal), b.offset),
        ),
        1. / len2,
    );
    let dir = mul(d, 1. / len2.sqrt());
    if point.iter().all(|v| v.is_finite()) {
        Some((point, dir))
    } else {
        None
    }
}

/// Line ∩ plane. `None` when the line is parallel to the plane.
fn line_plane(point: [f64; 3], dir: [f64; 3], plane: PlaneEq, tolerance: f64) -> Option<[f64; 3]> {
    let denom = dot(dir, plane.normal);
    if denom.abs() <= tolerance {
        return None;
    }
    let t = (plane.offset - dot(plane.normal, point)) / denom;
    let hit = add(point, mul(dir, t));
    if hit.iter().all(|v| v.is_finite()) {
        Some(hit)
    } else {
        None
    }
}

/// edge → sorted adjacent faces (via both outer and hole loops).
fn edge_face_adjacency(model: &Model, guard: &mut BudgetGuard) -> Result<Vec<Vec<usize>>> {
    let mut adjacency = vec![Vec::new(); model.edges.len()];
    for (face_id, face) in model.faces.iter().enumerate() {
        guard.tick()?;
        for &ring in std::iter::once(&face.outer).chain(&face.holes) {
            for coedge in &model.loops[ring].coedges {
                adjacency[coedge.edge].push(face_id);
            }
        }
    }
    for faces in &mut adjacency {
        faces.sort_unstable();
        faces.dedup();
    }
    Ok(adjacency)
}

fn volume_of(model: &Model) -> Result<f64> {
    Ok(
        crate::analysis::mass_properties(model, 1e-6, 500_000)?
            .signed_volume_mm3,
    )
}

// ---------------------------------------------------------------------------
// Entry point.
// ---------------------------------------------------------------------------

/// Suppress one recognized feature. On any failure the returned error is
/// typed and `model` is unchanged (the staged result is discarded — the
/// transaction contour of `transactions.rs` specialized to suppression).
pub fn suppress_feature(
    model: &Model,
    feature: &FeatureRef,
    budget: &Budget,
) -> Result<SuppressReport> {
    // Snapshot first: admission validates the input and pins the baseline.
    let snapshot = ModelSnapshot::new(model.clone())?;
    let source = snapshot.model();
    let mut guard = budget.guard("suppress-feature");
    guard.check()?;

    let (staged, kind, suppressed_faces, removed_edges, new_sharp_edges, estimate) =
        match feature {
            FeatureRef::FilletChain(chain) => {
                let (staged_model, removed, length) = suppress_fillet_chain(source, chain, &mut guard)?;
                let radius = 0.5 * (chain.radius_min + chain.radius_max);
                // Sharp-corner volume restored by cutting a rolling-ball blend:
                // (1 − π/4) r² per unit length of sharp edge.
                let estimate = (1. - std::f64::consts::FRAC_PI_4) * radius * radius * length;
                (staged_model, "fillet-chain", chain.faces.clone(), removed, 1, estimate)
            }
            FeatureRef::Hole(hole) => {
                let cluster: BTreeSet<usize> = hole
                    .faces
                    .iter()
                    .chain(&hole.step_faces)
                    .chain(&hole.bottom_faces)
                    .copied()
                    .collect();
                let (staged, removed) = suppress_by_deletion(source, &cluster, &mut guard)?;
                // Bore volume: angular fraction of the full cylinder. The
                // matcher reports angle_span with ±0.3 rad tolerance, which
                // would skew the estimate by ~5% for near-full bores; snap
                // to the full circle inside that documented tolerance band.
                let radius = 0.5 * hole.diameter;
                let full = 2. * std::f64::consts::PI;
                let span = if full - hole.angle_span <= 0.35 {
                    full
                } else {
                    hole.angle_span
                };
                let estimate = 0.5 * span * radius * radius * hole.depth;
                let faces: Vec<usize> = cluster.iter().copied().collect();
                (staged, "hole", faces, removed, 0, estimate)
            }
            FeatureRef::Pocket(pocket) => {
                if pocket.kind != PocketKind::Closed {
                    return Err(unsupported(
                        "Only closed pockets are suppressible; an open pocket rim shares the body boundary",
                    ));
                }
                let cluster: BTreeSet<usize> = pocket.faces.iter().copied().collect();
                let (staged, removed) = suppress_by_deletion(source, &cluster, &mut guard)?;
                let estimate = pocket.floor_area * pocket.depth;
                (staged, "pocket", pocket.faces.clone(), removed, 0, estimate)
            }
        };

    // Validate → audit → measure. Any failure here discards `staged`; the
    // snapshot (and the caller's model) is the rollback state.
    staged.validate()?;
    let audit = crate::solid_audit::audit_solid(&staged)?;
    let volume_before = volume_of(source)?;
    let volume_after = volume_of(&staged)?;
    guard.check()?;
    Ok(SuppressReport {
        model: staged,
        kind,
        suppressed_faces,
        removed_edges,
        new_sharp_edges,
        volume_before_mm3: volume_before,
        volume_after_mm3: volume_after,
        delta_volume_estimate_mm3: estimate,
        delta_volume_actual_mm3: volume_after - volume_before,
        audit,
    })
}

// ---------------------------------------------------------------------------
// Fillet-chain suppression: extend supports to their intersection.
// ---------------------------------------------------------------------------

/// Returns the rebuilt model, the number of removed edges and the length of
/// the new sharp edge.
fn suppress_fillet_chain(
    model: &Model,
    chain: &FilletChain,
    guard: &mut BudgetGuard,
) -> Result<(Model, usize, f64)> {
    let tolerance = model.tolerance_mm;
    let chain_set: BTreeSet<usize> = chain.faces.iter().copied().collect();
    if chain_set.is_empty() {
        return Err(error(SUPPRESS_INVALID, "Fillet chain has no faces"));
    }
    for &face in &chain.faces {
        if face >= model.faces.len() {
            return Err(error(
                SUPPRESS_INVALID,
                format!("Fillet chain face {face} is out of range"),
            ));
        }
    }
    let [s0, s1] = chain.supports;
    if s0 == s1 || chain_set.contains(&s0) || chain_set.contains(&s1) {
        return Err(error(
            SUPPRESS_INVALID,
            format!("Fillet chain supports [{s0}, {s1}] are degenerate"),
        ));
    }

    // 1. Support extensions must meet. This is checked before any topology
    //    work so the diverging case fails fast with the culpable faces named.
    let plane0 = face_plane_fit(model, s0)?;
    let plane1 = face_plane_fit(model, s1)?;
    let Some((line_point, line_dir)) = intersect_planes(plane0, plane1, 1e-9) else {
        return Err(error(
            SUPPRESS_DIVERGENT,
            format!(
                "Support faces {s0} and {s1} do not intersect: parallel or diverging extensions cannot close the gap left by the fillet chain"
            ),
        ));
    };

    // 2. Chain boundary structure: contact edges (against a support) and cap
    //    edges (against a face closing a chain end).
    let adjacency = edge_face_adjacency(model, guard)?;
    let mut contact_edges: BTreeSet<usize> = BTreeSet::new();
    let mut cap_edges: BTreeMap<usize, Vec<usize>> = BTreeMap::new(); // cap face -> edges
    let mut removed_edges: BTreeSet<usize> = BTreeSet::new();
    for (edge, faces) in adjacency.iter().enumerate() {
        guard.tick()?;
        let inside = faces.iter().filter(|f| chain_set.contains(f)).count();
        if inside == 0 {
            continue;
        }
        removed_edges.insert(edge);
        if inside == faces.len() {
            continue; // internal edge between chain faces
        }
        if inside != 1 || faces.len() != 2 {
            return Err(unsupported(format!(
                "Edge {edge} borders the fillet chain non-manifoldly; chain boundary must be manifold"
            )));
        }
        let outside = *faces
            .iter()
            .find(|f| !chain_set.contains(f))
            .expect("one outside face");
        if outside == s0 || outside == s1 {
            contact_edges.insert(edge);
        } else {
            cap_edges.entry(outside).or_default().push(edge);
        }
    }
    if contact_edges.is_empty() {
        return Err(error(
            SUPPRESS_INVALID,
            "Fillet chain is not adjacent to its declared supports",
        ));
    }
    // Iteration 1: exactly two cap faces closing the two chain ends.
    if cap_edges.len() != 2 {
        return Err(unsupported(format!(
            "Fillet chain has {} cap faces; exactly two planar caps are supported (ring fillets and forked ends are refused)",
            cap_edges.len()
        )));
    }

    // 3. Corner points: sharp line ∩ each cap plane. Plane ∩ plane gave one
    //    line and line ∩ plane one point — the branch nearest to the original
    //    neighborhood is the only branch, so no ambiguity needs resolving.
    let mut corner_of_cap = BTreeMap::new();
    for (&cap, _) in &cap_edges {
        guard.tick()?;
        let plane = face_plane_fit(model, cap)?;
        let Some(corner) = line_plane(line_point, line_dir, plane, 1e-9) else {
            return Err(error(
                SUPPRESS_DIVERGENT,
                format!(
                    "Cap face {cap} is parallel to the support intersection line; extensions do not close"
                ),
            ));
        };
        corner_of_cap.insert(cap, corner);
    }
    let corners: Vec<[f64; 3]> = corner_of_cap.values().copied().collect();
    let sharp_length = dist(corners[0], corners[1]);
    if !(sharp_length.is_finite() && sharp_length > tolerance * 8.) {
        return Err(unsupported(
            "Support extensions meet the caps in coincident points; the sharp edge would be degenerate",
        ));
    }

    // 4. Vertex substitution: every vertex incident to a removed edge must
    //    sit on exactly one cap (its substitute is that cap's corner point).
    let mut substitute: BTreeMap<usize, [f64; 3]> = BTreeMap::new();
    for &edge in &removed_edges {
        guard.tick()?;
        for &vertex in &model.edges[edge].vertices {
            if substitute.contains_key(&vertex) {
                continue;
            }
            let mut caps_of_vertex: BTreeSet<usize> = BTreeSet::new();
            for (other, faces) in adjacency.iter().enumerate() {
                if !cap_edges.values().flatten().any(|e| *e == other) {
                    continue;
                }
                if model.edges[other].vertices.contains(&vertex) {
                    let cap = *faces
                        .iter()
                        .find(|f| !chain_set.contains(f))
                        .expect("cap edge has an outside face");
                    caps_of_vertex.insert(cap);
                }
            }
            if caps_of_vertex.len() != 1 {
                return Err(unsupported(format!(
                    "Vertex {vertex} of the fillet neighborhood touches {} caps; chain-split interior vertices are refused in this iteration",
                    caps_of_vertex.len()
                )));
            }
            substitute.insert(vertex, corner_of_cap[caps_of_vertex.iter().next().expect("one cap")]);
        }
    }

    // 5. Every surviving face of the model must be planar — the neighborhood
    //    is rebuilt from trimmed polygons, which re-authors the extended
    //    supports instead of extrapolating their B-splines off-domain.
    for (face, _) in model.faces.iter().enumerate() {
        guard.tick()?;
        if !chain_set.contains(&face) {
            face_plane_fit(model, face)?;
        }
    }

    // 6. Rebuild: polygons of all surviving faces with substitutions.
    let mut polygons = Vec::with_capacity(model.faces.len() - chain_set.len());
    for shell in &model.shells {
        guard.tick()?;
        for usage in &shell.faces {
            if chain_set.contains(&usage.face) {
                continue;
            }
            let face = &model.faces[usage.face];
            let rings = |ring_id: usize, is_hole: bool| -> Result<Vec<[f64; 3]>> {
                let ids = loop_vertices(model, ring_id)?;
                if is_hole && ids.iter().any(|v| substitute.contains_key(v)) {
                    return Err(unsupported(
                        "Fillet chain meets a hole loop; only outer-loop neighborhoods are supported",
                    ));
                }
                let mut points: Vec<[f64; 3]> = ids
                    .iter()
                    .map(|&v| substitute.get(&v).copied().unwrap_or(model.vertices[v].point))
                    .collect();
                // Collapse consecutive duplicates (both arc endpoints map to
                // the same sharp corner), including across the wrap.
                points.dedup_by(|a, b| dist(*a, *b) <= tolerance * 4.);
                while points.len() > 1
                    && dist(points[0], points[points.len() - 1]) <= tolerance * 4.
                {
                    points.pop();
                }
                if points.len() < 3 {
                    return Err(unsupported(
                        "Suppression consumed a neighboring face; topology mutation is out of scope",
                    ));
                }
                if usage.reversed {
                    points.reverse();
                }
                Ok(points)
            };
            let outer = rings(face.outer, false)?;
            let mut holes = Vec::with_capacity(face.holes.len());
            for &ring in &face.holes {
                holes.push(rings(ring, true)?);
            }
            polygons.push(PlanarBoundary { outer, holes });
        }
    }
    let mut staged = model_from_trimmed_polygons(polygons, tolerance)?;
    staged.refresh_change_set(&[model]);
    staged.validate()?;
    Ok((staged, removed_edges.len(), sharp_length))
}

// ---------------------------------------------------------------------------
// Hole / pocket suppression: pure subgraph deletion, no extension needed.
// ---------------------------------------------------------------------------

/// Remove `cluster` faces, their loops and edges, and drop the rim
/// hole-loops on neighboring faces. Returns the compacted model and the
/// number of removed edges.
fn suppress_by_deletion(
    model: &Model,
    cluster: &BTreeSet<usize>,
    guard: &mut BudgetGuard,
) -> Result<(Model, usize)> {
    if cluster.is_empty() {
        return Err(error(SUPPRESS_INVALID, "Feature has no faces"));
    }
    for &face in cluster {
        if face >= model.faces.len() {
            return Err(error(
                SUPPRESS_INVALID,
                format!("Feature face {face} is out of range"),
            ));
        }
    }
    let adjacency = edge_face_adjacency(model, guard)?;

    // Rim edges: exactly one adjacent face inside the cluster, one outside.
    let mut removed_edges: BTreeSet<usize> = BTreeSet::new();
    let mut drop_loops: BTreeSet<usize> = BTreeSet::new();
    let mut outer_rim_faces: BTreeSet<usize> = BTreeSet::new();
    for (edge, faces) in adjacency.iter().enumerate() {
        guard.tick()?;
        let inside = faces.iter().filter(|f| cluster.contains(f)).count();
        if inside == 0 {
            continue;
        }
        removed_edges.insert(edge);
        if inside == faces.len() {
            continue;
        }
        if inside != 1 || faces.len() != 2 {
            return Err(unsupported(format!(
                "Edge {edge} borders the feature non-manifoldly; rim must be manifold"
            )));
        }
        let outside = *faces
            .iter()
            .find(|f| !cluster.contains(f))
            .expect("one outside face");
        // The outside face references this edge either through a hole loop
        // entirely owned by the feature (dropping it removes the opening) or
        // through its outer loop — the fragmented-mouth case, handled below
        // by merging the coplanar shard component into one face.
        let face = &model.faces[outside];
        let Some(ring) = std::iter::once(face.outer)
            .chain(face.holes.iter().copied())
            .find(|&ring| model.loops[ring].coedges.iter().any(|c| c.edge == edge))
        else {
            return Err(unsupported(format!(
                "Rim edge {edge} is not referenced by face {outside}"
            )));
        };
        if ring == face.outer {
            outer_rim_faces.insert(outside);
            continue;
        }
        if !model.loops[ring]
            .coedges
            .iter()
            .all(|c| {
                adjacency[c.edge]
                    .iter()
                    .any(|f| cluster.contains(f))
            })
        {
            return Err(unsupported(format!(
                "Rim loop of face {outside} is shared with geometry outside the feature; shared rims are refused"
            )));
        }
        drop_loops.insert(ring);
    }

    if !outer_rim_faces.is_empty() {
        return merge_coplanar_rims(
            model,
            cluster,
            &adjacency,
            &removed_edges,
            &outer_rim_faces,
            guard,
        );
    }

    // Removed loops: every loop of a cluster face plus the dropped rims.
    let mut removed_loops: BTreeSet<usize> = drop_loops;
    for &face in cluster {
        let face = &model.faces[face];
        removed_loops.insert(face.outer);
        removed_loops.extend(face.holes.iter().copied());
    }
    // Removed vertices: endpoints of removed edges not used by a kept edge.
    let mut used_vertices: BTreeSet<usize> = BTreeSet::new();
    for (edge, e) in model.edges.iter().enumerate() {
        if !removed_edges.contains(&edge) {
            used_vertices.extend(e.vertices);
        }
    }
    let removed_vertices: BTreeSet<usize> = removed_edges
        .iter()
        .flat_map(|&edge| model.edges[edge].vertices)
        .filter(|v| !used_vertices.contains(v))
        .collect();

    let staged = compact(
        model,
        cluster,
        &removed_loops,
        &removed_edges,
        &removed_vertices,
    )?;
    staged.validate()?;
    Ok((staged, removed_edges.len()))
}

/// Rebuild the model without the given entities, remapping every reference
/// and preserving the persistent ids of all kept entities.
fn compact(
    model: &Model,
    remove_faces: &BTreeSet<usize>,
    remove_loops: &BTreeSet<usize>,
    remove_edges: &BTreeSet<usize>,
    remove_vertices: &BTreeSet<usize>,
) -> Result<Model> {
    fn remap(count: usize, removed: &BTreeSet<usize>) -> BTreeMap<usize, usize> {
        let mut map = BTreeMap::new();
        let mut next = 0;
        for old in 0..count {
            if !removed.contains(&old) {
                map.insert(old, next);
                next += 1;
            }
        }
        map
    }
    let vertices = remap(model.vertices.len(), remove_vertices);
    let edges = remap(model.edges.len(), remove_edges);
    let loops = remap(model.loops.len(), remove_loops);
    let faces = remap(model.faces.len(), remove_faces);

    let mut out = Model(
        brep_topology::Model {
            vertices: model
                .vertices
                .iter()
                .enumerate()
                .filter(|(i, _)| !remove_vertices.contains(i))
                .map(|(_, v)| v.clone())
                .collect(),
            edges: model
                .edges
                .iter()
                .enumerate()
                .filter(|(i, _)| !remove_edges.contains(i))
                .map(|(_, e)| brep_topology::Edge {
                    degenerate: e.degenerate,
                    vertices: e.vertices.map(|v| vertices[&v]),
                    curve: e.curve.clone(),
                })
                .collect(),
            loops: model
                .loops
                .iter()
                .enumerate()
                .filter(|(i, _)| !remove_loops.contains(i))
                .map(|(_, l)| brep_topology::Loop {
                    coedges: l
                        .coedges
                        .iter()
                        .map(|c| brep_topology::Coedge {
                            edge: edges[&c.edge],
                            reversed: c.reversed,
                            pcurve: c.pcurve.clone(),
                        })
                        .collect(),
                })
                .collect(),
            faces: model
                .faces
                .iter()
                .enumerate()
                .filter(|(i, _)| !remove_faces.contains(i))
                .map(|(_, f)| brep_topology::Face {
                    surface: f.surface.clone(),
                    outer: loops[&f.outer],
                    holes: f
                        .holes
                        .iter()
                        .filter(|ring| !remove_loops.contains(ring))
                        .map(|ring| loops[ring])
                        .collect(),
                })
                .collect(),
            shells: Vec::new(),
            bodies: Vec::new(),
            tolerance_mm: model.tolerance_mm,
        },
        crate::TopologyIds {
            vertices: kept(&model.1.vertices, remove_vertices),
            edges: kept(&model.1.edges, remove_edges),
            loops: kept(&model.1.loops, remove_loops),
            faces: kept(&model.1.faces, remove_faces),
            shells: Vec::new(),
            bodies: Vec::new(),
            lineage: model.1.lineage.clone(),
            change_set: Default::default(),
        },
    );

    // Shells: drop removed faces; a shell that loses every face disappears.
    let mut shell_map = BTreeMap::new();
    for (old, shell) in model.shells.iter().enumerate() {
        let kept_uses: Vec<_> = shell
            .faces
            .iter()
            .filter(|u| !remove_faces.contains(&u.face))
            .map(|u| brep_topology::FaceUse {
                face: faces[&u.face],
                reversed: u.reversed,
            })
            .collect();
        if kept_uses.is_empty() {
            continue;
        }
        let new = out.shells.len();
        shell_map.insert(old, new);
        out.1.shells.push(model.1.shells[old]);
        out.shells.push(brep_topology::Shell {
            faces: kept_uses,
            closed: shell.closed,
        });
    }
    for (old, body) in model.bodies.iter().enumerate() {
        let Some(&outer_shell) = shell_map.get(&body.outer_shell) else {
            return Err(unsupported(format!(
                "Suppression consumed the outer shell of body {old}"
            )));
        };
        let inner_shells = body
            .inner_shells
            .iter()
            .filter_map(|s| shell_map.get(s).copied())
            .collect();
        out.1.bodies.push(model.1.bodies[old]);
        out.bodies.push(brep_topology::Body {
            outer_shell,
            inner_shells,
        });
    }
    out.refresh_change_set(&[model]);
    Ok(out)
}

fn kept<T: Clone>(ids: &[T], removed: &BTreeSet<usize>) -> Vec<T> {
    ids.iter()
        .enumerate()
        .filter(|(i, _)| !removed.contains(i))
        .map(|(_, id)| id.clone())
        .collect()
}

// ---------------------------------------------------------------------------
// Coplanar-shard merge: rim edges landing on *outer* loops.
//
// Boolean output commonly fragments a planar mouth region into shards whose
// outer loops carry one rim edge each. Suppression then merges the connected
// coplanar shard component into a single re-authored planar face: internal
// seam edges cancel pairwise, rim edges drop out, and the remaining boundary
// edges are re-chained into rings. The face surface is re-authored over the
// merged region (no off-domain extrapolation), and pcurves of reused edges
// are re-mapped into the new UV frame, following each edge's 3D curve under
// the rigid plane map (exact for lines and circles alike).
// ---------------------------------------------------------------------------

/// One directed edge use while chaining a ring: traversal orientation and
/// start vertex, already normalized for the face use's `reversed` flag.
#[derive(Clone, Copy, Debug)]
struct RingUse {
    edge: usize,
    reversed: bool,
    start: usize,
}

/// Is `face` used with `reversed = true` by any shell? (A face belongs to
/// exactly one shell use in every model this kernel authors.)
fn face_use_reversed(model: &Model, face: usize) -> bool {
    model
        .shells
        .iter()
        .flat_map(|s| s.faces.iter())
        .find(|u| u.face == face)
        .is_some_and(|u| u.reversed)
}

/// Two planes are the same geometric plane within `tolerance`, sign-aligned.
fn same_plane(a: PlaneEq, b: PlaneEq, tolerance: f64) -> bool {
    let align = dot(a.normal, b.normal);
    if align.abs() < 1. - 1e-9 {
        return false;
    }
    let signed_offset = if align > 0. { b.offset } else { -b.offset };
    (a.offset - signed_offset).abs() <= tolerance
}

/// Connected component of surviving faces coplanar with the seed face,
/// linked through shared edges.
fn coplanar_component(
    model: &Model,
    seed: usize,
    plane: PlaneEq,
    cluster: &BTreeSet<usize>,
    adjacency: &[Vec<usize>],
    guard: &mut BudgetGuard,
) -> Result<BTreeSet<usize>> {
    let tolerance = model.tolerance_mm * 8.;
    let mut component = BTreeSet::from([seed]);
    let mut stack = vec![seed];
    while let Some(face) = stack.pop() {
        guard.tick()?;
        for &ring in std::iter::once(&model.faces[face].outer).chain(&model.faces[face].holes) {
            for coedge in &model.loops[ring].coedges {
                for &neighbor in &adjacency[coedge.edge] {
                    if neighbor == face
                        || cluster.contains(&neighbor)
                        || component.contains(&neighbor)
                    {
                        continue;
                    }
                    // Non-planar neighbors are simply not coplanar shards.
                    let Ok(neighbor_plane) = face_plane_fit(model, neighbor) else {
                        continue;
                    };
                    if same_plane(plane, neighbor_plane, tolerance) {
                        component.insert(neighbor);
                        stack.push(neighbor);
                    }
                }
            }
        }
    }
    Ok(component)
}

/// Orthonormal in-plane basis for a unit normal (same construction as
/// `operations::plane_basis`).
fn plane_basis(normal: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    let axis = if normal[0].abs() <= normal[1].abs() && normal[0].abs() <= normal[2].abs() {
        [1., 0., 0.]
    } else if normal[1].abs() <= normal[2].abs() {
        [0., 1., 0.]
    } else {
        [0., 0., 1.]
    };
    let u = {
        let c = cross(normal, axis);
        let n = norm(c);
        mul(c, 1. / n)
    };
    (u, cross(normal, u))
}

/// Rigid-map a 3D curve into the 2D UV frame of a planar patch:
/// `uv(p) = (dot(p − origin, u) / len_u, dot(p − origin, v) / len_v)`.
/// Exact for any NURBS: the map is affine, so weights and knots carry over.
fn curve_to_uv(
    curve: &nurbs_core::curve::Curve,
    origin: [f64; 3],
    u: [f64; 3],
    v: [f64; 3],
    len_u: f64,
    len_v: f64,
) -> Result<nurbs_core::curve::Curve> {
    let mut mapped = curve.clone();
    for point in &mut mapped.control_points {
        if point.len() != 3 {
            return Err(unsupported("Edge curves must be 3D for pcurve mapping"));
        }
        let d = sub([point[0], point[1], point[2]], origin);
        *point = vec![dot(d, u) / len_u, dot(d, v) / len_v];
    }
    mapped.validate()?;
    Ok(mapped)
}

/// Merge path of [`suppress_by_deletion`]: every rim edge of the cluster
/// that sits on an *outer* loop belongs to a coplanar shard component; each
/// component is merged into one re-authored planar face with the mouth
/// filled. Returns the rebuilt model and the number of removed edges.
fn merge_coplanar_rims(
    model: &Model,
    cluster: &BTreeSet<usize>,
    adjacency: &[Vec<usize>],
    rim_edges: &BTreeSet<usize>,
    outer_rim_faces: &BTreeSet<usize>,
    guard: &mut BudgetGuard,
) -> Result<(Model, usize)> {
    let tolerance = model.tolerance_mm;

    // 1. Components: one per connected coplanar shard region carrying rim
    //    edges on outer loops.
    let mut components: Vec<BTreeSet<usize>> = Vec::new();
    let mut assigned: BTreeSet<usize> = BTreeSet::new();
    for &seed in outer_rim_faces {
        guard.tick()?;
        if assigned.contains(&seed) {
            continue;
        }
        let plane = face_plane_fit(model, seed)?;
        let component = coplanar_component(model, seed, plane, cluster, adjacency, guard)?;
        // Every shard of the component must be planar (checked inside) and
        // the component must swallow every outer-rim face it touches.
        for &face in &component {
            assigned.insert(face);
        }
        components.push(component);
    }

    // 2. Per component: cancel internal seams, drop rim edges, re-chain the
    //    remaining boundary into rings, build the merged face.
    struct MergedFace {
        surface: nurbs_core::surface::Surface,
        /// (coedges in traversal order) — outer first, holes after.
        rings: Vec<Vec<RingUse>>,
        /// Shell that owned the shards; the merged face joins it.
        shell: usize,
        /// Faces replaced by this merge.
        replaced: BTreeSet<usize>,
        /// Seam edges internal to the component (removed).
        seams: BTreeSet<usize>,
    }
    let mut merged = Vec::new();
    for component in &components {
        guard.tick()?;
        // Collect directed outer-loop uses, rim edges skipped.
        let mut uses: Vec<RingUse> = Vec::new();
        let mut kept_hole_rings: Vec<Vec<RingUse>> = Vec::new();
        let mut shell_of: Option<usize> = None;
        for &face in component {
            guard.tick()?;
            let flipped = face_use_reversed(model, face);
            let (shell, _) = model
                .shells
                .iter()
                .enumerate()
                .find_map(|(s, sh)| {
                    sh.faces
                        .iter()
                        .any(|u| u.face == face)
                        .then_some((s, ()))
                })
                .ok_or_else(|| {
                    error(SUPPRESS_INVALID, format!("Face {face} is not used by any shell"))
                })?;
            match shell_of {
                None => shell_of = Some(shell),
                Some(previous) if previous == shell => {}
                Some(_) => {
                    return Err(unsupported(
                        "Coplanar shard component spans several shells; merge is refused",
                    ));
                }
            }
            let record = |ring: usize, out: &mut Vec<RingUse>| {
                let wire = &model.loops[ring];
                let iter: Vec<RingUse> = wire
                    .coedges
                    .iter()
                    .filter(|c| !rim_edges.contains(&c.edge))
                    .map(|c| {
                        let reversed = c.reversed ^ flipped;
                        RingUse {
                            edge: c.edge,
                            reversed,
                            start: model.edges[c.edge].vertices[usize::from(reversed)],
                        }
                    })
                    .collect();
                out.extend(iter);
            };
            // Hole loops: fully rim → dropped with the feature; partially
            // rim → shared mouth, refused; untouched → kept as a hole ring.
            for &hole in &model.faces[face].holes {
                let rim_count = model.loops[hole]
                    .coedges
                    .iter()
                    .filter(|c| rim_edges.contains(&c.edge))
                    .count();
                if rim_count == model.loops[hole].coedges.len() {
                    continue;
                }
                if rim_count > 0 {
                    return Err(unsupported(
                        "A hole loop of a shard face is partly feature rim; shared rims are refused",
                    ));
                }
                let mut ring_uses = Vec::new();
                record(hole, &mut ring_uses);
                kept_hole_rings.push(ring_uses);
            }
            record(model.faces[face].outer, &mut uses);
        }
        let Some(shell) = shell_of else {
            return Err(error(SUPPRESS_INVALID, "Empty shard component"));
        };

        // Seam cancellation: an edge used twice inside the component is
        // internal (once per adjacent shard, opposite directions).
        let mut counts: BTreeMap<usize, usize> = BTreeMap::new();
        for use_ in &uses {
            *counts.entry(use_.edge).or_insert(0) += 1;
        }
        let mut seams = BTreeSet::new();
        let mut boundary: Vec<RingUse> = Vec::new();
        for use_ in uses {
            match counts[&use_.edge] {
                1 => boundary.push(use_),
                2 => {
                    seams.insert(use_.edge);
                }
                _ => {
                    return Err(unsupported(format!(
                        "Edge {} is used more than twice inside a shard component",
                        use_.edge
                    )));
                }
            }
        }

        // Chain boundary uses into closed rings by vertex connectivity.
        let mut outgoing: BTreeMap<usize, usize> = BTreeMap::new();
        for (i, use_) in boundary.iter().enumerate() {
            if outgoing.insert(use_.start, i).is_some() {
                return Err(unsupported(
                    "Shard boundary branches at a vertex; region is not simply mergeable",
                ));
            }
        }
        let end_of = |use_: RingUse| model.edges[use_.edge].vertices[usize::from(!use_.reversed)];
        let mut rings: Vec<Vec<RingUse>> = Vec::new();
        let mut done = vec![false; boundary.len()];
        for start_i in 0..boundary.len() {
            guard.tick()?;
            if done[start_i] {
                continue;
            }
            let mut ring = Vec::new();
            let mut i = start_i;
            loop {
                done[i] = true;
                let use_ = boundary[i];
                ring.push(use_);
                let next = end_of(use_);
                if next == boundary[start_i].start {
                    break;
                }
                i = *outgoing.get(&next).ok_or_else(|| {
                    unsupported("Shard boundary does not close after rim removal")
                })?;
                if done[i] {
                    return Err(unsupported("Shard boundary self-intersects"));
                }
                if ring.len() > boundary.len() {
                    return Err(unsupported("Shard boundary chaining diverged"));
                }
            }
            rings.push(ring);
        }

        // Classify rings by signed area along the component normal: the one
        // largest positive ring is the outer boundary; negative ones are
        // holes. Two positive rings would mean a disconnected region.
        let seed_plane = face_plane_fit(model, *component.iter().next().expect("non-empty"))?;
        let area_of = |ring: &[RingUse]| -> f64 {
            let points: Vec<[f64; 3]> = ring
                .iter()
                .map(|use_| model.vertices[use_.start].point)
                .collect();
            let n = points.len();
            let area = (0..n).fold([0.; 3], |sum, i| {
                add(sum, cross(points[i], points[(i + 1) % n]))
            });
            0.5 * dot(area, seed_plane.normal)
        };
        let mut outers: Vec<usize> = Vec::new();
        let mut holes: Vec<usize> = Vec::new();
        for (i, ring) in rings.iter().enumerate() {
            let area = area_of(ring);
            if area > tolerance * tolerance {
                outers.push(i);
            } else if area < -tolerance * tolerance {
                holes.push(i);
            } else {
                return Err(unsupported("Shard merge produced a zero-area ring"));
            }
        }
        if outers.len() != 1 {
            return Err(unsupported(
                "Shard component merges into several disconnected regions; refused",
            ));
        }
        let ordered_rings: Vec<Vec<RingUse>> = std::iter::once(outers[0])
            .chain(holes)
            .map(|i| rings[i].clone())
            .collect();

        // Re-author the merged face: bilinear patch over the UV bbox of all
        // ring points; pcurves re-mapped from the 3D edge curves.
        let outer_points: Vec<[f64; 3]> = ordered_rings[0]
            .iter()
            .map(|use_| model.vertices[use_.start].point)
            .collect();
        let n = outer_points.len();
        let area_vec = (0..n).fold([0.; 3], |sum, i| {
            add(sum, cross(outer_points[i], outer_points[(i + 1) % n]))
        });
        let length = norm(area_vec);
        if !(length.is_finite() && length > 0.) {
            return Err(unsupported("Merged shard face is degenerate"));
        }
        let normal = mul(area_vec, 1. / length);
        if dot(normal, seed_plane.normal) < 0. {
            return Err(unsupported(
                "Merged shard ring wound against the shard plane normal",
            ));
        }
        let (u, v) = plane_basis(normal);
        let origin = outer_points[0];
        let all_points: Vec<[f64; 3]> = ordered_rings
            .iter()
            .flat_map(|ring| ring.iter().map(|use_| model.vertices[use_.start].point))
            .collect();
        let uv_of = |p: [f64; 3]| {
            let d = sub(p, origin);
            [dot(d, u), dot(d, v)]
        };
        let uvs: Vec<[f64; 2]> = all_points.iter().map(|&p| uv_of(p)).collect();
        let min_u = uvs.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
        let max_u = uvs.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max);
        let min_v = uvs.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min);
        let max_v = uvs.iter().map(|p| p[1]).fold(f64::NEG_INFINITY, f64::max);
        if max_u - min_u <= tolerance || max_v - min_v <= tolerance {
            return Err(unsupported("Merged shard face collapsed"));
        }
        let surface_origin = add(origin, add(mul(u, min_u), mul(v, min_v)));
        let du = mul(u, max_u - min_u);
        let dv = mul(v, max_v - min_v);
        let surface = nurbs_core::surface::Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![surface_origin.to_vec(), add(surface_origin, dv).to_vec()],
                vec![
                    add(surface_origin, du).to_vec(),
                    add(add(surface_origin, du), dv).to_vec(),
                ],
            ],
            weights: vec![vec![1., 1.], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        };
        merged.push(MergedFace {
            surface,
            rings: ordered_rings,
            shell,
            replaced: component.clone(),
            seams,
        });
    }

    // 3. Assemble the staged model: cluster + shards removed, seams removed,
    //    merged faces appended, their loops appended, everything remapped.
    let mut remove_faces: BTreeSet<usize> = cluster.clone();
    let mut remove_edges: BTreeSet<usize> = rim_edges.clone();
    let mut remove_loops: BTreeSet<usize> = BTreeSet::new();
    for &face in cluster {
        let face = &model.faces[face];
        remove_loops.insert(face.outer);
        remove_loops.extend(face.holes.iter().copied());
    }
    for merged_face in &merged {
        for &face in &merged_face.replaced {
            remove_faces.insert(face);
            let face = &model.faces[face];
            remove_loops.insert(face.outer);
            remove_loops.extend(face.holes.iter().copied());
        }
        remove_edges.extend(merged_face.seams.iter().copied());
    }
    let mut used_vertices: BTreeSet<usize> = BTreeSet::new();
    for (edge, e) in model.edges.iter().enumerate() {
        if !remove_edges.contains(&edge) {
            used_vertices.extend(e.vertices);
        }
    }
    let remove_vertices: BTreeSet<usize> = remove_edges
        .iter()
        .flat_map(|&edge| model.edges[edge].vertices)
        .filter(|v| !used_vertices.contains(v))
        .collect();

    fn remap(count: usize, removed: &BTreeSet<usize>) -> BTreeMap<usize, usize> {
        let mut map = BTreeMap::new();
        let mut next = 0;
        for old in 0..count {
            if !removed.contains(&old) {
                map.insert(old, next);
                next += 1;
            }
        }
        map
    }
    let edges = remap(model.edges.len(), &remove_edges);

    // Old shell index → new shell index (compact drops emptied shells).
    let mut shell_remap: BTreeMap<usize, usize> = BTreeMap::new();
    {
        let mut next = 0;
        for (old, shell) in model.shells.iter().enumerate() {
            if shell.faces.iter().any(|u| !remove_faces.contains(&u.face)) {
                shell_remap.insert(old, next);
                next += 1;
            }
        }
    }

    let mut out = compact(
        model,
        &remove_faces,
        &remove_loops,
        &remove_edges,
        &remove_vertices,
    )?;

    // Append merged loops and faces; hook faces into their shells.
    for merged_face in &merged {
        guard.tick()?;
        let shell = *shell_remap.get(&merged_face.shell).ok_or_else(|| {
            unsupported("Shard shell vanished during suppression")
        })?;
        let mut loop_ids = Vec::new();
        // UV frame data for pcurve mapping, recomputed from the surface.
        let origin_corner: [f64; 3] = {
            let p = &merged_face.surface.control_points[0][0];
            [p[0], p[1], p[2]]
        };
        let du_cp = &merged_face.surface.control_points[1][0];
        let dv_cp = &merged_face.surface.control_points[0][1];
        let u_vec = sub(
            [du_cp[0], du_cp[1], du_cp[2]],
            origin_corner,
        );
        let v_vec = sub(
            [dv_cp[0], dv_cp[1], dv_cp[2]],
            origin_corner,
        );
        let len_u = norm(u_vec);
        let len_v = norm(v_vec);
        let u = mul(u_vec, 1. / len_u);
        let v = mul(v_vec, 1. / len_v);
        for ring in &merged_face.rings {
            let mut coedges = Vec::with_capacity(ring.len());
            for use_ in ring {
                let mapped = {
                    let mut curve = model.edges[use_.edge].curve.clone();
                    if use_.reversed {
                        curve = curve.reverse()?;
                    }
                    curve_to_uv(&curve, origin_corner, u, v, len_u, len_v)?
                };
                coedges.push(brep_topology::Coedge {
                    edge: edges[&use_.edge],
                    reversed: use_.reversed,
                    pcurve: mapped,
                });
            }
            loop_ids.push(out.loops.len());
            out.loops.push(brep_topology::Loop { coedges });
        }
        let face_id = out.faces.len();
        out.faces.push(brep_topology::Face {
            surface: merged_face.surface.clone(),
            outer: loop_ids[0],
            holes: loop_ids[1..].to_vec(),
        });
        // Shard uses were dropped by `compact` (shard faces are in
        // `remove_faces`); the shell gains the merged face instead.
        out.shells[shell].faces.push(brep_topology::FaceUse {
            face: face_id,
            reversed: false,
        });
    }
    out.rebuild_topology_ids();
    out.refresh_change_set(&[model]);
    out.validate()?;
    Ok((out, remove_edges.len()))
}

// ---------------------------------------------------------------------------
// Direct modeling: move face (checklist 871).
//
// Iteration 1 supports moving a *planar* face of a *planar polygonal* body
// along its outward normal. The neighborhood is rebuilt from substituted
// polygons (the same re-authoring strategy as suppression — neighbor faces
// are stretched or trimmed by moving their shared vertices, never by
// extrapolating B-splines off-domain). Neighbors are classified over the
// AAG: coplanar shards are *driven* (they move with the face as one rigid
// region), perpendicular ones are *absorbing* (they bound the move and are
// stretched/trimmed). Two topology mutations are handled honestly:
//
// * **Absorption** — a neighbor shard collapses to zero area (a thin wall's
//   top strip, a shrunk step): the face is dropped and the region re-sews by
//   vertex welding in the rebuild, leaving no zombie faces.
// * **Annihilation** — the moved plane lands exactly on an opposite parallel
//   face with an identical boundary (the far side of a consumed thin wall):
//   both faces cancel. Partial overlap or crossing past a parallel face is
//   refused with a typed error — conservative, never a guess.
//
// Curved faces, curved boundary edges and off-normal moves are outside this
// iteration and fail with `BREP_MOVE_UNSUPPORTED` naming the reason; the
// input model is never mutated (transaction contour as in suppression).
// Persistent ids of untouched faces are restored by geometric correspondence
// after the rebuild (see [`restore_unchanged_ids`]).
// ---------------------------------------------------------------------------

/// Error code when the move falls outside the supported planar subset.
pub const MOVE_UNSUPPORTED: &str = "BREP_MOVE_UNSUPPORTED";
/// Error code when the request itself is malformed.
pub const MOVE_INVALID: &str = "BREP_MOVE_INVALID";

/// Outcome of one successful face move.
#[derive(Clone, Debug)]
pub struct MoveFaceReport {
    /// The staged, validated and audited result model.
    pub model: Model,
    /// Input-model index of the moved face (owning-body local when the model
    /// holds several bodies — the edit is isolated via `body_edit`).
    pub moved_face: usize,
    /// Coplanar shard faces that moved together with the selected face.
    pub driven_faces: Vec<usize>,
    /// Perpendicular neighbors that bounded the move (AAG classification).
    pub absorbing_faces: Vec<usize>,
    /// Faces that collapsed to zero area and were absorbed by the re-sew.
    pub absorbed_faces: Vec<usize>,
    /// Faces cancelled in annihilating pairs (moved shard + far side of a
    /// consumed thin wall), both input indices per pair flattened.
    pub annihilated_faces: Vec<usize>,
    /// Signed displacement along the outward normal (mm).
    pub signed_distance_mm: f64,
    pub volume_before_mm3: f64,
    pub volume_after_mm3: f64,
    /// Declared analytic estimate: signed distance × driven region area.
    pub delta_volume_estimate_mm3: f64,
    /// Measured delta from `mass_properties` (after − before).
    pub delta_volume_actual_mm3: f64,
    /// Post-operation solid audit certificate (acceptance gate).
    pub audit: SolidAuditCertificate,
}

/// Move `face` by `direction * distance`. Iteration 1: the face must be
/// planar and `direction` must be parallel to its outward normal; the body
/// must be fully planar with straight edges. On any failure the model is
/// unchanged (the staged rebuild is discarded).
pub fn move_face(
    model: &Model,
    face: usize,
    direction: [f64; 3],
    distance: f64,
    budget: &Budget,
) -> Result<MoveFaceReport> {
    let snapshot = ModelSnapshot::new(model.clone())?;
    let source = snapshot.model();
    if face >= source.faces.len() {
        return Err(error(
            MOVE_INVALID,
            format!("Face {face} is out of range"),
        ));
    }
    for (i, v) in direction.iter().enumerate() {
        if !v.is_finite() {
            return Err(error(MOVE_INVALID, format!("direction[{i}] is not finite")));
        }
    }
    if !(distance.is_finite() && distance.abs() <= 1e6) {
        return Err(error(
            MOVE_INVALID,
            "Distance must be finite and within ±1000000 mm",
        ));
    }
    let length = norm(direction);
    if length <= 0. {
        return Err(error(MOVE_INVALID, "Direction must be nonzero"));
    }
    let unit_dir = mul(direction, 1. / length);

    let mut guard = budget.guard("move-face");
    guard.check()?;

    // Multi-body models: isolate the owning body, edit, reassemble without
    // touching other bodies' identities (body_edit contour).
    if source.bodies.len() > 1 {
        let mut staged_report = None;
        let edited = crate::body_edit::edit_face(source, face, |part, local| {
            let report = move_face_local(part, local, unit_dir, distance, budget, &mut guard)?;
            staged_report = Some(report);
            Ok(staged_report.as_ref().expect("report").model.clone())
        })?;
        let mut report = staged_report.expect("closure ran");
        // Re-audit and re-measure on the reassembled model; face indices in
        // the report are owning-body local (documented on the struct).
        edited.validate()?;
        let audit = crate::solid_audit::audit_solid(&edited)?;
        let volume_before = volume_of(source)?;
        let volume_after = volume_of(&edited)?;
        report.model = edited;
        report.audit = audit;
        report.volume_before_mm3 = volume_before;
        report.volume_after_mm3 = volume_after;
        report.delta_volume_actual_mm3 = volume_after - volume_before;
        return Ok(report);
    }
    move_face_local(source, face, unit_dir, distance, budget, &mut guard)
}

/// Core of [`move_face`] on one connected body.
fn move_face_local(
    model: &Model,
    face: usize,
    unit_dir: [f64; 3],
    distance: f64,
    budget: &Budget,
    guard: &mut BudgetGuard,
) -> Result<MoveFaceReport> {
    let tolerance = model.tolerance_mm;
    let linear = tolerance * 8.;
    let delta = mul(unit_dir, distance);

    // Iteration 1 admits planar polygonal bodies: every face planar, every
    // edge straight. Curved surroundings fail closed with the face named.
    for (i, f) in model.faces.iter().enumerate() {
        guard.tick()?;
        face_plane_fit(model, i).map_err(|_| {
            error(
                MOVE_UNSUPPORTED,
                format!("Face {i} is not planar; move-face/1 admits planar bodies only"),
            )
        })?;
        for &ring in std::iter::once(&f.outer).chain(&f.holes) {
            if model.loops[ring]
                .coedges
                .iter()
                .any(|c| model.edges[c.edge].curve.degree != 1)
            {
                return Err(error(
                    MOVE_UNSUPPORTED,
                    format!(
                        "Face {i} has curved boundary edges; move-face/1 admits straight edges only"
                    ),
                ));
            }
        }
    }

    // Canonical (outward-oriented) polygon per face.
    let polygons = canonical_polygons(model, &BTreeMap::new(), guard)?;
    let plane = polygons[face].plane;
    let align = dot(unit_dir, plane.normal);
    if align.abs() < 1. - 1e-9 {
        return Err(error(
            MOVE_UNSUPPORTED,
            "Off-normal moves of a planar face are outside move-face/1",
        ));
    }

    // Neighbor classification over the AAG (864/865 attributes): coplanar
    // shards are driven with the face; perpendicular neighbors bound the
    // move (absorbing); anything else is reported as absorbing too, since
    // oblique planar neighbors are still stretched by the rebuild.
    let mut aag = crate::aag::Aag::build(model, budget)?;
    aag.attach_face_attrs(model, budget)?;
    let adjacency = edge_face_adjacency(model, guard)?;
    let driven = coplanar_component(model, face, plane, &BTreeSet::new(), &adjacency, guard)?;
    let mut absorbing = Vec::new();
    for aag_edge in &aag.edges {
        guard.tick()?;
        let uses: Vec<usize> = aag_edge.uses.iter().map(|u| u.face).collect();
        if uses.contains(&face) {
            for &other in &uses {
                if other != face
                    && !driven.contains(&other)
                    && !absorbing.contains(&other)
                {
                    absorbing.push(other);
                }
            }
        }
    }

    // Moved vertices: every vertex touched by a driven face.
    let mut moved_vertices: BTreeSet<usize> = BTreeSet::new();
    for &d in &driven {
        for &ring in std::iter::once(&model.faces[d].outer).chain(&model.faces[d].holes) {
            guard.tick()?;
            for v in loop_vertices(model, ring)? {
                moved_vertices.insert(v);
            }
        }
    }
    let substitute: BTreeMap<usize, [f64; 3]> = moved_vertices
        .iter()
        .map(|&v| (v, add(model.vertices[v].point, delta)))
        .collect();

    // Declared ΔV estimate: signed distance along the outward normal times
    // the driven region's area (translation of a rigid planar region).
    let moved_area: f64 = driven
        .iter()
        .map(|&d| 0.5 * norm(polygons[d].area_vec))
        .sum();
    let signed_distance = dot(delta, plane.normal);

    let moved = canonical_polygons(model, &substitute, guard)?;

    // Absorption: a non-driven face whose polygon collapsed. Flipping: a
    // face whose orientation inverted (the move crossed its geometry) —
    // refused, partial overruns are out of scope.
    let mut absorbed: BTreeSet<usize> = BTreeSet::new();
    for (i, poly) in moved.iter().enumerate() {
        guard.tick()?;
        if driven.contains(&i) {
            continue;
        }
        let area_after = norm(poly.area_vec);
        let area_before = norm(polygons[i].area_vec);
        if area_after <= linear * linear {
            absorbed.insert(i);
            continue;
        }
        if area_before > linear * linear && dot(poly.area_vec, polygons[i].area_vec) < 0. {
            return Err(error(
                MOVE_UNSUPPORTED,
                format!(
                    "Move inverts face {i}; crossing geometry partially is refused (suppression handles full removal)"
                ),
            ));
        }
    }

    // Parallel-face interactions: annihilation when the moved plane lands on
    // an opposite face with an identical boundary; crossing when it passes
    // through. Both are detected on the canonical planes.
    let mut annihilated: BTreeSet<usize> = BTreeSet::new();
    for &d in &driven {
        guard.tick()?;
        if absorbed.contains(&d) {
            continue;
        }
        for (i, poly) in moved.iter().enumerate() {
            guard.tick()?;
            if driven.contains(&i) || absorbed.contains(&i) || annihilated.contains(&i) {
                continue;
            }
            let n2 = polygons[i].plane.normal;
            let parallel = dot(plane.normal, n2).abs();
            if parallel < 1. - 1e-9 {
                continue;
            }
            let o1 = dot(plane.normal, polygons[d].plane_point());
            let o2 = dot(plane.normal, poly.plane_point());
            let gap_before = o2 - o1;
            let gap_after = gap_before - dot(delta, plane.normal);
            if gap_before.abs() <= linear {
                continue; // already coplanar (driven shards): not an interaction
            }
            if gap_after.abs() <= linear {
                // Coincident after the move: exact annihilation or refusal.
                let opposite = dot(plane.normal, n2) < 0.;
                if opposite
                    && same_point_set(&moved[d].boundary.outer, &poly.boundary.outer, linear)
                {
                    annihilated.insert(d);
                    annihilated.insert(i);
                } else {
                    return Err(error(
                        MOVE_UNSUPPORTED,
                        format!(
                            "Move lands on face {i} with a different boundary; partial annihilation is refused"
                        ),
                    ));
                }
            } else if gap_before.signum() != gap_after.signum() {
                return Err(error(
                    MOVE_UNSUPPORTED,
                    format!(
                        "Move crosses face {i}; the limit is the absorbing neighbor, shorten the distance"
                    ),
                ));
            }
        }
    }

    let mut dropped: BTreeSet<usize> = absorbed.clone();
    dropped.extend(annihilated.iter().copied());
    if dropped.contains(&face) && annihilated.contains(&face) {
        // The selected face annihilated with its counterpart: legal (thin
        // wall consumed end to end).
    }
    let kept: Vec<PlanarBoundary> = moved
        .iter()
        .enumerate()
        .filter(|(i, _)| !dropped.contains(i))
        .map(|(_, p)| p.boundary.clone())
        .collect();
    if kept.len() < 4 {
        return Err(error(
            MOVE_UNSUPPORTED,
            "Move consumes the whole body; refused instead of returning an empty solid",
        ));
    }
    let mut staged = model_from_trimmed_polygons(kept, tolerance)
        .map_err(|e| Error::new(MOVE_UNSUPPORTED, format!("Move rebuild failed: {}", e.message)))?;
    restore_unchanged_ids(&mut staged, model, guard)?;
    staged.refresh_change_set(&[model]);
    staged.validate()?;
    let audit = crate::solid_audit::audit_solid(&staged)?;
    let volume_before = volume_of(model)?;
    let volume_after = volume_of(&staged)?;
    guard.check()?;
    Ok(MoveFaceReport {
        model: staged,
        moved_face: face,
        driven_faces: driven.iter().copied().filter(|&d| d != face).collect(),
        absorbing_faces: absorbing,
        absorbed_faces: absorbed.iter().copied().collect(),
        annihilated_faces: annihilated.iter().copied().collect(),
        signed_distance_mm: signed_distance,
        volume_before_mm3: volume_before,
        volume_after_mm3: volume_after,
        delta_volume_estimate_mm3: signed_distance * moved_area,
        delta_volume_actual_mm3: volume_after - volume_before,
        audit,
    })
}

/// One face's canonical (outward-oriented) polygonal boundary.
struct CanonicalPolygon {
    boundary: PlanarBoundary,
    /// Signed area vector of the outer ring (canonical orientation).
    area_vec: [f64; 3],
    /// Canonical outward plane of the face.
    plane: PlaneEq,
}

impl CanonicalPolygon {
    fn plane_point(&self) -> [f64; 3] {
        // Any boundary point lies on the plane; the first outer vertex is
        // guaranteed to exist (validated rings have ≥ 3 points).
        self.boundary.outer[0]
    }
}

/// Boundary of every face in canonical orientation (reversed face uses
/// flipped), with optional vertex substitution. Faces are indexed as in
/// `model.faces`; shell order is irrelevant.
fn canonical_polygons(
    model: &Model,
    substitute: &BTreeMap<usize, [f64; 3]>,
    guard: &mut BudgetGuard,
) -> Result<Vec<CanonicalPolygon>> {
    let mut reversed_of = vec![false; model.faces.len()];
    for shell in &model.shells {
        for usage in &shell.faces {
            reversed_of[usage.face] = usage.reversed;
        }
    }
    let mut out = Vec::with_capacity(model.faces.len());
    for (face_id, face) in model.faces.iter().enumerate() {
        guard.tick()?;
        let rings = |ring_id: usize| -> Result<Vec<[f64; 3]>> {
            let mut points: Vec<[f64; 3]> = loop_vertices(model, ring_id)?
                .iter()
                .map(|&v| substitute.get(&v).copied().unwrap_or(model.vertices[v].point))
                .collect();
            // Substitution can collapse a ring segment (a shard shrinking to
            // a triangle): weld consecutive duplicates, including the wrap.
            let tol = model.tolerance_mm * 4.;
            points.dedup_by(|a, b| dist(*a, *b) <= tol);
            while points.len() > 1
                && dist(points[0], points[points.len() - 1]) <= tol
            {
                points.pop();
            }
            if reversed_of[face_id] {
                points.reverse();
            }
            Ok(points)
        };
        let outer = rings(face.outer)?;
        let mut holes = Vec::with_capacity(face.holes.len());
        for &ring in &face.holes {
            holes.push(rings(ring)?);
        }
        // Rings collapsed below 3 points by substitution are legal: they are
        // zero-area and get absorbed by the caller, never rebuilt.
        let n = outer.len();
        let area_vec = (0..n).fold([0.; 3], |sum, i| {
            add(sum, cross(outer[i], outer[(i + 1) % n]))
        });
        let length = norm(area_vec);
        // Degenerate (zero-area) rings are legal here: move_face detects and
        // absorbs them. They carry a zero normal placeholder, never used for
        // geometry decisions (absorbed faces are skipped before that).
        let normal = if length.is_finite() && length > 0. {
            mul(area_vec, 1. / length)
        } else {
            [0., 0., 0.]
        };
        let plane_point = outer[0];
        out.push(CanonicalPolygon {
            boundary: PlanarBoundary { outer, holes },
            area_vec,
            plane: PlaneEq {
                normal,
                offset: dot(normal, plane_point),
            },
        });
    }
    Ok(out)
}

/// Do two rings cover the same point set (order-free, within `tolerance`)?
fn same_point_set(a: &[[f64; 3]], b: &[[f64; 3]], tolerance: f64) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut used = vec![false; b.len()];
    a.iter().all(|p| {
        b.iter()
            .enumerate()
            .position(|(i, q)| !used[i] && dist(*p, *q) <= tolerance)
            .map(|i| used[i] = true)
            .is_some()
    })
}

/// Restore persistent ids of geometrically untouched entities after a full
/// polygon rebuild: vertices match by exact point, edges by endpoint pair +
/// identical curve, loops by identical sorted edge sets, faces by identical
/// (outer, holes) loop sets, shells/bodies by identical face sets. Ambiguous
/// or changed entities keep their fresh rebuild ids. This is the 871 naming
/// criterion: ids of faces the move never touched survive the rebuild.
fn restore_unchanged_ids(staged: &mut Model, source: &Model, guard: &mut BudgetGuard) -> Result<()> {
    // Vertices: exact point equality, unique in both directions.
    let mut source_by_point: BTreeMap<[u64; 3], Vec<usize>> = BTreeMap::new();
    for (i, v) in source.vertices.iter().enumerate() {
        source_by_point.entry(v.point.map(|x| x.to_bits())).or_default().push(i);
    }
    let mut staged_by_point: BTreeMap<[u64; 3], Vec<usize>> = BTreeMap::new();
    for (i, v) in staged.vertices.iter().enumerate() {
        staged_by_point.entry(v.point.map(|x| x.to_bits())).or_default().push(i);
    }
    let mut vertex_map: Vec<Option<usize>> = vec![None; staged.vertices.len()];
    for (key, staged_ids) in &staged_by_point {
        if staged_ids.len() != 1 {
            continue;
        }
        if let Some(source_ids) = source_by_point.get(key) {
            if source_ids.len() == 1 {
                vertex_map[staged_ids[0]] = Some(source_ids[0]);
            }
        }
    }

    // Edges: mapped endpoint pair plus bit-identical curve (rebuilt straight
    // edges are deterministic given the same endpoints).
    let mut edge_map: Vec<Option<usize>> = vec![None; staged.edges.len()];
    for (i, edge) in staged.edges.iter().enumerate() {
        guard.tick()?;
        let (Some(a), Some(b)) = (
            vertex_map[edge.vertices[0]],
            vertex_map[edge.vertices[1]],
        ) else {
            continue;
        };
        let key = [a.min(b), a.max(b)];
        let matches: Vec<usize> = source
            .edges
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                let k = [e.vertices[0].min(e.vertices[1]), e.vertices[0].max(e.vertices[1])];
                k == key && e.curve == edge.curve && e.degenerate == edge.degenerate
            })
            .map(|(j, _)| j)
            .collect();
        if matches.len() == 1 {
            edge_map[i] = Some(matches[0]);
        }
    }

    // Loops: identical sorted mapped edge sets.
    let loop_key = |wire: &brep_topology::Loop<nurbs_core::curve::Curve>,
                    map: &Vec<Option<usize>>|
     -> Option<Vec<usize>> {
        let mut edges: Vec<usize> = wire
            .coedges
            .iter()
            .map(|c| map[c.edge])
            .collect::<Option<Vec<_>>>()?;
        edges.sort_unstable();
        Some(edges)
    };
    let source_loop_keys: BTreeMap<Vec<usize>, Vec<usize>> = {
        let mut map: BTreeMap<Vec<usize>, Vec<usize>> = BTreeMap::new();
        let identity: Vec<Option<usize>> = (0..source.edges.len()).map(Some).collect();
        for (j, wire) in source.loops.iter().enumerate() {
            if let Some(key) = loop_key(wire, &identity) {
                map.entry(key).or_default().push(j);
            }
        }
        map
    };
    let mut loop_map: Vec<Option<usize>> = vec![None; staged.loops.len()];
    for (i, wire) in staged.loops.iter().enumerate() {
        guard.tick()?;
        if let Some(key) = loop_key(wire, &edge_map) {
            if let Some(matches) = source_loop_keys.get(&key) {
                if matches.len() == 1 {
                    loop_map[i] = Some(matches[0]);
                }
            }
        }
    }

    // Faces: identical outer + hole loops (all matched).
    let face_key = |face: &brep_topology::Face<nurbs_core::surface::Surface>,
                    map: &Vec<Option<usize>>|
     -> Option<(usize, Vec<usize>)> {
        let outer = map[face.outer]?;
        let mut holes: Vec<usize> = face.holes.iter().map(|&l| map[l]).collect::<Option<_>>()?;
        holes.sort_unstable();
        Some((outer, holes))
    };
    let identity_loops: Vec<Option<usize>> = (0..source.loops.len()).map(Some).collect();
    let mut source_face_keys: BTreeMap<(usize, Vec<usize>), Vec<usize>> = BTreeMap::new();
    for (j, f) in source.faces.iter().enumerate() {
        if let Some(key) = face_key(f, &identity_loops) {
            source_face_keys.entry(key).or_default().push(j);
        }
    }
    let mut face_map: Vec<Option<usize>> = vec![None; staged.faces.len()];
    for (i, f) in staged.faces.iter().enumerate() {
        guard.tick()?;
        if let Some(key) = face_key(f, &loop_map) {
            if let Some(matches) = source_face_keys.get(&key) {
                if matches.len() == 1 {
                    face_map[i] = Some(matches[0]);
                }
            }
        }
    }

    // Shells and bodies: identical mapped face / shell sets.
    let mut shell_map: Vec<Option<usize>> = vec![None; staged.shells.len()];
    for (i, shell) in staged.shells.iter().enumerate() {
        let Some(mut faces) = shell
            .faces
            .iter()
            .map(|u| face_map[u.face])
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        faces.sort_unstable();
        let matches: Vec<usize> = source
            .shells
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                let mut f: Vec<usize> = s.faces.iter().map(|u| u.face).collect();
                f.sort_unstable();
                f == faces
            })
            .map(|(j, _)| j)
            .collect();
        if matches.len() == 1 {
            shell_map[i] = Some(matches[0]);
        }
    }

    for (i, mapped) in vertex_map.iter().enumerate() {
        if let Some(j) = mapped {
            staged.1.vertices[i] = source.1.vertices[*j];
        }
    }
    for (i, mapped) in edge_map.iter().enumerate() {
        if let Some(j) = mapped {
            staged.1.edges[i] = source.1.edges[*j];
        }
    }
    for (i, mapped) in loop_map.iter().enumerate() {
        if let Some(j) = mapped {
            staged.1.loops[i] = source.1.loops[*j];
        }
    }
    for (i, mapped) in face_map.iter().enumerate() {
        if let Some(j) = mapped {
            staged.1.faces[i] = source.1.faces[*j];
        }
    }
    for (i, mapped) in shell_map.iter().enumerate() {
        if let Some(j) = mapped {
            staged.1.shells[i] = source.1.shells[*j];
        }
    }
    let mut body_ids: Vec<Option<usize>> = vec![None; staged.bodies.len()];
    for (i, body) in staged.bodies.iter().enumerate() {
        let Some(outer) = shell_map.get(body.outer_shell).copied().flatten() else {
            continue;
        };
        let Some(mut inners) = body
            .inner_shells
            .iter()
            .map(|&s| shell_map[s])
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        inners.sort_unstable();
        let matches: Vec<usize> = source
            .bodies
            .iter()
            .enumerate()
            .filter(|(_, b)| {
                let mut s: Vec<usize> = b.inner_shells.clone();
                s.sort_unstable();
                b.outer_shell == outer && s == inners
            })
            .map(|(j, _)| j)
            .collect();
        if matches.len() == 1 {
            body_ids[i] = Some(matches[0]);
        }
    }
    for (i, mapped) in body_ids.iter().enumerate() {
        if let Some(j) = mapped {
            staged.1.bodies[i] = source.1.bodies[*j];
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Direct modeling: offset face (checklist 872).
//
// Iteration 1 introduces the first *curved* support in direct_edit:
// cylindrical faces. Offsetting a cylinder = re-authoring its surface at a
// new radius on the *same* axis: control points scale radially (exact for
// NURBS — weights and knots carry over), rim circles are re-authored at the
// new radius, straight seam edges are rebuilt from their moved endpoints,
// and planar caps keep their surfaces while their pcurves are re-mapped
// under the cap's rigid plane frame ([`curve_to_uv`], exact for lines and
// circles). Cylinder UV parameterization is angular, so radial scaling
// leaves the cylinder's own pcurves valid.
//
// Concentric constraint: coaxial cylinder clusters (e.g. the two walls of a
// shell/tube) must offset as a PAIR with the same signed delta, otherwise
// the wall thickness silently breaks. Clusters are detected from 865 face
// attributes (coaxial axes within tolerance) *before* any mutation. A single
// offset request on one member of a pair auto-expands to the whole pair by
// default (reported via `auto_expanded`); [`OffsetOptions::strict`] refuses
// with a diagnosis naming the partner faces.
//
// Blend guard: a tangent (Smooth/Tangent) neighbor outside the offset
// groups — a fixed-radius fillet rolling on the cylinder — would tear,
// because the radius change moves its contact lines. Refit of such blends
// is out of scope: the operation fails with `BREP_OFFSET_UNSUPPORTED`
// naming the blend face. Cones, tori, spheres and freeform faces are
// refused likewise. Planar faces delegate to [`move_face`] (offset along
// the outward normal), keeping one consistent door for direct edits.
//
// Same contours as 870/871: snapshot → staged clone mutation → validate →
// audit → measure; any failure discards the staged model and the input is
// bit-for-bit unchanged. Topology and persistent ids are untouched by the
// mutation (geometry-only edit), which the tests assert.
// ---------------------------------------------------------------------------

/// Error code when the offset falls outside the supported subset.
pub const OFFSET_UNSUPPORTED: &str = "BREP_OFFSET_UNSUPPORTED";
/// Error code when the request itself is malformed.
pub const OFFSET_INVALID: &str = "BREP_OFFSET_INVALID";

/// Options for [`offset_face`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OffsetOptions {
    /// Refuse (with a diagnosis naming the concentric partner) instead of
    /// auto-expanding a single-face offset to the whole concentric pair.
    pub strict: bool,
}

/// Outcome of one successful face offset.
#[derive(Clone, Debug)]
pub struct OffsetFaceReport {
    /// The staged, validated and audited result model.
    pub model: Model,
    /// Input-model index of the requested face.
    pub face: usize,
    /// "cylinder-radius" | "planar" (delegated to [`move_face`]).
    pub kind: &'static str,
    /// Every face whose radius changed: the requested face's coaxial sibling
    /// patches plus the concentric partner's, ascending.
    pub offset_faces: Vec<usize>,
    /// Distinct radii before the offset (ascending), one per coaxial group.
    pub old_radii: Vec<f64>,
    /// Matching radii after the offset.
    pub new_radii: Vec<f64>,
    /// The offset was auto-expanded to a concentric partner.
    pub auto_expanded: bool,
    /// Wall thickness (partner radius gap) before / after, when a pair was
    /// involved. Pair offsets preserve it exactly.
    pub wall_thickness_before_mm: Option<f64>,
    pub wall_thickness_after_mm: Option<f64>,
    /// Edges re-authored at the new radius / moved endpoints.
    pub recomputed_edges: usize,
    /// Cap pcurves re-mapped onto the unchanged planar surfaces.
    pub recomputed_pcurves: usize,
    pub volume_before_mm3: f64,
    pub volume_after_mm3: f64,
    /// Declared analytic estimate: π h Σ ±(r'² − r²) per coaxial group.
    pub delta_volume_estimate_mm3: f64,
    /// Measured delta from `mass_properties` (after − before).
    pub delta_volume_actual_mm3: f64,
    /// Post-operation solid audit certificate (acceptance gate).
    pub audit: SolidAuditCertificate,
}

/// Radial scale of `point` about `axis` from `old_r` to `new_r`: the axial
/// component is kept, the radial component scales by `new_r / old_r`.
fn radial_scale(axis: &Axis, old_r: f64, new_r: f64, point: [f64; 3]) -> [f64; 3] {
    let d = sub(point, axis.point);
    let axial = dot(d, axis.direction);
    let radial = sub(d, mul(axis.direction, axial));
    add(
        axis.point,
        add(mul(axis.direction, axial), mul(radial, new_r / old_r)),
    )
}

/// Radial distance of `point` to `axis`.
fn radial_distance(axis: &Axis, point: [f64; 3]) -> f64 {
    let d = sub(point, axis.point);
    let axial = dot(d, axis.direction);
    norm(sub(d, mul(axis.direction, axial)))
}

/// Two axis lines coincide within tolerances (direction parallel up to sign,
/// point of B on the line of A) — the 865 coaxiality criterion.
fn coaxial_axes(a: &Axis, b: &Axis, linear_tol: f64) -> bool {
    let align = dot(a.direction, b.direction).abs();
    if 1. - align > 1e-6 {
        return false;
    }
    let delta = sub(b.point, a.point);
    let off = sub(delta, mul(a.direction, dot(delta, a.direction)));
    norm(off) <= linear_tol
}

/// Offset `face` radially (cylinders) or along its normal (planes, via the
/// move-face path) by `offset_mm`. See the module section for the supported
/// subset; anything else fails with [`OFFSET_UNSUPPORTED`] and the input is
/// unchanged.
pub fn offset_face(
    model: &Model,
    face: usize,
    offset_mm: f64,
    options: OffsetOptions,
    budget: &Budget,
) -> Result<OffsetFaceReport> {
    let snapshot = ModelSnapshot::new(model.clone())?;
    let source = snapshot.model();
    if face >= source.faces.len() {
        return Err(error(OFFSET_INVALID, format!("Face {face} is out of range")));
    }
    if !(offset_mm.is_finite() && offset_mm.abs() <= 1e6) {
        return Err(error(
            OFFSET_INVALID,
            "Offset must be finite and within ±1000000 mm",
        ));
    }
    let mut guard = budget.guard("offset-face");
    guard.check()?;

    // Classify through the attributed AAG (864/865 attributes).
    let mut aag = crate::aag::Aag::build(source, budget)?;
    aag.attach_face_attrs(source, budget)?;
    let attrs = aag.nodes[face].attrs.as_ref().ok_or_else(|| {
        error(OFFSET_INVALID, format!("Face {face} carries no fitted attributes"))
    })?;
    match attrs.class {
        SurfaceClass::Plane => {
            // Offset of a planar face = move along the outward normal.
            let raw = face_plane_fit(source, face)?;
            let sign = if face_use_reversed(source, face) { -1. } else { 1. };
            let outward = mul(raw.normal, sign);
            let moved = move_face(source, face, outward, offset_mm, budget)?;
            return Ok(OffsetFaceReport {
                model: moved.model,
                face,
                kind: "planar",
                offset_faces: vec![],
                old_radii: vec![],
                new_radii: vec![],
                auto_expanded: false,
                wall_thickness_before_mm: None,
                wall_thickness_after_mm: None,
                recomputed_edges: 0,
                recomputed_pcurves: 0,
                volume_before_mm3: moved.volume_before_mm3,
                volume_after_mm3: moved.volume_after_mm3,
                delta_volume_estimate_mm3: moved.delta_volume_estimate_mm3,
                delta_volume_actual_mm3: moved.delta_volume_actual_mm3,
                audit: moved.audit,
            });
        }
        SurfaceClass::Cylinder => {}
        other => {
            return Err(error(
                OFFSET_UNSUPPORTED,
                format!(
                    "Face {face} is {other:?}; offset-face/1 supports cylinders and planes only"
                ),
            ));
        }
    }
    offset_cylinder(source, face, offset_mm, options, &aag, &mut guard)
}

/// Radial offset of a cylindrical face, with concentric-pair handling.
fn offset_cylinder(
    model: &Model,
    face: usize,
    offset_mm: f64,
    options: OffsetOptions,
    aag: &crate::aag::Aag,
    guard: &mut BudgetGuard,
) -> Result<OffsetFaceReport> {
    let linear = model.tolerance_mm * 8.;
    let attrs = aag.nodes[face].attrs.as_ref().expect("checked above");
    let axis = attrs
        .axis
        .ok_or_else(|| error(OFFSET_INVALID, format!("Face {face} has no fitted axis")))?;
    let radius = attrs
        .radius
        .ok_or_else(|| error(OFFSET_INVALID, format!("Face {face} has no fitted radius")))?;

    // 1. Concentric clusters, detected BEFORE any mutation: every
    //    cylindrical face coaxial with the target, clustered by radius.
    let mut groups: Vec<(f64, Vec<usize>)> = Vec::new(); // (radius, faces)
    for (i, node) in aag.nodes.iter().enumerate() {
        guard.tick()?;
        let Some(a) = &node.attrs else { continue };
        if a.class != SurfaceClass::Cylinder {
            continue;
        }
        let (Some(other_axis), Some(other_radius)) = (a.axis, a.radius) else {
            continue;
        };
        if !coaxial_axes(&axis, &other_axis, linear) {
            continue;
        }
        match groups
            .iter_mut()
            .find(|(r, _)| (*r - other_radius).abs() <= 1e-4 * other_radius)
        {
            Some((_, faces)) => faces.push(i),
            None => groups.push((other_radius, vec![i])),
        }
    }
    groups.sort_by(|a, b| a.0.total_cmp(&b.0));
    let Some(target) = groups
        .iter()
        .position(|(r, _)| (*r - radius).abs() <= 1e-4 * radius)
    else {
        return Err(error(
            OFFSET_INVALID,
            format!("Face {face} is missing from its own coaxial cluster"),
        ));
    };
    if groups.len() > 2 {
        return Err(error(
            OFFSET_UNSUPPORTED,
            "Stepped coaxial walls (more than two radii) are outside offset-face/1",
        ));
    }
    let partner = if groups.len() == 2 {
        Some(1 - target)
    } else {
        None
    };
    let auto_expanded = partner.is_some();
    if options.strict {
        if let Some(p) = partner {
            return Err(error(
                OFFSET_UNSUPPORTED,
                format!(
                    "Face {face} is one wall of a concentric pair; strict mode refuses single-member offsets. Partner faces: {:?} — drop strict to offset the pair",
                    groups[p].1
                ),
            ));
        }
    }

    // 2. Axial extents must match within a pair (partially overlapping walls
    //    would change thickness along z after a paired offset — conserva-
    //    tively refused).
    let axial_extent = |faces: &[usize]| -> [f64; 2] {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for &f in faces {
            for p in model.faces[f].surface.control_points.iter().flatten() {
                let t = dot(sub([p[0], p[1], p[2]], axis.point), axis.direction);
                lo = lo.min(t);
                hi = hi.max(t);
            }
        }
        [lo, hi]
    };
    let target_extent = axial_extent(&groups[target].1);
    if let Some(p) = partner {
        let pe = axial_extent(&groups[p].1);
        if (pe[0] - target_extent[0]).abs() > linear || (pe[1] - target_extent[1]).abs() > linear {
            return Err(error(
                OFFSET_UNSUPPORTED,
                "Concentric walls span different axial ranges; partial pairing is refused",
            ));
        }
    }
    let height = target_extent[1] - target_extent[0];

    // 3. Blend guard: a tangent neighbor outside the groups is a
    //    fixed-radius blend whose contact would tear — refuse, naming it.
    let group_faces: BTreeSet<usize> = groups.iter().flat_map(|(_, f)| f.iter().copied()).collect();
    for aag_edge in &aag.edges {
        guard.tick()?;
        if !matches!(aag_edge.class, DihedralClass::Smooth | DihedralClass::Tangent) {
            continue;
        }
        let uses: Vec<usize> = aag_edge.uses.iter().map(|u| u.face).collect();
        let inside = uses.iter().any(|u| group_faces.contains(u));
        let outside = uses.iter().find(|u| !group_faces.contains(u));
        if inside {
            if let Some(&other) = outside {
                return Err(error(
                    OFFSET_UNSUPPORTED,
                    format!(
                        "Face {other} is tangent to the offset group (fixed-radius blend); refitting blends is out of scope"
                    ),
                ));
            }
        }
    }

    // 4. Radii sanity: a wall pushed onto/past the axis is a topology
    //    mutation — refused, not sewn.
    let mut plan: Vec<(f64, f64, Vec<usize>)> = Vec::new(); // (old, new, faces)
    for (old_r, faces) in &groups {
        let new_r = old_r + offset_mm;
        if !(new_r.is_finite() && new_r > linear) {
            return Err(error(
                OFFSET_UNSUPPORTED,
                format!(
                    "Offset consumes the wall at radius {old_r} (new radius {new_r}); refused"
                ),
            ));
        }
        plan.push((*old_r, new_r, faces.clone()));
    }

    // 5. Staged mutation: surfaces scale radially, rim circles re-author at
    //    the new radius, straight edges rebuild from moved endpoints, cap
    //    pcurves re-map. Topology and ids are untouched.
    let mut staged = model.clone();
    for (old_r, new_r, faces) in &plan {
        guard.tick()?;
        for &f in faces {
            for point in staged.faces[f].surface.control_points.iter_mut().flatten() {
                if point.len() != 3 {
                    return Err(error(
                        OFFSET_UNSUPPORTED,
                        format!("Face {f} has non-3D control points"),
                    ));
                }
                let scaled = radial_scale(&axis, *old_r, *new_r, [point[0], point[1], point[2]]);
                *point = scaled.to_vec();
            }
        }
        // Vertices of any group face loop sitting on the old radius move out.
        let mut moved_vertices: BTreeSet<usize> = BTreeSet::new();
        for &f in faces {
            for &ring in std::iter::once(&model.faces[f].outer).chain(&model.faces[f].holes) {
                for coedge in &model.loops[ring].coedges {
                    for &v in &model.edges[coedge.edge].vertices {
                        moved_vertices.insert(v);
                    }
                }
            }
        }
        // Rim vertices may also sit on cap seams not owned by the group;
        // the geometric radius test catches every point on the old circle.
        for (v, vertex) in model.vertices.iter().enumerate() {
            guard.tick()?;
            if (radial_distance(&axis, vertex.point) - old_r).abs() <= linear {
                moved_vertices.insert(v);
            }
        }
        let moved: BTreeMap<usize, [f64; 3]> = moved_vertices
            .iter()
            .map(|&v| (v, radial_scale(&axis, *old_r, *new_r, model.vertices[v].point)))
            .collect();
        for (&v, &point) in &moved {
            staged.vertices[v].point = point;
        }

        // Edges: rebuild every edge touched by moved vertices; a curved edge
        // must live entirely on the old radius (a rim circle), else refuse.
        for (e, edge) in model.edges.iter().enumerate() {
            guard.tick()?;
            let touched = edge.vertices.iter().any(|v| moved.contains_key(&v));
            if !touched {
                continue;
            }
            if edge.degenerate {
                continue; // collapsed pole edges carry no extent
            }
            if edge.curve.degree == 1 {
                let a = staged.vertices[edge.vertices[0]].point.to_vec();
                let b = staged.vertices[edge.vertices[1]].point.to_vec();
                staged.edges[e].curve = crate::line(a, b);
            } else {
                // A rim circle of the group: the *curve* lies on the old
                // radius (control points of a NURBS circle do not — they sit
                // on the control polygon outside the arc, so sample the
                // curve itself). Uniform radial scaling of the control net
                // then re-authors the circle at the new radius exactly.
                let domain = edge.curve.domain();
                let mut all_on_radius = true;
                // Interior samples only: periodic rim curves may exclude the
                // seam endpoint from the active domain.
                for i in 1..8 {
                    guard.tick()?;
                    let t = domain[0] + (domain[1] - domain[0]) * i as f64 / 8.;
                    let point = edge.curve.evaluate(t)?.point;
                    if point.len() != 3
                        || (radial_distance(&axis, [point[0], point[1], point[2]]) - old_r).abs()
                            > linear * 4.
                    {
                        all_on_radius = false;
                        break;
                    }
                }
                if !all_on_radius {
                    return Err(error(
                        OFFSET_UNSUPPORTED,
                        format!(
                            "Edge {e} is curved but not a rim circle of the offset group; re-fit is out of scope"
                        ),
                    ));
                }
                let mut curve = edge.curve.clone();
                for p in &mut curve.control_points {
                    let scaled =
                        radial_scale(&axis, *old_r, *new_r, [p[0], p[1], p[2]]);
                    *p = scaled.to_vec();
                }
                staged.edges[e].curve = curve;
            }
        }
    }

    // 6. Cap re-trim: pcurves of changed edges on faces outside the groups
    //    re-map onto the (unchanged) planar surfaces via the rigid plane
    //    frame. Group faces keep their pcurves — radial scaling preserves
    //    the angular parameterization.
    let mut recomputed_pcurves = 0usize;
    let mut changed_edges: BTreeSet<usize> = BTreeSet::new();
    for (e, edge) in staged.edges.iter().enumerate() {
        if edge.curve != model.edges[e].curve {
            changed_edges.insert(e);
        }
    }
    let recomputed_edges = changed_edges.len();
    for (f, staged_face) in staged.faces.clone().iter().enumerate() {
        guard.tick()?;
        if group_faces.contains(&f) {
            continue;
        }
        // Does the face use any changed edge?
        let uses_changed = std::iter::once(&staged_face.outer)
            .chain(&staged_face.holes)
            .any(|&ring| staged.loops[ring].coedges.iter().any(|c| changed_edges.contains(&c.edge)));
        if !uses_changed {
            continue;
        }
        let attrs = aag.nodes[f].attrs.as_ref();
        if attrs.map(|a| a.class) != Some(SurfaceClass::Plane) {
            return Err(error(
                OFFSET_UNSUPPORTED,
                format!(
                    "Face {f} is not planar but shares an edge with the offset group; re-trim is out of scope"
                ),
            ));
        }
        let s = &staged_face.surface;
        if s.degree_u != 1 || s.degree_v != 1 || s.control_points.len() != 2 || s.control_points[0].len() != 2
        {
            return Err(error(
                OFFSET_UNSUPPORTED,
                format!(
                    "Cap face {f} is not a bilinear patch; pcurve re-map needs the explicit UV frame"
                ),
            ));
        }
        // The cap patch must grow with the rim: a rim circle is inscribed in
        // the patch's UV square, so a larger radius would otherwise leave the
        // surface domain. Every control point scales radially by the factor
        // of the group whose rim it belongs to: points sitting on a group
        // radius match exactly; corner points of the bounding square (beyond
        // every rim) take the outermost rim's factor. Axial components are
        // kept, so the patch stays planar and bilinear. Anything in between
        // (a control net we cannot attribute) is refused.
        let face_groups: BTreeSet<usize> = std::iter::once(&staged_face.outer)
            .chain(&staged_face.holes)
            .flat_map(|&ring| staged.loops[ring].coedges.iter())
            .filter(|c| changed_edges.contains(&c.edge))
            .filter_map(|c| {
                let v = model.edges[c.edge].vertices[0];
                let d = radial_distance(&axis, model.vertices[v].point);
                plan.iter()
                    .position(|(old_r, _, _)| (d - old_r).abs() <= linear)
            })
            .collect();
        if face_groups.is_empty() {
            return Err(error(
                OFFSET_UNSUPPORTED,
                format!("Cap face {f} shares no rim edge with the offset group"),
            ));
        }
        let outer_group = *face_groups.iter().next_back().expect("nonempty");
        let factors: Vec<f64> = plan
            .iter()
            .map(|(old_r, new_r, _)| new_r / old_r)
            .collect();
        let max_old = plan[outer_group].0;
        for p in staged.faces[f].surface.control_points.iter_mut().flatten() {
            if p.len() != 3 {
                return Err(error(
                    OFFSET_UNSUPPORTED,
                    format!("Cap face {f} has non-3D control points"),
                ));
            }
            let d = radial_distance(&axis, [p[0], p[1], p[2]]);
            let on = face_groups
                .iter()
                .find(|&&g| (d - plan[g].0).abs() <= linear);
            let factor = match on {
                Some(&g) => factors[g],
                None if d >= max_old => factors[outer_group],
                None => {
                    return Err(error(
                        OFFSET_UNSUPPORTED,
                        format!(
                            "Cap face {f} has a control point at radius {d} that matches no offset rim; re-trim is out of scope"
                        ),
                    ));
                }
            };
            let scaled = radial_scale(&axis, 1., factor, [p[0], p[1], p[2]]);
            *p = scaled.to_vec();
        }
        let s = &staged.faces[f].surface;
        let origin: [f64; 3] = {
            let p = &s.control_points[0][0];
            [p[0], p[1], p[2]]
        };
        let du = {
            let p = &s.control_points[1][0];
            sub([p[0], p[1], p[2]], origin)
        };
        let dv = {
            let p = &s.control_points[0][1];
            sub([p[0], p[1], p[2]], origin)
        };
        let (len_u, len_v) = (norm(du), norm(dv));
        if len_u <= linear || len_v <= linear {
            return Err(error(
                OFFSET_UNSUPPORTED,
                format!("Cap face {f} has a degenerate UV frame"),
            ));
        }
        let u = mul(du, 1. / len_u);
        let v = mul(dv, 1. / len_v);
        for &ring in std::iter::once(&staged_face.outer).chain(&staged_face.holes) {
            for ci in 0..staged.loops[ring].coedges.len() {
                let coedge = &staged.loops[ring].coedges[ci];
                if !changed_edges.contains(&coedge.edge) {
                    continue;
                }
                let mut curve = staged.edges[coedge.edge].curve.clone();
                if coedge.reversed {
                    curve = curve.reverse()?;
                }
                let mapped = curve_to_uv(&curve, origin, u, v, len_u, len_v)?;
                staged.loops[ring].coedges[ci].pcurve = mapped;
                recomputed_pcurves += 1;
            }
        }
    }

    // 7. Contour: validate → audit → measure. Failure discards the clone.
    staged.validate()?;
    let audit = crate::solid_audit::audit_solid(&staged)?;
    let volume_before = volume_of(model)?;
    let volume_after = volume_of(&staged)?;
    // ΔV estimate: outer groups add, inner (void-enclosing) groups subtract.
    // The bore wall is the group whose radius is smaller within a pair.
    let mut estimate = 0.;
    for (old_r, new_r, _) in &plan {
        estimate += std::f64::consts::PI * height * (new_r * new_r - old_r * old_r);
    }
    if plan.len() == 2 {
        // The pair formula above counts the bore growth as material gain;
        // the inner wall encloses void, so its term flips sign.
        let (old_r, new_r, _) = plan[0]; // ascending: smallest radius first
        estimate -= 2. * std::f64::consts::PI * height * (new_r * new_r - old_r * old_r);
    }
    let thickness = if plan.len() == 2 {
        Some(plan[1].0 - plan[0].0)
    } else {
        None
    };
    let mut offset_faces: Vec<usize> = group_faces.iter().copied().collect();
    offset_faces.sort_unstable();
    guard.check()?;
    Ok(OffsetFaceReport {
        model: staged,
        face,
        kind: "cylinder-radius",
        offset_faces,
        old_radii: plan.iter().map(|(r, _, _)| *r).collect(),
        new_radii: plan.iter().map(|(_, r, _)| *r).collect(),
        auto_expanded,
        wall_thickness_before_mm: thickness,
        wall_thickness_after_mm: thickness.map(|_| plan[1].1 - plan[0].1),
        recomputed_edges,
        recomputed_pcurves,
        volume_before_mm3: volume_before,
        volume_after_mm3: volume_after,
        delta_volume_estimate_mm3: estimate,
        delta_volume_actual_mm3: volume_after - volume_before,
        audit,
    })
}

// ---------------------------------------------------------------------------
// Tests.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aag::Aag;
    use crate::aag_fillets::find_fillet_chains;
    use crate::aag_features::{find_holes, find_pockets};
    use crate::analytic_features::analytic_fillet;

    fn budget() -> Budget {
        Budget::new(10_000_000, 8, 60_000).unwrap()
    }

    fn graph_with_attrs(model: &Model) -> Aag {
        let mut graph = Aag::build(model, &budget()).unwrap();
        graph.attach_face_attrs(model, &budget()).unwrap();
        graph
    }

    fn vertical_edges(model: &Model) -> Vec<usize> {
        model
            .edges
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                let a = model.vertices[e.vertices[0]].point;
                let b = model.vertices[e.vertices[1]].point;
                ((a[0] - b[0]).abs() <= 1e-12 && (a[1] - b[1]).abs() <= 1e-12).then_some(i)
            })
            .collect()
    }

    fn translated(model: &Model, delta: [f64; 3]) -> Model {
        crate::transform::affine(
            model,
            [
                [1., 0., 0., delta[0]],
                [0., 1., 0., delta[1]],
                [0., 0., 1., delta[2]],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap()
    }

    #[test]
    fn suppress_single_fillet_on_cuboid_restores_sharp_corner() {
        let sharp = crate::cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
        let edges = vertical_edges(&sharp);
        let (rounded, _) = analytic_fillet(&sharp, edges[0], 1.).unwrap();
        let graph = graph_with_attrs(&rounded);
        let chains = find_fillet_chains(&rounded, &graph, &budget()).unwrap();
        assert_eq!(chains.len(), 1);
        let before = rounded.clone();

        let report = suppress_feature(
            &rounded,
            &FeatureRef::FilletChain(chains[0].clone()),
            &budget(),
        )
        .unwrap();
        assert_eq!(report.kind, "fillet-chain");
        assert_eq!(report.new_sharp_edges, 1);
        // Validity gate ran inside suppress_feature; result is one solid body.
        report.model.validate().unwrap();
        assert_eq!(report.model.bodies.len(), 1);
        // The sharp corner is back: 8 vertices, box volume 1000.
        assert!(
            (report.volume_after_mm3 - 1000.).abs() < 1e-3,
            "sharp box volume: {}",
            report.volume_after_mm3
        );
        // ΔV within the declared estimate: (1 − π/4) r² L, r = 1, L = 10.
        let expected = (1. - std::f64::consts::FRAC_PI_4) * 10.;
        assert!(
            (report.delta_volume_actual_mm3 - expected).abs() <= 0.05 * expected,
            "actual ΔV {} vs estimate {}",
            report.delta_volume_actual_mm3,
            report.delta_volume_estimate_mm3
        );
        assert!(
            (report.delta_volume_estimate_mm3 - expected).abs() <= 0.01 * expected,
            "declared estimate {} vs analytic {}",
            report.delta_volume_estimate_mm3,
            expected
        );
        // Every new vertex lands on a corner of the original box.
        for v in &report.model.vertices {
            let on_corner = (0..8).any(|c| {
                let corner = [
                    if c & 1 == 0 { 0. } else { 10. },
                    if c & 2 == 0 { 0. } else { 10. },
                    if c & 4 == 0 { 0. } else { 10. },
                ];
                dist(v.point, corner) < 1e-6
            });
            assert!(on_corner, "vertex {:?} is not a box corner", v.point);
        }
        // Input untouched.
        assert_eq!(rounded, before);
    }

    #[test]
    fn suppress_through_hole_in_block() {
        let block = crate::cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
        let drill = translated(&crate::cylinder(2., 14.).unwrap(), [5., 5., -2.]);
        let model = crate::boolean(&block, &drill, "difference").unwrap();
        let graph = graph_with_attrs(&model);
        let holes = find_holes(&model, &graph, &budget()).unwrap();
        assert_eq!(holes.len(), 1, "one bore: {holes:?}");
        let before = model.clone();

        let report = suppress_feature(&model, &FeatureRef::Hole(holes[0].clone()), &budget())
            .unwrap();
        assert_eq!(report.kind, "hole");
        assert_eq!(report.new_sharp_edges, 0);
        report.model.validate().unwrap();
        // The bore is gone: volume back to the full block.
        assert!(
            (report.volume_after_mm3 - 1000.).abs() < 1e-2,
            "volume after hole suppression: {}",
            report.volume_after_mm3
        );
        // ΔV = π r² h = π · 4 · 10 within the mass-properties tolerance.
        let expected = std::f64::consts::PI * 4. * 10.;
        assert!(
            (report.delta_volume_actual_mm3 - expected).abs() <= 0.02 * expected,
            "actual ΔV {} vs π·4·10",
            report.delta_volume_actual_mm3
        );
        assert!(
            (report.delta_volume_estimate_mm3 - expected).abs() <= 0.01 * expected,
            "estimate {} vs π·4·10",
            report.delta_volume_estimate_mm3
        );
        // No cylindrical face survives.
        let graph = graph_with_attrs(&report.model);
        let holes = find_holes(&report.model, &graph, &budget()).unwrap();
        assert!(holes.is_empty(), "hole is gone: {holes:?}");
        assert_eq!(model, before);
    }

    #[test]
    fn suppress_closed_pocket_in_block() {
        // Canonical 867 fixture: floor at z=2 (36×26), walls z=2..20. The
        // boolean fragments the mouth region into coplanar shards whose outer
        // loops carry the rim edges — the merge path must re-sew them into
        // one face. All surroundings are planar.
        let model = crate::boolean(
            &crate::cuboid([0.; 3], [40., 30., 20.]).unwrap(),
            &crate::cuboid([2., 2., 2.], [38., 28., 22.]).unwrap(),
            "difference",
        )
        .unwrap();
        let graph = graph_with_attrs(&model);
        let pockets = find_pockets(&model, &graph, &budget()).unwrap();
        assert_eq!(pockets.len(), 1, "{pockets:?}");
        assert_eq!(pockets[0].kind, PocketKind::Closed);
        let before = model.clone();

        let report =
            suppress_feature(&model, &FeatureRef::Pocket(pockets[0].clone()), &budget())
                .unwrap();
        assert_eq!(report.kind, "pocket");
        report.model.validate().unwrap();
        // Filled pocket → full block 40×30×20.
        let full = 40. * 30. * 20.;
        assert!(
            (report.volume_after_mm3 - full).abs() < 1e-6 * full,
            "volume after pocket suppression: {}",
            report.volume_after_mm3
        );
        // ΔV = floor 36×26 × depth 18.
        let expected = 36. * 26. * 18.;
        assert!(
            (report.delta_volume_actual_mm3 - expected).abs() < 1e-6 * expected,
            "actual ΔV {}",
            report.delta_volume_actual_mm3
        );
        assert!(
            (report.delta_volume_estimate_mm3 - expected).abs() < 1e-6 * expected,
            "estimate {}",
            report.delta_volume_estimate_mm3
        );
        // No pocket remains; the model is a plain block.
        let graph = graph_with_attrs(&report.model);
        assert!(find_pockets(&report.model, &graph, &budget()).unwrap().is_empty());
        assert_eq!(model, before);
    }

    #[test]
    fn suppress_closed_pocket_in_cup() {
        // Round blind pocket (the bore of the cup doubles as a closed round
        // pocket, a documented matcher overlap): the mouth lives on the outer
        // loops of the coplanar annulus shards, so the merge path re-sews
        // them into one disk face; the curved bore wall is deleted outright.
        const CUP: [[f64; 2]; 6] = [
            [0., 0.],
            [5., 0.],
            [5., 6.],
            [2., 6.],
            [2., 2.],
            [0., 2.],
        ];
        let model = crate::analytic::revolve(&CUP).unwrap();
        let graph = graph_with_attrs(&model);
        let pockets = find_pockets(&model, &graph, &budget()).unwrap();
        assert_eq!(pockets.len(), 1, "one pocket: {pockets:?}");
        assert_eq!(pockets[0].kind, PocketKind::Closed);
        let before = model.clone();

        let report =
            suppress_feature(&model, &FeatureRef::Pocket(pockets[0].clone()), &budget())
                .unwrap();
        assert_eq!(report.kind, "pocket");
        report.model.validate().unwrap();
        // Filled pocket → solid cylinder r = 5, h = 6.
        let full = std::f64::consts::PI * 25. * 6.;
        assert!(
            (report.volume_after_mm3 - full).abs() < 1e-2 * full,
            "volume after pocket suppression: {}",
            report.volume_after_mm3
        );
        // ΔV = π r² depth = π · 4 · 4.
        let expected = std::f64::consts::PI * 4. * 4.;
        assert!(
            (report.delta_volume_actual_mm3 - expected).abs() <= 0.02 * expected,
            "actual ΔV {}",
            report.delta_volume_actual_mm3
        );
        assert!(
            (report.delta_volume_estimate_mm3 - expected).abs() <= 0.01 * expected,
            "estimate {}",
            report.delta_volume_estimate_mm3
        );
        assert_eq!(model, before);
    }

    #[test]
    fn diverging_supports_fail_with_face_ids_and_rollback() {
        let sharp = crate::cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
        let edges = vertical_edges(&sharp);
        let (rounded, _) = analytic_fillet(&sharp, edges[0], 1.).unwrap();
        let graph = graph_with_attrs(&rounded);
        let chains = find_fillet_chains(&rounded, &graph, &budget()).unwrap();
        assert_eq!(chains.len(), 1);
        // Artificially diverging surroundings: claim the blend rolls on the
        // two parallel side walls (x = 0 and x = 10). Their extensions never
        // meet, so suppression must refuse and name the faces.
        let parallel_walls: Vec<usize> = (0..rounded.faces.len())
            .filter(|&f| {
                let plane = face_plane_fit(&rounded, f);
                matches!(plane, Ok(p) if p.normal[0].abs() > 0.99 && !chains[0].faces.contains(&f))
            })
            .collect();
        assert_eq!(parallel_walls.len(), 2);
        let mut bogus = chains[0].clone();
        bogus.supports = [parallel_walls[0], parallel_walls[1]];

        let before = rounded.clone();
        let error =
            suppress_feature(&rounded, &FeatureRef::FilletChain(bogus), &budget()).unwrap_err();
        assert_eq!(error.code, SUPPRESS_DIVERGENT, "{error:?}");
        assert!(
            error.message.contains(&parallel_walls[0].to_string())
                && error.message.contains(&parallel_walls[1].to_string()),
            "culpable faces named: {}",
            error.message
        );
        // Honest rollback: the model is bit-for-bit unchanged.
        assert_eq!(rounded, before);
        let graph = graph_with_attrs(&rounded);
        assert_eq!(
            find_fillet_chains(&rounded, &graph, &budget()).unwrap().len(),
            1,
            "feature still there after rollback"
        );
    }

    #[test]
    fn open_pocket_is_refused_typed() {
        // Slot open to the body boundary: PocketKind::Open.
        let block = crate::cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
        let tool = crate::cuboid([0., 3., 6.], [7., 7., 12.]).unwrap();
        let model = crate::boolean(&block, &tool, "difference").unwrap();
        let graph = graph_with_attrs(&model);
        let pockets = find_pockets(&model, &graph, &budget()).unwrap();
        if let Some(pocket) = pockets.iter().find(|p| p.kind == PocketKind::Open) {
            let before = model.clone();
            let error =
                suppress_feature(&model, &FeatureRef::Pocket(pocket.clone()), &budget())
                    .unwrap_err();
            assert_eq!(error.code, SUPPRESS_UNSUPPORTED, "{error:?}");
            assert_eq!(model, before);
        }
    }

    #[test]
    fn out_of_range_feature_is_invalid() {
        let model = crate::cuboid([0., 0., 0.], [4., 4., 4.]).unwrap();
        let mut chain = FilletChain {
            faces: vec![usize::MAX],
            edges: vec![],
            kind: crate::aag_fillets::FilletKind::ConstantRadius,
            radius_min: 1.,
            radius_max: 1.,
            supports: [0, 1],
        };
        let before = model.clone();
        let error = suppress_feature(&model, &FeatureRef::FilletChain(chain.clone()), &budget())
            .unwrap_err();
        assert_eq!(error.code, SUPPRESS_INVALID, "{error:?}");
        chain.faces = vec![];
        let error = suppress_feature(&model, &FeatureRef::FilletChain(chain), &budget())
            .unwrap_err();
        assert_eq!(error.code, SUPPRESS_INVALID, "{error:?}");
        assert_eq!(model, before);
    }

    // ------------------------------------------------------------------
    // move face (871)
    // ------------------------------------------------------------------

    /// Index of the unique face whose canonical plane sits at `axis` = value
    /// with outward normal +1 along that axis (e.g. the x=10 wall of a box).
    fn face_at(model: &Model, axis: usize, value: f64) -> usize {
        model
            .faces
            .iter()
            .enumerate()
            .find_map(|(i, f)| {
                let points: Vec<[f64; 3]> = loop_vertices(model, f.outer)
                    .unwrap()
                    .iter()
                    .map(|&v| model.vertices[v].point)
                    .collect();
                (points
                    .iter()
                    .all(|p| (p[axis] - value).abs() < 1e-9))
                .then_some(i)
            })
            .expect("face at the given plane exists")
    }

    /// Ids of all faces sitting on the plane `axis` = `value`.
    fn face_ids_at(model: &Model, axis: usize, value: f64) -> Vec<crate::TopoId> {
        model
            .faces
            .iter()
            .enumerate()
            .filter(|(_, f)| {
                loop_vertices(model, f.outer)
                    .unwrap()
                    .iter()
                    .all(|&v| (model.vertices[v].point[axis] - value).abs() < 1e-9)
            })
            .map(|(i, _)| model.1.faces[i])
            .collect()
    }

    #[test]
    fn move_block_wall_extends_box() {
        let model = crate::cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
        let wall = face_at(&model, 0, 10.);
        let opposite_ids = face_ids_at(&model, 0, 0.);
        assert_eq!(opposite_ids.len(), 1);
        let before = model.clone();

        let report = move_face(&model, wall, [1., 0., 0.], 2., &budget()).unwrap();
        assert_eq!(report.moved_face, wall);
        assert!(report.driven_faces.is_empty(), "no coplanar shards on a box");
        assert_eq!(report.absorbing_faces.len(), 4, "four side walls bound the move");
        assert!(report.absorbed_faces.is_empty() && report.annihilated_faces.is_empty());
        report.model.validate().unwrap();
        assert!(
            (report.volume_after_mm3 - 1200.).abs() < 1e-6 * 1200.,
            "volume: {}",
            report.volume_after_mm3
        );
        // ΔV = 100 mm² × 2 mm, estimate and measurement agree to 1e-6 rel.
        assert!((report.delta_volume_estimate_mm3 - 200.).abs() < 1e-9);
        assert!(
            (report.delta_volume_actual_mm3 - 200.).abs() < 1e-6 * 200.,
            "actual ΔV {}",
            report.delta_volume_actual_mm3
        );
        // Persistent id of the untouched opposite face survives the rebuild.
        let after_ids = face_ids_at(&report.model, 0, 0.);
        assert_eq!(after_ids, opposite_ids, "untouched face id preserved");
        // Input untouched.
        assert_eq!(model, before);
    }

    #[test]
    fn move_pocket_floor_deeper_and_shallower() {
        let pocket = || {
            crate::boolean(
                &crate::cuboid([0.; 3], [40., 30., 20.]).unwrap(),
                &crate::cuboid([2., 2., 2.], [38., 28., 22.]).unwrap(),
                "difference",
            )
            .unwrap()
        };
        let model = pocket();
        let floor = face_at(&model, 2, 2.);
        let floor_area = 36. * 26.;
        let volume_before = 40. * 30. * 20. - floor_area * 18.;
        let before = model.clone();

        // Deeper: floor outward normal is +z; moving −z grows the void.
        let deeper = move_face(&model, floor, [0., 0., -1.], 1., &budget()).unwrap();
        deeper.model.validate().unwrap();
        assert!(
            (deeper.delta_volume_estimate_mm3 + floor_area).abs() < 1e-6,
            "estimate: {}",
            deeper.delta_volume_estimate_mm3
        );
        assert!(
            (deeper.volume_after_mm3 - (volume_before - floor_area)).abs()
                < 1e-6 * volume_before,
            "deeper volume: {}",
            deeper.volume_after_mm3
        );
        assert!(
            (deeper.delta_volume_actual_mm3 + floor_area).abs() < 1e-6 * floor_area,
            "deeper ΔV: {}",
            deeper.delta_volume_actual_mm3
        );

        // Shallower: +z by 1 mm gives the mirror delta.
        let shallower = move_face(&model, floor, [0., 0., 1.], 1., &budget()).unwrap();
        assert!(
            (shallower.volume_after_mm3 - (volume_before + floor_area)).abs()
                < 1e-6 * volume_before,
            "shallower volume: {}",
            shallower.volume_after_mm3
        );
        assert!(
            (shallower.delta_volume_actual_mm3 - floor_area).abs() < 1e-6 * floor_area,
            "shallower ΔV: {}",
            shallower.delta_volume_actual_mm3
        );
        // Untouched faces (block bottom z=0) keep their persistent ids.
        let bottom_before = face_ids_at(&model, 2, 0.);
        let bottom_after = face_ids_at(&shallower.model, 2, 0.);
        assert_eq!(bottom_before, bottom_after, "untouched shard ids preserved");
        assert_eq!(model, before);
    }

    #[test]
    fn move_absorbs_thin_wall_between_pockets() {
        let model = crate::boolean(
            &crate::cuboid([0., 0., 0.], [10., 10., 10.]).unwrap(),
            &crate::cuboid([2., 2., 4.], [4.75, 8., 12.]).unwrap(),
            "difference",
        )
        .unwrap();
        let model = crate::boolean(
            &model,
            &crate::cuboid([5.25, 2., 4.], [8., 8., 12.]).unwrap(),
            "difference",
        )
        .unwrap();
        let wall = face_at(&model, 0, 4.75);
        let volume_before = 1000. - 2. * (2.75 * 6. * 6.);
        let before = model.clone();

        // Outward normal of the pocket-1 wall points −x; pushing +x by the
        // full 0.5 mm wall thickness consumes the wall: the strip above it is
        // absorbed and the moved face annihilates with the far side x=5.25.
        let report = move_face(&model, wall, [1., 0., 0.], 0.5, &budget()).unwrap();
        report.model.validate().unwrap();
        assert!(!report.absorbed_faces.is_empty(), "top strip absorbed: {report:?}");
        assert!(
            report.annihilated_faces.contains(&wall),
            "moved face annihilated with the far wall side: {:?}",
            report.annihilated_faces
        );
        // Merged pocket 6×6×6: volume and ΔV are analytic.
        let volume_after = 1000. - 6. * 6. * 6.;
        assert!(
            (report.volume_after_mm3 - volume_after).abs() < 1e-6 * volume_before,
            "volume after absorption: {} (want {volume_after})",
            report.volume_after_mm3
        );
        assert!(
            (report.delta_volume_actual_mm3 - (-0.5 * 36.)).abs() < 1e-6 * 18.,
            "actual ΔV {}",
            report.delta_volume_actual_mm3
        );
        assert!(
            (report.delta_volume_estimate_mm3 - (-0.5 * 36.)).abs() < 1e-9,
            "estimate {}",
            report.delta_volume_estimate_mm3
        );
        // No zombie faces at the consumed wall planes.
        assert!(
            face_ids_at(&report.model, 0, 4.75).is_empty()
                && face_ids_at(&report.model, 0, 5.25).is_empty(),
            "no faces left at x=4.75 / x=5.25"
        );
        // The two pockets merged into one.
        let graph = graph_with_attrs(&report.model);
        let pockets = find_pockets(&report.model, &graph, &budget()).unwrap();
        assert_eq!(pockets.len(), 1, "merged pocket: {pockets:?}");
        assert_eq!(model, before);
    }

    #[test]
    fn move_top_plane_carries_coplanar_shards() {
        // The boolean fragments the block's top into coplanar shards; moving
        // one shard must drive the whole coplanar component.
        let model = crate::boolean(
            &crate::cuboid([0.; 3], [40., 30., 20.]).unwrap(),
            &crate::cuboid([2., 2., 2.], [38., 28., 22.]).unwrap(),
            "difference",
        )
        .unwrap();
        let top_shards = face_ids_at(&model, 2, 20.);
        assert!(top_shards.len() > 1, "fragmented top: {}", top_shards.len());
        let shard = face_at(&model, 2, 20.);
        let before = model.clone();

        let report = move_face(&model, shard, [0., 0., -1.], 1., &budget()).unwrap();
        assert_eq!(
            report.driven_faces.len(),
            top_shards.len() - 1,
            "every other coplanar shard is driven: {:?}",
            report.driven_faces
        );
        report.model.validate().unwrap();
        // The whole rim frame (40×30 minus the 36×26 mouth) went down 1 mm;
        // the pocket walls and floor are unchanged.
        let expected = -(40. * 30. - 36. * 26.);
        assert!(
            (report.delta_volume_estimate_mm3 - expected).abs() < 1e-6,
            "estimate {}",
            report.delta_volume_estimate_mm3
        );
        assert!(
            (report.delta_volume_actual_mm3 - expected).abs() < 1e-6 * expected.abs(),
            "actual ΔV {}",
            report.delta_volume_actual_mm3
        );
        // Driven shards kept their identity? No — they moved. The untouched
        // bottom shards keep theirs.
        let bottom_before = face_ids_at(&model, 2, 0.);
        let bottom_after = face_ids_at(&report.model, 2, 0.);
        assert_eq!(bottom_before, bottom_after);
        assert_eq!(model, before);
    }

    #[test]
    fn move_curved_face_is_refused_typed() {
        let model = crate::cylinder(2., 5.).unwrap();
        let side = model
            .faces
            .iter()
            .enumerate()
            .find_map(|(i, f)| {
                (f.surface.degree_u > 1 || f.surface.degree_v > 1).then_some(i)
            })
            .expect("cylinder side");
        let before = model.clone();
        let error = move_face(&model, side, [1., 0., 0.], 1., &budget()).unwrap_err();
        assert_eq!(error.code, MOVE_UNSUPPORTED, "{error:?}");
        assert!(error.message.contains(&side.to_string()));
        assert_eq!(model, before);
    }

    #[test]
    fn move_off_normal_is_refused_typed() {
        let model = crate::cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
        let wall = face_at(&model, 0, 10.);
        let before = model.clone();
        let s = 0.5 * std::f64::consts::SQRT_2;
        let error = move_face(&model, wall, [s, 0., s], 1., &budget()).unwrap_err();
        assert_eq!(error.code, MOVE_UNSUPPORTED, "{error:?}");
        assert_eq!(model, before);
    }

    #[test]
    fn move_through_parallel_face_is_refused_typed() {
        let model = crate::cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
        let wall = face_at(&model, 0, 10.);
        let before = model.clone();
        // Crossing the opposite wall (x=0) overshoots the absorbing limit.
        let error = move_face(&model, wall, [-1., 0., 0.], 12., &budget()).unwrap_err();
        assert_eq!(error.code, MOVE_UNSUPPORTED, "{error:?}");
        assert_eq!(model, before);
        // And the full-body consumption (landing exactly on x=0) fails too.
        let error = move_face(&model, wall, [-1., 0., 0.], 10., &budget()).unwrap_err();
        assert_eq!(model, before);
        let _ = error;
    }

    #[test]
    fn move_invalid_input_is_refused() {
        let model = crate::cuboid([0., 0., 0.], [4., 4., 4.]).unwrap();
        let before = model.clone();
        assert_eq!(
            move_face(&model, usize::MAX, [1., 0., 0.], 1., &budget())
                .unwrap_err()
                .code,
            MOVE_INVALID
        );
        assert_eq!(
            move_face(&model, 0, [0., 0., 0.], 1., &budget())
                .unwrap_err()
                .code,
            MOVE_INVALID
        );
        assert_eq!(
            move_face(&model, 0, [1., 0., 0.], f64::NAN, &budget())
                .unwrap_err()
                .code,
            MOVE_INVALID
        );
        assert_eq!(model, before);
    }

    // ------------------------------------------------------------------
    // offset face (872)
    // ------------------------------------------------------------------

    /// Radial distance of every surface sample of `faces` to `axis`;
    /// returns (min, max) over a small grid — a wall-thickness probe.
    fn radial_samples(model: &Model, faces: &[usize], axis: &Axis) -> (f64, f64) {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for &f in faces {
            let sampler = nurbs_core::surface::SurfaceSampler::new(&model.faces[f].surface).unwrap();
            for i in 0..=4 {
                for j in 0..=4 {
                    let p = sampler
                        .evaluate(i as f64 / 4., j as f64 / 4.)
                        .unwrap()
                        .point;
                    let r = radial_distance(axis, [p[0], p[1], p[2]]);
                    lo = lo.min(r);
                    hi = hi.max(r);
                }
            }
        }
        (lo, hi)
    }

    fn tube_wall_faces(model: &Model) -> (Vec<usize>, Vec<usize>, Axis) {
        let mut aag = Aag::build(model, &budget()).unwrap();
        aag.attach_face_attrs(model, &budget()).unwrap();
        let mut walls: Vec<(usize, f64)> = vec![];
        let mut axis = None;
        for (i, node) in aag.nodes.iter().enumerate() {
            let a = node.attrs.as_ref().unwrap();
            if a.class == SurfaceClass::Cylinder {
                axis = a.axis;
                walls.push((i, a.radius.unwrap()));
            }
        }
        // Inner wall = the smaller of the two fitted radii (radius-agnostic,
        // so the helper works before AND after the offset).
        let mut radii: Vec<f64> = walls.iter().map(|(_, r)| *r).collect();
        radii.sort_by(f64::total_cmp);
        radii.dedup_by(|b, a| (*a - *b).abs() < 1e-3);
        assert_eq!(radii.len(), 2, "a tube has two coaxial wall radii");
        let (inner_r, outer_r) = (radii[0], radii[1]);
        let inner = walls
            .iter()
            .filter(|(_, r)| (*r - inner_r).abs() < 1e-3)
            .map(|(i, _)| *i)
            .collect();
        let outer = walls
            .iter()
            .filter(|(_, r)| (*r - outer_r).abs() < 1e-3)
            .map(|(i, _)| *i)
            .collect();
        (inner, outer, axis.expect("tube walls have an axis"))
    }
    #[test]
    fn offset_tube_wall_auto_expands_to_concentric_pair() {
        let model = crate::analytic::tube(5., 2., 6.).unwrap();
        let (inner, outer, axis) = tube_wall_faces(&model);
        assert!(!inner.is_empty() && !outer.is_empty());
        let before = model.clone();

        let report = offset_face(&model, inner[0], 0.5, OffsetOptions::default(), &budget())
            .unwrap();
        assert_eq!(report.kind, "cylinder-radius");
        assert!(report.auto_expanded, "pair auto-expanded");
        // Both walls moved: sibling patches of both cylinders.
        for &f in inner.iter().chain(&outer) {
            assert!(report.offset_faces.contains(&f), "face {f} offset");
        }
        assert_eq!(report.old_radii.len(), 2);
        assert!((report.old_radii[0] - 2.).abs() < 1e-3 && (report.old_radii[1] - 5.).abs() < 1e-3);
        assert!((report.new_radii[0] - 2.5).abs() < 1e-9 && (report.new_radii[1] - 5.5).abs() < 1e-9);
        // Wall thickness preserved exactly (pair offset by the same delta).
        let tb = report.wall_thickness_before_mm.unwrap();
        let ta = report.wall_thickness_after_mm.unwrap();
        assert!((tb - 3.).abs() < 1e-3 && (ta - tb).abs() < 1e-9, "{tb} -> {ta}");
        report.model.validate().unwrap();

        // Acceptance probe: wall thickness in a sample grid, 1e-9 · scale.
        let scale = 6.;
        let (inner_after, outer_after, axis_after) = tube_wall_faces(&report.model);
        let (ilo, ihi) = radial_samples(&report.model, &inner_after, &axis_after);
        let (olo, ohi) = radial_samples(&report.model, &outer_after, &axis_after);
        assert!((ilo - 2.5).abs() < 1e-9 * scale && (ihi - 2.5).abs() < 1e-9 * scale,
            "inner radius samples: {ilo}..{ihi}");
        assert!((olo - 5.5).abs() < 1e-9 * scale && (ohi - 5.5).abs() < 1e-9 * scale,
            "outer radius samples: {olo}..{ohi}");
        let thickness = olo - ihi;
        assert!((thickness - 3.).abs() < 1e-9 * scale, "thickness {thickness}");
        let _ = axis;

        // ΔV = π h [(5.5²−5²) − (2.5²−2²)] = π·6·(5.25−2.25) = 18π.
        let expected = std::f64::consts::PI * 6. * (5.25 - 2.25);
        assert!(
            (report.delta_volume_estimate_mm3 - expected).abs() < 1e-6 * expected,
            "estimate {} vs {expected}",
            report.delta_volume_estimate_mm3
        );
        assert!(
            (report.delta_volume_actual_mm3 - expected).abs() < 2e-3 * expected,
            "actual ΔV {} vs {expected}",
            report.delta_volume_actual_mm3
        );
        // Geometry-only mutation: persistent ids untouched everywhere.
        assert_eq!(report.model.1.faces, before.1.faces);
        assert_eq!(report.model.1.edges, before.1.edges);
        assert_eq!(report.model.1.vertices, before.1.vertices);
        assert_eq!(model, before);
    }

    #[test]
    fn offset_single_member_of_pair_strict_refuses_with_diagnosis() {
        let model = crate::analytic::tube(5., 2., 6.).unwrap();
        let (inner, outer, _) = tube_wall_faces(&model);
        let before = model.clone();
        let error = offset_face(
            &model,
            inner[0],
            0.5,
            OffsetOptions { strict: true },
            &budget(),
        )
        .unwrap_err();
        assert_eq!(error.code, OFFSET_UNSUPPORTED, "{error:?}");
        assert!(
            outer.iter().any(|f| error.message.contains(&f.to_string())),
            "partner faces named: {}",
            error.message
        );
        assert_eq!(model, before);
    }

    #[test]
    fn offset_solid_cylinder_changes_radius() {
        let model = crate::cylinder(2., 5.).unwrap();
        let (side,) = {
            let mut aag = Aag::build(&model, &budget()).unwrap();
            aag.attach_face_attrs(&model, &budget()).unwrap();
            let side: Vec<usize> = aag
                .nodes
                .iter()
                .enumerate()
                .filter(|(_, n)| {
                    n.attrs.as_ref().unwrap().class == SurfaceClass::Cylinder
                })
                .map(|(i, _)| i)
                .collect();
            assert!(!side.is_empty());
            (side,)
        };
        let before = model.clone();
        let report = offset_face(&model, side[0], 1., OffsetOptions::default(), &budget())
            .unwrap();
        assert!(!report.auto_expanded, "no concentric partner on a solid");
        assert_eq!(report.old_radii.len(), 1);
        assert!((report.new_radii[0] - 3.).abs() < 1e-9);
        report.model.validate().unwrap();
        // ΔV = π·5·(9−4) = 25π.
        let expected = std::f64::consts::PI * 5. * (9. - 4.);
        assert!(
            (report.delta_volume_actual_mm3 - expected).abs() < 2e-3 * expected,
            "actual ΔV {} vs {expected}",
            report.delta_volume_actual_mm3
        );
        assert!(
            (report.delta_volume_estimate_mm3 - expected).abs() < 1e-6 * expected,
            "estimate {}",
            report.delta_volume_estimate_mm3
        );
        assert_eq!(model, before);
    }

    #[test]
    fn offset_cylinder_next_to_fillet_is_refused_typed() {
        // Rounded cuboid: the fillet cylinder rolls tangentially on two
        // planar walls; changing its radius would tear the tangency.
        let sharp = crate::cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
        let edges: Vec<usize> = sharp
            .edges
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                let a = sharp.vertices[e.vertices[0]].point;
                let b = sharp.vertices[e.vertices[1]].point;
                ((a[0] - b[0]).abs() <= 1e-12 && (a[1] - b[1]).abs() <= 1e-12).then_some(i)
            })
            .collect();
        let (rounded, _) = crate::analytic_features::analytic_fillet(&sharp, edges[0], 1.).unwrap();
        let mut aag = Aag::build(&rounded, &budget()).unwrap();
        aag.attach_face_attrs(&rounded, &budget()).unwrap();
        let fillet = aag
            .nodes
            .iter()
            .enumerate()
            .find_map(|(i, n)| {
                (n.attrs.as_ref().unwrap().class == SurfaceClass::Cylinder).then_some(i)
            })
            .expect("fillet cylinder");
        let before = rounded.clone();
        let error = offset_face(&rounded, fillet, 0.5, OffsetOptions::default(), &budget())
            .unwrap_err();
        assert_eq!(error.code, OFFSET_UNSUPPORTED, "{error:?}");
        assert!(
            error.message.contains("tangent") || error.message.contains("blend"),
            "blend diagnosis: {}",
            error.message
        );
        assert_eq!(rounded, before);
    }

    #[test]
    fn offset_planar_face_delegates_to_move() {
        let model = crate::cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
        let wall = face_at(&model, 0, 10.);
        let before = model.clone();
        let report = offset_face(&model, wall, 2., OffsetOptions::default(), &budget()).unwrap();
        assert_eq!(report.kind, "planar");
        assert!((report.volume_after_mm3 - 1200.).abs() < 1e-6 * 1200.);
        assert!(
            (report.delta_volume_actual_mm3 - 200.).abs() < 1e-6 * 200.,
            "actual ΔV {}",
            report.delta_volume_actual_mm3
        );
        assert_eq!(model, before);
    }

    #[test]
    fn offset_unsupported_classes_are_refused_typed() {
        // Torus and cone faces are outside offset-face/1.
        let torus = crate::torus(4., 1.).unwrap();
        let before = torus.clone();
        let face = torus
            .faces
            .iter()
            .enumerate()
            .find_map(|(i, f)| {
                (f.surface.degree_u > 1 || f.surface.degree_v > 1).then_some(i)
            })
            .expect("torus side");
        let error = offset_face(&torus, face, 0.5, OffsetOptions::default(), &budget())
            .unwrap_err();
        assert_eq!(error.code, OFFSET_UNSUPPORTED, "{error:?}");
        assert_eq!(torus, before);

        let cone = crate::analytic::frustum(3., 1.5, 4.).unwrap();
        let before = cone.clone();
        let error = offset_face(&cone, 0, 0.5, OffsetOptions::default(), &budget()).unwrap_err();
        assert!(
            error.code == OFFSET_UNSUPPORTED || error.code == OFFSET_INVALID,
            "{error:?}"
        );
        assert_eq!(cone, before);

        // Offset consuming the bore is a topology mutation: refused.
        let model = crate::analytic::tube(5., 2., 6.).unwrap();
        let (inner, _, _) = tube_wall_faces(&model);
        let before = model.clone();
        let error = offset_face(&model, inner[0], -2.5, OffsetOptions::default(), &budget())
            .unwrap_err();
        assert_eq!(error.code, OFFSET_UNSUPPORTED, "{error:?}");
        assert_eq!(model, before);
    }
}
