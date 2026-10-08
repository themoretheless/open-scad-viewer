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
mod tests {
    use super::*;

    fn budget() -> Budget {
        Budget::new(1_000_000, 8, 60_000).unwrap()
    }

    /// Round pocket revolved from an analytic wire: cylindrical wall blends
    /// into the flat floor through a quarter-arc (torus after revolution).
    /// Both line↔arc joints are G1 tangent and must classify Smooth.
    fn revolved_round_pocket() -> Model {
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
        // Material-left wire in the (r, z) half-plane: body z 0..8, radius 12,
        // pocket radius 10 with floor at z=5 and a radius-1 floor blend.
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
    fn revolve_wire_tangent_joints_classify_smooth() {
        let model = revolved_round_pocket();
        let graph = Aag::build(&model, &budget()).unwrap();
        // The full revolution quarters every joint circle; locate the joint
        // edges geometrically by their midpoints: wall↔fillet at (r=10, z=6),
        // fillet↔floor at (r=9, z=5).
        let mut joints = vec![];
        for edge in &graph.edges {
            let curve = &model.edges[edge.edge].curve;
            let [a, b] = curve.domain();
            let p = curve.evaluate((a + b) / 2.).unwrap().point;
            let (r, z) = (p[0].hypot(p[1]), p[2]);
            if ((r - 10.).abs() < 1e-6 && (z - 6.).abs() < 1e-6)
                || ((r - 9.).abs() < 1e-6 && (z - 5.).abs() < 1e-6)
            {
                joints.push((edge.edge, edge.class));
            }
        }
        assert_eq!(joints.len(), 8, "two joint circles, quartered: {joints:?}");
        assert!(
            joints
                .iter()
                .all(|&(_, class)| class == DihedralClass::Smooth),
            "tangent line→arc joints must be Smooth, got {joints:?}"
        );
        // The only non-matchable edges are the axis pole collapses, which are
        // genuinely degenerate geometry, not misclassified tangent joints.
        assert_eq!(graph.edges_of_class(DihedralClass::Mixed), Vec::<usize>::new());
        for edge in &graph.edges {
            if edge.class == DihedralClass::Degenerate {
                assert!(
                    model.edges[edge.edge].degenerate,
                    "edge {} falsely degenerate",
                    edge.edge
                );
            }
        }
    }

    #[test]
    fn cuboid_edges_are_all_convex() {
        let model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let graph = Aag::build(&model, &budget()).unwrap();
        assert_eq!(graph.nodes.len(), 6);
        assert_eq!(graph.edges.len(), 12);
        assert!(
            graph
                .edges
                .iter()
                .all(|e| e.class == DihedralClass::Convex && e.samples == AAG_DEFAULT_SAMPLES),
            "every box edge is convex with full sample evidence"
        );
        for node in &graph.nodes {
            assert_eq!(node.edges.len(), 4);
            assert_eq!(node.neighbors.len(), 4);
        }
        // Sign evidence sits strictly on the convex side.
        for edge in &graph.edges {
            assert!(edge.sign_max < -0.5 && edge.sign_min >= -1.0 - 1e-12);
        }
    }

    #[test]
    fn l_prism_has_one_concave_vertical_edge() {
        // L-shaped profile: one reentrant (270° material) corner.
        let profile = [[0., 0.], [2., 0.], [2., 1.], [1., 1.], [1., 2.], [0., 2.]];
        let model = crate::extrude_polygon(&profile, 0., 1.).unwrap();
        let graph = Aag::build(&model, &budget()).unwrap();
        let concave = graph.edges_of_class(DihedralClass::Concave);
        assert_eq!(
            concave.len(),
            1,
            "exactly the reentrant vertical edge is concave: {concave:?}"
        );
        let convex = graph.edges_of_class(DihedralClass::Convex);
        assert_eq!(concave.len() + convex.len(), graph.edges.len());
        assert!(graph.edges.iter().all(|e| e.class.is_matchable()));
    }

    /// Minimal hand-assembled model: one shared edge between two quad faces
    /// plus outline edges; geometry is exact, validation is the caller's job
    /// (AAG reads topology and geometry but never mutates or revalidates).
    fn two_quad_sheet() -> (Model, usize) {
        use brep_topology::{FaceUse, Loop, Shell, Vertex};
        let line = |a: [f64; 3], b: [f64; 3]| nurbs_core::curve::Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![a.to_vec(), b.to_vec()],
            weights: vec![1., 1.],
            periodic: false,
        };
        let quad = |c: [[f64; 3]; 4]| nurbs_core::surface::Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![vec![c[0].to_vec(), c[3].to_vec()], vec![c[1].to_vec(), c[2].to_vec()]],
            weights: vec![vec![1., 1.], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        };
        let points = [
            [0., 0., 0.],
            [1., 0., 0.],
            [2., 0., 0.],
            [0., 1., 0.],
            [1., 1., 0.],
            [2., 1., 0.],
        ];
        let mut m = Model(
            brep_topology::Model {
                vertices: points.map(|p| Vertex { point: p }).to_vec(),
                edges: vec![],
                loops: vec![],
                faces: vec![],
                shells: vec![],
                bodies: vec![],
                tolerance_mm: 1e-7,
            },
            crate::TopologyIds::default(),
        );
        let quads = [[0, 1, 4, 3], [1, 2, 5, 4]];
        let mut shared = std::collections::BTreeMap::new();
        let mut shared_edge = None;
        for corners in quads {
            let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
            let mut coedges = vec![];
            for i in 0..4 {
                let (a, b) = (corners[i], corners[(i + 1) % 4]);
                let key = (a.min(b), a.max(b));
                let edge = *shared.entry(key).or_insert_with(|| {
                    let id = m.0.edges.len();
                    m.0.edges.push(crate::Edge {
                        degenerate: false,
                        vertices: [key.0, key.1],
                        curve: line(points[key.0], points[key.1]),
                    });
                    id
                });
                if key == (1, 4) {
                    shared_edge = Some(edge);
                }
                coedges.push(crate::Coedge {
                    edge,
                    reversed: a > b,
                    pcurve: {
                        let (a, b) = (uv[i], uv[(i + 1) % 4]);
                        nurbs_core::curve::Curve {
                            degree: 1,
                            knots: vec![0., 0., 1., 1.],
                            control_points: vec![a.to_vec(), b.to_vec()],
                            weights: vec![1., 1.],
                            periodic: false,
                        }
                    },
                });
            }
            let outer = m.0.loops.len();
            m.0.loops.push(Loop { coedges });
            m.0.faces.push(crate::Face {
                surface: quad(corners.map(|i| points[i])),
                outer,
                holes: vec![],
            });
        }
        m.0.shells.push(Shell {
            faces: (0..2)
                .map(|face| FaceUse {
                    face,
                    reversed: false,
                })
                .collect(),
            closed: false,
        });
        (m, shared_edge.unwrap())
    }

