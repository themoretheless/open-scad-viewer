//! Exact Boolean engine for a canonical sphere against a faceted analytic
//! "mate" solid whose boundary meets the sphere along circles: the box
//! (every face plane cuts a circle) and the axial cylinder (cap planes cut
//! circles, the wall meets the sphere in horizontal rings).
//!
//! The mate contributes the curve network: per face, exact section circles
//! with the phi intervals that lie inside the face, bounded by "hit" vertices
//! where the sphere meets the mate's edges (shared by the two faces at that
//! edge). The engine splits every arc where it crosses a sphere patch seam
//! and wherever its sweep exceeds a quadrant, so each piece is a single-span
//! rational quadratic inside one mate face and one sphere patch; it then
//! reads face and patch regions off small planar UV arrangements (boundary
//! pieces plus arc pcurves), classifies each region against the other
//! operand by an interior sample, trims every kept face's surface to its
//! region, and assembles through `imprint_pipeline`.
//!
//! Every degenerate configuration is an explicit `BREP_UNSUPPORTED_OPERATION`,
//! never a numerical fallback: tangencies, a section through a sphere patch
//! pole or vertex, a hit or seam event that cannot be certified apart from
//! its neighbours, arcs meeting tangentially in UV, and any region sample
//! that falls inside the error band of a classification.
use crate::imprint_pipeline::{self, SpatialRelation};
use crate::intersections::sphere_sphere::{self, CanonicalSphere, PatchUvSection};
use crate::{Coedge, Edge, Face, FaceUse, Loop, Model, Vertex};
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};
use std::collections::BTreeMap;

pub(crate) const TAU: f64 = std::f64::consts::TAU;
/// UV clearance demanded between an event and a patch corner.
const CORNER: f64 = 1e-7;
/// UV boundary-membership margin.
pub(crate) const BOUNDARY_UV: f64 = 1e-9;
/// Largest 3D sweep of one arc piece (single-span exactness margin).
const MAX_SPAN: f64 = std::f64::consts::FRAC_PI_2 + 1e-9;
/// Angular clearance between two events on one section circle.
pub(crate) const EVENT_GAP: f64 = 1e-8;
/// Offsets tried when sampling a UV region interior.
const SAMPLE_DELTAS: [f64; 4] = [1e-2, 1e-3, 1e-4, 1e-5];

pub(crate) fn unsupported(message: impl Into<String>) -> Error {
    Error::new("BREP_UNSUPPORTED_OPERATION", message)
}
pub(crate) fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub(crate) fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub(crate) fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub(crate) fn norm(a: [f64; 3]) -> f64 {
    a[0].hypot(a[1]).hypot(a[2])
}
pub(crate) fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    norm(sub(a, b))
}
pub(crate) fn point3(p: &[f64]) -> [f64; 3] {
    [p[0], p[1], p[2]]
}

/// Original shell orientation of one face of a single-shell model.
pub(crate) fn face_reversed(model: &Model, face: usize) -> bool {
    model.shells[0]
        .faces
        .iter()
        .find(|use_| use_.face == face)
        .map(|use_| use_.reversed)
        .unwrap_or(false)
}

/// Affine inversion of a bilinear affine face `P(u,v) = P00 + u U + v V`.
pub(crate) fn affine_uv_of(surface: &Surface, p: [f64; 3]) -> [f64; 2] {
    let p00 = point3(&surface.control_points[0][0]);
    let u = sub(point3(&surface.control_points[1][0]), p00);
    let v = sub(point3(&surface.control_points[0][1]), p00);
    let rel = sub(p, p00);
    [dot(rel, u) / dot(u, u), dot(rel, v) / dot(v, v)]
}

/// Exact pcurve of a planar arc on an affine face: the affine image of its
/// control points with unchanged weights.
pub(crate) fn affine_arc_pcurve(surface: &Surface, arc: &Curve) -> Curve {
    Curve {
        degree: arc.degree,
        knots: arc.knots.clone(),
        control_points: arc
            .control_points
            .iter()
            .map(|p| affine_uv_of(surface, point3(p)).to_vec())
            .collect(),
        weights: arc.weights.clone(),
        periodic: false,
    }
}

/// Signed-distance contact of the sphere with an oriented plane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PlaneContact {
    Inside,
    Outside,
    Crossing,
    Tangent,
}

pub(crate) fn plane_contact(
    origin: [f64; 3],
    outward: [f64; 3],
    sphere: &CanonicalSphere,
    band: f64,
) -> (PlaneContact, f64) {
    let d = dot(outward, sub(sphere.center, origin));
    let r = sphere.radius;
    let kind = if d < -(r + band) {
        PlaneContact::Inside
    } else if d > r + band {
        PlaneContact::Outside
    } else if d.abs() < r - band {
        PlaneContact::Crossing
    } else {
        PlaneContact::Tangent
    };
    (kind, d)
}

// ---------------------------------------------------------------------------
// Sphere patch UV helpers (quarter disk: u-axis, quarter arc, v-axis)
// ---------------------------------------------------------------------------

fn piece_of(w: [f64; 2]) -> Result<usize> {
    for corner in [[0., 0.], [1., 0.], [0., 1.]] {
        if (w[0] - corner[0]).hypot(w[1] - corner[1]) <= CORNER {
            return Err(unsupported(
                "Sphere Boolean: section passes through a sphere vertex region",
            ));
        }
    }
    let on_u_axis = w[1].abs() <= BOUNDARY_UV;
    let on_arc = (w[0] * w[0] + w[1] * w[1] - 1.).abs() <= BOUNDARY_UV;
    let on_v_axis = w[0].abs() <= BOUNDARY_UV;
    match (on_u_axis, on_arc, on_v_axis) {
        (true, false, false) => Ok(0),
        (false, true, false) => Ok(1),
        (false, false, true) => Ok(2),
        _ => Err(unsupported(
            "Sphere Boolean: section endpoint is not on exactly one patch boundary piece",
        )),
    }
}
fn snap_to_piece(piece: usize, w: [f64; 2]) -> [f64; 2] {
    match piece {
        0 => [w[0], 0.],
        2 => [0., w[1]],
        _ => {
            let n = w[0].hypot(w[1]);
            [w[0] / n, w[1] / n]
        }
    }
}
/// Piece parameter in loop (CCW) direction: piece 0 is (0,0)->(1,0), piece 1
/// the rational quarter arc (1,0)->(0,1), piece 2 is (0,1)->(0,0).
fn piece_tau(piece: usize, w: [f64; 2]) -> f64 {
    match piece {
        0 => w[0],
        2 => 1. - w[1],
        _ => quarter_arc_parameter(w[0], w[1]),
    }
}
/// Parameter of the point at direction (cos, sin) on the canonical
/// quarter arc from angle 0 to 90 degrees (weights 1, cos(pi/4), 1).
pub(crate) fn quarter_arc_parameter(cos: f64, sin: f64) -> f64 {
    let s = sin / (1. + cos);
    let aw = sphere_sphere::ARC_WEIGHT;
    s / (aw + s * (1. - aw))
}
fn strictly_inside_quarter_disk(uv: [f64; 2], margin: f64) -> bool {
    uv[0] > margin && uv[1] > margin && uv[0] * uv[0] + uv[1] * uv[1] < 1. - margin
}

// ---------------------------------------------------------------------------
// Curve network
// ---------------------------------------------------------------------------

/// Section circle on one mate face (a face may carry several).
pub(crate) struct Circle {
    pub(crate) face: usize,
    pub(crate) center: [f64; 3],
    pub(crate) rho: f64,
    pub(crate) e1: [f64; 3],
    pub(crate) e2: [f64; 3],
    /// Edge hits on this circle: (phi, vertex). The mate turns them into
    /// `inside` intervals.
    pub(crate) hits: Vec<(f64, usize)>,
    /// Parts of the circle inside the face, as phi intervals with `hi > lo`;
    /// a full ring is a single `[0, TAU]` interval without hits.
    pub(crate) inside: Vec<[f64; 2]>,
    /// Hit vertex at the start / end of each inside interval (None for a ring).
    pub(crate) ends: Vec<[Option<usize>; 2]>,
    /// Interior events (sphere-seam crossings) per interval.
    pub(crate) events: Vec<Vec<(f64, usize)>>,
    /// The circle is a great circle in a sphere seam plane: it coincides
    /// with four original sphere edges instead of cutting the patches.
    seam: bool,
}

