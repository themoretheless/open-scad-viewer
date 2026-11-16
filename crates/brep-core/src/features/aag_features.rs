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
use nurbs_core::surface::SurfaceRefSampler;

/// Sample grid side per face for angular-span and axial-extent estimation.
const HOLE_SAMPLE_GRID: usize = 9;
/// Axis-direction alignment tolerance for coaxial clustering (radians, via
/// `1 - |dot|` on unit directions).
const AXIS_DOT_TOLERANCE: f64 = 1e-6;
/// Relative radius tolerance for clustering sibling cylinder patches.
const RADIUS_REL_TOLERANCE: f64 = 1e-4;
/// Multiplier on `model.tolerance_mm` for coaxial and same-surface checks.
const LINEAR_TOLERANCE_FACTOR: f64 = 100.;
/// Lower bound of the coaxial and same-surface linear tolerance.
const LINEAR_TOLERANCE_FLOOR: f64 = 1e-7;
/// Ray-parity probe offset relative to the feature depth.
const PROBE_OFFSET_DEPTH_FRACTION: f64 = 1e-3;
/// Lower bound of the probe offset, in multiples of `model.tolerance_mm`.
const PROBE_OFFSET_TOLERANCE_FACTOR: f64 = 10.;
/// Error text when face attributes are missing or stale.
const STALE_ATTRS_MESSAGE: &str =
    "AAG face attributes missing or stale; run attach_face_attrs first";

/// Linear tolerance for coaxial checks: a small multiple of the modeling
/// tolerance, so genuine axis splits merge but distinct features do not.
fn linear_tolerance(model: &Model) -> f64 {
    (model.tolerance_mm * LINEAR_TOLERANCE_FACTOR).max(LINEAR_TOLERANCE_FLOOR)
}

/// Offset of a ray-parity probe past a feature boundary of `depth`.
fn probe_offset(model: &Model, depth: f64) -> f64 {
    (PROBE_OFFSET_DEPTH_FRACTION * depth).max(PROBE_OFFSET_TOLERANCE_FACTOR * model.tolerance_mm)
}