    #[test]
    fn seam_edge_on_one_face_is_flagged_not_classified() {
        // Analytic cylinder/sphere in this kernel split periodic supports into
        // regular patches, so a seam is assembled synthetically: a shell using
        // one face twice makes the shared edge a two-use single-face edge.
        use brep_topology::{FaceUse, Shell};
        let (mut m, shared) = two_quad_sheet();
        m.0.shells[0] = Shell {
            faces: vec![
                FaceUse {
                    face: 0,
                    reversed: false,
                },
                FaceUse {
                    face: 0,
                    reversed: true,
                },
            ],
            closed: false,
        };
        let graph = Aag::build(&m, &budget()).unwrap();
        let seam = graph.edge(shared).unwrap();
        assert_eq!(seam.class, DihedralClass::Seam);
        assert_eq!(seam.uses.len(), 2);
        assert_eq!(seam.uses[0].face, seam.uses[1].face);
        assert!(!seam.class.is_matchable());
        assert_eq!(seam.samples, 0, "seam is trapped before sampling");
    }

    #[test]
    fn collapsed_edge_is_flagged_degenerate_not_classified() {
        let mut model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        // A pole-collapse edge carries the explicit marker; AAG must trap it
        // before touching normals near the pole.
        model.edges[0].degenerate = true;
        let graph = Aag::build(&model, &budget()).unwrap();
        let edge = graph.edge(0).unwrap();
        assert_eq!(edge.class, DihedralClass::Degenerate);
        assert_eq!(edge.samples, 0);
        // The other eleven edges still classify normally.
        assert_eq!(
            graph.edges_of_class(DihedralClass::Convex).len(),
            graph.edges.len() - 1
        );
    }

