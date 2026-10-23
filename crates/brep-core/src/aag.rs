//! Attributed Adjacency Graph over a validated [`Model`].
//!
//! Nodes are face indices; edges are B-rep edge indices carrying a dihedral
//! attribute. The dihedral sign is sampled at several interior points along
//! the edge curve — never once at the midpoint — because on curved edges the
//! sign can change. An edge whose sample signs disagree is [`DihedralClass::Mixed`]
//! and must be excluded from template matching. Degenerate/zero-length edges
//! and seam edges of periodic surfaces (garbage normals) are detected before
//! classification and reported as [`DihedralClass::Degenerate`] /
//! [`DihedralClass::Seam`] instead of being silently classified.
//!
//! Sign convention at one sample: with outward normals `n_a`, `n_b` and the
//! coedge traversal tangent `t` on face A, `d = cross(n_a, t)` points into
//! the interior of face A; `s = dot(normalize(d), n_b)` is negative for a
//! convex (material-below-270°) edge and positive for a concave one.

use crate::analysis::surface_classify::{
    Axis, SurfaceClass, classify_surface, face_area,
};
use crate::{Error, Model, Result};
use nurbs_core::foundation::guards::{Budget, BudgetGuard, Fnv1a, require_finite_f64};
use nurbs_core::surface::SurfaceSampler;

/// Default interior sample count per edge. Enough to expose sign changes on
/// single-inflection curves; cost is linear in edge count.
pub const AAG_DEFAULT_SAMPLES: usize = 9;
/// Dead zone on the normalized dihedral sign: |s| below this reads as tangent.
const SIGN_TOLERANCE: f64 = 1e-9;
/// Squared-distance-from-1 threshold on `dot(n_a, n_b)` for a smooth joint.
const SMOOTH_DOT_TOLERANCE: f64 = 1e-12;

/// Dihedral attribute of one graph edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DihedralClass {
    /// Material angle below 180°: outward normals fold away from each other.
    Convex,
    /// Material angle above 180°: inside corner (pocket wall/floor joint).
    Concave,
    /// Normals nearly antiparallel: coplanar joint with flipped orientation.
    Tangent,
    /// Normals nearly parallel: G1-continuous joint (fillet/wall transition).
    Smooth,
    /// Sample signs disagree along the edge; excluded from template matches.
    Mixed,
    /// Zero-length or collapsed edge, or a sample hit a pole with no normal.
    Degenerate,
    /// Both incident uses lie on one face (periodic surface seam).
    Seam,
    /// Fewer or more than two incident face uses (open or non-manifold).
    Boundary,
}

impl DihedralClass {
    /// Template matching (866–869) must only consume these classes.
    pub fn is_matchable(self) -> bool {
        matches!(
            self,
            Self::Convex | Self::Concave | Self::Tangent | Self::Smooth
        )
    }
}

/// One face-side use of a graph edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AagEdgeUse {
    pub face: usize,
    /// `FaceUse::reversed` of the owning shell use: flips the outward normal.
    pub reversed: bool,
}

/// One graph edge: a B-rep edge plus its dihedral attribute and evidence.
#[derive(Clone, Debug, PartialEq)]
pub struct AagEdge {
    /// Index into `model.edges`.
    pub edge: usize,
    /// Incident face uses in deterministic (first-seen) order.
    pub uses: Vec<AagEdgeUse>,
    pub class: DihedralClass,
    /// Minimum and maximum sampled dihedral sign (`NaN`-free; both zero when
    /// no classifiable sample was taken, e.g. degenerate/seam/boundary).
    pub sign_min: f64,
    pub sign_max: f64,
    /// Interior samples actually evaluated for classification.
    pub samples: usize,
}

/// One graph node: a face with its incident graph edges and adjacent faces.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AagNode {
    /// Index into `model.faces`.
    pub face: usize,
    /// Indices into [`Aag::edges`], ascending.
    pub edges: Vec<usize>,
    /// Adjacent face indices (the other side of each non-boundary edge),
    /// ascending, deduplicated; the face itself is never listed.
    pub neighbors: Vec<usize>,
    /// Cached face attributes (checklist 865). `None` until
    /// [`Aag::attach_face_attrs`] runs; the graph builder never fills it, so
    /// a plain graph stays cheap.
    pub attrs: Option<FaceAttrs>,
}

