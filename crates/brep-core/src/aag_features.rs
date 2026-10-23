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

/// Feature probes use fractions of each unchanged source chart, whose knot
/// domain may be a retained subinterval after a boolean operation.
fn face_parameters(model: &Model, face: usize, u: f64, v: f64) -> [f64; 2] {
    let s = &model.faces[face].surface;
    let map = |t: f64, a: f64, b: f64| {
        if t == 0. { a } else if t == 1. { b } else { (1. - t) * a + t * b }
    };
    [map(u, s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]),
     map(v, s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()])]
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
            let [u, v] = face_parameters(model, face, u, v);
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
            let [u, v] = face_parameters(model, face, u, v);
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

mod pockets;
pub use pockets::{PocketKind,PocketWalls,PocketFeature,find_pockets};



#[cfg(test)]
#[path = "tests/aag_features.rs"]
mod tests;