    #[test]
    fn coplanar_split_sheet_reads_smooth_and_boundary() {
        // Two coplanar quads sewn along one edge: the shared edge is smooth,
        // the outline edges are boundary (one use each).
        let (m, shared) = two_quad_sheet();
        let graph = Aag::build(&m, &budget()).unwrap();
        let smooth = graph.edges_of_class(DihedralClass::Smooth);
        assert_eq!(smooth, vec![shared], "the sewn coplanar edge is G1-smooth");
        let edge = graph.edge(shared).unwrap();
        assert_eq!(edge.uses.len(), 2);
        assert_ne!(edge.uses[0].face, edge.uses[1].face);
        assert_eq!(
            graph.edges_of_class(DihedralClass::Boundary).len(),
            m.edges.len() - 1,
            "open outline edges stay unclassified boundary"
        );
        // Adjacency: the two quads are mutual neighbors through the seam.
        assert_eq!(graph.nodes[0].neighbors, vec![1]);
        assert_eq!(graph.nodes[1].neighbors, vec![0]);
    }

    #[test]
    fn mixed_sign_aggregation_is_flagged() {
        // Curved-edge inflection: convex and concave samples on one edge must
        // aggregate to Mixed, never to a silent majority vote.
        assert_eq!(
            decide_class(&[SampleClass::Convex, SampleClass::Concave, SampleClass::Convex]),
            DihedralClass::Mixed
        );
        assert_eq!(
            decide_class(&[SampleClass::Smooth, SampleClass::Convex]),
            DihedralClass::Mixed
        );
        assert_eq!(
            decide_class(&[SampleClass::Convex, SampleClass::Convex]),
            DihedralClass::Convex
        );
        assert_eq!(decide_class(&[]), DihedralClass::Mixed);
    }

    #[test]
    fn budget_exhaustion_is_a_typed_error() {
        let model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let tight = Budget::with_iterations(2).unwrap();
        let error = Aag::build(&model, &tight).unwrap_err();
        assert!(
            error.code.contains("RESOURCE") || error.code.contains("BUDGET"),
            "budget exhaustion must surface as a resource error: {error:?}"
        );
        // A generous budget on the same model succeeds.
        assert!(Aag::build(&model, &budget()).is_ok());
    }

    #[test]
    fn invalid_sample_count_is_rejected() {
        let model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        assert!(Aag::build_sampled(&model, &budget(), 2).is_err());
        assert!(Aag::build_sampled(&model, &budget(), 4096).is_err());
    }

    #[test]
    fn large_model_builds_within_a_second() {
        // MAX_FACES (1024) caps one Model below the 10^4-face acceptance
        // target, so the largest in-contract synthetic is assembled by
        // concatenating validated cuboids: 150 boxes = 900 faces / 1800
        // edges. Build cost is linear in edges × samples, so a 10^4-face
        // model (~11x this) stays well under the 1 s bound if this does.
        let unit = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        use brep_topology::Shell;
        let copies = 150;
        let mut m = Model(
            brep_topology::Model {
                vertices: vec![],
                edges: vec![],
                loops: vec![],
                faces: vec![],
                shells: vec![],
                bodies: vec![],
                tolerance_mm: unit.tolerance_mm,
            },
            crate::TopologyIds::default(),
        );
        for _ in 0..copies {
            let (v, e, l, f, s) = (
                m.0.vertices.len(),
                m.0.edges.len(),
                m.0.loops.len(),
                m.0.faces.len(),
                m.0.shells.len(),
            );
            m.0.vertices.extend(unit.vertices.iter().cloned());
            m.0.edges.extend(unit.edges.iter().map(|edge| crate::Edge {
                degenerate: edge.degenerate,
                vertices: [edge.vertices[0] + v, edge.vertices[1] + v],
                curve: edge.curve.clone(),
            }));
            m.0.loops.extend(unit.loops.iter().map(|wire| crate::Loop {
                coedges: wire
                    .coedges
                    .iter()
                    .map(|c| crate::Coedge {
                        edge: c.edge + e,
                        reversed: c.reversed,
                        pcurve: c.pcurve.clone(),
                    })
                    .collect(),
            }));
            m.0.faces.extend(unit.faces.iter().map(|face| crate::Face {
                surface: face.surface.clone(),
                outer: face.outer + l,
                holes: face.holes.iter().map(|h| h + l).collect(),
            }));
            m.0.shells.extend(unit.shells.iter().map(|shell| Shell {
                faces: shell
                    .faces
                    .iter()
                    .map(|u| brep_topology::FaceUse {
                        face: u.face + f,
                        reversed: u.reversed,
                    })
                    .collect(),
                closed: shell.closed,
            }));
            m.0.bodies.extend(unit.bodies.iter().map(|body| brep_topology::Body {
                outer_shell: body.outer_shell + s,
                inner_shells: body.inner_shells.iter().map(|i| i + s).collect(),
            }));
        }
        assert_eq!(m.faces.len(), copies * 6);
        let start = std::time::Instant::now();
        let graph = Aag::build(&m, &budget()).unwrap();
        let elapsed = start.elapsed();
        assert_eq!(graph.nodes.len(), copies * 6);
        assert_eq!(graph.edges.len(), copies * 12);
        assert!(
            graph
                .edges
                .iter()
                .all(|e| e.class == DihedralClass::Convex)
        );
        // Hard 1 s bound applies to optimized builds; the debug suite runs
        // under parallel load, where wall-clock assertions are flaky.
        let bound = if cfg!(debug_assertions) { 20.0 } else { 1.0 };
        assert!(
            elapsed.as_secs_f64() < bound,
            "graph build on {} faces took {elapsed:?} (bound {bound} s)",
            copies * 6
        );
    }