/// Cached per-face attributes (checklist 865): best-fit surface class,
/// rotational axis/radius, trimmed area, fillet-likeness and the geometry
/// hash used for invalidation.
#[derive(Clone, Debug, PartialEq)]
pub struct FaceAttrs {
    /// Index into `model.faces`.
    pub face: usize,
    /// Best-fit surface class within [`FaceAttrsOptions::fit_tolerance`].
    pub class: SurfaceClass,
    /// Axis for rotational surfaces (cylinder/cone/torus).
    pub axis: Option<Axis>,
    /// Center for spheres; `None` otherwise.
    pub center: Option<[f64; 3]>,
    /// Radius for cylinder/sphere; minor radius for torus; `None` otherwise.
    pub radius: Option<f64>,
    /// Trimmed face area (Green's theorem over the authored UV trims).
    pub area: f64,
    /// Small-radius rotational face with at least two distinct smooth (G1)
    /// neighbors — the signature of a fillet/blend face.
    pub fillet_like: bool,
    /// Maximum sampled deviation of the best-fit primitive.
    pub fit_deviation: f64,
    /// Deterministic hash of the face geometry at attribute build time; a
    /// mismatch against [`face_geometry_hash`] marks the cache stale.
    pub geometry_hash: u64,
}

/// Tuning for [`Aag::attach_face_attrs_with`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FaceAttrsOptions {
    /// Absolute length tolerance accepting a best-fit analytic class. `None`
    /// defaults to `max(1e-6, 1e-4 · model bbox diagonal)` — comfortably above
    /// the noise of an imported "almost analytic" surface, far below feature
    /// sizes.
    pub fit_tolerance: Option<f64>,
    /// Radius at or below which a rotational face may be fillet-like. `None`
    /// defaults to 5% of the model bbox diagonal.
    pub fillet_radius_max: Option<f64>,
}

impl Default for FaceAttrsOptions {
    fn default() -> Self {
        Self {
            fit_tolerance: None,
            fillet_radius_max: None,
        }
    }
}

/// Deterministic hash of one face's geometry: surface degrees, knots, control
/// points, weights and periodicity, plus every trim pcurve and coedge flag.
/// Any mutation of the face geometry changes this hash; `Model` carries no
/// version counter, so staleness of cached [`FaceAttrs`] is detected by
/// comparing against this hash (see [`Aag::stale_face_attrs`]).
pub fn face_geometry_hash(model: &Model, face_index: usize) -> u64 {
    let mut hash = Fnv1a::new();
    let Some(face) = model.faces.get(face_index) else {
        return hash.finish();
    };
    let surface = &face.surface;
    hash.write(&(surface.degree_u as u64).to_le_bytes());
    hash.write(&(surface.degree_v as u64).to_le_bytes());
    hash.write(&[surface.periodic_u as u8, surface.periodic_v as u8]);
    for k in surface.knots_u.iter().chain(&surface.knots_v) {
        hash.write_f64(*k);
    }
    for (row, wrow) in surface.control_points.iter().zip(&surface.weights) {
        for (p, w) in row.iter().zip(wrow) {
            for &c in p {
                hash.write_f64(c);
            }
            hash.write_f64(*w);
        }
    }
    for &wire in std::iter::once(&face.outer).chain(&face.holes) {
        for coedge in &model.loops[wire].coedges {
            hash.write(&(coedge.edge as u64).to_le_bytes());
            hash.write(&[coedge.reversed as u8]);
            let pcurve = &coedge.pcurve;
            hash.write(&(pcurve.degree as u64).to_le_bytes());
            for k in &pcurve.knots {
                hash.write_f64(*k);
            }
            for (p, w) in pcurve.control_points.iter().zip(&pcurve.weights) {
                for &c in p {
                    hash.write_f64(c);
                }
                hash.write_f64(*w);
            }
        }
    }
    hash.finish()
}

fn model_bbox_diagonal(model: &Model) -> f64 {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for p in model
        .faces
        .iter()
        .flat_map(|f| f.surface.control_points.iter().flatten())
    {
        for i in 0..3.min(p.len()) {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    }
    let diag = norm([hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]]);
    if diag.is_finite() && diag > 0. { diag } else { 1. }
}

