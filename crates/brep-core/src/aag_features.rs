//! Template matching over the [`Aag`]: hole recognition (checklist 866) and
//! pocket recognition (checklist 867).
//!
//! A hole is a cluster of cylindrical faces (one logical cylinder may be
//! split into several patches — a tube's inner wall is four quadrant
//! patches) whose outward normals point *toward* the bore axis: the cylinder
//! encloses void, not material. This orientation test, not edge convexity,
//! is the hole/boss discriminator — the opening rim of a blind hole is a
//! convex edge (90° material wedge) while its floor ring is concave, so "all
//! edges concave" is not a viable signature in this kernel's sign convention.
//! Boundary edges of the cluster must still be matchable (convex, concave,
//! tangent or smooth); mixed/degenerate/seam evidence rejects the candidate.
//! Matching is by **angular range**, not
//! circle completeness: a half-hole on the body boundary (cylinder spanning
//! π) matches exactly like a full one, and the coverage is reported as
//! [`HoleFeature::angle_span`].
//!
//! Clustering: sibling cylinder faces connected by smooth edges, sharing an
//! axis (direction and point-on-line within tolerance) and radius, are
//! unioned into one feature. This absorbs both periodic-surface splits and
//! intersecting holes on a common axis; holes on *different* axes stay
//! separate features by construction. Documented choice: coaxial clusters
//! with different radii are NOT merged — the larger one at the entry side is
//! reported as the counterbore step of the smaller ([`HoleEntry::Counterbore`]).
//!
//! Through/blind is decided by a ray-parity probe just past each bore end on
//! the axis (inside material = floor, outside = open) — sign-convention-free
//! and uniform for full, half and stepped holes.
//!
//! Threaded holes: `HoleEntry::Threaded` is reserved but **never returned** —
//! the kernel's analytic corpus has no helical ridge geometry to detect, and
//! no thread attribute exists on `FaceAttrs`. The variant is the seam for an
//! attribute-driven pipeline (e.g. imported STEP with thread callouts) and is
//! documented as unreachable from pure geometry.
//!
//! # Pockets (867)
//!
//! A pocket is a subgraph of floor faces (planes) plus wall faces (planes,
//! cones or cylinders — round pockets) reachable from a floor seed across
//! concave edges, with sibling patches and **fillet chains collapsed**:
//! [`crate::aag_fillets::find_fillet_chains`] runs first and every chain is
//! contracted into a transparent supernode, so a pocket with a floor blend
//! matches the same floor+wall template (the chain becomes a virtual edge
//! between its supports). Validation: internal edges must be concave/smooth,
//! boundary (rim) edges convex; wall outward normals must point into the
//! cavity — verified by a ray-parity probe just past each wall. This probe,
//! not the edge-sign pattern, is the pocket/boss discriminator: a boss on a
//! plate yields the mirror subgraph (concave base ring, convex top), and the
//! probe lands inside material there. Nested pockets report a hierarchy via
//! [`PocketFeature::parent`]/[`PocketFeature::children`], derived from rim
//! faces landing inside another pocket. Open vs closed
//! ([`PocketKind`]): a floor face reaching the exterior through a convex
//! boundary edge means the pocket opens to the body boundary. Documented
//! limits: multi-floor pockets with islands, rim fillets and floor steps are
//! out of scope (conservative rejection, never a guess); a round blind
//! pocket also satisfies the hole template — the matchers are independent
//! and overlapping there is expected.

use crate::aag::{Aag, DihedralClass, FaceAttrs};
use crate::analysis::surface_classify::{Axis, SurfaceClass};
use crate::{Error, Model, Result};
use nurbs_core::foundation::guards::{Budget, BudgetGuard, require_finite_f64};
use nurbs_core::surface::SurfaceSampler;

/// Sample grid side per face for angular-span and axial-extent estimation.
const HOLE_SAMPLE_GRID: usize = 9;
/// Axis-direction alignment tolerance for coaxial clustering (radians, via
/// `1 - |dot|` on unit directions).
const AXIS_DOT_TOLERANCE: f64 = 1e-6;
/// Relative radius tolerance for clustering sibling cylinder patches.
const RADIUS_REL_TOLERANCE: f64 = 1e-4;

fn features_error(message: impl Into<String>) -> Error {
    Error::new("BREP_AAG_FEATURES_INPUT", message)
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// Through/blind classification of a hole.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HoleKind {
    /// Both axial ends open (the cylinder pierces the body).
    Through,
    /// One end closed by a floor face (the hole has a bottom).
    Blind,
}

/// Entry geometry at the open end of a hole.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HoleEntry {
    /// Cylinder meets the exterior plane directly.
    Plain,
    /// A coaxial cone flares at the entry (countersink).
    Countersink,
    /// A coaxial larger cylinder steps at the entry (counterbore).
    Counterbore,
    /// Reserved for attribute-driven thread tagging. Never produced from
    /// geometry alone: no helical ridge heuristic is implemented and
    /// [`FaceAttrs`] carries no thread flag. Do not match on this variant
    /// as reachable.
    Threaded,
}

/// One recognized hole: the wall faces, its axis and size, and classification.
#[derive(Clone, Debug, PartialEq)]
pub struct HoleFeature {
    /// Cylindrical wall faces of the main bore (`model.faces` indices,
    /// ascending). Sibling patches of one logical cylinder are merged.
    pub faces: Vec<usize>,
    /// Faces of the entry step (cone for countersink, larger cylinder for
    /// counterbore), ascending; empty for a plain entry.
    pub step_faces: Vec<usize>,
    /// Floor faces closing a blind hole, ascending; empty for through holes.
    pub bottom_faces: Vec<usize>,
    pub kind: HoleKind,
    pub entry: HoleEntry,
    /// Best-fit bore axis (from 865 attributes of the wall faces).
    pub axis: Axis,
    /// Bore diameter = 2 × best-fit cylinder radius.
    pub diameter: f64,
    /// Axial extent of the sampled bore wall (main cylinder only; the entry
    /// step is not included).
    pub depth: f64,
    /// Total angular coverage of the bore wall around the axis, radians in
    /// (0, 2π]. Full holes read ≈ 2π; a half-hole on the body boundary
    /// reads ≈ π.
    pub angle_span: f64,
}

/// Coaxiality check: unit directions parallel and point of B on the line of A.
fn coaxial(a: &Axis, b: &Axis, linear_tol: f64) -> bool {
    let align = dot(a.direction, b.direction).abs();
    if 1. - align > AXIS_DOT_TOLERANCE {
        return false;
    }
    let delta = sub(b.point, a.point);
    let off = sub(delta, crate_scale(a.direction, dot(delta, a.direction)));
    norm(off) <= linear_tol
}

fn crate_scale(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}

/// Sample points of one face on its surface grid (budget-charged).
fn face_samples(
    model: &Model,
    face: usize,
    guard: &mut BudgetGuard,
) -> Result<Vec<[f64; 3]>> {
    let sampler = SurfaceSampler::new(&model.faces[face].surface)?;
    let n = HOLE_SAMPLE_GRID;
    let mut points = Vec::with_capacity(n * n);
    for i in 0..n {
        for j in 0..n {
            guard.tick()?;
            // Endpoints included: the axial extent must reach the rims.
            let u = i as f64 / (n - 1) as f64;
            let v = j as f64 / (n - 1) as f64;
            let p = sampler.evaluate(u, v)?.point;
            if p.len() != 3 {
                return Err(features_error("hole matching expects 3D surface points"));
            }
            require_finite_f64(p[0], "sample.x")?;
            require_finite_f64(p[1], "sample.y")?;
            require_finite_f64(p[2], "sample.z")?;
            points.push([p[0], p[1], p[2]]);
        }
    }
    Ok(points)
}

/// Angular coverage around `axis` of `points`: sort by angle, subtract the
/// largest gap from 2π. Robust to patches and to any starting meridian.
fn angular_span(points: &[[f64; 3]], axis: &Axis) -> f64 {
    // Local orthonormal basis ⟂ axis.
    let d = axis.direction;
    let seed = if d[0].abs() <= d[1].abs() && d[0].abs() <= d[2].abs() {
        [1., 0., 0.]
    } else if d[1].abs() <= d[2].abs() {
        [0., 1., 0.]
    } else {
        [0., 0., 1.]
    };
    let e1_raw = cross(d, seed);
    let e1 = crate_scale(e1_raw, 1. / norm(e1_raw));
    let e2 = cross(d, e1);
    let mut angles: Vec<f64> = points
        .iter()
        .map(|p| {
            let rel = sub(*p, axis.point);
            let axial = crate_scale(d, dot(rel, d));
            let radial = sub(rel, axial);
            let x = dot(radial, e1);
            let y = dot(radial, e2);
            y.atan2(x)
        })
        .collect();
    if angles.len() < 2 {
        return 0.;
    }
    angles.sort_by(|a, b| a.total_cmp(b));
    let tau = 2. * std::f64::consts::PI;
    let mut gap = 0f64;
    for i in 0..angles.len() {
        let next = if i + 1 == angles.len() {
            angles[0] + tau
        } else {
            angles[i + 1]
        };
        gap = gap.max(next - angles[i]);
    }
    (tau - gap).clamp(0., tau)
}