    // ------------------------------------------------------------------
    // Face attributes (checklist 865)
    // ------------------------------------------------------------------
    use crate::analysis::surface_classify::SurfaceClass;

    fn attrs_of(graph: &Aag, face: usize) -> &FaceAttrs {
        graph.nodes[face].attrs.as_ref().expect("attrs attached")
    }

    #[test]
    fn cuboid_faces_are_planes_with_unit_area() {
        let model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let mut graph = Aag::build(&model, &budget()).unwrap();
        graph.attach_face_attrs(&model, &budget()).unwrap();
        for node in &graph.nodes {
            let attrs = attrs_of(&graph, node.face);
            assert_eq!(attrs.class, SurfaceClass::Plane);
            assert!((attrs.area - 1.).abs() < 1e-9, "unit cube face area");
            assert!(!attrs.fillet_like, "a box has no fillet faces");
            assert!(attrs.axis.is_none() && attrs.radius.is_none());
        }
    }

    #[test]
    fn cylinder_model_faces_classify_exactly() {
        let (radius, height) = (1.5, 4.);
        let model = crate::analytic::cylinder(radius, height).unwrap();
        let mut graph = Aag::build(&model, &budget()).unwrap();
        graph.attach_face_attrs(&model, &budget()).unwrap();
        let mut lateral = 0;
        let mut caps = 0;
        for node in &graph.nodes {
            let attrs = attrs_of(&graph, node.face);
            match attrs.class {
                SurfaceClass::Cylinder => {
                    lateral += 1;
                    assert!((attrs.radius.unwrap() - radius).abs() < 1e-6);
                    let axis = attrs.axis.unwrap();
                    assert!(
                        (axis.direction[2].abs() - 1.).abs() < 1e-6,
                        "cylinder axis along z: {axis:?}"
                    );
                }
                SurfaceClass::Plane => {
                    caps += 1;
                    assert!(
                        (attrs.area - std::f64::consts::PI * radius * radius).abs()
                            < 1e-6 * radius * radius,
                        "cap area"
                    );
                }
                other => panic!("unexpected class on analytic cylinder: {other:?}"),
            }
        }
        assert_eq!(caps, 2, "two planar caps");
        assert_eq!(lateral, model.faces.len() - 2, "all other faces lateral");
    }

    #[test]
    fn frustum_side_faces_classify_as_cone() {
        let model = crate::analytic::frustum(2., 1., 3.).unwrap();
        let mut graph = Aag::build(&model, &budget()).unwrap();
        graph.attach_face_attrs(&model, &budget()).unwrap();
        let mut cones = 0;
        let mut planes = 0;
        for node in &graph.nodes {
            let attrs = attrs_of(&graph, node.face);
            match attrs.class {
                SurfaceClass::Cone => {
                    cones += 1;
                    let axis = attrs.axis.unwrap();
                    assert!(
                        (axis.direction[2].abs() - 1.).abs() < 1e-3,
                        "cone axis along z: {axis:?}"
                    );
                }
                SurfaceClass::Plane => planes += 1,
                other => panic!("unexpected class on frustum: {other:?} ({attrs:?})"),
            }
        }
        assert_eq!(planes, 2);
        assert_eq!(cones, model.faces.len() - 2);
    }