/// Attributed adjacency graph over one model. Cheap to keep: only indices,
/// attributes and sign evidence; no geometry is copied.
#[derive(Clone, Debug, PartialEq)]
pub struct Aag {
    pub nodes: Vec<AagNode>,
    pub edges: Vec<AagEdge>,
}

fn aag_error(message: impl Into<String>) -> Error {
    Error::new("BREP_AAG_INPUT", message)
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

/// Per-sample raw class before aggregation across the edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SampleClass {
    Convex,
    Concave,
    Tangent,
    Smooth,
}

/// Aggregate per-sample classes into the edge attribute. Any disagreement is
/// [`DihedralClass::Mixed`]: a sign that wanders along the edge must never be
/// silently averaged into a template-matchable class.
fn decide_class(samples: &[SampleClass]) -> DihedralClass {
    let Some((&first, rest)) = samples.split_first() else {
        return DihedralClass::Mixed;
    };
    if rest.iter().all(|s| *s == first) {
        match first {
            SampleClass::Convex => DihedralClass::Convex,
            SampleClass::Concave => DihedralClass::Concave,
            SampleClass::Tangent => DihedralClass::Tangent,
            SampleClass::Smooth => DihedralClass::Smooth,
        }
    } else {
        DihedralClass::Mixed
    }
}

/// Classify one sample from outward normals and the traversal tangent on the
/// first face. Returns `None` when the sample cannot support a sign
/// (degenerate tangent or interior direction); callers treat that as
/// unreliable geometry, not as a zero sign.
fn classify_sample(n_a: [f64; 3], t_a: [f64; 3], n_b: [f64; 3]) -> Option<SampleClass> {
    let alignment = dot(n_a, n_b);
    if alignment > 1. - SMOOTH_DOT_TOLERANCE.sqrt() {
        return Some(SampleClass::Smooth);
    }
    if alignment < -(1. - SMOOTH_DOT_TOLERANCE.sqrt()) {
        return Some(SampleClass::Tangent);
    }
    let interior = cross(n_a, t_a);
    let length = norm(interior);
    if !(length > 0.) || !length.is_finite() {
        return None;
    }
    let sign = dot(interior.map(|v| v / length), n_b);
    if sign.abs() <= SIGN_TOLERANCE {
        // Numerically tangent but not smooth by normal alignment: normals
        // differ yet the wedge reads flat. Refuse a sharp verdict here.
        None
    } else if sign < 0. {
        Some(SampleClass::Convex)
    } else {
        Some(SampleClass::Concave)
    }
}

impl Aag {
    /// Build the graph over `model`, spending at most `budget` on iteration
    /// work. Every edge sample ticks the budget once; exhaustion aborts with
    /// a typed `resource()` error, never a partial graph.
    pub fn build(model: &Model, budget: &Budget) -> Result<Aag> {
        Self::build_sampled(model, budget, AAG_DEFAULT_SAMPLES)
    }