/// Neighbor faces of `face` across its edges of the requested classes.
fn neighbors_via(
    graph: &Aag,
    face: usize,
    accept: impl Fn(DihedralClass) -> bool,
) -> Vec<usize> {
    let mut out: Vec<usize> = graph.nodes[face]
        .edges
        .iter()
        .filter(|&&e| accept(graph.edges[e].class))
        .flat_map(|&e| graph.edges[e].uses.iter().map(|u| u.face))
        .filter(|&other| other != face)
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// Hole/boss discriminator: the shell's outward normals at wall samples.
/// On a hole the cylinder encloses void, so outward normals point toward the
/// axis (mean radial dot < 0); on a boss they point away. Samples must agree
/// decisively (|mean| > 0.5 on unit vectors); an ambiguous wall is not a hole.
fn is_void_wall(
    model: &Model,
    faces: &[usize],
    axis: &Axis,
    guard: &mut BudgetGuard,
) -> Result<bool> {
    let mut sum = 0f64;
    let mut count = 0usize;
    for &face in faces {
        guard.tick()?;
        // Outward normal flips with the shell's reversed use of the face.
        let reversed = model
            .shells
            .iter()
            .flat_map(|s| s.faces.iter())
            .find(|u| u.face == face)
            .is_some_and(|u| u.reversed);
        let sampler = SurfaceSampler::new(&model.faces[face].surface)?;
        for i in 0..3 {
            guard.tick()?;
            let u = (i as f64 + 0.5) / 3.;
            let v = (i as f64 + 0.5) / 3.;
            let eval = sampler.evaluate(u, v)?;
            let Some(mut n) = eval.unit_normal() else {
                continue;
            };
            if reversed {
                n = [-n[0], -n[1], -n[2]];
            }
            let p = &eval.point;
            if p.len() != 3 {
                return Err(features_error("hole matching expects 3D surface points"));
            }
            let rel = sub([p[0], p[1], p[2]], axis.point);
            let radial = sub(rel, crate_scale(axis.direction, dot(rel, axis.direction)));
            let r = norm(radial);
            if r <= 0. {
                continue;
            }
            sum += dot(n, crate_scale(radial, 1. / r));
            count += 1;
        }
    }
    if count == 0 {
        return Ok(false);
    }
    Ok(sum / (count as f64) < -0.5)
}

/// Entry-step signature of one neighbor face: a coaxial cone (countersink)
/// or a coaxial strictly larger cylinder (counterbore).
fn step_kind(fa: &FaceAttrs, axis: &Axis, radius: f64, linear_tol: f64) -> Option<HoleEntry> {
    match fa.class {
        SurfaceClass::Cone if fa.axis.is_some_and(|a| coaxial(&a, axis, linear_tol)) => {
            Some(HoleEntry::Countersink)
        }
        SurfaceClass::Cylinder
            if fa.axis.is_some_and(|a| coaxial(&a, axis, linear_tol))
                && fa.radius.is_some_and(|r| r > radius * (1. + RADIUS_REL_TOLERANCE)) =>
        {
            Some(HoleEntry::Counterbore)
        }
        _ => None,
    }
}

/// Union-find over cylinder faces linked by smooth edges with same axis/radius.
fn cluster_cylinders(graph: &Aag, linear_tol: f64) -> Vec<Vec<usize>> {
    let cylinders: Vec<usize> = graph
        .nodes
        .iter()
        .filter(|n| {
            n.attrs
                .as_ref()
                .is_some_and(|a| a.class == SurfaceClass::Cylinder && a.axis.is_some() && a.radius.is_some())
        })
        .map(|n| n.face)
        .collect();
    let mut parent: Vec<usize> = (0..cylinders.len()).collect();
    fn root(parent: &mut Vec<usize>, i: usize) -> usize {
        let mut r = i;
        while parent[r] != r {
            r = parent[r];
        }
        let mut c = i;
        while parent[c] != c {
            let next = parent[c];
            parent[c] = r;
            c = next;
        }
        r
    }
    let index_of = |face: usize| cylinders.iter().position(|&f| f == face);
    for (i, &face) in cylinders.iter().enumerate() {
        let attrs = graph.nodes[face].attrs.as_ref().expect("filtered above");
        let (axis_a, radius_a) = (attrs.axis.unwrap(), attrs.radius.unwrap());
        for sib in neighbors_via(graph, face, |c| c == DihedralClass::Smooth) {
            let Some(j) = index_of(sib) else { continue };
            let sattrs = graph.nodes[sib].attrs.as_ref().expect("filtered above");
            let (axis_b, radius_b) = (sattrs.axis.unwrap(), sattrs.radius.unwrap());
            let same_radius =
                (radius_a - radius_b).abs() <= RADIUS_REL_TOLERANCE * radius_a.max(radius_b);
            if same_radius && coaxial(&axis_a, &axis_b, linear_tol) {
                let (ra, rb) = (root(&mut parent, i), root(&mut parent, j));
                if ra != rb {
                    parent[ra] = rb;
                }
            }
        }
    }
    let mut groups: std::collections::BTreeMap<usize, Vec<usize>> = Default::default();
    for (i, &face) in cylinders.iter().enumerate() {
        let r = root(&mut parent, i);
        groups.entry(r).or_default().push(face);
    }
    groups.into_values().collect()
}

/// Find holes in `model` by template matching over `graph` (checklist 866).
///
/// `graph` must carry face attributes (run [`Aag::attach_face_attrs`] first);
/// stale or missing attributes are a typed input error, never a silent skip.
/// Every candidate face, edge and sample grid ticks `budget`; exhaustion
/// aborts with a typed resource error.
pub fn find_holes(model: &Model, graph: &Aag, budget: &Budget) -> Result<Vec<HoleFeature>> {
    require_finite_f64(model.tolerance_mm, "model.tolerance_mm")?;
    if graph.nodes.len() != model.faces.len() {
        return Err(features_error("AAG node count does not match model faces"));
    }
    if !graph.stale_face_attrs(model).is_empty() {
        return Err(features_error(
            "AAG face attributes missing or stale; run attach_face_attrs first",
        ));
    }
    let mut guard: BudgetGuard = budget.guard("aag-find-holes");
    guard.check()?;
    // Linear tolerance for coaxial checks: a small multiple of the modeling
    // tolerance, so genuine axis splits merge but distinct holes do not.
    let linear_tol = (model.tolerance_mm * 100.).max(1e-7);

    let clusters = cluster_cylinders(graph, linear_tol);
    let mut holes = Vec::new();
    for faces in clusters {
        guard.tick()?;
        // Boundary edges of the cluster: incident edges with a use outside it.
        let mut boundary_edges: Vec<usize> = Vec::new();
        let mut reject = false;
        for &face in &faces {
            guard.tick()?;
            for &e in &graph.nodes[face].edges {
                let edge = &graph.edges[e];
                if edge.uses.iter().all(|u| faces.contains(&u.face)) {
                    continue; // internal sibling seam
                }
                match edge.class {
                    // Non-matchable evidence (mixed sign, degenerate, open
                    // boundary) must never feed a template verdict.
                    c if !c.is_matchable() => {
                        reject = true;
                        break;
                    }
                    _ => boundary_edges.push(e),
                }
            }
            if reject {
                break;
            }
        }
        if reject || boundary_edges.is_empty() {
            continue;
        }

        let first = graph.nodes[faces[0]].attrs.as_ref().expect("clustered");
        let axis = first.axis.unwrap();
        let radius = faces
            .iter()
            .map(|&f| graph.nodes[f].attrs.as_ref().unwrap().radius.unwrap())
            .sum::<f64>()
            / faces.len() as f64;

        // Hole vs boss: only a cylinder enclosing void (outward normals
        // toward the axis) is a bore wall.
        if !is_void_wall(model, &faces, &axis, &mut guard)? {
            continue;
        }

        // Sampled geometry: axial extent and angular coverage of the wall.
        let mut points = Vec::new();
        for &face in &faces {
            points.extend(face_samples(model, face, &mut guard)?);
        }
        let mut t_min = f64::INFINITY;
        let mut t_max = f64::NEG_INFINITY;
        for p in &points {
            let t = dot(sub(*p, axis.point), axis.direction);
            t_min = t_min.min(t);
            t_max = t_max.max(t);
        }
        let depth = t_max - t_min;
        let angle_span = angular_span(&points, &axis);

        // Assign each boundary edge to an axial end by its midpoint, or mark
        // it as a side edge (cut-plane edges of a partial hole span the full
        // depth and belong to neither end).
        let mut end_neighbors: [Vec<usize>; 2] = [Vec::new(), Vec::new()]; // [min, max]
        let quarter = depth * 0.25;
        for &e in &boundary_edges {
            guard.tick()?;
            let edge = &model.edges[graph.edges[e].edge];
            let domain = edge.curve.domain();
            let mid = edge.curve.evaluate(0.5 * (domain[0] + domain[1]))?.point;
            if mid.len() != 3 {
                return Err(features_error("hole matching expects 3D edge points"));
            }
            let t = dot(sub([mid[0], mid[1], mid[2]], axis.point), axis.direction);
            let side = if t - t_min <= quarter {
                Some(0)
            } else if t_max - t <= quarter {
                Some(1)
            } else {
                None
            };
            if let Some(s) = side {
                for u in &graph.edges[e].uses {
                    if !faces.contains(&u.face) && !end_neighbors[s].contains(&u.face) {
                        end_neighbors[s].push(u.face);
                    }
                }
            }
        }

        // Classify each end. Through/blind is decided by a ray-parity probe
        // just past the end on the bore axis: inside material = floor (blind),
        // outside = open. This is sign-convention-free and works uniformly
        // for full holes, half-holes (where the axis lies on a cut plane —
        // the probe is offset toward the void side) and stepped holes (the
        // bore end behind a counterbore shoulder probes the wider void).
        let mut closed = [false, false];
        let mut step_faces: Vec<usize> = Vec::new();
        let mut bottom_faces: Vec<usize> = Vec::new();
        let mut entry = HoleEntry::Plain;
        // Mean radial direction of wall samples: points into the void.
        let mut void_dir = [0.; 3];
        for p in &points {
            let rel = sub(*p, axis.point);
            let radial = sub(rel, crate_scale(axis.direction, dot(rel, axis.direction)));
            void_dir = [
                void_dir[0] + radial[0],
                void_dir[1] + radial[1],
                void_dir[2] + radial[2],
            ];
        }
        let void_len = norm(void_dir);
        let void_dir = if void_len > 0. {
            crate_scale(void_dir, 1. / void_len)
        } else {
            [0.; 3]
        };
        let eps = (1e-3 * depth).max(10. * model.tolerance_mm);
        let probe_offset = 0.25 * radius;
        let probe_dirs = [
            [1., 0.317, 0.173],
            [-0.219, 1., 0.413],
            [0.271, -0.193, 1.],
        ];
        for s in 0..2 {
            guard.tick()?;
            let t_end = if s == 0 { t_min } else { t_max };
            let sign = if s == 0 { -1. } else { 1. };
            let probe = [
                axis.point[0] + axis.direction[0] * (t_end + sign * eps) + void_dir[0] * probe_offset,
                axis.point[1] + axis.direction[1] * (t_end + sign * eps) + void_dir[1] * probe_offset,
                axis.point[2] + axis.direction[2] * (t_end + sign * eps) + void_dir[2] * probe_offset,
            ];
            guard.tick()?;
            let report = crate::ray_parity::classify_point(
                model, probe, &probe_dirs, model.tolerance_mm, 10_000, 200_000,
            )?;
            closed[s] = report.parity == Some(true);
            if closed[s] {
                bottom_faces.extend(end_neighbors[s].iter().copied());
            }

            // Entry step detection from the graph neighborhood at this end:
            // a coaxial cone (countersink) or larger coaxial cylinder
            // (counterbore), reached directly or across a shoulder plane.
            for &nb in &end_neighbors[s] {
                guard.tick()?;
                let attrs = graph.nodes[nb].attrs.as_ref().expect("stale checked");
                if let Some(kind) = step_kind(attrs, &axis, radius, linear_tol) {
                    if entry == HoleEntry::Plain {
                        entry = kind;
                    }
                    step_faces.push(nb);
                    continue;
                }
                // Shoulder plane: look across its far ring (concave edges
                // only — its convex edges lead back into the bore itself,
                // and an exterior cap's far side is the body wall reached
                // through convex edges, which must not read as a step).
                if attrs.class == SurfaceClass::Plane {
                    for far in neighbors_via(graph, nb, |c| c == DihedralClass::Concave) {
                        if far == nb || faces.contains(&far) {
                            continue;
                        }
                        let fa = graph.nodes[far].attrs.as_ref().expect("stale checked");
                        if matches!(fa.class, SurfaceClass::Plane) {
                            continue; // sibling floor/shoulder patch
                        }
                        if let Some(kind) = step_kind(fa, &axis, radius, linear_tol) {
                            if entry == HoleEntry::Plain {
                                entry = kind;
                            }
                            step_faces.push(far);
                            step_faces.push(nb); // keep the shoulder too
                        }
                    }
                }
            }
        }

        let kind = match (closed[0], closed[1]) {
            (true, true) => continue,   // closed cavity, not a hole
            (false, false) => HoleKind::Through,
            _ => HoleKind::Blind,
        };
        step_faces.sort_unstable();
        step_faces.dedup();
        bottom_faces.sort_unstable();
        bottom_faces.dedup();
        let mut faces = faces;
        faces.sort_unstable();
        holes.push(HoleFeature {
            faces,
            step_faces,
            bottom_faces,
            kind,
            entry,
            axis,
            diameter: 2. * radius,
            depth,
            angle_span,
        });
    }
    // Counterbore merge: the larger coaxial step cylinder is itself a
    // cylinder cluster and gets recognized as its own shallow "hole". Fold
    // it into the bore it steps: any hole whose faces are all listed as
    // another hole's step faces is a step, not a standalone feature.
    let standalone: Vec<bool> = holes
        .iter()
        .enumerate()
        .map(|(i, h)| {
            !holes.iter().enumerate().any(|(j, other)| {
                i != j && h.faces.iter().all(|f| other.step_faces.contains(f))
            })
        })
        .collect();
    let mut holes: Vec<HoleFeature> = holes
        .into_iter()
        .zip(standalone)
        .filter_map(|(h, keep)| keep.then_some(h))
        .collect();
    holes.sort_by_key(|h| h.faces[0]);
    guard.check()?;
    Ok(holes)
}

// ==========================================================================
// Pocket recognition (checklist 867)
// ==========================================================================

fn pockets_error(message: impl Into<String>) -> Error {
    Error::new("BREP_AAG_POCKETS_INPUT", message)
}

/// Open/closed classification of a pocket (by its mouth).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PocketKind {
    /// The floor is fully enclosed by walls: the mouth is a closed rim loop.
    Closed,
    /// At least one floor face reaches the exterior through a convex
    /// boundary edge: the pocket opens to the body boundary (slot).
    Open,
}