/// Union-find root with path compression.
fn union_root(parent: &mut [usize], i: usize) -> usize {
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
    let sampler = SurfaceRefSampler::new(&model.faces[face].surface)?;
    let n = HOLE_SAMPLE_GRID;
    let mut points = Vec::with_capacity(n * n);
    for i in 0..n {
        for j in 0..n {
            guard.tick()?;
            // Endpoints included: the axial extent must reach the rims.
            let u = i as f64 / (n - 1) as f64;
            let v = j as f64 / (n - 1) as f64;
            let [u, v] = face_parameters(model, face, u, v);
            let p = sampler.point(u, v)?;
            require_finite_f64(p[0], "sample.x")?;
            require_finite_f64(p[1], "sample.y")?;
            require_finite_f64(p[2], "sample.z")?;
            points.push(p);
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
        let sampler = SurfaceRefSampler::new(&model.faces[face].surface)?;
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
            let p = eval.point;
            let rel = sub(p, axis.point);
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
    // (face, axis, radius) of every fully attributed cylinder face.
    let cylinders: Vec<(usize, Axis, f64)> = graph
        .nodes
        .iter()
        .filter_map(|n| {
            let a = n.attrs.as_ref()?;
            if a.class != SurfaceClass::Cylinder {
                return None;
            }
            Some((n.face, a.axis?, a.radius?))
        })
        .collect();
    let mut parent: Vec<usize> = (0..cylinders.len()).collect();
    let index_of = |face: usize| cylinders.iter().position(|&(f, ..)| f == face);
    for (i, &(face, axis_a, radius_a)) in cylinders.iter().enumerate() {
        for sib in neighbors_via(graph, face, |c| c == DihedralClass::Smooth) {
            let Some(j) = index_of(sib) else { continue };
            let (_, axis_b, radius_b) = cylinders[j];
            let same_radius =
                (radius_a - radius_b).abs() <= RADIUS_REL_TOLERANCE * radius_a.max(radius_b);
            if same_radius && coaxial(&axis_a, &axis_b, linear_tol) {
                let (ra, rb) = (union_root(&mut parent, i), union_root(&mut parent, j));
                if ra != rb {
                    parent[ra] = rb;
                }
            }
        }
    }
    let mut groups: std::collections::BTreeMap<usize, Vec<usize>> = Default::default();
    for (i, &(face, ..)) in cylinders.iter().enumerate() {
        let r = union_root(&mut parent, i);
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
        return Err(features_error(STALE_ATTRS_MESSAGE));
    }
    let mut guard: BudgetGuard = budget.guard("aag-find-holes");
    guard.check()?;
    let matcher = HoleMatcher {
        model,
        graph,
        linear_tol: linear_tolerance(model),
    };

    let clusters = cluster_cylinders(graph, matcher.linear_tol);
    let mut holes = Vec::new();
    for faces in clusters {
        guard.tick()?;
        if let Some(hole) = matcher.match_cluster(faces, &mut guard)? {
            holes.push(hole);
        }
    }
    let mut holes = drop_step_holes(holes);
    holes.sort_by_key(|h| h.faces[0]);
    guard.check()?;
    Ok(holes)
}

/// Fraction of the bore depth within which a boundary edge belongs to an end.
const END_BAND_DEPTH_FRACTION: f64 = 0.25;
/// Radial offset of an end probe toward the void, as a fraction of radius.
const END_PROBE_RADIUS_FRACTION: f64 = 0.25;

/// Fresh attributes of one face, or the stale-attributes input error.
fn fresh_attrs(graph: &Aag, face: usize) -> Result<&FaceAttrs> {
    graph.nodes[face]
        .attrs
        .as_ref()
        .ok_or_else(|| features_error(STALE_ATTRS_MESSAGE))
}

/// Sampled bore wall: points, axial extent and angular coverage.
struct BoreSamples {
    points: Vec<[f64; 3]>,
    t_min: f64,
    t_max: f64,
}

/// Classification of both bore ends.
struct EndScan {
    closed: [bool; 2],
    step_faces: Vec<usize>,
    bottom_faces: Vec<usize>,
    entry: HoleEntry,
}

impl EndScan {
    fn record_step(&mut self, kind: HoleEntry) {
        if self.entry == HoleEntry::Plain {
            self.entry = kind;
        }
    }
}

/// Hole template matching over one cylinder cluster at a time.
struct HoleMatcher<'a> {
    model: &'a Model,
    graph: &'a Aag,
    linear_tol: f64,
}

impl HoleMatcher<'_> {
    /// Match one cylinder cluster; `None` when it is not a hole.
    fn match_cluster(
        &self,
        faces: Vec<usize>,
        guard: &mut BudgetGuard,
    ) -> Result<Option<HoleFeature>> {
        let Some(boundary_edges) = self.boundary_edges(&faces, guard)? else {
            return Ok(None);
        };
        let (axis, radius) = self.axis_and_radius(&faces)?;

        // Hole vs boss: only a cylinder enclosing void (outward normals
        // toward the axis) is a bore wall.
        if !is_void_wall(self.model, &faces, &axis, guard)? {
            return Ok(None);
        }

        let bore = self.sample_bore(&faces, &axis, guard)?;
        let depth = bore.t_max - bore.t_min;
        let angle_span = angular_span(&bore.points, &axis);
        let end_neighbors = self.end_neighbors(&faces, &boundary_edges, &axis, &bore, guard)?;
        let mut ends = self.scan_ends(&faces, &axis, radius, &bore, &end_neighbors, guard)?;

        let kind = match (ends.closed[0], ends.closed[1]) {
            (true, true) => return Ok(None), // closed cavity, not a hole
            (false, false) => HoleKind::Through,
            _ => HoleKind::Blind,
        };
        ends.step_faces.sort_unstable();
        ends.step_faces.dedup();
        ends.bottom_faces.sort_unstable();
        ends.bottom_faces.dedup();
        let mut faces = faces;
        faces.sort_unstable();
        Ok(Some(HoleFeature {
            faces,
            step_faces: ends.step_faces,
            bottom_faces: ends.bottom_faces,
            kind,
            entry: ends.entry,
            axis,
            diameter: 2. * radius,
            depth,
            angle_span,
        }))
    }

    /// Boundary edges of the cluster: incident edges with a use outside it.
    /// `None` when the cluster has none or carries non-matchable evidence.
    fn boundary_edges(
        &self,
        faces: &[usize],
        guard: &mut BudgetGuard,
    ) -> Result<Option<Vec<usize>>> {
        let mut boundary_edges: Vec<usize> = Vec::new();
        for &face in faces {
            guard.tick()?;
            for &e in &self.graph.nodes[face].edges {
                let edge = &self.graph.edges[e];
                if edge.uses.iter().all(|u| faces.contains(&u.face)) {
                    continue; // internal sibling seam
                }
                // Non-matchable evidence (mixed sign, degenerate, open
                // boundary) must never feed a template verdict.
                if !edge.class.is_matchable() {
                    return Ok(None);
                }
                boundary_edges.push(e);
            }
        }
        Ok((!boundary_edges.is_empty()).then_some(boundary_edges))
    }

    /// Bore axis (first face) and mean radius of the cluster.
    fn axis_and_radius(&self, faces: &[usize]) -> Result<(Axis, f64)> {
        let stale = || features_error(STALE_ATTRS_MESSAGE);
        let first = faces.first().ok_or_else(stale)?;
        let axis = fresh_attrs(self.graph, *first)?.axis.ok_or_else(stale)?;
        let mut sum = 0f64;
        for &f in faces {
            sum += fresh_attrs(self.graph, f)?.radius.ok_or_else(stale)?;
        }
        Ok((axis, sum / faces.len() as f64))
    }

    /// Sampled geometry: axial extent of the wall.
    fn sample_bore(
        &self,
        faces: &[usize],
        axis: &Axis,
        guard: &mut BudgetGuard,
    ) -> Result<BoreSamples> {
        let mut points = Vec::new();
        for &face in faces {
            points.extend(face_samples(self.model, face, guard)?);
        }
        let mut t_min = f64::INFINITY;
        let mut t_max = f64::NEG_INFINITY;
        for p in &points {
            let t = dot(sub(*p, axis.point), axis.direction);
            t_min = t_min.min(t);
            t_max = t_max.max(t);
        }
        Ok(BoreSamples {
            points,
            t_min,
            t_max,
        })
    }

    /// Assign each boundary edge to an axial end by its midpoint, or mark
    /// it as a side edge (cut-plane edges of a partial hole span the full
    /// depth and belong to neither end). Returns `[min, max]` neighbors.
    fn end_neighbors(
        &self,
        faces: &[usize],
        boundary_edges: &[usize],
        axis: &Axis,
        bore: &BoreSamples,
        guard: &mut BudgetGuard,
    ) -> Result<[Vec<usize>; 2]> {
        let mut end_neighbors: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
        let quarter = (bore.t_max - bore.t_min) * END_BAND_DEPTH_FRACTION;
        for &e in boundary_edges {
            guard.tick()?;
            let edge = &self.model.edges[self.graph.edges[e].edge];
            let domain = edge.curve.domain();
            let mid = edge.curve.evaluate(0.5 * (domain[0] + domain[1]))?.point;
            if mid.len() != 3 {
                return Err(features_error("hole matching expects 3D edge points"));
            }
            let t = dot(sub([mid[0], mid[1], mid[2]], axis.point), axis.direction);
            let side = if t - bore.t_min <= quarter {
                Some(0)
            } else if bore.t_max - t <= quarter {
                Some(1)
            } else {
                None
            };
            if let Some(s) = side {
                for u in &self.graph.edges[e].uses {
                    if !faces.contains(&u.face) && !end_neighbors[s].contains(&u.face) {
                        end_neighbors[s].push(u.face);
                    }
                }
            }
        }
        Ok(end_neighbors)
    }

    /// Classify each end. Through/blind is decided by a ray-parity probe
    /// just past the end on the bore axis: inside material = floor (blind),
    /// outside = open. This is sign-convention-free and works uniformly
    /// for full holes, half-holes (where the axis lies on a cut plane —
    /// the probe is offset toward the void side) and stepped holes (the
    /// bore end behind a counterbore shoulder probes the wider void).
    fn scan_ends(
        &self,
        faces: &[usize],
        axis: &Axis,
        radius: f64,
        bore: &BoreSamples,
        end_neighbors: &[Vec<usize>; 2],
        guard: &mut BudgetGuard,
    ) -> Result<EndScan> {
        let mut scan = EndScan {
            closed: [false, false],
            step_faces: Vec::new(),
            bottom_faces: Vec::new(),
            entry: HoleEntry::Plain,
        };
        let void_dir = void_direction(&bore.points, axis);
        let eps = probe_offset(self.model, bore.t_max - bore.t_min);
        let radial_offset = END_PROBE_RADIUS_FRACTION * radius;
        for (s, neighbors) in end_neighbors.iter().enumerate() {
            guard.tick()?;
            let (t_end, sign) = if s == 0 {
                (bore.t_min, -1.)
            } else {
                (bore.t_max, 1.)
            };
            let along = t_end + sign * eps;
            let probe: [f64; 3] = std::array::from_fn(|k| {
                axis.point[k] + axis.direction[k] * along + void_dir[k] * radial_offset
            });
            guard.tick()?;
            scan.closed[s] = pockets::probe_inside(self.model, probe)? == Some(true);
            if scan.closed[s] {
                scan.bottom_faces.extend(neighbors.iter().copied());
            }
            self.scan_end_steps(faces, axis, radius, neighbors, &mut scan, guard)?;
        }
        Ok(scan)
    }

    /// Entry step detection from the graph neighborhood at one end: a
    /// coaxial cone (countersink) or larger coaxial cylinder (counterbore),
    /// reached directly or across a shoulder plane.
    fn scan_end_steps(
        &self,
        faces: &[usize],
        axis: &Axis,
        radius: f64,
        neighbors: &[usize],
        scan: &mut EndScan,
        guard: &mut BudgetGuard,
    ) -> Result<()> {
        for &nb in neighbors {
            guard.tick()?;
            let attrs = fresh_attrs(self.graph, nb)?;
            if let Some(kind) = step_kind(attrs, axis, radius, self.linear_tol) {
                scan.record_step(kind);
                scan.step_faces.push(nb);
                continue;
            }
            // Shoulder plane: look across its far ring (concave edges
            // only — its convex edges lead back into the bore itself,
            // and an exterior cap's far side is the body wall reached
            // through convex edges, which must not read as a step).
            if attrs.class != SurfaceClass::Plane {
                continue;
            }
            for far in neighbors_via(self.graph, nb, |c| c == DihedralClass::Concave) {
                if far == nb || faces.contains(&far) {
                    continue;
                }
                let fa = fresh_attrs(self.graph, far)?;
                if matches!(fa.class, SurfaceClass::Plane) {
                    continue; // sibling floor/shoulder patch
                }
                if let Some(kind) = step_kind(fa, axis, radius, self.linear_tol) {
                    scan.record_step(kind);
                    scan.step_faces.push(far);
                    scan.step_faces.push(nb); // keep the shoulder too
                }
            }
        }
        Ok(())
    }
}

