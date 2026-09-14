//! Regularized curved Boolean closure for canonical sphere/sphere pairs.
//!
//! Both operands must be canonical stereographic sphere solids as certified by
//! `intersections::sphere_sphere::recognize` (eight rational biquadratic
//! patches over six ordinary vertices). A provably disjoint or strictly
//! contained pair resolves by regularized empty algebra — a strict containment
//! difference keeps the outer body with the inner sphere as an inverted cavity
//! shell. A transverse pair is imprinted: the exact rational section circle is
//! segmented at every patch-boundary crossing of BOTH spheres, every crossed
//! patch is split into its two exact UV regions, regions are classified
//! against the other sphere with an outward error band, and the kept regions
//! are stitched into one closed shell. Section pcurves are the exact degree-2
//! Bernstein conversion of the inverted 3D section arc, so the validated
//! pcurve/edge correspondence holds to roundoff rather than to a fit.
//!
//! Anything outside this certificate is an explicit
//! `BREP_UNSUPPORTED_OPERATION`, never a numerical fallback: tangency and
//! coincidence bands, section planes through a patch pole (great-circle UV
//! lines), sections through original vertices or patch corners, section
//! circles tangent to a patch boundary, coincident crossings of both spheres'
//! patch boundaries, and patch-interior caps on BOTH spheres (no patch
//! crossing exists to anchor the imprint segmentation).
use crate::intersections::sphere_sphere::{
    self, CanonicalSphere, PatchUvSection, SphereSphereComponent,
};
use crate::intersections::Options;
use crate::{Body, Coedge, Edge, Face, FaceUse, Loop, Model, Shell, TopologyIds, Vertex};
use nurbs_core::{Error, Result, curve::Curve};
use std::collections::BTreeMap;

const TAU: f64 = std::f64::consts::TAU;
const PI: f64 = std::f64::consts::PI;
/// Weight of the canonical 90-degree trim arc.
const ARC_W: f64 = std::f64::consts::FRAC_1_SQRT_2;
/// UV clearance demanded between a section endpoint and a patch corner.
const CORNER: f64 = 1e-7;
/// UV clearance demanded between an interior section ring and the patch rim.
const RING_CLEAR: f64 = 1e-7;
/// UV boundary-membership margin for chord endpoints and segment midpoints.
const BOUNDARY_UV: f64 = 1e-9;
/// Largest 3D sweep of one section segment (single-span exactness margin).
const MAX_SPAN: f64 = std::f64::consts::FRAC_PI_2 + 1e-9;

fn unsupported(message: impl Into<String>) -> Error {
    Error::new("BREP_UNSUPPORTED_OPERATION", message)
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn norm(a: [f64; 3]) -> f64 {
    a[0].hypot(a[1]).hypot(a[2])
}
fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    norm(sub(a, b))
}

/// Sphere/sphere Boolean entry: `Ok(None)` unless both operands are canonical
/// spheres and the operation is union, difference or intersection.
pub(crate) fn boolean(a: &Model, b: &Model, operation: &str) -> Result<Option<Model>> {
    if !matches!(operation, "union" | "difference" | "intersection") {
        return Ok(None);
    }
    let (Some(sa), Some(sb)) = (sphere_sphere::recognize(a)?, sphere_sphere::recognize(b)?)
    else {
        return Ok(None);
    };
    let report = sphere_sphere::intersect_sphere_sphere(a, b, Options::default())?;
    if let Some(unresolved) = report.unresolved.first() {
        return Err(unsupported(format!(
            "Sphere/sphere Boolean cannot certify the intersection ({:?}); \
             tangency, coincidence and vertex/grazing contacts are not regularized",
            unresolved.reason
        )));
    }
    let tolerance = a.tolerance_mm.max(b.tolerance_mm);
    match &report.components[..] {
        [] => Ok(Some(separated_or_contained(
            a, b, operation, &sa, &sb, tolerance,
        )?)),
        [SphereSphereComponent::Circle {
            curve,
            center,
            radius,
            normal,
            ..
        }] => Ok(Some(Imprint::run(
            a, b, operation, sa, sb, curve, *center, *radius, *normal, tolerance,
        )?)),
        _ => Err(unsupported(
            "Sphere/sphere Boolean: unexpected multiple intersection components",
        )),
    }
}

/// Empty-component resolution: provably disjoint or strictly contained pairs.
fn separated_or_contained(
    a: &Model,
    b: &Model,
    operation: &str,
    sa: &CanonicalSphere,
    sb: &CanonicalSphere,
    tolerance: f64,
) -> Result<Model> {
    // Same outward classification band as the intersection query.
    let delta = sub(sb.center, sa.center);
    let scale = delta.iter().map(|v| v.abs()).fold(0., f64::max);
    let distance = if scale == 0. {
        0.
    } else {
        let scaled = delta.map(|x| x / scale);
        scaled[0].hypot(scaled[1]).hypot(scaled[2]) * scale
    };
    let band =
        sa.error + sb.error + 16. * f64::EPSILON * (distance + scale + sa.radius + sb.radius + 1.);
    let d_lo = (distance - band).max(0.);
    let d_hi = distance + band;
    let sum = sa.radius + sb.radius;
    let diff = (sa.radius - sb.radius).abs();
    if d_lo > sum + band {
        return Ok(match operation {
            "intersection" => Model::empty(tolerance)?,
            "difference" => a.clone(),
            _ => crate::boolean_support::separated_union(a, b)?,
        });
    }
    if d_hi < (diff - band).max(0.) {
        let (outer, inner, a_is_outer) = if sa.radius >= sb.radius {
            (a, b, true)
        } else {
            (b, a, false)
        };
        return Ok(match operation {
            "union" => outer.clone(),
            "intersection" => inner.clone(),
            _ => {
                if a_is_outer {
                    cavity(outer, inner, tolerance)?
                } else {
                    Model::empty(tolerance)?
                }
            }
        });
    }
    Err(unsupported(
        "Sphere/sphere Boolean: separation classification falls inside the error band",
    ))
}