impl Circle {
    pub(crate) fn new(face: usize, center: [f64; 3], rho: f64, e1: [f64; 3], e2: [f64; 3]) -> Self {
        Self {
            face,
            center,
            rho,
            e1,
            e2,
            hits: Vec::new(),
            inside: Vec::new(),
            ends: Vec::new(),
            events: Vec::new(),
            seam: false,
        }
    }
    pub(crate) fn phi_of(&self, point: [f64; 3]) -> f64 {
        let d = sub(point, self.center);
        dot(d, self.e2).atan2(dot(d, self.e1)).rem_euclid(TAU)
    }
    pub(crate) fn point_at(&self, phi: f64) -> [f64; 3] {
        let (sin, cos) = phi.sin_cos();
        std::array::from_fn(|k| self.center[k] + self.rho * (cos * self.e1[k] + sin * self.e2[k]))
    }
    pub(crate) fn push_ring(&mut self) {
        self.inside.push([0., TAU]);
        self.ends.push([None, None]);
        self.events.push(Vec::new());
    }
    pub(crate) fn push_interval(&mut self, lo: f64, hi: f64, from: usize, to: usize) {
        self.inside.push([lo, hi]);
        self.ends.push([Some(from), Some(to)]);
        self.events.push(Vec::new());
    }
    /// Sort the hits and refuse coincident ones. Returns the sorted hits.
    pub(crate) fn sorted_hits(&self) -> Result<Vec<(f64, usize)>> {
        let mut hits = self.hits.clone();
        hits.sort_by(|a, b| a.0.total_cmp(&b.0));
        let n = hits.len();
        for k in 0..n {
            let gap = if k + 1 < n {
                hits[k + 1].0 - hits[k].0
            } else {
                hits[0].0 + TAU - hits[k].0
            };
            if n > 1 && gap < EVENT_GAP {
                return Err(unsupported(
                    "Sphere Boolean: two edge hits coincide on one section circle",
                ));
            }
        }
        Ok(hits)
    }
}

/// A special vertex of the result: an edge hit, a seam crossing or a
/// refinement point, with every original edge it lies on as
/// (operand, edge, edge parameter).
struct VRec {
    point: [f64; 3],
    on_edges: Vec<(usize, usize, f64)>,
    /// The vertex coincides with an original vertex (operand, index).
    orig: Option<(usize, usize)>,
}

