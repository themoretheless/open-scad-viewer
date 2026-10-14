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

mod suppression;
use suppression::*;


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

mod rims;
use rims::*;


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

mod movement;
pub use movement::{MoveFaceReport, move_face};


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

mod offset;
pub use offset::{OffsetOptions, OffsetFaceReport, offset_face};


// ---------------------------------------------------------------------------
// Tests.
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "tests/direct_edit.rs"]
mod tests;