/// Strict containment difference: outer body with the inner sphere's boundary
/// as an inverted cavity shell.
fn cavity(outer: &Model, inner: &Model, tolerance: f64) -> Result<Model> {
    let mut result = outer.clone();
    result.tolerance_mm = tolerance;
    let (v, e, l, f, s) = (
        outer.vertices.len(),
        outer.edges.len(),
        outer.loops.len(),
        outer.faces.len(),
        outer.shells.len(),
    );
    result.vertices.extend(inner.vertices.iter().cloned());
    result.edges.extend(inner.edges.iter().cloned().map(|mut edge| {
        edge.vertices.iter_mut().for_each(|id| *id += v);
        edge
    }));
    result.loops.extend(inner.loops.iter().cloned().map(|mut wire| {
        wire.coedges.iter_mut().for_each(|use_| use_.edge += e);
        wire
    }));
    result.faces.extend(inner.faces.iter().cloned().map(|mut face| {
        face.outer += l;
        face.holes.iter_mut().for_each(|id| *id += l);
        face
    }));
    result.shells.push(Shell {
        faces: inner.shells[0]
            .faces
            .iter()
            .map(|use_| FaceUse {
                face: use_.face + f,
                reversed: !use_.reversed,
            })
            .collect(),
        closed: true,
    });
    result.bodies[0].inner_shells.push(s);
    result.inherit_topology_ids(&[outer, inner]);
    result.validate()?;
    Ok(result)
}