/// One arc piece of the intersection network.
pub(crate) struct Arc {
    pub(crate) face: usize,
    /// Owning sphere patch; `None` when the piece runs along a sphere seam
    /// and is an original sphere edge (or a piece of one).
    pub(crate) patch: Option<usize>,
    pub(crate) ends: [VKey; 2],
    /// Exact rational quadratic over [0, 1], +phi orientation.
    pub(crate) curve: Curve,
    key: EKey,
    /// The +phi orientation runs against the 3D edge `key`.
    reversed: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum VKey {
    Orig(usize, usize),
    Special(usize),
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum EKey {
    Arc(usize),
    Orig(usize, usize),
    /// Piece `j` of split edge `(operand, edge)`.
    Piece(usize, usize, usize),
}

/// One undirected UV piece of a chart arrangement.
struct Piece {
    v: [usize; 2],
    /// Forward pcurve, from v[0] to v[1].
    pcurve: Curve,
    key: EKey,
    /// Forward direction runs against the 3D edge.
    reversed: bool,
}

/// The faceted analytic operand paired with the sphere.
pub(crate) trait Mate: Sized {
    fn model(&self) -> &Model;
    /// Strict solid classification with the shared error band; ambiguous
    /// samples are refusals.
    fn inside(&self, point: [f64; 3], band: f64) -> Result<bool>;
    /// Chart coordinates of a boundary point known to lie on `face`.
    fn uv_of(&self, face: usize, point: [f64; 3]) -> Result<[f64; 2]>;
    /// Exact pcurve of a network arc on `face`; the engine snaps the ends to
    /// the chart vertices and verifies the curve against the surface.
    fn arc_pcurve(&self, face: usize, arc: &Curve) -> Result<Curve>;
    /// Author the section network into `imprint.circles`, registering hits
    /// through `Imprint::hit`. `Some` resolves the pair by empty algebra
    /// before any arc is needed.
    fn network(&self, imprint: &mut Imprint<'_, Self>) -> Result<Option<SpatialRelation>>;
}

pub(crate) struct Imprint<'m, M: Mate> {
    src: [&'m Model; 2],
    pub(crate) mate_index: usize,
    pub(crate) mate: &'m M,
    pub(crate) sphere: CanonicalSphere,
    pub(crate) band: f64,
    want_inside: [bool; 2],
    flip: [bool; 2],
    pub(crate) circles: Vec<Circle>,
    /// Sphere: per face, coedge index of each UV boundary piece.
    sphere_pieces: [[usize; 3]; 8],
    specials: Vec<VRec>,
    /// (operand, edge) -> sorted (t, special vertex).
    splits: BTreeMap<(usize, usize), Vec<(f64, usize)>>,
    /// UV of a special vertex in a sphere patch chart.
    patch_uv: BTreeMap<(usize, usize), [f64; 2]>,
    arcs: Vec<Arc>,
    vertices: Vec<Vertex>,
    edges: Vec<Edge>,
    loops: Vec<Loop>,
    faces: Vec<Face>,
    shell: Vec<FaceUse>,
    vmap: BTreeMap<VKey, usize>,
    emap: BTreeMap<EKey, usize>,
}

/// Run the engine for `a op b`, where operand `mate_index` is the mate and
/// the other operand the recognized sphere.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run<M: Mate>(
    a: &Model,
    b: &Model,
    mate_index: usize,
    mate: &M,
    sphere: CanonicalSphere,
    band: f64,
    operation: &str,
) -> Result<Model> {
    let tolerance = a.tolerance_mm.max(b.tolerance_mm);
    let mut imprint = Imprint::new(a, b, mate_index, mate, sphere, band, operation);
    if let Some(relation) = mate.network(&mut imprint)? {
        return imprint_pipeline::regularized_empty_algebra(a, b, operation, relation);
    }
    if let Some(relation) = imprint.no_arcs_relation()? {
        return imprint_pipeline::regularized_empty_algebra(a, b, operation, relation);
    }
    imprint.sphere_crossings()?;
    imprint.refine_and_build_arcs()?;
    imprint.emit_mate()?;
    imprint.emit_sphere()?;
    match imprint_pipeline::assemble_imprint_components(
        imprint.vertices,
        imprint.edges,
        imprint.loops,
        imprint.faces,
        imprint.shell,
        tolerance,
        &[a, b],
    ) {
        Ok(model) => Ok(model),
        Err(e) if e.code == "BREP_RESOURCE_LIMIT" => Err(e),
        Err(e) => Err(unsupported(format!(
            "Sphere Boolean result failed validation: {}",
            e.message
        ))),
    }
}

impl<'m, M: Mate> Imprint<'m, M> {
    fn new(
        a: &'m Model,
        b: &'m Model,
        mate_index: usize,
        mate: &'m M,
        sphere: CanonicalSphere,
        band: f64,
        operation: &str,
    ) -> Self {
        let src = [a, b];
        let sphere_model = src[1 - mate_index];
        let sphere_pieces: [[usize; 3]; 8] = std::array::from_fn(|face| {
            let wire = &sphere_model.loops[sphere_model.faces[face].outer];
            let mut map = [usize::MAX; 3];
            for (i, coedge) in wire.coedges.iter().enumerate() {
                let piece = if sphere_sphere::unit_quarter_arc(&coedge.pcurve) {
                    1
                } else if sphere_sphere::axis_line(&coedge.pcurve, [0., 0.], [1., 0.]) {
                    0
                } else {
                    2
                };
                map[piece] = i;
            }
            map
        });
        // Operand order for these two arrays is call order (a, b).
        let want_inside = [
            operation == "intersection",
            matches!(operation, "intersection" | "difference"),
        ];
        let flip = [false, operation == "difference"];
        Self {
            src,
            mate_index,
            mate,
            sphere,
            band,
            want_inside,
            flip,
            circles: Vec::new(),
            sphere_pieces,
            specials: Vec::new(),
            splits: BTreeMap::new(),
            patch_uv: BTreeMap::new(),
            arcs: Vec::new(),
            vertices: Vec::new(),
            edges: Vec::new(),
            loops: Vec::new(),
            faces: Vec::new(),
            shell: Vec::new(),
            vmap: BTreeMap::new(),
            emap: BTreeMap::new(),
        }
    }

    fn sphere_index(&self) -> usize {
        1 - self.mate_index
    }
    fn sphere_model(&self) -> &'m Model {
        self.src[self.sphere_index()]
    }

    fn attach_edge(&mut self, id: usize, o: usize, e: usize, t: f64) {
        self.specials[id].on_edges.push((o, e, t));
        let table = self.splits.entry((o, e)).or_default();
        table.push((t, id));
        table.sort_by(|a, b| a.0.total_cmp(&b.0));
    }
    fn special(&mut self, point: [f64; 3]) -> usize {
        let id = self.specials.len();
        self.specials.push(VRec {
            point,
            on_edges: Vec::new(),
            orig: None,
        });
        id
    }
    /// Register a sphere hit on mate edge `e` at parameter `t`.
    pub(crate) fn hit(&mut self, point: [f64; 3], e: usize, t: f64) -> usize {
        let id = self.special(point);
        self.attach_edge(id, self.mate_index, e, t);
        id
    }
    /// True when the point is strictly inside the sphere; ambiguous → refusal.
    pub(crate) fn inside_sphere(&self, point: [f64; 3]) -> Result<bool> {
        let d = dist(point, self.sphere.center) - self.sphere.radius;
        if d < -self.band {
            Ok(true)
        } else if d > self.band {
            Ok(false)
        } else {
            Err(unsupported(
                "Sphere Boolean: region classification falls within the error band of the sphere",
            ))
        }
    }
    /// Containment relation "mate contains sphere" in call order.
    pub(crate) fn mate_contains_sphere(&self) -> SpatialRelation {
        if self.mate_index == 0 {
            SpatialRelation::AContainsB
        } else {
            SpatialRelation::BContainsA
        }
    }
    fn sphere_contains_mate(&self) -> SpatialRelation {
        if self.mate_index == 0 {
            SpatialRelation::BContainsA
        } else {
            SpatialRelation::AContainsB
        }
    }

    /// No arcs at all means the boundaries never meet: containment by a
    /// single certified sample each way, else disjoint.
    fn no_arcs_relation(&self) -> Result<Option<SpatialRelation>> {
        if self.circles.iter().any(|c| !c.inside.is_empty()) {
            return Ok(None);
        }
        let mate_vertex = self.src[self.mate_index].vertices[0].point;
        if self.inside_sphere(mate_vertex)? {
            return Ok(Some(self.sphere_contains_mate()));
        }
        if self.mate.inside(self.sphere.center, self.band)? {
            return Ok(Some(self.mate_contains_sphere()));
        }
        Ok(Some(SpatialRelation::Disjoint))
    }

    /// Sphere seam crossings of every inside arc. A crossing that coincides
    /// with an interval end (a mate edge hit on a seam) is merged into that
    /// hit vertex, which then lies on both operands' edges.
    fn sphere_crossings(&mut self) -> Result<()> {
        let o = self.sphere_index();
        let scale = self.sphere.radius;
        for i in 0..self.circles.len() {
            if self.circles[i].inside.is_empty() {
                continue;
            }
            let (normal, middle) = {
                let c = &self.circles[i];
                (cross(c.e1, c.e2), c.center)
            };
            if self.seam_edges(i)?.is_some() {
                self.circles[i].seam = true;
                continue;
            }
            let sections = sphere_sphere::patch_sections(&self.sphere, normal, middle);
            for (patch, section) in sections {
                let PatchUvSection::Circle {
                    center: cc,
                    radius: ruv,
                    start,
                    end,
                } = section
                else {
                    return Err(unsupported(
                        "Sphere Boolean: section plane contains a sphere patch pole \
                         (great-circle section through original vertices)",
                    ));
                };
                if end - start >= TAU - 1e-9 {
                    let q = cc[0].hypot(cc[1]);
                    if cc[0] - ruv <= CORNER || cc[1] - ruv <= CORNER || q + ruv >= 1. - CORNER {
                        return Err(unsupported(
                            "Sphere Boolean: section circle is tangent to a sphere patch boundary",
                        ));
                    }
                    continue;
                }
                for angle in [start, end] {
                    let raw = [cc[0] + ruv * angle.cos(), cc[1] + ruv * angle.sin()];
                    let piece = piece_of(raw)?;
                    let w = snap_to_piece(piece, raw);
                    let tau = piece_tau(piece, w);
                    let model = self.sphere_model();
                    let coedge = &model.loops[model.faces[patch].outer].coedges
                        [self.sphere_pieces[patch][piece]];
                    let edge = coedge.edge;
                    let t = if coedge.reversed { 1. - tau } else { tau };
                    if !(1e-9..=1. - 1e-9).contains(&t) {
                        return Err(unsupported(
                            "Sphere Boolean: section passes through a sphere vertex",
                        ));
                    }
                    let point = point3(&model.faces[patch].surface.evaluate(w[0], w[1])?.point);
                    let circle = &self.circles[i];
                    let phi = circle.phi_of(point);
                    // Which inside interval owns this crossing, and whether it
                    // coincides with one of the interval's hit ends.
                    let mut owner = None;
                    for (k, span) in circle.inside.iter().enumerate() {
                        let local = span[0] + (phi - span[0]).rem_euclid(TAU);
                        if local < span[1] || (span[1] - local).abs() < EVENT_GAP {
                            let ends = circle.ends[k];
                            let at_end = if local - span[0] < EVENT_GAP {
                                ends[0]
                            } else if span[1] - local < EVENT_GAP {
                                ends[1]
                            } else {
                                None
                            };
                            if at_end.is_none() && !(local < span[1]) {
                                continue;
                            }
                            owner = Some((k, local, at_end));
                            break;
                        }
                    }
                    let Some((k, local, at_end)) = owner else {
                        continue;
                    };
                    let existing = self
                        .splits
                        .get(&(o, edge))
                        .and_then(|table| table.iter().find(|(et, _)| (et - t).abs() <= 1e-9))
                        .map(|(_, id)| *id);
                    let id = match (existing, at_end) {
                        (Some(id), _) => id,
                        (None, Some(hit)) => {
                            if dist(self.specials[hit].point, point) > 1e-7 * scale + self.band {
                                return Err(unsupported(
                                    "Sphere Boolean: a seam crossing falls within the event \
                                     band of an edge hit without coinciding with it",
                                ));
                            }
                            self.attach_edge(hit, o, edge, t);
                            hit
                        }
                        (None, None) => {
                            let id = self.special(point);
                            self.attach_edge(id, o, edge, t);
                            self.circles[i].events[k].push((local, id));
                            id
                        }
                    };
                    self.patch_uv.insert((patch, id), w);
                }
            }
        }
        Ok(())
    }

    /// Split each inside interval at its events, refine to quadrant sweeps
    /// and author the exact arc pieces with their owner patches.
    fn refine_and_build_arcs(&mut self) -> Result<()> {
        for i in 0..self.circles.len() {
            if self.circles[i].seam {
                self.seam_arcs(i)?;
                continue;
            }
            let count = self.circles[i].inside.len();
            for k in 0..count {
                let (span, ends, mut events) = {
                    let c = &self.circles[i];
                    (c.inside[k], c.ends[k], c.events[k].clone())
                };
                events.sort_by(|a, b| a.0.total_cmp(&b.0));
                for w in events.windows(2) {
                    if w[1].0 - w[0].0 < EVENT_GAP {
                        return Err(unsupported(
                            "Sphere Boolean: two seam crossings coincide on one arc",
                        ));
                    }
                }
                let ring = ends[0].is_none();
                let mut stations: Vec<(f64, usize)> = Vec::new();
                if let Some(v0) = ends[0] {
                    stations.push((span[0], v0));
                }
                stations.extend(events.iter().copied());
                if ring && stations.is_empty() {
                    // Seed a ring that lies inside one patch and one face.
                    for q in 0..4 {
                        let phi = 0.37 + q as f64 * std::f64::consts::FRAC_PI_2;
                        let point = self.circles[i].point_at(phi);
                        let id = self.special(point);
                        stations.push((phi, id));
                    }
                }
                let close = if ring {
                    let first = stations[0];
                    (first.0 + TAU, first.1)
                } else {
                    (span[1], ends[1].unwrap())
                };
                stations.push(close);
                let mut refined: Vec<(f64, usize)> = Vec::new();
                for w in 0..stations.len() - 1 {
                    refined.push(stations[w]);
                    let (lo, hi) = (stations[w].0, stations[w + 1].0);
                    let parts = ((hi - lo) / MAX_SPAN).ceil().max(1.) as usize;
                    for p in 1..parts {
                        let phi = lo + (hi - lo) * p as f64 / parts as f64;
                        let point = self.circles[i].point_at(phi);
                        let id = self.special(point);
                        refined.push((phi, id));
                    }
                }
                refined.push(*stations.last().unwrap());
                for w in 0..refined.len() - 1 {
                    let (phi0, v0) = refined[w];
                    let (phi1, v1) = refined[w + 1];
                    let gap = phi1 - phi0;
                    let weight = (gap / 2.).cos();
                    if !(weight > 0.5) {
                        return Err(unsupported(
                            "Sphere Boolean: arc piece exceeds a quadrant after refinement",
                        ));
                    }
                    let circle = &self.circles[i];
                    let mid = phi0 + gap / 2.;
                    let (sin, cos) = mid.sin_cos();
                    let shoulder: [f64; 3] = std::array::from_fn(|a| {
                        circle.center[a]
                            + (circle.rho / weight) * (cos * circle.e1[a] + sin * circle.e2[a])
                    });
                    let curve = Curve {
                        degree: 2,
                        knots: vec![0., 0., 0., 1., 1., 1.],
                        control_points: vec![
                            self.specials[v0].point.to_vec(),
                            shoulder.to_vec(),
                            self.specials[v1].point.to_vec(),
                        ],
                        weights: vec![1., weight, 1.],
                        periodic: false,
                    };
                    let midpoint = circle.point_at(mid);
                    let patch = self.owner_patch(midpoint)?;
                    let face = circle.face;
                    let key = EKey::Arc(self.arcs.len());
                    self.arcs.push(Arc {
                        face,
                        patch: Some(patch),
                        ends: [VKey::Special(v0), VKey::Special(v1)],
                        curve,
                        key,
                        reversed: false,
                    });
                }
            }
        }
        if self.arcs.len() > 512 {
            return Err(Error::new(
                "BREP_RESOURCE_LIMIT",
                "Sphere Boolean: arc network exceeds the admitted size",
            ));
        }
        Ok(())
    }

    /// The original sphere edges lying on circle `i`, as (edge, phi at
    /// vertex 0, phi at vertex 1 unwrapped near it), when the circle is a
    /// great circle in a seam plane of the sphere. `None` otherwise.
    fn seam_edges(&self, i: usize) -> Result<Option<Vec<(usize, f64, f64)>>> {
        let circle = &self.circles[i];
        let sphere = &self.sphere;
        if (circle.rho - sphere.radius).abs() > self.band
            || dist(circle.center, sphere.center) > self.band
        {
            return Ok(None);
        }
        let normal = cross(circle.e1, circle.e2);
        // The seam planes are spanned by the sphere frame axes: the pole
        // axis and the two equator directions of any patch.
        let aligned = (0..8).any(|patch| {
            let (a, b, pole) = sphere.frame(patch);
            [a, b, pole]
                .into_iter()
                .any(|axis| dot(normal, axis).abs() > 1. - 1e-9)
        });
        if !aligned {
            return Ok(None);
        }
        let model = self.sphere_model();
        let on_plane = |p: [f64; 3]| dot(normal, sub(p, circle.center)).abs() <= self.band;
        let mut edges = Vec::new();
        for (e, edge) in model.edges.iter().enumerate() {
            let p0 = model.vertices[edge.vertices[0]].point;
            let p1 = model.vertices[edge.vertices[1]].point;
            let mid = point3(&edge.curve.evaluate(0.5)?.point);
            if on_plane(p0) && on_plane(p1) && on_plane(mid) {
                let phi0 = circle.phi_of(p0);
                let phi1 = phi0 + (circle.phi_of(p1) - phi0 + std::f64::consts::PI).rem_euclid(TAU)
                    - std::f64::consts::PI;
                if ((phi1 - phi0).abs() - std::f64::consts::FRAC_PI_2).abs() > 1e-9 {
                    return Err(unsupported(
                        "Sphere Boolean: seam circle edge is not a quarter arc",
                    ));
                }
                edges.push((e, phi0, phi1));
            }
        }
        if edges.len() != 4 {
            return Err(unsupported(
                "Sphere Boolean: seam circle does not consist of four sphere edges",
            ));
        }
        Ok(Some(edges))
    }

    /// Pieces of a seam circle: every inside interval is covered by original
    /// sphere edges, split where a mate edge hit lies inside one of them.
    /// Hits that coincide with sphere vertices become those vertices.
    fn seam_arcs(&mut self, i: usize) -> Result<()> {
        let o = self.sphere_index();
        let edges = self
            .seam_edges(i)?
            .ok_or_else(|| unsupported("Sphere Boolean: seam circle lost its edges"))?;
        let model = self.sphere_model();
        let face = self.circles[i].face;
        let count = self.circles[i].inside.len();
        for k in 0..count {
            let (span, ends) = {
                let c = &self.circles[i];
                (c.inside[k], c.ends[k])
            };
            if !self.circles[i].events[k].is_empty() {
                return Err(unsupported(
                    "Sphere Boolean: seam circle carries interior seam events",
                ));
            }
            // Stations: interval ends, then every sphere vertex on the circle
            // strictly inside the interval.
            let mut stations: Vec<(f64, VKey)> = Vec::new();
            let mut end_station =
                |phi: f64, id: Option<usize>, this: &mut Self| -> Result<Option<(f64, VKey)>> {
                    let Some(id) = id else { return Ok(None) };
                    // A hit at a sphere vertex takes that vertex's identity.
                    let point = this.specials[id].point;
                    for (e, phi0, phi1) in &edges {
                        for (vi, vphi) in [(0usize, *phi0), (1usize, *phi1)] {
                            let v = model.edges[*e].vertices[vi];
                            if dist(model.vertices[v].point, point)
                                <= this.band + 1e-9 * this.sphere.radius
                            {
                                this.specials[id].orig = Some((o, v));
                            }
                            let _ = vphi;
                        }
                    }
                    Ok(Some((phi, VKey::Special(id))))
                };
            if let Some(st) = end_station(span[0], ends[0], self)? {
                stations.push(st);
            }
            let (lo, hi) = (span[0], span[1]);
            let ring = ends[0].is_none();
            for (e, phi0, phi1) in &edges {
                for (vi, vphi) in [(0usize, *phi0), (1usize, *phi1)] {
                    let mut local = lo + (vphi - lo).rem_euclid(TAU);
                    if ring && local > hi - EVENT_GAP {
                        local -= TAU;
                    }
                    let admitted = if ring {
                        local >= lo - EVENT_GAP && local < hi - EVENT_GAP
                    } else {
                        local > lo + EVENT_GAP && local < hi - EVENT_GAP
                    };
                    if admitted {
                        let v = model.edges[*e].vertices[vi];
                        if !stations.iter().any(|(_, key)| *key == VKey::Orig(o, v)) {
                            stations.push((local, VKey::Orig(o, v)));
                        }
                    }
                }
            }
            stations.sort_by(|a, b| a.0.total_cmp(&b.0));
            if let Some(st) = end_station(span[1], ends[1], self)? {
                stations.push(st);
            } else {
                let first = stations[0];
                stations.push((first.0 + TAU, first.1));
            }
            for w in 0..stations.len() - 1 {
                let (pa, ka) = stations[w];
                let (pb, kb) = stations[w + 1];
                if pb - pa < EVENT_GAP {
                    return Err(unsupported(
                        "Sphere Boolean: coincident stations on a seam circle",
                    ));
                }
                if pb - pa > std::f64::consts::FRAC_PI_2 + 1e-9 {
                    return Err(unsupported(
                        "Sphere Boolean: seam circle piece is not covered by one sphere edge",
                    ));
                }
                // The sphere edge covering [pa, pb].
                let mid = (pa + pb) / 2.;
                let mut owner = None;
                for (e, phi0, phi1) in &edges {
                    let (elo, ehi) = (phi0.min(*phi1), phi0.max(*phi1));
                    let local = elo + (mid - elo).rem_euclid(TAU);
                    if local > elo && local < ehi {
                        owner = Some((*e, *phi0, *phi1));
                        break;
                    }
                }
                let Some((e, phi0, phi1)) = owner else {
                    return Err(unsupported(
                        "Sphere Boolean: seam circle piece escapes the sphere edges",
                    ));
                };
                let forward = phi1 > phi0;
                // Edge parameters of the station points, inverted on the
                // edge curve itself (meridians and equator arcs are
                // parameterized differently).
                let circle = &self.circles[i];
                let limit = self.band + 1e-9 * self.sphere.radius;
                let ta = curve_parameter_of(&model.edges[e].curve, circle.point_at(pa), limit)?;
                let tb = curve_parameter_of(&model.edges[e].curve, circle.point_at(pb), limit)?;
                let (t_lo, t_hi) = (ta.min(tb), ta.max(tb));
                let snap = |t: f64| {
                    if t < 1e-9 {
                        0.
                    } else if t > 1. - 1e-9 {
                        1.
                    } else {
                        t
                    }
                };
                let (t_lo, t_hi) = (snap(t_lo), snap(t_hi));
                // Register interior hits as splits of this sphere edge.
                for (t, key) in [
                    (if forward { t_lo } else { t_hi }, ka),
                    (if forward { t_hi } else { t_lo }, kb),
                ] {
                    if let VKey::Special(id) = key {
                        if self.specials[id].orig.is_none() && t > 0. && t < 1. {
                            let known = self
                                .splits
                                .get(&(o, e))
                                .map(|table| {
                                    table
                                        .iter()
                                        .any(|(et, eid)| *eid == id && (et - t).abs() <= 1e-12)
                                })
                                .unwrap_or(false);
                            if !known {
                                self.attach_edge(id, o, e, t);
                            }
                        }
                    }
                }
                let whole = t_lo == 0. && t_hi == 1.;
                let (key, curve) = if whole {
                    (EKey::Orig(o, e), model.edges[e].curve.clone())
                } else {
                    let table = self.splits.get(&(o, e)).cloned().unwrap_or_default();
                    let n = table.len() + 1;
                    let j = (0..n)
                        .find(|&j| {
                            let lo = if j == 0 { 0. } else { table[j - 1].0 };
                            let hi = if j == n - 1 { 1. } else { table[j].0 };
                            (lo - t_lo).abs() <= 1e-12 && (hi - t_hi).abs() <= 1e-12
                        })
                        .ok_or_else(|| {
                            unsupported("Sphere Boolean: seam piece does not match the edge split")
                        })?;
                    (
                        EKey::Piece(o, e, j),
                        normalized(model.edges[e].curve.trim(t_lo, t_hi)?),
                    )
                };
                let curve = if forward { curve } else { curve.reverse()? };
                self.arcs.push(Arc {
                    face,
                    patch: None,
                    ends: [ka, kb],
                    curve,
                    key,
                    reversed: !forward,
                });
            }
        }
        Ok(())
    }

    /// 3D point of a chart vertex key.
    fn point_of(&self, key: VKey) -> [f64; 3] {
        match key {
            VKey::Special(id) => self.specials[id].point,
            VKey::Orig(o, v) => self.src[o].vertices[v].point,
        }
    }

    /// The single sphere patch containing a point strictly inside its chart.
    fn owner_patch(&self, point: [f64; 3]) -> Result<usize> {
        let mut found = None;
        for patch in 0..8 {
            let Some(uv) = self.sphere.invert_uv(patch, point) else {
                continue;
            };
            if strictly_inside_quarter_disk(uv, BOUNDARY_UV) {
                if found.is_some() {
                    return Err(unsupported(
                        "Sphere Boolean: arc midpoint lies in two sphere patches",
                    ));
                }
                found = Some(patch);
            }
        }
        found.ok_or_else(|| unsupported("Sphere Boolean: arc midpoint lies on a sphere seam"))
    }

    // -----------------------------------------------------------------------
    // Result topology
    // -----------------------------------------------------------------------

    fn vertex(&mut self, key: VKey) -> usize {
        let key = match key {
            VKey::Special(id) => match self.specials[id].orig {
                Some((o, v)) => VKey::Orig(o, v),
                None => key,
            },
            other => other,
        };
        if let Some(&id) = self.vmap.get(&key) {
            return id;
        }
        let point = match key {
            VKey::Special(id) => self.specials[id].point,
            VKey::Orig(o, v) => self.src[o].vertices[v].point,
        };
        let id = self.vertices.len();
        self.vertices.push(Vertex { point });
        self.vmap.insert(key, id);
        id
    }

    /// Vertex key at one original edge parameter of operand `o`.
    fn vkey_at(&self, o: usize, edge: usize, t: f64) -> Result<VKey> {
        let src = &self.src[o].edges[edge];
        if t <= 0. {
            return Ok(VKey::Orig(o, src.vertices[0]));
        }
        if t >= 1. {
            return Ok(VKey::Orig(o, src.vertices[1]));
        }
        let table = self
            .splits
            .get(&(o, edge))
            .ok_or_else(|| unsupported("Sphere Boolean: split edge has no event table"))?;
        for &(et, id) in table {
            if (et - t).abs() <= 1e-12 {
                return Ok(VKey::Special(id));
            }
        }
        Err(unsupported(
            "Sphere Boolean: split parameter does not match an event",
        ))
    }

    fn edge(&mut self, key: EKey) -> Result<usize> {
        if let Some(&id) = self.emap.get(&key) {
            return Ok(id);
        }
        let edge = match key {
            EKey::Arc(k) => {
                let v0 = self.vertex(self.arcs[k].ends[0]);
                let v1 = self.vertex(self.arcs[k].ends[1]);
                Edge {
                    degenerate: false,
                    vertices: [v0, v1],
                    curve: self.arcs[k].curve.clone(),
                }
            }
            EKey::Orig(o, e) => {
                let src = &self.src[o].edges[e];
                let v0 = self.vertex(VKey::Orig(o, src.vertices[0]));
                let v1 = self.vertex(VKey::Orig(o, src.vertices[1]));
                Edge {
                    degenerate: false,
                    vertices: [v0, v1],
                    curve: src.curve.clone(),
                }
            }
            EKey::Piece(o, e, j) => {
                let table = self
                    .splits
                    .get(&(o, e))
                    .cloned()
                    .ok_or_else(|| unsupported("Sphere Boolean: piece of an unsplit edge"))?;
                let t_lo = if j == 0 { 0. } else { table[j - 1].0 };
                let t_hi = if j == table.len() { 1. } else { table[j].0 };
                let v0 = self.vertex(self.vkey_at(o, e, t_lo)?);
                let v1 = self.vertex(self.vkey_at(o, e, t_hi)?);
                Edge {
                    degenerate: false,
                    vertices: [v0, v1],
                    curve: self.src[o].edges[e].curve.trim(t_lo, t_hi)?,
                }
            }
        };
        let id = self.edges.len();
        self.edges.push(edge);
        self.emap.insert(key, id);
        Ok(id)
    }

    /// Boundary pieces of one original face of operand `o`, in loop order,
    /// split at the events on its edges: (from, to, forward pcurve, 3D edge
    /// key, forward runs against the 3D edge).
    fn boundary_pieces(
        &self,
        o: usize,
        face: usize,
    ) -> Result<Vec<(VKey, VKey, Curve, EKey, bool)>> {
        let model = self.src[o];
        let wire = &model.loops[model.faces[face].outer];
        let mut out = Vec::new();
        for coedge in &wire.coedges {
            let e = coedge.edge;
            let table = self.splits.get(&(o, e)).cloned().unwrap_or_default();
            if table.is_empty() {
                let src = &model.edges[e];
                let (va, vb) = if coedge.reversed {
                    (src.vertices[1], src.vertices[0])
                } else {
                    (src.vertices[0], src.vertices[1])
                };
                out.push((
                    VKey::Orig(o, va),
                    VKey::Orig(o, vb),
                    normalized(coedge.pcurve.clone()),
                    EKey::Orig(o, e),
                    coedge.reversed,
                ));
                continue;
            }
            let count = table.len() + 1;
            let order: Vec<usize> = if coedge.reversed {
                (0..count).rev().collect()
            } else {
                (0..count).collect()
            };
            for j in order {
                let t_lo = if j == 0 { 0. } else { table[j - 1].0 };
                let t_hi = if j == count - 1 { 1. } else { table[j].0 };
                let (tau_a, tau_b) = if coedge.reversed {
                    (1. - t_hi, 1. - t_lo)
                } else {
                    (t_lo, t_hi)
                };
                let (ka, kb) = if coedge.reversed {
                    (self.vkey_at(o, e, t_hi)?, self.vkey_at(o, e, t_lo)?)
                } else {
                    (self.vkey_at(o, e, t_lo)?, self.vkey_at(o, e, t_hi)?)
                };
                out.push((
                    ka,
                    kb,
                    normalized(coedge.pcurve.trim(tau_a, tau_b)?),
                    EKey::Piece(o, e, j),
                    coedge.reversed,
                ));
            }
        }
        Ok(out)
    }

    /// Exact pcurve of an arc in its sphere patch: degree-2 Bernstein
    /// conversion of the inverted 3D arc, resampled against the surface.
    fn patch_arc_pcurve(&self, k: usize, uv0: [f64; 2], uv1: [f64; 2]) -> Result<Curve> {
        let arc = &self.arcs[k];
        let Some(patch) = arc.patch else {
            return Err(unsupported(
                "Sphere Boolean: seam piece has no owning patch",
            ));
        };
        let (a, b, pole) = self.sphere.frame(patch);
        let center = self.sphere.center;
        let radius = self.sphere.radius;
        let curve = &arc.curve;
        let sample = |t: f64| -> (f64, f64, f64) {
            let b0 = (1. - t) * (1. - t);
            let b1 = 2. * t * (1. - t);
            let b2 = t * t;
            let mut h = [0.; 3];
            let mut hw = 0.;
            for (i, basis) in [b0, b1, b2].into_iter().enumerate() {
                let w = basis * curve.weights[i];
                for axis in 0..3 {
                    h[axis] += w * curve.control_points[i][axis];
                }
                hw += w;
            }
            let d = std::array::from_fn(|axis| h[axis] - center[axis] * hw);
            (dot(d, a), dot(d, b), radius * hw + dot(d, pole))
        };
        let f0 = sample(0.);
        let f1 = sample(0.5);
        let f2 = sample(1.);
        let bern = |f: (f64, f64, f64), i: usize| -> f64 {
            match i {
                0 => f.0,
                2 => f.2,
                _ => 2. * f.1 - 0.5 * f.0 - 0.5 * f.2,
            }
        };
        let mut control_points = Vec::with_capacity(3);
        let mut weights = Vec::with_capacity(3);
        for i in 0..3 {
            let den = bern((f0.2, f1.2, f2.2), i);
            if !(den > 0.) {
                return Err(unsupported(
                    "Sphere Boolean: arc leaves the patch hemisphere",
                ));
            }
            control_points.push(vec![
                bern((f0.0, f1.0, f2.0), i) / den,
                bern((f0.1, f1.1, f2.1), i) / den,
            ]);
            weights.push(den);
        }
        control_points[0] = uv0.to_vec();
        control_points[2] = uv1.to_vec();
        let pcurve = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points,
            weights,
            periodic: false,
        };
        let surface = &self.sphere_model().faces[patch].surface;
        verify_pcurve(
            &pcurve,
            curve,
            surface,
            1e-9 * radius.max(1.) + self.sphere.error,
        )?;
        Ok(pcurve)
    }