/// Mean radial direction of wall samples: points into the void.
fn void_direction(points: &[[f64; 3]], axis: &Axis) -> [f64; 3] {
    let mut void_dir = [0.; 3];
    for p in points {
        let rel = sub(*p, axis.point);
        let radial = sub(rel, crate_scale(axis.direction, dot(rel, axis.direction)));
        void_dir = [
            void_dir[0] + radial[0],
            void_dir[1] + radial[1],
            void_dir[2] + radial[2],
        ];
    }
    let void_len = norm(void_dir);
    if void_len > 0. {
        crate_scale(void_dir, 1. / void_len)
    } else {
        [0.; 3]
    }
}

/// Counterbore merge: the larger coaxial step cylinder is itself a
/// cylinder cluster and gets recognized as its own shallow "hole". Fold
/// it into the bore it steps: any hole whose faces are all listed as
/// another hole's step faces is a step, not a standalone feature.
fn drop_step_holes(holes: Vec<HoleFeature>) -> Vec<HoleFeature> {
    let standalone: Vec<bool> = holes
        .iter()
        .enumerate()
        .map(|(i, h)| {
            !holes
                .iter()
                .enumerate()
                .any(|(j, other)| i != j && h.faces.iter().all(|f| other.step_faces.contains(f)))
        })
        .collect();
    holes
        .into_iter()
        .zip(standalone)
        .filter_map(|(h, keep)| keep.then_some(h))
        .collect()
}

// ==========================================================================
// Pocket recognition (checklist 867)
// ==========================================================================

mod pockets;
pub use pockets::{PocketFeature, PocketKind, PocketWalls, find_pockets};

mod small;
pub use small::*;
mod boss;
pub use boss::{BossFeature, BossKind, find_bosses};
mod rib;
pub use rib::{RibFeature, find_ribs};

#[cfg(test)]
#[path = "../tests/aag_small_features.rs"]
mod small_features_tests;
#[cfg(test)]
#[path = "../tests/aag_features.rs"]
mod tests;