/// One sphere's recognized structure plus its section data.
struct Side {
    canon: CanonicalSphere,
    /// Per face: coedge index of each UV boundary piece (u-axis, quarter arc,
    /// v-axis) inside the face's outer loop.
    pieces: [[usize; 3]; 8],
    /// Original shell FaceUse orientation per face.
    reversed: [bool; 8],
    sections: BTreeMap<usize, PatchSec>,
}
impl Side {
    fn new(model: &Model, canon: CanonicalSphere) -> Self {
        let pieces: [[usize; 3]; 8] = std::array::from_fn(|face| {
            let wire = &model.loops[model.faces[face].outer];
            let mut map = [usize::MAX; 3];
            for (i, coedge) in wire.coedges.iter().enumerate() {
                let piece = if sphere_sphere::unit_quarter_arc(&coedge.pcurve) {
                    1
                } else if sphere_sphere::axis_line(&coedge.pcurve, [0., 0.], [1., 0.]) {
                    0
                } else {
                    // Certified by `recognize`: the (0,1)->(0,0) v-axis line.
                    2
                };
                map[piece] = i;
            }
            map
        });
        let reversed: [bool; 8] = std::array::from_fn(|face| {
            model.shells[0]
                .faces
                .iter()
                .find(|use_| use_.face == face)
                .map(|use_| use_.reversed)
                .unwrap_or(false)
        });
        Self {
            canon,
            pieces,
            reversed,
            sections: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum PatchSec {
    /// Section circle strictly interior to the patch quarter disk.
    Ring { cc: [f64; 2], ruv: f64 },
    /// Section circle clipped to a swept interval; endpoints on the rim.
    Chord {
        cc: [f64; 2],
        start: f64,
        end: f64,
        /// Snapped boundary UV of the span endpoints (start, end).
        w: [[f64; 2]; 2],
        /// Boundary piece of each endpoint (0: v=0, 1: quarter arc, 2: u=0).
        piece: [usize; 2],
        /// Piece parameter (loop direction) of each endpoint.
        tau: [f64; 2],
        /// Global crossing id of each endpoint.
        xing: [usize; 2],
    },
}

/// Global section-circle crossing: a sphere patch-boundary crossing or a
/// refinement point inserted to cap the 3D sweep of one segment.
struct Xing {
    phi: f64,
    point: [f64; 3],
    /// (sphere, edge, edge parameter) for patch-boundary crossings.
    src: Option<(usize, usize, f64)>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum VKey {
    Cross(usize),
    Orig(usize, usize),
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum EKey {
    Sect(usize),
    Orig(usize, usize),
    Piece(usize, usize, usize),
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Keep {
    Drop,
    Whole,
    Fwd,
    Back,
    RingIn,
    RingOut,
}

/// Boundary piece of a snapped UV point; corners are refused by the caller.
fn piece_of(w: [f64; 2]) -> Result<usize> {
    for corner in [[0., 0.], [1., 0.], [0., 1.]] {
        if (w[0] - corner[0]).hypot(w[1] - corner[1]) <= CORNER {
            return Err(unsupported(
                "Sphere/sphere Boolean: section passes through a sphere vertex region",
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
            "Sphere/sphere Boolean: section endpoint is not on exactly one patch boundary piece",
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
        _ => {
            // Invert the rational quarter arc: S = tan(theta/2) = v/(1+u),
            // tau = S / (cos(pi/4) + S (1 - cos(pi/4))).
            let s = w[1] / (1. + w[0]);
            s / (ARC_W + s * (1. - ARC_W))
        }
    }
}
/// UV midpoint of one boundary piece sub-interval (for region sampling).
fn piece_uv_mid(piece: usize, ta: f64, tb: f64) -> [f64; 2] {
    let mid = (ta + tb) / 2.;
    match piece {
        0 => [mid, 0.],
        2 => [0., 1. - mid],
        _ => {
            let s = ARC_W * mid / (1. - mid + ARC_W * mid);
            [(1. - s * s) / (1. + s * s), 2. * s / (1. + s * s)]
        }
    }
}
/// Pieces of the CCW boundary chain from (piece, tau) to (piece, tau).
fn chain_pieces(from: (usize, f64), to: (usize, f64)) -> Vec<(usize, f64, f64)> {
    let mut out = Vec::new();
    let (mut piece, mut ta) = from;
    loop {
        let tb = if piece == to.0 { to.1 } else { 1. };
        out.push((piece, ta, tb));
        if piece == to.0 {
            break;
        }
        piece = (piece + 1) % 3;
        ta = 0.;
    }
    out
}
/// Corner UV at the END of each CCW boundary piece.
fn piece_end_corner(piece: usize) -> [f64; 2] {
    match piece {
        0 => [1., 0.],
        1 => [0., 1.],
        _ => [0., 0.],
    }
}
fn psi_of(cc: [f64; 2], uv: [f64; 2]) -> f64 {
    (uv[1] - cc[1]).atan2(uv[0] - cc[0])
}
fn unwrap_near(value: f64, reference: f64) -> f64 {
    reference + (value - reference + PI).rem_euclid(TAU) - PI
}

struct Imprint<'m> {
    src: [&'m Model; 2],
    sides: [Side; 2],
    center: [f64; 3],
    rho: f64,
    e1: [f64; 3],
    e2: [f64; 3],
    band: f64,
    want_inside: [bool; 2],
    flip: [bool; 2],
    xings: Vec<Xing>,
    sorted: Vec<usize>,
    arc_plus: Vec<Curve>,
    /// Per sphere, segment index -> owning patch.
    member: [Vec<usize>; 2],
    /// Per sphere, segment index -> midpoint UV in its owning patch.
    mid_uv: [Vec<[f64; 2]>; 2],
    /// Per (sphere, patch): segment indices in +psi order with, for each, the
    /// flag "segment's +phi direction is the patch's +psi direction".
    orders: BTreeMap<(usize, usize), (Vec<usize>, Vec<bool>)>,
    keeps: [[Keep; 8]; 2],
    edge_cross: BTreeMap<(usize, usize), Vec<(f64, usize)>>,
    uv: BTreeMap<(usize, usize, usize), [f64; 2]>,
    vertices: Vec<Vertex>,
    edges: Vec<Edge>,
    loops: Vec<Loop>,
    faces: Vec<Face>,
    shell: Vec<FaceUse>,
    vmap: BTreeMap<VKey, usize>,
    emap: BTreeMap<EKey, usize>,
    pcmap: BTreeMap<(usize, usize, usize), Curve>,
}

impl<'m> Imprint<'m> {
    #[allow(clippy::too_many_arguments)]
    fn run(
        a: &'m Model,
        b: &'m Model,
        operation: &str,
        sa: CanonicalSphere,
        sb: CanonicalSphere,
        curve: &Curve,
        center: [f64; 3],
        rho: f64,
        normal: [f64; 3],
        tolerance: f64,
    ) -> Result<Model> {
        let canons = [sa, sb];
        // Section basis from the exact circle curve: t=0 and t=1 are the
        // first two 90-degree arc endpoints, i.e. phi=0 and phi=90 degrees.
        let p0 = curve.evaluate(0.)?.point;
        let p1 = curve.evaluate(1.)?.point;
        let e1 = std::array::from_fn(|k| (p0[k] - center[k]) / rho);
        let e2 = std::array::from_fn(|k| (p1[k] - center[k]) / rho);
        let d = dist(canons[0].center, canons[1].center);
        let scale = canons[0].radius + canons[1].radius + d + 1.;
        let band = canons[0].error + canons[1].error + 1e-9 * scale + 64. * f64::EPSILON * scale;
        let mut imprint = Self {
            src: [a, b],
            sides: [Side::new(a, canons[0].clone()), Side::new(b, canons[1].clone())],
            center,
            rho,
            e1,
            e2,
            band,
            want_inside: [
                operation == "intersection",
                matches!(operation, "intersection" | "difference"),
            ],
            flip: [false, operation == "difference"],
            xings: Vec::new(),
            sorted: Vec::new(),
            arc_plus: Vec::new(),
            member: [Vec::new(), Vec::new()],
            mid_uv: [Vec::new(), Vec::new()],
            orders: BTreeMap::new(),
            keeps: [[Keep::Drop; 8]; 2],
            edge_cross: BTreeMap::new(),
            uv: BTreeMap::new(),
            vertices: Vec::new(),
            edges: Vec::new(),
            loops: Vec::new(),
            faces: Vec::new(),
            shell: Vec::new(),
            vmap: BTreeMap::new(),
            emap: BTreeMap::new(),
            pcmap: BTreeMap::new(),
        };
        imprint.scan_sections(normal)?;
        imprint.register_crossings()?;
        imprint.refine()?;
        imprint.build_segments()?;
        imprint.assign_members()?;
        imprint.order_segments()?;
        imprint.classify()?;
        imprint.assemble()?;
        let mut model = Model(
            brep_topology::Model {
                vertices: imprint.vertices,
                edges: imprint.edges,
                loops: imprint.loops,
                faces: imprint.faces,
                shells: vec![Shell {
                    faces: imprint.shell,
                    closed: true,
                }],
                bodies: vec![Body {
                    outer_shell: 0,
                    inner_shells: vec![],
                }],
                tolerance_mm: tolerance,
            },
            TopologyIds::default(),
        );
        model.inherit_topology_ids(&[a, b]);
        match model.validate() {
            Ok(_) => Ok(model),
            Err(e) if e.code == "BREP_RESOURCE_LIMIT" => Err(e),
            Err(e) => Err(unsupported(format!(
                "Sphere/sphere Boolean result failed validation: {}",
                e.message
            ))),
        }
    }

    fn phi_of(&self, point: [f64; 3]) -> f64 {
        let d = sub(point, self.center);
        dot(d, self.e2).atan2(dot(d, self.e1)).rem_euclid(TAU)
    }
    fn circle_point(&self, phi: f64) -> [f64; 3] {
        let (sin, cos) = phi.sin_cos();
        std::array::from_fn(|k| self.center[k] + self.rho * (cos * self.e1[k] + sin * self.e2[k]))
    }
    fn surface_point(&self, s: usize, patch: usize, uv: [f64; 2]) -> Result<[f64; 3]> {
        let p = self.src[s].faces[patch]
            .surface
            .evaluate(uv[0], uv[1])?
            .point;
        Ok([p[0], p[1], p[2]])
    }

    /// Per-patch UV sections of the section plane; rings and chords only.
    /// Registers one crossing per (sphere, edge, parameter) and stores the
    /// snapped endpoint UV per (sphere, patch, crossing).
    fn scan_sections(&mut self, normal: [f64; 3]) -> Result<()> {
        for s in 0..2 {
            let sections = sphere_sphere::patch_sections(&self.sides[s].canon, normal, self.center);
            for (patch, section) in sections {
                let PatchUvSection::Circle {
                    center: cc,
                    radius: ruv,
                    start,
                    end,
                } = section
                else {
                    return Err(unsupported(
                        "Sphere/sphere Boolean: section plane contains a sphere patch pole \
                         (great-circle section through original vertices)",
                    ));
                };
                if end - start >= TAU - 1e-9 {
                    let q = cc[0].hypot(cc[1]);
                    if cc[0] - ruv <= RING_CLEAR
                        || cc[1] - ruv <= RING_CLEAR
                        || q + ruv >= 1. - RING_CLEAR
                    {
                        return Err(unsupported(
                            "Sphere/sphere Boolean: section circle is tangent to a sphere patch boundary",
                        ));
                    }
                    self.sides[s].sections.insert(patch, PatchSec::Ring { cc, ruv });
                    continue;
                }
                let mut record = PatchSec::Chord {
                    cc,
                    start,
                    end,
                    w: [[0.; 2]; 2],
                    piece: [0; 2],
                    tau: [0.; 2],
                    xing: [usize::MAX; 2],
                };
                for (which, angle) in [(0usize, start), (1, end)] {
                    let raw = [cc[0] + ruv * angle.cos(), cc[1] + ruv * angle.sin()];
                    let piece = piece_of(raw)?;
                    let w = snap_to_piece(piece, raw);
                    let tau = piece_tau(piece, w);
                    let coedge = &self.src[s].loops[self.src[s].faces[patch].outer].coedges
                        [self.sides[s].pieces[patch][piece]];
                    let edge_t = if coedge.reversed { 1. - tau } else { tau };
                    if !(1e-9..=1. - 1e-9).contains(&edge_t) {
                        return Err(unsupported(
                            "Sphere/sphere Boolean: section passes through a sphere vertex",
                        ));
                    }
                    let point = self.surface_point(s, patch, w)?;
                    let id = self.crossing_at(s, coedge.edge, edge_t, point);
                    if let PatchSec::Chord {
                        w: ws,
                        piece: ps,
                        tau: ts,
                        xing: xs,
                        ..
                    } = &mut record
                    {
                        ws[which] = w;
                        ps[which] = piece;
                        ts[which] = tau;
                        xs[which] = id;
                    }
                    self.uv.insert((s, patch, id), w);
                }
                self.sides[s].sections.insert(patch, record);
            }
        }
        Ok(())
    }

    /// Find or create the global crossing for one (sphere, edge, parameter).
    fn crossing_at(&mut self, sphere: usize, edge: usize, edge_t: f64, point: [f64; 3]) -> usize {
        for (id, xing) in self.xings.iter().enumerate() {
            if let Some((s, e, t)) = xing.src {
                if s == sphere && e == edge && (t - edge_t).abs() <= 1e-9 {
                    return id;
                }
            }
        }
        let id = self.xings.len();
        let phi = self.phi_of(point);
        self.xings.push(Xing {
            phi,
            point,
            src: Some((sphere, edge, edge_t)),
        });
        id
    }

    /// Sort crossings, reject coincidences and near-tangent crowding.
    fn register_crossings(&mut self) -> Result<()> {
        if self.xings.len() < 2 {
            return Err(unsupported(
                "Sphere/sphere Boolean: section circle is patch-interior on both spheres; \
                 cap-only imprints are not yet supported",
            ));
        }
        for i in 0..self.xings.len() {
            for j in i + 1..self.xings.len() {
                if dist(self.xings[i].point, self.xings[j].point) <= 1e-9 {
                    return Err(unsupported(
                        "Sphere/sphere Boolean: section crossings coincide \
                         (shared vertex or simultaneous patch-boundary event)",
                    ));
                }
            }
        }
        self.sorted = (0..self.xings.len()).collect();
        self.sorted
            .sort_by(|&i, &j| self.xings[i].phi.total_cmp(&self.xings[j].phi));
        for k in 0..self.sorted.len() {
            let i0 = self.sorted[k];
            let i1 = self.sorted[(k + 1) % self.sorted.len()];
            let gap = (self.xings[i1].phi - self.xings[i0].phi).rem_euclid(TAU);
            if gap * self.rho < 1e-9 {
                return Err(unsupported(
                    "Sphere/sphere Boolean: section crossings are too close to certify",
                ));
            }
        }
        Ok(())
    }

    /// Split segments whose 3D sweep exceeds one quadrant.
    fn refine(&mut self) -> Result<()> {
        for _ in 0..8 {
            let mut split = None;
            for k in 0..self.sorted.len() {
                let i0 = self.sorted[k];
                let i1 = self.sorted[(k + 1) % self.sorted.len()];
                let gap = (self.xings[i1].phi - self.xings[i0].phi).rem_euclid(TAU);
                if gap > MAX_SPAN {
                    split = Some((k, i0, gap));
                    break;
                }
            }
            let Some((k, i0, gap)) = split else {
                return Ok(());
            };
            let phi = (self.xings[i0].phi + gap / 2.).rem_euclid(TAU);
            let point = self.circle_point(phi);
            let id = self.xings.len();
            self.xings.push(Xing {
                phi,
                point,
                src: None,
            });
            self.sorted.insert(k + 1, id);
        }
        Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Sphere/sphere Boolean: section refinement did not converge",
        ))
    }

    /// Segment curves (single-span exact rational arcs, +phi orientation) and
    /// the per-edge split tables.
    fn build_segments(&mut self) -> Result<()> {
        let n = self.sorted.len();
        for k in 0..n {
            let i0 = self.sorted[k];
            let i1 = self.sorted[(k + 1) % n];
            let gap = (self.xings[i1].phi - self.xings[i0].phi).rem_euclid(TAU);
            let weight = (gap / 2.).cos();
            if !(weight > 0.) {
                return Err(unsupported(
                    "Sphere/sphere Boolean: section segment exceeds a semicircle",
                ));
            }
            let mid = self.xings[i0].phi + gap / 2.;
            let (sin, cos) = mid.sin_cos();
            let shoulder: [f64; 3] = std::array::from_fn(|i| {
                self.center[i] + (self.rho / weight) * (cos * self.e1[i] + sin * self.e2[i])
            });
            self.arc_plus.push(Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: vec![
                    self.xings[i0].point.to_vec(),
                    shoulder.to_vec(),
                    self.xings[i1].point.to_vec(),
                ],
                weights: vec![1., weight, 1.],
                periodic: false,
            });
        }
        for (id, xing) in self.xings.iter().enumerate() {
            if let Some((s, e, t)) = xing.src {
                self.edge_cross.entry((s, e)).or_default().push((t, id));
            }
        }
        for splits in self.edge_cross.values_mut() {
            splits.sort_by(|a, b| a.0.total_cmp(&b.0));
        }
        Ok(())
    }

    /// Assign every segment to the one patch of each sphere that contains its
    /// midpoint strictly inside the quarter disk.
    fn assign_members(&mut self) -> Result<()> {
        let n = self.sorted.len();
        for s in 0..2 {
            let mut member = vec![usize::MAX; n];
            let mut mid_uv = vec![[0.; 2]; n];
            for k in 0..n {
                let i0 = self.sorted[k];
                let i1 = self.sorted[(k + 1) % n];
                let gap = (self.xings[i1].phi - self.xings[i0].phi).rem_euclid(TAU);
                let mid = self.circle_point(self.xings[i0].phi + gap / 2.);
                let mut found = None;
                for &patch in self.sides[s].sections.keys() {
                    let Some(uv) = self.sides[s].canon.invert_uv(patch, mid) else {
                        continue;
                    };
                    if uv[0] > BOUNDARY_UV
                        && uv[1] > BOUNDARY_UV
                        && uv[0] * uv[0] + uv[1] * uv[1] < 1. - BOUNDARY_UV
                    {
                        if found.is_some() {
                            return Err(unsupported(
                                "Sphere/sphere Boolean: section segment midpoint lies in two patches",
                            ));
                        }
                        found = Some((patch, uv));
                    }
                }
                let Some((patch, uv)) = found else {
                    return Err(unsupported(
                        "Sphere/sphere Boolean: section segment midpoint escapes every patch",
                    ));
                };
                member[k] = patch;
                mid_uv[k] = uv;
            }
            self.member[s] = member;
            self.mid_uv[s] = mid_uv;
        }
        Ok(())
    }

    /// UV of a crossing in a patch chart: snapped boundary value for chord
    /// endpoints, exact rational inversion otherwise.
    fn uv_at(&mut self, s: usize, patch: usize, id: usize) -> Result<[f64; 2]> {
        if let Some(&uv) = self.uv.get(&(s, patch, id)) {
            return Ok(uv);
        }
        let point = self.xings[id].point;
        let uv = self.sides[s]
            .canon
            .invert_uv(patch, point)
            .ok_or_else(|| {
                unsupported("Sphere/sphere Boolean: section point escapes its patch chart")
            })?;
        if uv[0] < -1e-7
            || uv[1] < -1e-7
            || uv[0] > 1. + 1e-7
            || uv[1] > 1. + 1e-7
            || uv[0] * uv[0] + uv[1] * uv[1] > 1. + 1e-7
        {
            return Err(unsupported(
                "Sphere/sphere Boolean: section point outside the patch quarter disk",
            ));
        }
        self.uv.insert((s, patch, id), uv);
        Ok(uv)
    }

    /// Order member segments along each sectioned patch in +psi (ccw around
    /// the UV section center) and fix the phi/psi direction per segment.
    fn order_segments(&mut self) -> Result<()> {
        let n = self.sorted.len();
        for s in 0..2 {
            let patches: Vec<usize> = self.sides[s].sections.keys().copied().collect();
            for patch in patches {
                let (cc, span) = match self.sides[s].sections[&patch] {
                    PatchSec::Ring { cc, .. } => (cc, None),
                    PatchSec::Chord { cc, start, end, .. } => (cc, Some((start, end))),
                };
                let members: Vec<usize> = (0..n).filter(|&k| self.member[s][k] == patch).collect();
                if members.is_empty() {
                    return Err(unsupported(
                        "Sphere/sphere Boolean: sectioned patch owns no section segment",
                    ));
                }
                let mut keyed: Vec<(f64, usize)> = Vec::with_capacity(members.len());
                for &k in &members {
                    let raw = psi_of(cc, self.mid_uv[s][k]);
                    let key = match span {
                        Some((start, end)) => {
                            let psi = start + (raw - start).rem_euclid(TAU);
                            if psi > end + 1e-6 {
                                return Err(unsupported(
                                    "Sphere/sphere Boolean: section segment escapes its patch span",
                                ));
                            }
                            psi
                        }
                        None => raw.rem_euclid(TAU),
                    };
                    keyed.push((key, k));
                }
                keyed.sort_by(|a, b| a.0.total_cmp(&b.0));
                // Contiguity: consecutive members must share a crossing.
                for w in keyed.windows(2) {
                    let end_id = self.sorted[(w[0].1 + 1) % n];
                    if end_id != self.sorted[w[1].1] {
                        return Err(unsupported(
                            "Sphere/sphere Boolean: patch section segments are not contiguous",
                        ));
                    }
                }
                let mut order = Vec::with_capacity(members.len());
                let mut plus = Vec::with_capacity(members.len());
                for &(psi_mid, k) in &keyed {
                    let i0 = self.sorted[k];
                    let i1 = self.sorted[(k + 1) % n];
                    let uv0 = self.uv_at(s, patch, i0)?;
                    let uv1 = self.uv_at(s, patch, i1)?;
                    let psi0 = unwrap_near(psi_of(cc, uv0), psi_mid);
                    let psi1 = unwrap_near(psi_of(cc, uv1), psi_mid);
                    if (psi1 - psi0).abs() <= 1e-12 {
                        return Err(unsupported(
                            "Sphere/sphere Boolean: section segment collapses in UV",
                        ));
                    }
                    order.push(k);
                    plus.push(psi1 > psi0);
                }
                if plus.iter().any(|&p| p != plus[0]) {
                    return Err(unsupported(
                        "Sphere/sphere Boolean: section direction flips inside one patch",
                    ));
                }
                self.orders.insert((s, patch), (order, plus));
            }
        }
        Ok(())
    }

    /// Classify one 3D point against the other sphere: true = strictly inside.
    fn classify_point(&self, other: usize, point: [f64; 3]) -> Result<bool> {
        let canon = &self.sides[other].canon;
        let d = dist(point, canon.center);
        if d < canon.radius - self.band {
            Ok(true)
        } else if d > canon.radius + self.band {
            Ok(false)
        } else {
            Err(unsupported(
                "Sphere/sphere Boolean: region classification falls within \
                 the error band of the other sphere",
            ))
        }
    }

    /// Region classes of one chord patch: (section-forward region, section-
    /// backward region). The forward region is bounded by the chord w0->w1
    /// and the ccw boundary chain w1->w0; the backward region takes the rest.
    fn classify_chord(&self, s: usize, patch: usize) -> Result<(bool, bool)> {
        let PatchSec::Chord { w, piece, tau, .. } = self.sides[s].sections[&patch] else {
            return Err(unsupported("Sphere/sphere Boolean: internal section kind mismatch"));
        };
        let other = 1 - s;
        // Chains: forward region chain runs w1 -> w0, backward chain w0 -> w1.
        let classify_chain = |from: (usize, f64), to: (usize, f64)| -> Result<Option<bool>> {
            let chain = chain_pieces(from, to);
            let corners: Vec<[f64; 2]> = chain
                .iter()
                .filter(|(_, _, tb)| *tb == 1.)
                .map(|(p, _, _)| piece_end_corner(*p))
                .collect();
            if !corners.is_empty() {
                let mut class = None;
                for corner in corners {
                    let point = self.surface_point(s, patch, corner)?;
                    let inside = self.classify_point(other, point)?;
                    if let Some(previous) = class {
                        if previous != inside {
                            return Err(unsupported(
                                "Sphere/sphere Boolean: patch corners disagree about the other sphere",
                            ));
                        }
                    } else {
                        class = Some(inside);
                    }
                }
                return Ok(class);
            }
            // Corner-less lens: one boundary sub-piece; sample just inside it.
            let [(piece, ta, tb)] = chain[..] else {
                return Err(unsupported(
                    "Sphere/sphere Boolean: corner-less region with multiple boundary pieces",
                ));
            };
            let uv_mid = piece_uv_mid(piece, ta, tb);
            let inward = match piece {
                0 => [0., 1.],
                2 => [1., 0.],
                _ => [-uv_mid[0], -uv_mid[1]],
            };
            let mut class = None;
            for delta in [1e-2, 1e-3, 1e-4] {
                let uv = [
                    uv_mid[0] + inward[0] * delta,
                    uv_mid[1] + inward[1] * delta,
                ];
                let point = self.surface_point(s, patch, uv)?;
                let inside = self.classify_point(other, point)?;
                if let Some(previous) = class {
                    if previous != inside {
                        return Err(unsupported(
                            "Sphere/sphere Boolean: lens region samples disagree",
                        ));
                    }
                } else {
                    class = Some(inside);
                }
            }
            Ok(class)
        };
        let back_class = classify_chain((piece[0], tau[0]), (piece[1], tau[1]))?
            .ok_or_else(|| unsupported("Sphere/sphere Boolean: unclassified region"))?;
        let fwd_class = classify_chain((piece[1], tau[1]), (piece[0], tau[0]))?
            .ok_or_else(|| unsupported("Sphere/sphere Boolean: unclassified region"))?;
        if fwd_class == back_class {
            return Err(unsupported(
                "Sphere/sphere Boolean: section does not separate the patch regions",
            ));
        }
        let _ = w;
        Ok((fwd_class, back_class))
    }

    /// Decide the kept region of every patch of both spheres.
    fn classify(&mut self) -> Result<()> {
        for s in 0..2 {
            let other = 1 - s;
            for patch in 0..8 {
                let keep = match self.sides[s].sections.get(&patch).copied() {
                    None => {
                        let pole = self.surface_point(s, patch, [0., 0.])?;
                        if self.classify_point(other, pole)? == self.want_inside[s] {
                            Keep::Whole
                        } else {
                            Keep::Drop
                        }
                    }
                    Some(PatchSec::Ring { cc, .. }) => {
                        let class_in =
                            self.classify_point(other, self.surface_point(s, patch, cc)?)?;
                        let mut class_out = None;
                        for corner in [[0., 0.], [1., 0.], [0., 1.]] {
                            let inside =
                                self.classify_point(other, self.surface_point(s, patch, corner)?)?;
                            if let Some(previous) = class_out {
                                if previous != inside {
                                    return Err(unsupported(
                                        "Sphere/sphere Boolean: ring patch corners disagree",
                                    ));
                                }
                            } else {
                                class_out = Some(inside);
                            }
                        }
                        if Some(class_in) == class_out {
                            return Err(unsupported(
                                "Sphere/sphere Boolean: ring does not separate the patch regions",
                            ));
                        }
                        if class_in == self.want_inside[s] {
                            Keep::RingIn
                        } else {
                            Keep::RingOut
                        }
                    }
                    Some(PatchSec::Chord { .. }) => {
                        let (fwd, _back) = self.classify_chord(s, patch)?;
                        if fwd == self.want_inside[s] {
                            Keep::Fwd
                        } else {
                            Keep::Back
                        }
                    }
                };
                self.keeps[s][patch] = keep;
            }
        }
        Ok(())
    }

    fn vertex(&mut self, key: VKey) -> usize {
        if let Some(&id) = self.vmap.get(&key) {
            return id;
        }
        let point = match key {
            VKey::Cross(id) => self.xings[id].point,
            VKey::Orig(s, v) => self.src[s].vertices[v].point,
        };
        let id = self.vertices.len();
        self.vertices.push(Vertex { point });
        self.vmap.insert(key, id);
        id
    }

    /// Vertex key at one original-edge parameter: original endpoint or
    /// crossing.
    fn vkey_at(&self, s: usize, edge: usize, t: f64) -> Result<VKey> {
        let src = &self.src[s].edges[edge];
        if t <= 0. {
            return Ok(VKey::Orig(s, src.vertices[0]));
        }
        if t >= 1. {
            return Ok(VKey::Orig(s, src.vertices[1]));
        }
        let splits = self.edge_cross.get(&(s, edge)).ok_or_else(|| {
            unsupported("Sphere/sphere Boolean: split edge has no crossing table")
        })?;
        for &(et, id) in splits {
            if (et - t).abs() <= 1e-12 {
                return Ok(VKey::Cross(id));
            }
        }
        Err(unsupported(
            "Sphere/sphere Boolean: split parameter does not match a crossing",
        ))
    }

    fn edge(&mut self, key: EKey) -> Result<usize> {
        if let Some(&id) = self.emap.get(&key) {
            return Ok(id);
        }
        let edge = match key {
            EKey::Sect(k) => {
                let n = self.sorted.len();
                let v0 = self.vertex(VKey::Cross(self.sorted[k]));
                let v1 = self.vertex(VKey::Cross(self.sorted[(k + 1) % n]));
                Edge {
                    degenerate: false,
                    vertices: [v0, v1],
                    curve: self.arc_plus[k].clone(),
                }
            }
            EKey::Orig(s, e) => {
                let src = &self.src[s].edges[e];
                let v0 = self.vertex(VKey::Orig(s, src.vertices[0]));
                let v1 = self.vertex(VKey::Orig(s, src.vertices[1]));
                Edge {
                    degenerate: false,
                    vertices: [v0, v1],
                    curve: src.curve.clone(),
                }
            }
            EKey::Piece(s, e, j) => {
                let splits = self.edge_cross.get(&(s, e)).cloned().ok_or_else(|| {
                    unsupported("Sphere/sphere Boolean: piece of an uncrossed edge")
                })?;
                let t_lo = if j == 0 { 0. } else { splits[j - 1].0 };
                let t_hi = if j == splits.len() { 1. } else { splits[j].0 };
                let v0 = self.vertex(self.vkey_at(s, e, t_lo)?);
                let v1 = self.vertex(self.vkey_at(s, e, t_hi)?);
                Edge {
                    degenerate: false,
                    vertices: [v0, v1],
                    curve: self.src[s].edges[e].curve.trim(t_lo, t_hi)?,
                }
            }
        };
        let id = self.edges.len();
        self.edges.push(edge);
        self.emap.insert(key, id);
        Ok(id)
    }

    /// Exact pcurve of section segment k in patch `patch` of sphere `s`, in
    /// the +phi direction: degree-2 Bernstein conversion of the inverted 3D
    /// arc, endpoints snapped to the shared crossing UV values.
    ///
    /// For the patch frame (a, b, pole), sphere center c and radius r the
    /// inversion of a point with homogeneous sample (H, hw) is
    /// u = d.a / (r hw + d.pole), v = d.b / (r hw + d.pole) with
    /// d = H - c hw. Numerator and denominator are degree-2 in the arc
    /// parameter, so three homogeneous samples fix the exact Bernstein form.
    fn sect_pcurve(&mut self, s: usize, patch: usize, k: usize) -> Result<Curve> {
        if let Some(curve) = self.pcmap.get(&(s, patch, k)) {
            return Ok(curve.clone());
        }
        let (a, b, pole) = self.sides[s].canon.frame(patch);
        let center = self.sides[s].canon.center;
        let radius = self.sides[s].canon.radius;
        let arc = self.arc_plus[k].clone();
        let sample = |t: f64| -> Result<(f64, f64, f64)> {
            // Homogeneous de Casteljau on the single-span quadratic arc.
            let b0 = (1. - t) * (1. - t);
            let b1 = 2. * t * (1. - t);
            let b2 = t * t;
            let mut h = [0.; 3];
            let mut hw = 0.;
            for (i, basis) in [b0, b1, b2].into_iter().enumerate() {
                let w = basis * arc.weights[i];
                for axis in 0..3 {
                    h[axis] += w * arc.control_points[i][axis];
                }
                hw += w;
            }
            let d = std::array::from_fn(|axis| h[axis] - center[axis] * hw);
            let nu = dot(d, a);
            let nv = dot(d, b);
            let den = radius * hw + dot(d, pole);
            Ok((nu, nv, den))
        };
        let f0 = sample(0.)?;
        let f1 = sample(0.5)?;
        let f2 = sample(1.)?;
        // Bernstein coefficients of a degree-2 polynomial from three samples.
        let bern = |f: (f64, f64, f64), i: usize| -> f64 {
            match i {
                0 => f.0,
                2 => f.2,
                _ => 2. * f.1 - 0.5 * f.0 - 0.5 * f.2,
            }
        };
        let n = self.sorted.len();
        let id0 = self.sorted[k];
        let id1 = self.sorted[(k + 1) % n];
        let uv0 = self.uv_at(s, patch, id0)?;
        let uv1 = self.uv_at(s, patch, id1)?;
        let mut control_points = Vec::with_capacity(3);
        let mut weights = Vec::with_capacity(3);
        for i in 0..3 {
            let den = bern((f0.2, f1.2, f2.2), i);
            if !(den > 0.) {
                return Err(unsupported(
                    "Sphere/sphere Boolean: section arc leaves the patch hemisphere",
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
        // Certified correspondence: the pcurve through the surface must track
        // the 3D arc pointwise, not merely at the endpoints.
        let limit = 1e-9 * radius.max(1.);
        for i in 0..=8 {
            let t = i as f64 / 8.;
            let uv = pcurve.evaluate(t)?.point;
            let through = self.surface_point(s, patch, [uv[0], uv[1]])?;
            let along = arc.evaluate(t)?.point;
            if dist(through, [along[0], along[1], along[2]]) > limit {
                return Err(unsupported(
                    "Sphere/sphere Boolean: section pcurve does not track the 3D arc",
                ));
            }
        }
        self.pcmap.insert((s, patch, k), pcurve.clone());
        Ok(pcurve)
    }

    /// Coedge of section segment k traversed in the +phi (true) or -phi
    /// (false) direction, in patch `patch` of sphere `s`.
    fn sect_coedge(&mut self, s: usize, patch: usize, k: usize, plus_phi: bool) -> Result<Coedge> {
        let edge = self.edge(EKey::Sect(k))?;
        let pcurve = self.sect_pcurve(s, patch, k)?;
        let pcurve = if plus_phi { pcurve } else { pcurve.reverse()? };
        Ok(Coedge {
            edge,
            reversed: !plus_phi,
            pcurve,
        })
    }

    /// Clone of an uncrossed original face loop with remapped edges.
    fn whole_coedges(&mut self, s: usize, patch: usize) -> Result<Vec<Coedge>> {
        let src = self.src[s];
        let wire = &src.loops[src.faces[patch].outer];
        let mut out = Vec::with_capacity(wire.coedges.len());
        for oc in &wire.coedges {
            let edge = self.edge(EKey::Orig(s, oc.edge))?;
            out.push(Coedge {
                edge,
                reversed: oc.reversed,
                pcurve: oc.pcurve.clone(),
            });
        }
        Ok(out)
    }

    /// One boundary chain element: a (possibly trimmed) original edge piece
    /// with the trimmed original pcurve; both keep the original
    /// parameterization, so the coedge direction flag is unchanged.
    fn chain_coedge(
        &mut self,
        s: usize,
        patch: usize,
        piece: usize,
        tau_a: f64,
        tau_b: f64,
    ) -> Result<Coedge> {
        let src = self.src[s];
        let oc = &src.loops[src.faces[patch].outer].coedges[self.sides[s].pieces[patch][piece]];
        let edge_id = oc.edge;
        let reversed = oc.reversed;
        let t = |tau: f64| if reversed { 1. - tau } else { tau };
        let t_lo = t(tau_a).min(t(tau_b));
        let t_hi = t(tau_a).max(t(tau_b));
        let whole = tau_a == 0. && tau_b == 1.;
        let (ekey, mut pcurve) = if whole {
            (EKey::Orig(s, edge_id), oc.pcurve.clone())
        } else {
            let splits = self
                .edge_cross
                .get(&(s, edge_id))
                .cloned()
                .ok_or_else(|| unsupported("Sphere/sphere Boolean: chain piece of an uncrossed edge"))?;
            let count = splits.len() + 1;
            let j = (0..count)
                .find(|&j| {
                    let lo = if j == 0 { 0. } else { splits[j - 1].0 };
                    let hi = if j == count - 1 { 1. } else { splits[j].0 };
                    (lo - t_lo).abs() <= 1e-12 && (hi - t_hi).abs() <= 1e-12
                })
                .ok_or_else(|| {
                    unsupported("Sphere/sphere Boolean: chain piece does not match the edge split")
                })?;
            (EKey::Piece(s, edge_id, j), oc.pcurve.trim(tau_a, tau_b)?)
        };
        if !whole {
            // Snap crossing endpoints to the stored per-patch UV values so UV
            // chain junctions close exactly.
            for (tau, first) in [(tau_a, true), (tau_b, false)] {
                if let VKey::Cross(id) = self.vkey_at(s, edge_id, t(tau))? {
                    let uv = self.uv_at(s, patch, id)?;
                    let n = pcurve.control_points.len();
                    pcurve.control_points[if first { 0 } else { n - 1 }] = uv.to_vec();
                }
            }
        }
        let edge = self.edge(ekey)?;
        Ok(Coedge {
            edge,
            reversed,
            pcurve,
        })
    }

    /// CCW boundary chain coedges from one chord endpoint to the other.
    fn chain_coedges(
        &mut self,
        s: usize,
        patch: usize,
        from: (usize, f64),
        to: (usize, f64),
    ) -> Result<Vec<Coedge>> {
        let mut out = Vec::new();
        for (piece, ta, tb) in chain_pieces(from, to) {
            out.push(self.chain_coedge(s, patch, piece, ta, tb)?);
        }
        Ok(out)
    }

    /// Section coedges of one patch along (+psi) or against (-psi) the UV
    /// section-circle direction.
    fn section_loop(&mut self, s: usize, patch: usize, forward: bool) -> Result<Vec<Coedge>> {
        let (order, plus) = self
            .orders
            .get(&(s, patch))
            .cloned()
            .ok_or_else(|| unsupported("Sphere/sphere Boolean: patch has no segment order"))?;
        let indices: Vec<usize> = if forward {
            (0..order.len()).collect()
        } else {
            (0..order.len()).rev().collect()
        };
        let mut out = Vec::with_capacity(indices.len());
        for i in indices {
            let plus_phi = if forward { plus[i] } else { !plus[i] };
            out.push(self.sect_coedge(s, patch, order[i], plus_phi)?);
        }
        Ok(out)
    }

    fn push_loop(&mut self, coedges: Vec<Coedge>) -> usize {
        let id = self.loops.len();
        self.loops.push(Loop { coedges });
        id
    }

    fn emit_face(&mut self, s: usize, patch: usize, outer: Vec<Coedge>, holes: Vec<Vec<Coedge>>) {
        let outer = self.push_loop(outer);
        let holes = holes.into_iter().map(|h| self.push_loop(h)).collect();
        let face = self.faces.len();
        self.faces.push(Face {
            surface: self.src[s].faces[patch].surface.clone(),
            outer,
            holes,
        });
        self.shell.push(FaceUse {
            face,
            reversed: self.sides[s].reversed[patch] ^ self.flip[s],
        });
    }

    /// Emit the kept region faces of both spheres.
    fn assemble(&mut self) -> Result<()> {
        for s in 0..2 {
            for patch in 0..8 {
                match self.keeps[s][patch] {
                    Keep::Drop => {}
                    Keep::Whole => {
                        let coedges = self.whole_coedges(s, patch)?;
                        self.emit_face(s, patch, coedges, vec![]);
                    }
                    Keep::RingIn => {
                        let coedges = self.section_loop(s, patch, true)?;
                        self.emit_face(s, patch, coedges, vec![]);
                    }
                    Keep::RingOut => {
                        let outer = self.whole_coedges(s, patch)?;
                        let hole = self.section_loop(s, patch, false)?;
                        self.emit_face(s, patch, outer, vec![hole]);
                    }
                    Keep::Fwd | Keep::Back => {
                        let PatchSec::Chord { piece, tau, .. } = self.sides[s].sections[&patch]
                        else {
                            return Err(unsupported(
                                "Sphere/sphere Boolean: internal section kind mismatch",
                            ));
                        };
                        let forward = self.keeps[s][patch] == Keep::Fwd;
                        let mut coedges = self.section_loop(s, patch, forward)?;
                        // Forward region: boundary chain w1 -> w0; backward: w0 -> w1.
                        let (from, to) = if forward {
                            ((piece[1], tau[1]), (piece[0], tau[0]))
                        } else {
                            ((piece[0], tau[0]), (piece[1], tau[1]))
                        };
                        coedges.extend(self.chain_coedges(s, patch, from, to)?);
                        self.emit_face(s, patch, coedges, vec![]);
                    }
                }
            }
        }
        Ok(())
    }
}