    /// UV of a special vertex in a sphere patch chart: the snapped seam value
    /// when it lies on the patch boundary, else a strict interior inversion.
    fn patch_vertex_uv(&mut self, patch: usize, id: usize) -> Result<[f64; 2]> {
        if let Some(&uv) = self.patch_uv.get(&(patch, id)) {
            return Ok(uv);
        }
        let point = self.specials[id].point;
        let uv = self
            .sphere
            .invert_uv(patch, point)
            .ok_or_else(|| unsupported("Sphere Boolean: network vertex escapes its patch chart"))?;
        if !strictly_inside_quarter_disk(uv, CORNER) {
            return Err(unsupported(
                "Sphere Boolean: a network vertex lies on a sphere seam",
            ));
        }
        self.patch_uv.insert((patch, id), uv);
        Ok(uv)
    }

    fn emit_face(
        &mut self,
        o: usize,
        face: usize,
        reversed: bool,
        outer: Vec<Coedge>,
        holes: Vec<Vec<Coedge>>,
        surface: Option<Surface>,
    ) {
        let outer_id = self.loops.len();
        self.loops.push(Loop { coedges: outer });
        let mut hole_ids = Vec::with_capacity(holes.len());
        for hole in holes {
            hole_ids.push(self.loops.len());
            self.loops.push(Loop { coedges: hole });
        }
        let id = self.faces.len();
        self.faces.push(Face {
            surface: surface.unwrap_or_else(|| self.src[o].faces[face].surface.clone()),
            outer: outer_id,
            holes: hole_ids,
        });
        self.shell.push(FaceUse {
            face: id,
            reversed: reversed ^ self.flip[o],
        });
    }