    /// Build with an explicit interior sample count (at least 3). More
    /// samples catch sign changes on higher-order curves at linear cost.
    pub fn build_sampled(model: &Model, budget: &Budget, samples: usize) -> Result<Aag> {
        require_finite_f64(model.tolerance_mm, "model.tolerance_mm")?;
        if samples < 3 || samples > 1024 {
            return Err(aag_error("AAG sample count must be in 3..=1024"));
        }
        let mut guard: BudgetGuard = budget.guard("aag-build");
        guard.check()?;

        // Incident face uses per edge, in deterministic traversal order.
        let mut uses: Vec<Vec<AagEdgeUse>> = vec![Vec::new(); model.edges.len()];
        for shell in &model.shells {
            for use_ in &shell.faces {
                guard.tick()?;
                let face = &model.faces[use_.face];
                for &wire in std::iter::once(&face.outer).chain(&face.holes) {
                    for coedge in &model.loops[wire].coedges {
                        uses[coedge.edge].push(AagEdgeUse {
                            face: use_.face,
                            reversed: use_.reversed,
                        });
                    }
                }
            }
        }

        let mut edges = Vec::with_capacity(model.edges.len());
        let mut node_edges = vec![Vec::<usize>::new(); model.faces.len()];
        for (edge_id, (edge, uses)) in model.edges.iter().zip(uses).enumerate() {
            guard.tick()?;
            let mut aag_edge = AagEdge {
                edge: edge_id,
                uses: uses.clone(),
                class: DihedralClass::Boundary,
                sign_min: 0.,
                sign_max: 0.,
                samples: 0,
            };
            // Pre-classification traps: collapsed geometry and periodic seams
            // produce garbage normals; report them instead of guessing.
            if edge.degenerate {
                aag_edge.class = DihedralClass::Degenerate;
            } else if uses.len() == 2 && uses[0].face == uses[1].face {
                aag_edge.class = DihedralClass::Seam;
            } else if uses.len() == 2 {
                let class = classify_edge(model, edge_id, &uses, samples, &mut guard, &mut aag_edge)?;
                aag_edge.class = class;
            }
            for use_ in &uses {
                node_edges[use_.face].push(edges.len());
            }
            edges.push(aag_edge);
        }

        let mut nodes = Vec::with_capacity(model.faces.len());
        for (face, mut incident) in node_edges.into_iter().enumerate() {
            guard.tick()?;
            incident.sort_unstable();
            incident.dedup();
            let mut neighbors: Vec<usize> = incident
                .iter()
                .flat_map(|&e| edges[e].uses.iter().map(move |u| u.face))
                .filter(|&other| other != face)
                .collect();
            neighbors.sort_unstable();
            neighbors.dedup();
            nodes.push(AagNode {
                face,
                edges: incident,
                neighbors,
                attrs: None,
            });
        }
        guard.check()?;
        Ok(Aag { nodes, edges })
    }

    /// Graph edge for a B-rep edge index.
    pub fn edge(&self, edge: usize) -> Option<&AagEdge> {
        self.edges.iter().find(|e| e.edge == edge)
    }

    /// All edges of one class, as B-rep edge indices in ascending order.
    pub fn edges_of_class(&self, class: DihedralClass) -> Vec<usize> {
        self.edges
            .iter()
            .filter(|e| e.class == class)
            .map(|e| e.edge)
            .collect()
    }

    /// One-shot face-attribute pass (checklist 865): classify every face
    /// surface by best-fit, measure its trimmed area and mark fillet-like
    /// faces, caching the result in each node. See
    /// [`Aag::attach_face_attrs_with`].
    pub fn attach_face_attrs(&mut self, model: &Model, budget: &Budget) -> Result<()> {
        self.attach_face_attrs_with(model, budget, &FaceAttrsOptions::default())
    }

    /// Attribute pass with explicit tolerances. Idempotent: re-running on an
    /// unmutated model recomputes and replaces the same values. Every face
    /// ticks the budget for classification and quadrature; exhaustion aborts
    /// with a typed resource error and leaves prior attributes untouched.
    pub fn attach_face_attrs_with(
        &mut self,
        model: &Model,
        budget: &Budget,
        options: &FaceAttrsOptions,
    ) -> Result<()> {
        if self.nodes.len() != model.faces.len() {
            return Err(aag_error("AAG node count does not match model faces"));
        }
        let diagonal = model_bbox_diagonal(model);
        let fit_tolerance = options
            .fit_tolerance
            .unwrap_or_else(|| (1e-4 * diagonal).max(1e-6));
        require_finite_f64(fit_tolerance, "fit_tolerance")?;
        if fit_tolerance <= 0. {
            return Err(aag_error("fit_tolerance must be positive"));
        }
        let fillet_radius_max = options.fillet_radius_max.unwrap_or(0.05 * diagonal);
        require_finite_f64(fillet_radius_max, "fillet_radius_max")?;
        let mut guard: BudgetGuard = budget.guard("aag-face-attrs");
        guard.check()?;

        let mut attrs = Vec::with_capacity(model.faces.len());
        for face in 0..model.faces.len() {
            guard.tick()?;
            let classification =
                classify_surface(&model.faces[face].surface, fit_tolerance, budget)?;
            let area = face_area(model, face, budget)?;
            require_finite_f64(area, "face area")?;
            attrs.push((
                classification.class,
                classification.axis,
                classification.center,
                classification.radius,
                classification.max_deviation,
                area,
                face_geometry_hash(model, face),
            ));
        }
        // Fillet-likeness needs the graph neighborhood, so it is resolved in
        // a second pass once every face carries a class.
        for (face, (class, axis, center, radius, fit_deviation, area, geometry_hash)) in
            attrs.into_iter().enumerate()
        {
            guard.tick()?;
            let rotational = matches!(class, SurfaceClass::Cylinder | SurfaceClass::Torus);
            let small_radius = radius.is_some_and(|r| r <= fillet_radius_max);
            let smooth_neighbors: usize = self.nodes[face]
                .edges
                .iter()
                .filter(|&&e| self.edges[e].class == DihedralClass::Smooth)
                .flat_map(|&e| self.edges[e].uses.iter())
                .map(|u| u.face)
                .filter(|&other| other != face)
                .collect::<std::collections::BTreeSet<_>>()
                .len();
            let fillet_like = rotational && small_radius && smooth_neighbors >= 2;
            self.nodes[face].attrs = Some(FaceAttrs {
                face,
                class,
                axis,
                center,
                radius,
                area,
                fillet_like,
                fit_deviation,
                geometry_hash,
            });
        }
        guard.check()?;
        Ok(())
    }