/// Wall slope classification of a pocket.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PocketWalls {
    /// Wall outward normals are perpendicular to the floor normal.
    Straight,
    /// At least one wall leans off perpendicular (draft angle reported).
    Drafted,
}

/// One recognized pocket: floor, walls, absorbed floor fillets, parameters
/// and nesting hierarchy.
#[derive(Clone, Debug, PartialEq)]
pub struct PocketFeature {
    /// All member faces (floor + wall + fillet), ascending.
    pub faces: Vec<usize>,
    pub floor_faces: Vec<usize>,
    pub wall_faces: Vec<usize>,
    /// Faces of collapsed fillet chains absorbed into the subgraph (floor
    /// blends), ascending.
    pub fillet_faces: Vec<usize>,
    /// Rim faces outside the pocket that its boundary edges land on,
    /// ascending. For a nested pocket these lie inside the parent.
    pub rim_faces: Vec<usize>,
    pub kind: PocketKind,
    pub walls: PocketWalls,
    /// Max wall lean-off from perpendicular, degrees; `Some` only for
    /// [`PocketWalls::Drafted`]. Positive means the mouth is wider than the
    /// floor (draft opens upward).
    pub draft_angle_deg: Option<f64>,
    /// Rim-to-floor distance along the floor normal.
    pub depth: f64,
    /// Sum of trimmed floor face areas.
    pub floor_area: f64,
    /// Outward normal of the floor (points into the cavity).
    pub floor_normal: [f64; 3],
    /// Index of the enclosing pocket in the returned vector, if nested.
    pub parent: Option<usize>,
    /// Indices of pockets nested directly inside this one.
    pub children: Vec<usize>,
}