    /// Whole original face, edges remapped and split where seam pieces or
    /// hits divided them.
    fn emit_whole(&mut self, o: usize, face: usize, reversed: bool) -> Result<()> {
        let mut out = Vec::new();
        for (_, _, pcurve, key, rev) in self.boundary_pieces(o, face)? {
            let edge = self.edge(key)?;
            out.push(Coedge {
                edge,
                reversed: rev,
                pcurve,
            });
        }
        self.emit_face(o, face, reversed, out, vec![], None);
        Ok(())
    }

    /// The face surface trimmed to the UV bounding box of a region and
    /// re-parameterized to the unit square, with the region pcurves mapped by
    /// the same affine change (exact for lines and rational arcs). The
    /// control hull of the result becomes tight: the solid audit proves body
    /// separation from surface control points, and downstream consumers keep
    /// their unit-domain assumption.
    fn tight_surface(
        &self,
        o: usize,
        face: usize,
        outer: &mut [Coedge],
        holes: &mut [Vec<Coedge>],
    ) -> Result<Surface> {
        let source = &self.src[o].faces[face].surface;
        let du = [
            source.knots_u[source.degree_u],
            source.knots_u[source.knots_u.len() - 1 - source.degree_u],
        ];
        let dv = [
            source.knots_v[source.degree_v],
            source.knots_v[source.knots_v.len() - 1 - source.degree_v],
        ];
        let mut lo = [f64::INFINITY; 2];
        let mut hi = [f64::NEG_INFINITY; 2];
        for coedge in outer.iter().chain(holes.iter().flatten()) {
            for p in &coedge.pcurve.control_points {
                for k in 0..2 {
                    lo[k] = lo[k].min(p[k]);
                    hi[k] = hi[k].max(p[k]);
                }
            }
        }
        let margin = 1e-12;
        let u0 = (lo[0] - margin).max(du[0]);
        let u1 = (hi[0] + margin).min(du[1]);
        let v0 = (lo[1] - margin).max(dv[0]);
        let v1 = (hi[1] + margin).min(dv[1]);
        if !(u1 - u0 > 1e-9) || !(v1 - v0 > 1e-9) {
            return Err(unsupported("Sphere Boolean: face region collapses in UV"));
        }
        if u0 == du[0] && u1 == du[1] && v0 == dv[0] && v1 == dv[1] {
            return Ok(source.clone());
        }
        let mut surface = source.trim([u0, u1, v0, v1])?;
        surface.knots_u = surface
            .knots_u
            .iter()
            .map(|k| (k - u0) / (u1 - u0))
            .collect();
        surface.knots_v = surface
            .knots_v
            .iter()
            .map(|k| (k - v0) / (v1 - v0))
            .collect();
        for coedge in outer.iter_mut().chain(holes.iter_mut().flatten()) {
            for p in &mut coedge.pcurve.control_points {
                p[0] = ((p[0] - u0) / (u1 - u0)).clamp(0., 1.);
                p[1] = ((p[1] - v0) / (v1 - v0)).clamp(0., 1.);
            }
        }
        Ok(surface)
    }