    #[test]
    fn sphere_patches_classify_as_sphere() {
        let radius = 2.;
        let model = crate::analytic::sphere(radius).unwrap();
        let mut graph = Aag::build(&model, &budget()).unwrap();
        graph.attach_face_attrs(&model, &budget()).unwrap();
        for node in &graph.nodes {
            let attrs = attrs_of(&graph, node.face);
            assert_eq!(
                attrs.class,
                SurfaceClass::Sphere,
                "face {} on an analytic sphere: {attrs:?}",
                node.face
            );
            assert!((attrs.radius.unwrap() - radius).abs() < 1e-5);
            let center = attrs.center.unwrap();
            assert!(center.iter().all(|c| c.abs() < 1e-5), "centered: {center:?}");
        }
    }

    #[test]
    fn torus_patches_classify_as_torus() {
        let (major, minor) = (3., 1.);
        let model = crate::analytic::torus(major, minor).unwrap();
        let mut graph = Aag::build(&model, &budget()).unwrap();
        graph.attach_face_attrs(&model, &budget()).unwrap();
        for node in &graph.nodes {
            let attrs = attrs_of(&graph, node.face);
            assert_eq!(
                attrs.class,
                SurfaceClass::Torus,
                "face {} on an analytic torus: {attrs:?}",
                node.face
            );
            assert!((attrs.radius.unwrap() - minor).abs() < 1e-4);
        }
    }

    #[test]
    fn fillet_face_is_marked_fillet_like() {
        // One rolled edge on a box: the new cylindrical blend face has a
        // small radius and two smooth (G1) neighbors — the fillet signature.
        // `analytic_fillet` admits vertical (+Z) cuboid edges; find one.
        let model = crate::cuboid([0.; 3], [4., 4., 4.]).unwrap();
        let vertical = (0..model.edges.len())
            .find(|&e| {
                let edge = &model.edges[e];
                let domain = edge.curve.domain();
                let a = edge.curve.evaluate(domain[0]).unwrap().point;
                let b = edge.curve.evaluate(domain[1]).unwrap().point;
                (a[0] - b[0]).abs() < 1e-12
                    && (a[1] - b[1]).abs() < 1e-12
                    && (a[2] - b[2]).abs() > 1.
            })
            .expect("cuboid has vertical edges");
        let (model, _certificate) = crate::analytic_features::analytic_fillet(&model, vertical, 0.5)
            .expect("fillet on box edge");
        let mut graph = Aag::build(&model, &budget()).unwrap();
        // Fillet radius 0.5 exceeds the 5%-of-diagonal default threshold for
        // this 4 mm box, so pass the fillet band explicitly.
        let options = FaceAttrsOptions {
            fit_tolerance: None,
            fillet_radius_max: Some(1.0),
        };
        graph
            .attach_face_attrs_with(&model, &budget(), &options)
            .unwrap();
        let fillets: Vec<_> = graph
            .nodes
            .iter()
            .filter(|n| attrs_of(&graph, n.face).fillet_like)
            .collect();
        assert_eq!(fillets.len(), 1, "exactly the blend face: {fillets:?}");
        let attrs = attrs_of(&graph, fillets[0].face);
        assert_eq!(attrs.class, SurfaceClass::Cylinder);
        assert!((attrs.radius.unwrap() - 0.5).abs() < 1e-3);
        // No planar support face may be misclassified as a fillet.
        for node in &graph.nodes {
            let attrs = attrs_of(&graph, node.face);
            if attrs.class == SurfaceClass::Plane {
                assert!(!attrs.fillet_like);
            }
        }
    }

    #[test]
    fn face_attrs_respect_budget() {
        let model = crate::analytic::cylinder(1., 2.).unwrap();
        let mut graph = Aag::build(&model, &budget()).unwrap();
        let tight = Budget::with_iterations(10).unwrap();
        let error = graph.attach_face_attrs(&model, &tight).unwrap_err();
        assert!(
            error.code.contains("RESOURCE") || error.code.contains("BUDGET"),
            "budget exhaustion must surface as a resource error: {error:?}"
        );
        // A generous budget on the same model succeeds.
        graph.attach_face_attrs(&model, &budget()).unwrap();
        assert!(graph.nodes.iter().all(|n| n.attrs.is_some()));
    }