/// Outward unit normal of one face at (u, v), honoring the shell's reversed
/// use; `None` at poles/singular charts.
fn outward_normal(
    model: &Model,
    face: usize,
    u: f64,
    v: f64,
) -> Result<Option<[f64; 3]>> {
    let reversed = model
        .shells
        .iter()
        .flat_map(|s| s.faces.iter())
        .find(|use_| use_.face == face)
        .is_some_and(|use_| use_.reversed);
    let sampler = SurfaceSampler::new(&model.faces[face].surface)?;
    let Some(mut n) = sampler.evaluate(u, v)?.unit_normal() else {
        return Ok(None);
    };
    if reversed {
        n = [-n[0], -n[1], -n[2]];
    }
    Ok(Some(n))
}

/// Surface point of one face at (u, v).
fn face_point(model: &Model, face: usize, u: f64, v: f64) -> Result<[f64; 3]> {
    let sampler = SurfaceSampler::new(&model.faces[face].surface)?;
    let p = sampler.evaluate(u, v)?.point;
    if p.len() != 3 {
        return Err(pockets_error("pocket matching expects 3D surface points"));
    }
    Ok([p[0], p[1], p[2]])
}

/// Ray-parity probe: is `point` inside material? `None` = unresolved.
fn probe_inside(model: &Model, point: [f64; 3]) -> Result<Option<bool>> {
    let dirs = [[1., 0.317, 0.173], [-0.219, 1., 0.413], [0.271, -0.193, 1.]];
    Ok(
        crate::ray_parity::classify_point(model, point, &dirs, model.tolerance_mm, 10_000, 200_000)?
            .parity,
    )
}

/// Role of a supernode inside a candidate pocket subgraph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PocketRole {
    Floor,
    Wall,
    /// Collapsed fillet chain (transparent blend between floor and wall).
    Fillet,
}