    /// Arrange one chart and emit its kept regions.
    fn emit_chart(
        &mut self,
        o: usize,
        face: usize,
        reversed: bool,
        pieces: Vec<Piece>,
        vertex_count: usize,
    ) -> Result<()> {
        let want = self.want_inside[o];
        let arrangement = arrange(pieces, vertex_count)?;
        for region in &arrangement.regions {
            let sample = region_sample(&arrangement, region)?;
            let point = point3(
                &self.src[o].faces[face]
                    .surface
                    .evaluate(sample[0], sample[1])?
                    .point,
            );
            let inside = if o == self.mate_index {
                self.inside_sphere(point)?
            } else {
                self.mate.inside(point, self.band)?
            };
            if inside != want {
                continue;
            }
            let mut outer = self.cycle_coedges(&arrangement, region.outer)?;
            let mut holes = Vec::with_capacity(region.holes.len());
            for &hole in &region.holes {
                holes.push(self.cycle_coedges(&arrangement, hole)?);
            }
            let surface = self.tight_surface(o, face, &mut outer, &mut holes)?;
            self.emit_face(o, face, reversed, outer, holes, Some(surface));
        }
        Ok(())
    }

    fn cycle_coedges(&mut self, arrangement: &Arrangement, cycle: usize) -> Result<Vec<Coedge>> {
        let mut out = Vec::new();
        for &h in &arrangement.cycles[cycle] {
            let half = &arrangement.halves[h];
            let piece = &arrangement.pieces[half.piece];
            let edge = self.edge(piece.key)?;
            let (pcurve, reversed) = if half.forward {
                (piece.pcurve.clone(), piece.reversed)
            } else {
                (piece.pcurve.reverse()?, !piece.reversed)
            };
            out.push(Coedge {
                edge,
                reversed,
                pcurve,
            });
        }
        Ok(out)
    }