    #[test]
    fn face_attrs_invalidate_on_geometry_mutation() {
        // `Model` has no mutation versioning: staleness is detected by the
        // deterministic face-geometry hash, refreshed per face.
        let model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let mut graph = Aag::build(&model, &budget()).unwrap();
        graph.attach_face_attrs(&model, &budget()).unwrap();
        assert!(graph.stale_face_attrs(&model).is_empty());

        // Mutate one face's surface geometry in place.
        let mut mutated = model.clone();
        mutated.faces[0].surface.control_points[0][0][2] += 0.25;
        let stale = graph.stale_face_attrs(&mutated);
        assert_eq!(stale, vec![0], "only the mutated face goes stale");
        let refreshed = graph.refresh_face_attrs(&mutated, &budget()).unwrap();
        assert_eq!(refreshed, 1);
        assert!(graph.stale_face_attrs(&mutated).is_empty());
        let attrs = attrs_of(&graph, 0);
        // Bilinear quad with one corner lifted by 0.25: area exceeds the flat
        // 1.0 by ~1.5% and the face is no longer planar.
        assert!(
            attrs.area > 1.005 && attrs.area < 1.2,
            "area must track the mutation: {}",
            attrs.area
        );
        assert_ne!(attrs.class, SurfaceClass::Plane);
        // Untouched faces kept their original hash and attributes.
        for face in 1..6 {
            assert_eq!(
                attrs_of(&graph, face).geometry_hash,
                face_geometry_hash(&mutated, face)
            );
        }
        // Refresh on a clean cache is a no-op.
        assert_eq!(graph.refresh_face_attrs(&mutated, &budget()).unwrap(), 0);
    }

    #[test]
    fn noisy_cylinder_model_classifies_via_best_fit() {
        // Imported-geometry simulation: nudge every control point of the
        // lateral patches by ~1e-5, so the exact surface type is a perturbed
        // B-spline; with a fit tolerance above the noise, best-fit must still
        // classify every lateral face as Cylinder (and caps stay Plane).
        let (radius, height) = (1.5, 4.);
        let mut model = crate::analytic::cylinder(radius, height).unwrap();
        let noise = 1e-5;
        for (i, face) in model.faces.iter_mut().enumerate() {
            if face.surface.degree_u == 1 && face.surface.degree_v == 1 {
                continue; // planar cap
            }
            for (r, row) in face.surface.control_points.iter_mut().enumerate() {
                for (c, p) in row.iter_mut().enumerate() {
                    let phase = (i * 7 + r * 3 + c) as f64;
                    p[0] += noise * phase.sin();
                    p[1] += noise * (phase * 1.7).cos();
                    p[2] += noise * (phase * 0.3).sin();
                }
            }
        }
        let mut graph = Aag::build(&model, &budget()).unwrap();
        let options = FaceAttrsOptions {
            fit_tolerance: Some(1e-3),
            fillet_radius_max: None,
        };
        graph
            .attach_face_attrs_with(&model, &budget(), &options)
            .unwrap();
        let mut lateral = 0;
        let mut caps = 0;
        for node in &graph.nodes {
            let attrs = attrs_of(&graph, node.face);
            match attrs.class {
                SurfaceClass::Cylinder => {
                    lateral += 1;
                    assert!((attrs.radius.unwrap() - radius).abs() < 1e-2);
                    assert!(attrs.fit_deviation <= 1e-3 + 1e-9);
                }
                SurfaceClass::Plane => caps += 1,
                other => panic!("noisy cylinder face {} misclassified: {other:?}", node.face),
            }
        }
        assert_eq!(caps, 2);
        assert_eq!(lateral, model.faces.len() - 2);
        // Below the noise level the lateral faces must refuse the class.
        let strict = FaceAttrsOptions {
            fit_tolerance: Some(1e-9),
            fillet_radius_max: None,
        };
        graph
            .attach_face_attrs_with(&model, &budget(), &strict)
            .unwrap();
        assert!(
            graph
                .nodes
                .iter()
                .all(|n| attrs_of(&graph, n.face).class != SurfaceClass::Cylinder),
            "with tolerance below the noise, no face may claim Cylinder"
        );
    }
}