/// Find pockets in `model` by subgraph matching over `graph` (checklist 867).
///
/// Same contract as [`find_holes`]: `graph` must carry fresh face
/// attributes; work is charged to `budget`. Floor fillet chains from
/// [`crate::aag_fillets::find_fillet_chains`] are collapsed into transparent
/// supernodes before matching.
pub fn find_pockets(model: &Model, graph: &Aag, budget: &Budget) -> Result<Vec<PocketFeature>> {
    require_finite_f64(model.tolerance_mm, "model.tolerance_mm")?;
    if graph.nodes.len() != model.faces.len() {
        return Err(pockets_error("AAG node count does not match model faces"));
    }
    if !graph.stale_face_attrs(model).is_empty() {
        return Err(pockets_error(
            "AAG face attributes missing or stale; run attach_face_attrs first",
        ));
    }
    let mut guard: BudgetGuard = budget.guard("aag-find-pockets");
    guard.check()?;
    let n = model.faces.len();

    // Fillet-chain collapse: every chain becomes one supernode.
    let chains = crate::aag_fillets::find_fillet_chains(model, graph, budget)?;
    let mut parent: Vec<usize> = (0..n).collect();
    fn root(parent: &mut Vec<usize>, i: usize) -> usize {
        let mut r = i;
        while parent[r] != r {
            r = parent[r];
        }
        let mut c = i;
        while parent[c] != c {
            let next = parent[c];
            parent[c] = r;
            c = next;
        }
        r
    }
    let mut fillet_roots: std::collections::BTreeSet<usize> = Default::default();
    for chain in &chains {
        guard.tick()?;
        for &f in chain.faces.iter().skip(1) {
            let (a, b) = (root(&mut parent, chain.faces[0]), root(&mut parent, f));
            if a != b {
                parent[b] = a;
            }
        }
        fillet_roots.insert(root(&mut parent, chain.faces[0]));
    }
    let roots: Vec<usize> = (0..n).map(|i| root(&mut parent, i)).collect();

    // Effective adjacency between supernodes (skip chain-internal edges).
    // (root_a, root_b, class, aag edge index)
    let mut adj: Vec<(usize, usize, DihedralClass, usize)> = Vec::new();
    for (ei, edge) in graph.edges.iter().enumerate() {
        guard.tick()?;
        if edge.uses.len() != 2 {
            continue;
        }
        let (ra, rb) = (roots[edge.uses[0].face], roots[edge.uses[1].face]);
        if ra != rb {
            adj.push((ra, rb, edge.class, ei));
        }
    }
    let attrs_of = |face: usize| graph.nodes[face].attrs.as_ref().expect("stale checked");
    let wall_eligible = |face: usize| {
        matches!(
            attrs_of(face).class,
            SurfaceClass::Plane | SurfaceClass::Cone | SurfaceClass::Cylinder
        )
    };
    // Same-surface sibling test across a smooth edge.
    let linear_tol = (model.tolerance_mm * 100.).max(1e-7);
    let mut plane_key_cache: std::collections::BTreeMap<usize, ([f64; 3], f64)> = Default::default();
    let mut plane_key = |model: &Model, face: usize| -> Result<Option<([f64; 3], f64)>> {
        if let Some(k) = plane_key_cache.get(&face) {
            return Ok(Some(*k));
        }
        let Some(n) = outward_normal(model, face, 0.5, 0.5)? else {
            return Ok(None);
        };
        let p = face_point(model, face, 0.5, 0.5)?;
        let key = (n, dot(n, p));
        plane_key_cache.insert(face, key);
        Ok(Some(key))
    };
    let same_surface = |model: &Model,
                        a: usize,
                        b: usize,
                        plane_key: &mut dyn FnMut(&Model, usize) -> Result<Option<([f64; 3], f64)>>|
     -> Result<bool> {
        let (ca, cb) = (attrs_of(a).class, attrs_of(b).class);
        if ca != cb {
            return Ok(false);
        }
        match ca {
            SurfaceClass::Plane => {
                let (Some((na, oa)), Some((nb, ob))) = (plane_key(model, a)?, plane_key(model, b)?)
                else {
                    return Ok(false);
                };
                Ok(dot(na, nb) > 1. - AXIS_DOT_TOLERANCE && (oa - ob).abs() <= linear_tol)
            }
            SurfaceClass::Cone => Ok(match (attrs_of(a).axis, attrs_of(b).axis) {
                (Some(x), Some(y)) => coaxial(&x, &y, linear_tol),
                _ => false,
            }),
            SurfaceClass::Cylinder => {
                let (ra, rb_) = (attrs_of(a).radius, attrs_of(b).radius);
                Ok(match (attrs_of(a).axis, attrs_of(b).axis, ra, rb_) {
                    (Some(x), Some(y), Some(u), Some(v)) => {
                        coaxial(&x, &y, linear_tol)
                            && (u - v).abs() <= RADIUS_REL_TOLERANCE * u.max(v)
                    }
                    _ => false,
                })
            }
            _ => Ok(false),
        }
    };

    // BFS subgraph growth from every plane seed.
    let mut seen: std::collections::BTreeMap<Vec<usize>, usize> = Default::default();
    let mut pockets: Vec<PocketFeature> = Vec::new();
    for seed in 0..n {
        guard.tick()?;
        if fillet_roots.contains(&seed) || attrs_of(seed).class != SurfaceClass::Plane {
            continue;
        }
        let mut role: std::collections::BTreeMap<usize, PocketRole> = Default::default();
        role.insert(seed, PocketRole::Floor);
        let mut queue = std::collections::VecDeque::from([seed]);
        while let Some(r) = queue.pop_front() {
            guard.tick()?;
            let r_role = role[&r];
            for &(ra, rb, class, _) in adj.iter().filter(|(a, b, ..)| *a == r || *b == r) {
                let q = if ra == r { rb } else { ra };
                if role.contains_key(&q) {
                    continue;
                }
                let add = match class {
                    // Walls attach to the floor (or to a floor blend) through
                    // reentrant edges. Walls never expand further: a concave
                    // edge past a wall is a multi-level step, out of scope.
                    DihedralClass::Concave => match r_role {
                        PocketRole::Floor | PocketRole::Fillet if wall_eligible(q) => {
                            Some(PocketRole::Wall)
                        }
                        _ => None,
                    },
                    // Transparent fillet supernodes, and same-surface sibling
                    // patches (partitioned faces keep their role).
                    DihedralClass::Smooth => {
                        if fillet_roots.contains(&q) {
                            Some(PocketRole::Fillet)
                        } else if wall_eligible(q)
                            && matches!(r_role, PocketRole::Floor | PocketRole::Wall | PocketRole::Fillet)
                            && same_surface(model, r, q, &mut plane_key)?
                        {
                            Some(match r_role {
                                PocketRole::Floor => PocketRole::Floor,
                                _ => PocketRole::Wall,
                            })
                        } else if r_role == PocketRole::Fillet && wall_eligible(q) {
                            // Far support of a collapsed blend. It is usually
                            // a wall — but BFS order may reach a coplanar
                            // floor sibling through the blend before its own
                            // seam edges: a plane coplanar with the seed stays
                            // floor, never a 90°-leaning "wall".
                            if attrs_of(q).class == SurfaceClass::Plane
                                && same_surface(model, seed, q, &mut plane_key)?
                            {
                                Some(PocketRole::Floor)
                            } else {
                                Some(PocketRole::Wall)
                            }
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                if let Some(rl) = add {
                    role.insert(q, rl);
                    queue.push_back(q);
                }
            }
        }

        let floors: Vec<usize> = role
            .iter()
            .filter(|(_, rl)| **rl == PocketRole::Floor)
            .map(|(&r, _)| r)
            .collect();
        let walls: Vec<usize> = role
            .iter()
            .filter(|(_, rl)| **rl == PocketRole::Wall)
            .map(|(&r, _)| r)
            .collect();
        let fillets: Vec<usize> = role
            .iter()
            .filter(|(_, rl)| **rl == PocketRole::Fillet)
            .map(|(&r, _)| r)
            .collect();
        if walls.is_empty() {
            continue;
        }
        let in_subgraph = |r: usize| role.contains_key(&r);

        // Edge-pattern validation: internal edges concave/smooth, boundary
        // (rim) edges convex — with the collapse, floor blends read as
        // internal smooth edges.
        let mut valid = true;
        let mut rim_faces: Vec<usize> = Vec::new();
        let mut rim_points: Vec<[f64; 3]> = Vec::new();
        let mut floor_has_boundary = false;
        for &(ra, rb, class, ei) in &adj {
            guard.tick()?;
            match (in_subgraph(ra), in_subgraph(rb)) {
                (true, true) => {
                    if !matches!(class, DihedralClass::Concave | DihedralClass::Smooth) {
                        valid = false;
                        break;
                    }
                }
                (true, false) | (false, true) => {
                    if class != DihedralClass::Convex {
                        valid = false;
                        break;
                    }
                    let (inside, outside) = if in_subgraph(ra) { (ra, rb) } else { (rb, ra) };
                    if role[&inside] == PocketRole::Floor {
                        floor_has_boundary = true;
                    }
                    for u in &graph.edges[ei].uses {
                        if roots[u.face] == outside && !rim_faces.contains(&u.face) {
                            rim_faces.push(u.face);
                        }
                    }
                    let edge = &model.edges[graph.edges[ei].edge];
                    let domain = edge.curve.domain();
                    let mid = edge.curve.evaluate(0.5 * (domain[0] + domain[1]))?.point;
                    if mid.len() == 3 {
                        rim_points.push([mid[0], mid[1], mid[2]]);
                    }
                }
                _ => {}
            }
        }
        if !valid || rim_points.is_empty() {
            continue;
        }

        // Geometry and probes.
        let Some(floor_normal) = outward_normal(model, seed, 0.5, 0.5)? else {
            continue;
        };
        let floor_center = face_point(model, seed, 0.5, 0.5)?;
        let floor_offset = dot(floor_normal, floor_center);
        let depth = rim_points
            .iter()
            .map(|p| dot(floor_normal, *p) - floor_offset)
            .fold(0f64, f64::max);
        if !(depth.is_finite() && depth > 10. * model.tolerance_mm) {
            continue;
        }
        let eps = (1e-3 * depth).max(10. * model.tolerance_mm);
        // Wall leans are cheap (surface normals only) and gate the expensive
        // ray-parity probes below.
        let mut draft_max = 0f64;
        let mut wall_dirs: Vec<([f64; 3], [f64; 3])> = Vec::new(); // (center, normal)
        let mut walls_ok = true;
        for &w in &walls {
            guard.tick()?;
            let (Some(nw), Ok(wc)) = (outward_normal(model, w, 0.5, 0.5)?, face_point(model, w, 0.5, 0.5))
            else {
                walls_ok = false;
                break;
            };
            let lean = dot(nw, floor_normal).clamp(-1., 1.).asin().to_degrees();
            // A wall leaning more than 45° off perpendicular is parallel to
            // the floor normal — that "wall" faces the floor, so the seed
            // is a side wall, not the true floor. Reject this reading.
            if lean.abs() > 45. {
                walls_ok = false;
                break;
            }
            draft_max = draft_max.max(lean.abs());
            wall_dirs.push((wc, nw));
        }
        if !walls_ok {
            continue;
        }

        let expand = |roots_of: &[usize]| -> Vec<usize> {
            let mut out: Vec<usize> = (0..n).filter(|&f| roots_of.contains(&roots[f])).collect();
            out.sort_unstable();
            out
        };
        let floor_faces = expand(&floors);
        let wall_faces = expand(&walls);
        let fillet_faces = expand(&fillets);
        let floor_area = floor_faces.iter().map(|&f| attrs_of(f).area).sum();
        let mut faces = floor_faces.clone();
        faces.extend_from_slice(&wall_faces);
        faces.extend_from_slice(&fillet_faces);
        faces.sort_unstable();
        rim_faces.sort_unstable();

        // The same cavity can validate from many seeds (every coplanar floor
        // fragment, and a slot even reads as its own rotated self). Probes
        // are the expensive part: only run them for a new face set or for a
        // strictly deeper reading of one already found.
        let replace_idx = match seen.get(&faces) {
            Some(&idx) if depth <= pockets[idx].depth => None,
            Some(&idx) => Some(Some(idx)),
            None => Some(None),
        };
        let Some(replace_idx) = replace_idx else {
            continue;
        };

        // The cavity side of the floor must be void.
        guard.tick()?;
        let above_floor = [
            floor_center[0] + floor_normal[0] * eps,
            floor_center[1] + floor_normal[1] * eps,
            floor_center[2] + floor_normal[2] * eps,
        ];
        if probe_inside(model, above_floor)? != Some(false) {
            continue;
        }
        // Pocket/boss discriminator: just past every wall (outward normal
        // points into the cavity) must be void. A boss probe lands in
        // material. Unresolved rays reject conservatively.
        for (wc, nw) in &wall_dirs {
            guard.tick()?;
            let probe = [wc[0] + nw[0] * eps, wc[1] + nw[1] * eps, wc[2] + nw[2] * eps];
            if probe_inside(model, probe)? != Some(false) {
                walls_ok = false;
                break;
            }
        }
        if !walls_ok {
            continue;
        }

        let drafted = draft_max > 0.5;
        let candidate = PocketFeature {
            faces,
            floor_faces,
            wall_faces,
            fillet_faces,
            rim_faces,
            kind: if floor_has_boundary {
                PocketKind::Open
            } else {
                PocketKind::Closed
            },
            walls: if drafted {
                PocketWalls::Drafted
            } else {
                PocketWalls::Straight
            },
            draft_angle_deg: drafted.then_some(draft_max),
            depth,
            floor_area,
            floor_normal,
            parent: None,
            children: vec![],
        };
        match replace_idx {
            Some(idx) => pockets[idx] = candidate,
            None => {
                seen.insert(candidate.faces.clone(), pockets.len());
                pockets.push(candidate);
            }
        }
    }

    pockets.sort_by_key(|p| p.faces[0]);
    // Hierarchy: a pocket's rim landing inside another pocket's face set
    // makes it a child of that pocket.
    let owner: std::collections::BTreeMap<usize, usize> = pockets
        .iter()
        .enumerate()
        .flat_map(|(i, p)| p.faces.iter().map(move |&f| (f, i)))
        .collect();
    let parents: Vec<Option<usize>> = pockets
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let mut votes: std::collections::BTreeMap<usize, usize> = Default::default();
            for f in &p.rim_faces {
                if let Some(&j) = owner.get(f) {
                    // The rim of a nested pocket lands on the parent's floor,
                    // so the parent's rim-to-floor span exceeds the child's.
                    // This also breaks the reverse vote (the parent's rim
                    // touches the child's walls at the pit mouth).
                    if j != i && pockets[j].depth > p.depth {
                        *votes.entry(j).or_default() += 1;
                    }
                }
            }
            votes.into_iter().max_by_key(|(_, c)| *c).map(|(j, _)| j)
        })
        .collect();
    for (i, p) in parents.iter().enumerate() {
        pockets[i].parent = *p;
        if let Some(j) = *p {
            pockets[j].children.push(i);
        }
    }
    guard.check()?;
    Ok(pockets)
}


#[cfg(test)]
mod tests {
    use super::*;

    fn budget() -> Budget {
        Budget::new(1_000_000, 8, 60_000).unwrap()
    }

    fn graph_with_attrs(model: &Model) -> Aag {
        let mut graph = Aag::build(model, &budget()).unwrap();
        graph.attach_face_attrs(model, &budget()).unwrap();
        graph
    }

    /// Cup profile in the (r, z) half-plane: outer radius 5, height 6, bore
    /// radius 2 from z=2 to z=6 (blind hole, floor at z=2). CCW.
    const CUP: [[f64; 2]; 6] = [
        [0., 0.],
        [5., 0.],
        [5., 6.],
        [2., 6.],
        [2., 2.],
        [0., 2.],
    ];

    #[test]
    fn through_hole_in_tube() {
        // Inner wall of a tube is four quadrant patches of one cylinder:
        // split-cylinder grouping must merge them into one through hole.
        let model = crate::analytic::tube(5., 2., 6.).unwrap();
        let graph = graph_with_attrs(&model);
        let holes = find_holes(&model, &graph, &budget()).unwrap();
        assert_eq!(holes.len(), 1, "exactly the bore: {holes:?}");
        let hole = &holes[0];
        assert_eq!(hole.kind, HoleKind::Through);
        assert_eq!(hole.entry, HoleEntry::Plain);
        assert!(hole.faces.len() >= 4, "all inner patches merged: {:?}", hole.faces);
        assert!((hole.diameter - 4.).abs() < 1e-4, "diameter: {}", hole.diameter);
        assert!((hole.depth - 6.).abs() < 1e-6, "depth: {}", hole.depth);
        assert!(
            (hole.angle_span - 2. * std::f64::consts::PI).abs() < 0.3,
            "full circle: {}",
            hole.angle_span
        );
        assert!(hole.bottom_faces.is_empty());
        // The outer wall (convex edges) must not appear anywhere.
        assert!(hole.faces.iter().all(|&f| {
            graph.nodes[f]
                .edges
                .iter()
                .all(|&e| graph.edges[e].class != DihedralClass::Convex)
        }));
    }

    #[test]
    fn blind_hole_in_revolved_cup() {
        let model = crate::analytic::revolve(&CUP).unwrap();
        let graph = graph_with_attrs(&model);
        let holes = find_holes(&model, &graph, &budget()).unwrap();
        assert_eq!(holes.len(), 1, "{holes:?}");
        let hole = &holes[0];
        assert_eq!(hole.kind, HoleKind::Blind);
        assert_eq!(hole.entry, HoleEntry::Plain);
        assert!((hole.diameter - 4.).abs() < 1e-4);
        assert!((hole.depth - 4.).abs() < 1e-6, "depth z=2..6: {}", hole.depth);
        assert!(!hole.bottom_faces.is_empty(), "floor face reported");
        let axis = hole.axis;
        assert!((axis.direction[2].abs() - 1.).abs() < 1e-6, "bore along z");
    }

    #[test]
    fn countersink_blind_hole() {
        // Bore r=2 for z in 2..4, then a cone flaring to r=4 at z=6.
        let profile = [
            [0., 0.],
            [5., 0.],
            [5., 6.],
            [4., 6.],
            [2., 4.],
            [2., 2.],
            [0., 2.],
        ];
        let model = crate::analytic::revolve(&profile).unwrap();
        let graph = graph_with_attrs(&model);
        let holes = find_holes(&model, &graph, &budget()).unwrap();
        assert_eq!(holes.len(), 1, "{holes:?}");
        let hole = &holes[0];
        assert_eq!(hole.kind, HoleKind::Blind);
        assert_eq!(hole.entry, HoleEntry::Countersink);
        assert!((hole.diameter - 4.).abs() < 1e-4);
        assert!(
            !hole.step_faces.is_empty(),
            "cone step faces reported (one per patch)"
        );
        assert!(
            hole.step_faces.iter().all(|&f| {
                graph.nodes[f].attrs.as_ref().unwrap().class == SurfaceClass::Cone
            }),
            "step faces are the cone patches"
        );
    }

    #[test]
    fn counterbore_blind_hole() {
        // Bore r=2 for z in 2..4, larger coaxial cylinder r=4 for z in 4..6.
        let profile = [
            [0., 0.],
            [5., 0.],
            [5., 6.],
            [4., 6.],
            [4., 4.],
            [2., 4.],
            [2., 2.],
            [0., 2.],
        ];
        let model = crate::analytic::revolve(&profile).unwrap();
        let graph = graph_with_attrs(&model);
        let holes = find_holes(&model, &graph, &budget()).unwrap();
        assert_eq!(holes.len(), 1, "{holes:?}");
        let hole = &holes[0];
        assert_eq!(hole.kind, HoleKind::Blind);
        assert_eq!(hole.entry, HoleEntry::Counterbore);
        assert!((hole.diameter - 4.).abs() < 1e-4);
        assert!(!hole.step_faces.is_empty(), "counterbore step faces reported");
    }

    #[test]
    fn half_hole_on_body_boundary_matches_by_angle_range() {
        // Partial (180°) revolution of the cup: the bore wall spans π, not a
        // full circle, and the two cut planes cap the profile. Matching must
        // not require a complete circumference.
        let model = crate::analytic::revolve_angle(&CUP, 180.).unwrap();
        let graph = graph_with_attrs(&model);
        let holes = find_holes(&model, &graph, &budget()).unwrap();
        assert_eq!(holes.len(), 1, "{holes:?}");
        let hole = &holes[0];
        assert_eq!(hole.kind, HoleKind::Blind);
        assert!(
            (hole.angle_span - std::f64::consts::PI).abs() < 0.35,
            "half coverage, not full: {}",
            hole.angle_span
        );
        assert!((hole.diameter - 4.).abs() < 1e-4);
    }

    #[test]
    fn solid_primitives_have_no_holes() {
        // Boss cylinders and boxes read convex at the wall boundary: rejected.
        for model in [
            crate::cuboid([0.; 3], [4.; 3]).unwrap(),
            crate::analytic::cylinder(2., 5.).unwrap(),
            crate::analytic::sphere(3.).unwrap(),
            crate::analytic::torus(4., 1.).unwrap(),
            crate::analytic::frustum(3., 1.5, 4.).unwrap(),
        ] {
            let graph = graph_with_attrs(&model);
            let holes = find_holes(&model, &graph, &budget()).unwrap();
            assert!(holes.is_empty(), "false positive: {holes:?}");
        }
    }

    #[test]
    fn missing_attrs_are_a_typed_error() {
        let model = crate::analytic::tube(5., 2., 6.).unwrap();
        let graph = Aag::build(&model, &budget()).unwrap();
        let error = find_holes(&model, &graph, &budget()).unwrap_err();
        assert_eq!(error.code, "BREP_AAG_FEATURES_INPUT");
    }

    #[test]
    fn budget_exhaustion_is_a_typed_error() {
        let model = crate::analytic::tube(5., 2., 6.).unwrap();
        let graph = graph_with_attrs(&model);
        let tight = Budget::with_iterations(2).unwrap();
        let error = find_holes(&model, &graph, &tight).unwrap_err();
        assert!(
            error.code.contains("RESOURCE") || error.code.contains("BUDGET"),
            "budget exhaustion must surface as a resource error: {error:?}"
        );
    }

    #[test]
    fn corpus_precision_recall_and_through_blind() {
        // Labeled corpus: five models with exactly one hole each, four solid
        // controls with none. Precision and recall must both be 100% (≥ 95%
        // required) and through/blind must be exact on 100% of the corpus.
        let cases: Vec<(Model, Option<HoleKind>)> = vec![
            (crate::analytic::tube(5., 2., 6.).unwrap(), Some(HoleKind::Through)),
            (crate::analytic::revolve(&CUP).unwrap(), Some(HoleKind::Blind)),
            (
                crate::analytic::revolve(&[
                    [0., 0.],
                    [5., 0.],
                    [5., 6.],
                    [4., 6.],
                    [2., 4.],
                    [2., 2.],
                    [0., 2.],
                ])
                .unwrap(),
                Some(HoleKind::Blind),
            ),
            (
                crate::analytic::revolve(&[
                    [0., 0.],
                    [5., 0.],
                    [5., 6.],
                    [4., 6.],
                    [4., 4.],
                    [2., 4.],
                    [2., 2.],
                    [0., 2.],
                ])
                .unwrap(),
                Some(HoleKind::Blind),
            ),
            (
                crate::analytic::revolve_angle(&CUP, 180.).unwrap(),
                Some(HoleKind::Blind),
            ),
            (crate::cuboid([0.; 3], [4.; 3]).unwrap(), None),
            (crate::analytic::cylinder(2., 5.).unwrap(), None),
            (crate::analytic::sphere(3.).unwrap(), None),
            (crate::analytic::torus(4., 1.).unwrap(), None),
        ];
        let (mut tp, mut fp, mut fn_) = (0usize, 0usize, 0usize);
        let (mut kind_ok, mut kind_total) = (0usize, 0usize);
        for (model, expected) in &cases {
            let graph = graph_with_attrs(model);
            let holes = find_holes(model, &graph, &budget()).unwrap();
            match (expected, holes.len()) {
                (Some(want), 1) => {
                    tp += 1;
                    kind_total += 1;
                    if holes[0].kind == *want {
                        kind_ok += 1;
                    }
                }
                (Some(_), 0) => fn_ += 1,
                (Some(_), _) => {
                    tp += 1;
                    fp += holes.len() - 1;
                }
                (None, n) => fp += n,
            }
        }
        let precision = tp as f64 / (tp + fp).max(1) as f64;
        let recall = tp as f64 / (tp + fn_).max(1) as f64;
        assert!(
            precision >= 0.95 && recall >= 0.95,
            "precision {precision}, recall {recall} (tp={tp} fp={fp} fn={fn_})"
        );
        assert_eq!(
            kind_ok, kind_total,
            "through/blind must be correct on 100% of the corpus"
        );
    }

    // ------------------------------------------------------------------
    // Pockets (checklist 867)
    // ------------------------------------------------------------------

    /// Closed rectangular pocket: floor at z=2 (36×26), walls z=2..20.
    fn closed_pocket_model() -> Model {
        crate::boolean(
            &crate::cuboid([0.; 3], [40., 30., 20.]).unwrap(),
            &crate::cuboid([2., 2., 2.], [38., 28., 22.]).unwrap(),
            "difference",
        )
        .unwrap()
    }

    #[test]
    fn closed_pocket_straight_walls() {
        let model = closed_pocket_model();
        let graph = graph_with_attrs(&model);
        let pockets = find_pockets(&model, &graph, &budget()).unwrap();
        assert_eq!(pockets.len(), 1, "{pockets:?}");
        let p = &pockets[0];
        assert_eq!(p.kind, PocketKind::Closed);
        assert_eq!(p.walls, PocketWalls::Straight);
        assert_eq!(p.draft_angle_deg, None);
        assert!((p.depth - 18.).abs() < 1e-6, "depth: {}", p.depth);
        assert!(
            (p.floor_area - 36. * 26.).abs() < 1e-6,
            "floor area: {}",
            p.floor_area
        );
        assert!((p.floor_normal[2] - 1.).abs() < 1e-9, "floor normal +z");
        assert!(p.parent.is_none() && p.children.is_empty());
        assert!(!p.floor_faces.is_empty() && p.wall_faces.len() >= 4);
        assert!(p.fillet_faces.is_empty(), "no blends in this model");
    }

    #[test]
    fn open_pocket_at_body_boundary() {
        // Slot cut through the x=40 side: the floor reaches the exterior.
        let model = crate::boolean(
            &crate::cuboid([0.; 3], [40., 30., 20.]).unwrap(),
            &crate::cuboid([30., 2., 2.], [42., 28., 22.]).unwrap(),
            "difference",
        )
        .unwrap();
        let graph = graph_with_attrs(&model);
        let pockets = find_pockets(&model, &graph, &budget()).unwrap();
        assert_eq!(pockets.len(), 1, "{pockets:?}");
        let p = &pockets[0];
        assert_eq!(p.kind, PocketKind::Open, "slot opens at x=40");
        assert!((p.depth - 18.).abs() < 1e-6);
        assert_eq!(p.wall_faces.len(), 3, "three walls, the fourth side open");
    }

    #[test]
    fn drafted_pocket_reports_draft_angle() {
        // Truncated-pyramid tool: 16×16 at z=2 growing to 22×22 at z=25,
        // i.e. 3 mm of lean over 23 mm of height on each side.
        let tool = crate::operations::faceted_loft(&[
            vec![
                [12., 7., 2.],
                [28., 7., 2.],
                [28., 23., 2.],
                [12., 23., 2.],
            ],
            vec![
                [9., 4., 25.],
                [31., 4., 25.],
                [31., 26., 25.],
                [9., 26., 25.],
            ],
        ])
        .unwrap();
        let model = crate::boolean(
            &crate::cuboid([0.; 3], [40., 30., 20.]).unwrap(),
            &tool,
            "difference",
        )
        .unwrap();
        let graph = graph_with_attrs(&model);
        let pockets = find_pockets(&model, &graph, &budget()).unwrap();
        assert_eq!(pockets.len(), 1, "{pockets:?}");
        let p = &pockets[0];
        assert_eq!(p.walls, PocketWalls::Drafted, "{p:?}");
        let expected = (3f64 / 23.).atan().to_degrees();
        let draft = p.draft_angle_deg.unwrap();
        assert!(
            (draft - expected).abs() < 0.2,
            "draft {draft}, expected {expected}"
        );
        assert_eq!(p.kind, PocketKind::Closed);
    }

    /// Open pocket with two floor blends, built as a prism: the cross-section
    /// is a 40×20 rectangle with a 8-wide, 3-deep notch whose bottom corners
    /// are rounded (r=1 quarter arcs), extruded z=0..10. The revolved
    /// counterpart is `revolved_round_pocket_model` (reachable since the AAG
    /// pcurve-synchronization fix in `aag::classify_edge`).
    fn filleted_pocket_model() -> Model {
        let line = |a: [f64; 2], b: [f64; 2]| nurbs_core::curve::Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![a.to_vec(), b.to_vec()],
            weights: vec![1., 1.],
            periodic: false,
        };
        let arc = |p0: [f64; 2], p1: [f64; 2], p2: [f64; 2]| nurbs_core::curve::Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![p0.to_vec(), p1.to_vec(), p2.to_vec()],
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
            periodic: false,
        };
        let wire = vec![
            line([0., 0.], [40., 0.]),
            line([40., 0.], [40., 20.]),
            line([40., 20.], [24., 20.]),
            line([24., 20.], [24., 18.]),
            arc([24., 18.], [24., 17.], [23., 17.]),
            line([23., 17.], [17., 17.]),
            arc([17., 17.], [16., 17.], [16., 18.]),
            line([16., 18.], [16., 20.]),
            line([16., 20.], [0., 20.]),
            line([0., 20.], [0., 0.]),
        ];
        crate::prism::extrude(&[wire], 0., 10.).unwrap()
    }

    /// Round pocket revolved from an analytic wire (regression for the AAG
    /// pcurve-synchronization fix): cylindrical wall, quarter-arc floor blend
    /// (torus), flat disk floor. Body z 0..8, radius 12, pocket radius 10
    /// with floor at z=5 and a radius-1 blend.
    fn revolved_round_pocket_model() -> Model {
        let line = |a: [f64; 2], b: [f64; 2]| nurbs_core::curve::Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![a.to_vec(), b.to_vec()],
            weights: vec![1., 1.],
            periodic: false,
        };
        let arc = |p0: [f64; 2], p1: [f64; 2], p2: [f64; 2]| nurbs_core::curve::Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![p0.to_vec(), p1.to_vec(), p2.to_vec()],
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
            periodic: false,
        };
        let wire = vec![
            line([0., 0.], [12., 0.]),
            line([12., 0.], [12., 8.]),
            line([12., 8.], [10., 8.]),
            line([10., 8.], [10., 6.]),
            arc([10., 6.], [10., 5.], [9., 5.]),
            line([9., 5.], [0., 5.]),
            line([0., 5.], [0., 0.]),
        ];
        crate::revolve_wire(&wire, 1e-7).unwrap()
    }

    #[test]
    fn revolved_round_pocket_matches_through_fixed_aag() {
        let model = revolved_round_pocket_model();
        let graph = graph_with_attrs(&model);
        // Sanity: tangent line→arc joints classify Smooth, not Degenerate.
        assert_eq!(
            graph.edges_of_class(crate::aag::DihedralClass::Degenerate),
            graph
                .edges
                .iter()
                .filter(|e| model.edges[e.edge].degenerate)
                .map(|e| e.edge)
                .collect::<Vec<_>>(),
            "no falsely degenerate tangent joints"
        );
        let pockets = find_pockets(&model, &graph, &budget()).unwrap();
        assert_eq!(pockets.len(), 1, "{pockets:?}");
        let p = &pockets[0];
        assert_eq!(p.kind, PocketKind::Closed, "round pocket is blind");
        assert_eq!(p.walls, PocketWalls::Straight);
        assert!(!p.fillet_faces.is_empty(), "torus blend absorbed");
        assert!(p.fillet_faces.iter().all(|f| p.faces.contains(f)));
        // Rim at z=8, floor at z=5; floor disk radius 9.
        assert!((p.depth - 3.).abs() < 1e-6, "depth: {}", p.depth);
        assert!(
            (p.floor_area - 81. * std::f64::consts::PI).abs() < 1e-4,
            "floor area: {}",
            p.floor_area
        );
        assert!((p.floor_normal[2] - 1.).abs() < 1e-9, "floor normal +z");
    }

    #[test]
    fn pocket_with_floor_fillet_collapses_chain() {
        let model = filleted_pocket_model();
        let graph = graph_with_attrs(&model);
        // Sanity: both blends are real fillet chains for 869.
        let chains = crate::aag_fillets::find_fillet_chains(&model, &graph, &budget()).unwrap();
        assert_eq!(chains.len(), 2, "two floor blend chains: {chains:?}");
        let pockets = find_pockets(&model, &graph, &budget()).unwrap();
        assert_eq!(pockets.len(), 1, "{pockets:?}");
        let p = &pockets[0];
        assert_eq!(p.kind, PocketKind::Open, "the prism slot is through in z");
        assert_eq!(p.walls, PocketWalls::Straight);
        let chain_faces: usize = chains.iter().map(|c| c.faces.len()).sum();
        assert_eq!(
            p.fillet_faces.len(),
            chain_faces,
            "both chains are absorbed into the pocket"
        );
        assert!(p.fillet_faces.iter().all(|f| p.faces.contains(f)));
        // Depth: rim at y=20 down to the floor at y=17; floor is 6×10.
        assert!((p.depth - 3.).abs() < 1e-6, "depth: {}", p.depth);
        assert!(
            (p.floor_area - 60.).abs() < 1e-6,
            "floor area: {}",
            p.floor_area
        );
        assert!((p.floor_normal[1] - 1.).abs() < 1e-9, "floor normal +y");
        assert_eq!(p.wall_faces.len(), 2, "two straight walls");
    }

    #[test]
    fn nested_pockets_form_a_hierarchy() {
        // Outer pocket floor at z=2; a deeper pit cut into that floor down
        // to z=1 — its rim lands on the outer pocket's floor.
        let outer = closed_pocket_model();
        let model = crate::boolean(
            &outer,
            &crate::cuboid([13., 13., 1.], [27., 27., 12.]).unwrap(),
            "difference",
        )
        .unwrap();
        let graph = graph_with_attrs(&model);
        let pockets = find_pockets(&model, &graph, &budget()).unwrap();
        assert_eq!(pockets.len(), 2, "two pockets, not one merged: {pockets:?}");
        let (outer_i, inner_i) = pockets
            .iter()
            .enumerate()
            .partition::<Vec<_>, _>(|(_, p)| p.depth > 2.);
        let outer = &pockets[outer_i[0].0];
        let inner = &pockets[inner_i[0].0];
        assert!(outer.parent.is_none(), "outer is top-level");
        assert_eq!(inner.parent, Some(outer_i[0].0), "inner nests in outer");
        assert_eq!(outer.children, vec![inner_i[0].0]);
        assert!((inner.depth - 1.).abs() < 1e-6, "inner depth: {}", inner.depth);
        // The inner rim must land on the outer pocket's floor.
        assert!(
            inner
                .rim_faces
                .iter()
                .all(|f| outer.faces.contains(f) || outer.floor_faces.contains(f)),
            "rim lands inside the parent: {:?}",
            inner.rim_faces
        );
    }

    #[test]
    fn boss_and_primitives_have_no_pockets() {
        // Boss on a plate: the mirror subgraph (concave base ring, convex
        // top rim) must be rejected by the convex internal corner edges and
        // the wall probes landing in material. Zero false positives.
        let boss = crate::boolean(
            &crate::cuboid([0.; 3], [40., 30., 2.]).unwrap(),
            &crate::cuboid([15., 10., 2.], [25., 20., 12.]).unwrap(),
            "union",
        )
        .unwrap();
        for model in [
            boss,
            crate::cuboid([0.; 3], [4.; 3]).unwrap(),
            // A through tube's annular cap grows the bore wall as a
            // "wall"; the concave bottom ring must reject the subgraph.
            crate::analytic::tube(5., 2., 6.).unwrap(),
            crate::analytic::cylinder(2., 5.).unwrap(),
        ] {
            let graph = graph_with_attrs(&model);
            let pockets = find_pockets(&model, &graph, &budget()).unwrap();
            assert!(pockets.is_empty(), "false positive pocket: {pockets:?}");
        }
    }

    #[test]
    fn pockets_require_fresh_attrs_and_budget() {
        let model = closed_pocket_model();
        let graph = Aag::build(&model, &budget()).unwrap();
        let error = find_pockets(&model, &graph, &budget()).unwrap_err();
        assert_eq!(error.code, "BREP_AAG_POCKETS_INPUT");
        let graph = graph_with_attrs(&model);
        let tight = Budget::with_iterations(2).unwrap();
        let error = find_pockets(&model, &graph, &tight).unwrap_err();
        assert!(
            error.code.contains("RESOURCE") || error.code.contains("BUDGET"),
            "budget exhaustion must surface as a resource error: {error:?}"
        );
    }

    #[test]
    fn pocket_corpus_recall_and_hierarchy() {
        // Labeled corpus: five pocket models (closed, open, drafted,
        // floor-filleted, nested×2 = 6 expected pockets) and two negative
        // controls (boss, plain block). Recall ≥ 90% required; hierarchy and
        // zero boss false positives asserted exactly.
        let drafted_tool = crate::operations::faceted_loft(&[
            vec![
                [12., 7., 2.],
                [28., 7., 2.],
                [28., 23., 2.],
                [12., 23., 2.],
            ],
            vec![
                [9., 4., 25.],
                [31., 4., 25.],
                [31., 26., 25.],
                [9., 26., 25.],
            ],
        ])
        .unwrap();
        let nested = crate::boolean(
            &closed_pocket_model(),
            &crate::cuboid([13., 13., 1.], [27., 27., 12.]).unwrap(),
            "difference",
        )
        .unwrap();
        let boss = crate::boolean(
            &crate::cuboid([0.; 3], [40., 30., 2.]).unwrap(),
            &crate::cuboid([15., 10., 2.], [25., 20., 12.]).unwrap(),
            "union",
        )
        .unwrap();
        let cases: Vec<(Model, usize)> = vec![
            (closed_pocket_model(), 1),
            (
                crate::boolean(
                    &crate::cuboid([0.; 3], [40., 30., 20.]).unwrap(),
                    &crate::cuboid([30., 2., 2.], [42., 28., 22.]).unwrap(),
                    "difference",
                )
                .unwrap(),
                1,
            ),
            (
                crate::boolean(
                    &crate::cuboid([0.; 3], [40., 30., 20.]).unwrap(),
                    &drafted_tool,
                    "difference",
                )
                .unwrap(),
                1,
            ),
            (filleted_pocket_model(), 1),
            (nested, 2),
            (boss, 0),
            (crate::cuboid([0.; 3], [4.; 3]).unwrap(), 0),
        ];
        let (mut found, mut expected_total, mut extra) = (0usize, 0usize, 0usize);
        for (model, want) in &cases {
            let graph = graph_with_attrs(model);
            let pockets = find_pockets(model, &graph, &budget()).unwrap();
            found += pockets.len().min(*want);
            extra += pockets.len().saturating_sub(*want);
            expected_total += want;
        }
        let recall = found as f64 / expected_total.max(1) as f64;
        assert!(
            recall >= 0.9 && extra == 0,
            "recall {recall} ({found}/{expected_total}), false positives {extra}"
        );
    }

}