    /// Mate faces: untouched faces whole (classified by their centre sample),
    /// crossed faces through arrangement.
    fn emit_mate(&mut self) -> Result<()> {
        let o = self.mate_index;
        let want = self.want_inside[o];
        let model = self.src[o];
        for face in 0..model.faces.len() {
            let reversed = face_reversed(model, face);
            let arcs: Vec<usize> = (0..self.arcs.len())
                .filter(|&k| self.arcs[k].face == face)
                .collect();
            if arcs.is_empty() {
                let centre = point3(&model.faces[face].surface.evaluate(0.5, 0.5)?.point);
                if self.inside_sphere(centre)? == want {
                    self.emit_whole(o, face, reversed)?;
                }
                continue;
            }
            let mut chart = ChartVertices::default();
            let mut pieces = Vec::new();
            for (ka, kb, pcurve, key, rev) in self.boundary_pieces(o, face)? {
                let ua = pcurve_point(&pcurve, 0.)?;
                let ub = pcurve_point(&pcurve, 1.)?;
                let a = chart.index(self.chart_key(ka), ua);
                let b = chart.index(self.chart_key(kb), ub);
                pieces.push(Piece {
                    v: [a, b],
                    pcurve,
                    key,
                    reversed: rev,
                });
            }
            let surface = model.faces[face].surface.clone();
            for k in arcs {
                let [ka, kb] = self.arcs[k].ends;
                let mut pcurve = self.mate.arc_pcurve(face, &self.arcs[k].curve)?;
                let ua = self.mate.uv_of(face, self.point_of(ka))?;
                let ub = self.mate.uv_of(face, self.point_of(kb))?;
                let a = chart.index(self.chart_key(ka), ua);
                let b = chart.index(self.chart_key(kb), ub);
                let n = pcurve.control_points.len();
                pcurve.control_points[0] = chart.uvs[a].to_vec();
                pcurve.control_points[n - 1] = chart.uvs[b].to_vec();
                verify_pcurve(
                    &pcurve,
                    &self.arcs[k].curve,
                    &surface,
                    1e-9 * self.sphere.radius.max(1.) + self.band,
                )?;
                pieces.push(Piece {
                    v: [a, b],
                    pcurve,
                    key: self.arcs[k].key,
                    reversed: self.arcs[k].reversed,
                });
            }
            self.emit_chart(o, face, reversed, pieces, chart.uvs.len())?;
        }
        Ok(())
    }

    /// Chart identity of a vertex key: specials that coincide with an
    /// original vertex share its chart slot.
    fn chart_key(&self, key: VKey) -> VKey {
        match key {
            VKey::Special(id) => match self.specials[id].orig {
                Some((o, v)) => VKey::Orig(o, v),
                None => key,
            },
            other => other,
        }
    }

    /// Sphere patches: untouched patches whole, crossed through arrangement.
    fn emit_sphere(&mut self) -> Result<()> {
        let o = self.sphere_index();
        let want = self.want_inside[o];
        let model = self.sphere_model();
        for patch in 0..8 {
            let reversed = face_reversed(model, patch);
            let arcs: Vec<usize> = (0..self.arcs.len())
                .filter(|&k| self.arcs[k].patch == Some(patch))
                .collect();
            if arcs.is_empty() {
                // An interior sample, never the pole: seam-coincident cuts
                // pass exactly through the sphere vertices.
                let sample = point3(&model.faces[patch].surface.evaluate(0.3, 0.3)?.point);
                if self.mate.inside(sample, self.band)? == want {
                    self.emit_whole(o, patch, reversed)?;
                }
                continue;
            }
            let mut chart = ChartVertices::default();
            let mut pieces = Vec::new();
            for (ka, kb, mut pcurve, key, rev) in self.boundary_pieces(o, patch)? {
                let ua = match ka {
                    VKey::Special(id) => self.patch_vertex_uv(patch, id)?,
                    VKey::Orig(..) => pcurve_point(&pcurve, 0.)?,
                };
                let ub = match kb {
                    VKey::Special(id) => self.patch_vertex_uv(patch, id)?,
                    VKey::Orig(..) => pcurve_point(&pcurve, 1.)?,
                };
                let n = pcurve.control_points.len();
                pcurve.control_points[0] = ua.to_vec();
                pcurve.control_points[n - 1] = ub.to_vec();
                let a = chart.index(self.chart_key(ka), ua);
                let b = chart.index(self.chart_key(kb), ub);
                pieces.push(Piece {
                    v: [a, b],
                    pcurve,
                    key,
                    reversed: rev,
                });
            }
            for k in arcs {
                let [ka, kb] = self.arcs[k].ends;
                let (VKey::Special(v0), VKey::Special(v1)) = (ka, kb) else {
                    return Err(unsupported(
                        "Sphere Boolean: patch arc bounded by an original vertex",
                    ));
                };
                let ua = self.patch_vertex_uv(patch, v0)?;
                let ub = self.patch_vertex_uv(patch, v1)?;
                let pcurve = self.patch_arc_pcurve(k, ua, ub)?;
                let a = chart.index(ka, ua);
                let b = chart.index(kb, ub);
                pieces.push(Piece {
                    v: [a, b],
                    pcurve,
                    key: EKey::Arc(k),
                    reversed: false,
                });
            }
            self.emit_chart(o, patch, reversed, pieces, chart.uvs.len())?;
        }
        Ok(())
    }
}

/// Parameter of the point on a single-span arc `curve` (domain [0, 1])
/// nearest to `point`, by ternary search on the squared distance, which is
/// unimodal along an arc of at most a half turn. Refuses when the point is
/// not on the curve within `limit`.
fn curve_parameter_of(curve: &Curve, point: [f64; 3], limit: f64) -> Result<f64> {
    let d2 = |t: f64| -> Result<f64> {
        let p = point3(&curve.evaluate(t)?.point);
        Ok(dot(sub(p, point), sub(p, point)))
    };
    let (mut lo, mut hi) = (0., 1.);
    for _ in 0..200 {
        let m1 = lo + (hi - lo) / 3.;
        let m2 = hi - (hi - lo) / 3.;
        if d2(m1)? <= d2(m2)? {
            hi = m2;
        } else {
            lo = m1;
        }
        if hi - lo < 1e-16 {
            break;
        }
    }
    let t = (lo + hi) / 2.;
    // Endpoint snap: the ends are the exact vertices.
    let t = if t < 1e-9 {
        0.
    } else if t > 1. - 1e-9 {
        1.
    } else {
        t
    };
    if d2(t)?.sqrt() > limit {
        return Err(unsupported(
            "Sphere Boolean: station point is not on its sphere edge",
        ));
    }
    Ok(t)
}

/// Sample `pcurve` through `surface` against the 3D `curve` at matching
/// parameters; both are single-span over [0, 1].
fn verify_pcurve(pcurve: &Curve, curve: &Curve, surface: &Surface, limit: f64) -> Result<()> {
    for i in 0..=8 {
        let t = i as f64 / 8.;
        let uv = pcurve.evaluate(t)?.point;
        let through = point3(&surface.evaluate(uv[0], uv[1])?.point);
        let along = point3(&curve.evaluate(t)?.point);
        if dist(through, along) > limit {
            return Err(unsupported(
                "Sphere Boolean: arc pcurve does not track the 3D arc on its surface",
            ));
        }
    }
    Ok(())
}

/// Chart vertex table: topology key -> chart index, with its UV.
#[derive(Default)]
struct ChartVertices {
    keys: Vec<VKey>,
    uvs: Vec<[f64; 2]>,
}
impl ChartVertices {
    fn index(&mut self, key: VKey, uv: [f64; 2]) -> usize {
        if let Some(i) = self.keys.iter().position(|k| *k == key) {
            i
        } else {
            self.keys.push(key);
            self.uvs.push(uv);
            self.keys.len() - 1
        }
    }
}

// ---------------------------------------------------------------------------
// Planar UV arrangement: cycles of a piece graph, regions with holes
// ---------------------------------------------------------------------------

struct Half {
    piece: usize,
    forward: bool,
    from: usize,
    to: usize,
    /// Outgoing tangent angle at `from`.
    angle: f64,
}