    /// Faces whose cached attributes are stale: either never computed or the
    /// face geometry hash changed since. `Model` has no mutation versioning,
    /// so invalidation is hash-based and deterministic.
    pub fn stale_face_attrs(&self, model: &Model) -> Vec<usize> {
        self.nodes
            .iter()
            .filter(|node| {
                node.attrs
                    .as_ref()
                    .is_none_or(|a| a.geometry_hash != face_geometry_hash(model, node.face))
            })
            .map(|node| node.face)
            .collect()
    }

    /// Recompute attributes for stale faces only; returns how many were
    /// refreshed. Neighbor-dependent `fillet_like` is re-resolved for every
    /// face afterwards, because a refreshed neighbor can flip the verdict.
    pub fn refresh_face_attrs(&mut self, model: &Model, budget: &Budget) -> Result<usize> {
        let stale = self.stale_face_attrs(model);
        if stale.is_empty() {
            return Ok(0);
        }
        let options = FaceAttrsOptions::default();
        let diagonal = model_bbox_diagonal(model);
        let fit_tolerance = options
            .fit_tolerance
            .unwrap_or_else(|| (1e-4 * diagonal).max(1e-6));
        let mut guard: BudgetGuard = budget.guard("aag-face-attrs-refresh");
        for &face in &stale {
            guard.tick()?;
            let classification =
                classify_surface(&model.faces[face].surface, fit_tolerance, budget)?;
            let area = face_area(model, face, budget)?;
            let previous = self.nodes[face].attrs.take();
            self.nodes[face].attrs = Some(FaceAttrs {
                face,
                class: classification.class,
                axis: classification.axis,
                center: classification.center,
                radius: classification.radius,
                area,
                fillet_like: previous.is_some_and(|p| p.fillet_like),
                fit_deviation: classification.max_deviation,
                geometry_hash: face_geometry_hash(model, face),
            });
        }
        let fillet_radius_max = options.fillet_radius_max.unwrap_or(0.05 * diagonal);
        for face in 0..self.nodes.len() {
            guard.tick()?;
            let Some(attrs) = &self.nodes[face].attrs else {
                continue;
            };
            let rotational = matches!(attrs.class, SurfaceClass::Cylinder | SurfaceClass::Torus);
            let small_radius = attrs.radius.is_some_and(|r| r <= fillet_radius_max);
            let smooth_neighbors: usize = self.nodes[face]
                .edges
                .iter()
                .filter(|&&e| self.edges[e].class == DihedralClass::Smooth)
                .flat_map(|&e| self.edges[e].uses.iter())
                .map(|u| u.face)
                .filter(|&other| other != face)
                .collect::<std::collections::BTreeSet<_>>()
                .len();
            if let Some(attrs) = &mut self.nodes[face].attrs {
                attrs.fillet_like = rotational && small_radius && smooth_neighbors >= 2;
            }
        }
        guard.check()?;
        Ok(stale.len())
    }
}