struct Region {
    outer: usize,
    holes: Vec<usize>,
}

struct Arrangement {
    pieces: Vec<Piece>,
    halves: Vec<Half>,
    /// Half-edge ids of each cycle, in traversal order.
    cycles: Vec<Vec<usize>>,
    /// Sampled polygon of each cycle.
    polygons: Vec<Vec<[f64; 2]>>,
    regions: Vec<Region>,
}

fn pcurve_point(curve: &Curve, t: f64) -> Result<[f64; 2]> {
    let p = curve.evaluate(t)?.point;
    Ok([p[0], p[1]])
}

/// The same curve over the knot domain [0, 1]; `Curve::trim` keeps the
/// sub-domain of its source, and every chart routine here samples in [0, 1].
fn normalized(curve: Curve) -> Curve {
    let [a, b] = curve.domain();
    if a == 0. && b == 1. {
        return curve;
    }
    let mut out = curve;
    out.knots = out.knots.iter().map(|k| (k - a) / (b - a)).collect();
    out
}

fn tangent_angle(curve: &Curve, at_start: bool) -> Result<f64> {
    let n = curve.control_points.len();
    let (p, q) = if at_start {
        (&curve.control_points[0], &curve.control_points[1])
    } else {
        (&curve.control_points[n - 1], &curve.control_points[n - 2])
    };
    let d = [q[0] - p[0], q[1] - p[1]];
    let len = d[0].hypot(d[1]);
    if !(len > 1e-12) {
        return Err(unsupported(
            "Sphere Boolean: degenerate pcurve tangent in the UV arrangement",
        ));
    }
    Ok(d[1].atan2(d[0]))
}

fn signed_area(polygon: &[[f64; 2]]) -> f64 {
    let mut area = 0.;
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[(i + 1) % polygon.len()];
        area += a[0] * b[1] - b[0] * a[1];
    }
    area / 2.
}

fn point_in_polygon(polygon: &[[f64; 2]], p: [f64; 2]) -> bool {
    let mut winding = 0i32;
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[(i + 1) % polygon.len()];
        let side = (b[0] - a[0]) * (p[1] - a[1]) - (p[0] - a[0]) * (b[1] - a[1]);
        if a[1] <= p[1] {
            if b[1] > p[1] && side > 0. {
                winding += 1;
            }
        } else if b[1] <= p[1] && side < 0. {
            winding -= 1;
        }
    }
    winding != 0
}

fn arrange(pieces: Vec<Piece>, vertex_count: usize) -> Result<Arrangement> {
    let mut halves = Vec::with_capacity(pieces.len() * 2);
    for (i, piece) in pieces.iter().enumerate() {
        halves.push(Half {
            piece: i,
            forward: true,
            from: piece.v[0],
            to: piece.v[1],
            angle: tangent_angle(&piece.pcurve, true)?,
        });
        halves.push(Half {
            piece: i,
            forward: false,
            from: piece.v[1],
            to: piece.v[0],
            angle: tangent_angle(&piece.pcurve, false)?,
        });
    }
    let mut outgoing: Vec<Vec<(f64, usize)>> = vec![Vec::new(); vertex_count];
    for (h, half) in halves.iter().enumerate() {
        outgoing[half.from].push((half.angle, h));
    }
    for list in outgoing.iter_mut() {
        if list.len() < 2 {
            return Err(unsupported(
                "Sphere Boolean: dangling vertex in the UV arrangement",
            ));
        }
        list.sort_by(|a, b| a.0.total_cmp(&b.0));
        for w in 0..list.len() {
            let a = list[w].0;
            let b = if w + 1 == list.len() {
                list[0].0 + TAU
            } else {
                list[w + 1].0
            };
            if b - a < 1e-9 {
                return Err(unsupported(
                    "Sphere Boolean: two UV branches leave a vertex tangentially",
                ));
            }
        }
    }
    // next(h): at v = to(h), the outgoing half-edge clockwise-adjacent to
    // twin(h), which keeps the bounded region on the left.
    let mut next = vec![usize::MAX; halves.len()];
    for h in 0..halves.len() {
        let twin = h ^ 1;
        let v = halves[h].to;
        let list = &outgoing[v];
        let idx = list
            .iter()
            .position(|&(_, id)| id == twin)
            .ok_or_else(|| unsupported("Sphere Boolean: twin half-edge missing"))?;
        next[h] = list[(idx + list.len() - 1) % list.len()].1;
    }
    let mut cycles: Vec<Vec<usize>> = Vec::new();
    let mut visited = vec![false; halves.len()];
    for start in 0..halves.len() {
        if visited[start] {
            continue;
        }
        let mut cycle = Vec::new();
        let mut h = start;
        loop {
            if visited[h] {
                return Err(unsupported(
                    "Sphere Boolean: UV arrangement walk is not a simple cycle",
                ));
            }
            visited[h] = true;
            cycle.push(h);
            h = next[h];
            if h == start {
                break;
            }
        }
        cycles.push(cycle);
    }
    let mut polygons = Vec::with_capacity(cycles.len());
    let mut areas = Vec::with_capacity(cycles.len());
    for cycle in &cycles {
        let mut polygon = Vec::new();
        for &h in cycle {
            let piece = &pieces[halves[h].piece];
            for s in 0..16 {
                let t = s as f64 / 16.;
                let t = if halves[h].forward { t } else { 1. - t };
                polygon.push(pcurve_point(&piece.pcurve, t)?);
            }
        }
        areas.push(signed_area(&polygon));
        polygons.push(polygon);
    }
    // CCW cycles bound regions; CW cycles are holes of the smallest CCW
    // cycle containing a point just to their left, or the chart exterior.
    let mut regions: Vec<Region> = Vec::new();
    let mut region_of_cycle: BTreeMap<usize, usize> = BTreeMap::new();
    for (c, &area) in areas.iter().enumerate() {
        if area > 0. {
            region_of_cycle.insert(c, regions.len());
            regions.push(Region {
                outer: c,
                holes: Vec::new(),
            });
        }
    }
    for (c, &area) in areas.iter().enumerate() {
        if area > 0. {
            continue;
        }
        let probe = left_probe(&pieces, &halves, &cycles[c], &polygons, None)?;
        let mut best: Option<(f64, usize)> = None;
        for (&outer, &r) in &region_of_cycle {
            if point_in_polygon(&polygons[outer], probe) {
                let a = areas[outer];
                if best.map(|(ba, _)| a < ba).unwrap_or(true) {
                    best = Some((a, r));
                }
            }
        }
        if let Some((_, r)) = best {
            regions[r].holes.push(c);
        }
    }
    Ok(Arrangement {
        pieces,
        halves,
        cycles,
        polygons,
        regions,
    })
}

/// A point just left of some half-edge of a cycle; with a constraint it must
/// lie inside that outer polygon and outside the listed holes.
fn left_probe(
    pieces: &[Piece],
    halves: &[Half],
    cycle: &[usize],
    polygons: &[Vec<[f64; 2]>],
    constraint: Option<(usize, &[usize])>,
) -> Result<[f64; 2]> {
    for &h in cycle {
        let half = &halves[h];
        let piece = &pieces[half.piece];
        let (t0, t1) = if half.forward {
            (0.5, 0.5 + 1e-4)
        } else {
            (0.5, 0.5 - 1e-4)
        };
        let m = pcurve_point(&piece.pcurve, t0)?;
        let q = pcurve_point(&piece.pcurve, t1)?;
        let d = [q[0] - m[0], q[1] - m[1]];
        let len = d[0].hypot(d[1]);
        if !(len > 0.) {
            continue;
        }
        let left = [-d[1] / len, d[0] / len];
        for delta in SAMPLE_DELTAS {
            let p = [m[0] + left[0] * delta, m[1] + left[1] * delta];
            match constraint {
                None => return Ok(p),
                Some((outer, holes)) => {
                    if point_in_polygon(&polygons[outer], p)
                        && holes
                            .iter()
                            .all(|&hole| !point_in_polygon(&polygons[hole], p))
                    {
                        return Ok(p);
                    }
                }
            }
        }
    }
    Err(unsupported(
        "Sphere Boolean: could not sample a UV region interior",
    ))
}

fn region_sample(arrangement: &Arrangement, region: &Region) -> Result<[f64; 2]> {
    left_probe(
        &arrangement.pieces,
        &arrangement.halves,
        &arrangement.cycles[region.outer],
        &arrangement.polygons,
        Some((region.outer, &region.holes)),
    )
}