/// Sample the dihedral sign along one two-manifold edge and aggregate.
fn classify_edge(
    model: &Model,
    edge_id: usize,
    uses: &[AagEdgeUse],
    samples: usize,
    guard: &mut BudgetGuard,
    out: &mut AagEdge,
) -> Result<DihedralClass> {
    let edge = &model.edges[edge_id];
    let domain = edge.curve.domain();
    require_finite_f64(domain[0], "edge.domain[0]")?;
    require_finite_f64(domain[1], "edge.domain[1]")?;
    let samplers = [
        SurfaceSampler::new(&model.faces[uses[0].face].surface)?,
        SurfaceSampler::new(&model.faces[uses[1].face].surface)?,
    ];
    // One coedge of each incident face, to map edge parameters into face UV.
    let coedge_of = |face: usize| -> Option<&crate::Coedge> {
        std::iter::once(&model.faces[face].outer)
            .chain(&model.faces[face].holes)
            .flat_map(|&wire| model.loops[wire].coedges.iter())
            .find(|c| c.edge == edge_id)
    };
    let (Some(coedge_a), Some(coedge_b)) = (coedge_of(uses[0].face), coedge_of(uses[1].face))
    else {
        return Ok(DihedralClass::Boundary);
    };

    let mut classes = Vec::with_capacity(samples);
    let (mut sign_min, mut sign_max) = (f64::INFINITY, f64::NEG_INFINITY);
    let mut extent = 0f64;
    let mut previous: Option<[f64; 3]> = None;
    for i in 0..samples {
        guard.tick()?;
        let t = (i + 1) as f64 / (samples + 1) as f64;
        let u = domain[0] + t * (domain[1] - domain[0]);
        let eval = edge.curve.evaluate(u)?;
        let (Some(d1), true) = (&eval.d1, eval.point.len() == 3) else {
            return Ok(DihedralClass::Degenerate);
        };
        let tangent: [f64; 3] = [d1[0], d1[1], d1[2]];
        let point: [f64; 3] = [eval.point[0], eval.point[1], eval.point[2]];
        require_finite_f64(point[0], "edge.point.x")?;
        require_finite_f64(point[1], "edge.point.y")?;
        require_finite_f64(point[2], "edge.point.z")?;
        if let Some(p) = previous {
            extent = extent.max(norm([
                point[0] - p[0],
                point[1] - p[1],
                point[2] - p[2],
            ]));
        }
        previous = Some(point);

        let mut normals = [[0.; 3]; 2];
        let mut traversal = tangent;
        for (side, (coedge, use_)) in [(coedge_a, uses[0]), (coedge_b, uses[1])]
            .into_iter()
            .enumerate()
        {
            // Validated model contract: pcurve at fraction f agrees with the
            // 3D edge at fraction `reversed ? 1 - f : f`. Both incident
            // pcurves must therefore be sampled at the fraction that maps to
            // THIS sample's 3D point — coedge traversal directions along the
            // shared edge differ, so a bare shared t compares opposite ends.
            let pcurve_domain = coedge.pcurve.domain();
            let f = if coedge.reversed { 1. - t } else { t };
            let pu = pcurve_domain[0] + f * (pcurve_domain[1] - pcurve_domain[0]);
            let uv = coedge.pcurve.evaluate(pu)?.point;
            if uv.len() != 2 {
                return Err(aag_error("AAG expects 2D pcurves"));
            }
            let Some(mut n) = samplers[side].evaluate(uv[0], uv[1])?.unit_normal() else {
                // A pole or a singular chart gives no reliable normal: this is
                // exactly the garbage-normal case to flag, not to classify.
                return Ok(DihedralClass::Degenerate);
            };
            if use_.reversed {
                n = [-n[0], -n[1], -n[2]];
            }
            normals[side] = n;
            if side == 0 && coedge.reversed {
                traversal = [-tangent[0], -tangent[1], -tangent[2]];
            }
        }
        let Some(class) = classify_sample(normals[0], traversal, normals[1]) else {
            return Ok(DihedralClass::Degenerate);
        };
        let interior = cross(normals[0], traversal);
        let length = norm(interior);
        let sign = dot(interior.map(|v| v / length), normals[1]);
        sign_min = sign_min.min(sign);
        sign_max = sign_max.max(sign);
        classes.push(class);
    }
    out.samples = classes.len();
    if sign_min.is_finite() {
        out.sign_min = sign_min;
        out.sign_max = sign_max;
    }
    // Sampled chord extent below the model tolerance: geometrically collapsed.
    if extent <= model.tolerance_mm {
        return Ok(DihedralClass::Degenerate);
    }
    Ok(decide_class(&classes))
}

#[cfg(test)]
#[path = "tests/aag.rs"]
mod tests;
