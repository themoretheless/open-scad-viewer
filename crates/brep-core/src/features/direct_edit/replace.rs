use super::*;
use crate::analysis::surface_classify::classify_surface;
use nurbs_core::curve::{Curve, CurveSampler};
use nurbs_core::foundation::project_curve_report;
use nurbs_core::surface::{Surface, SurfaceRefSampler};

/// Proof that one re-authored edge and its target pcurve describe the same
/// geometry. Exact identities report their actual predicate work; bounded
/// certificates report the number of verified agreement cells.
#[derive(Clone, Debug)]
pub struct EdgeCorrespondenceCertificate {
    pub edge: usize,
    pub exact: bool,
    pub agreement_cells: usize,
    pub max_residual_mm: f64,
}

#[derive(Clone, Debug)]
pub struct ReplaceFaceReport {
    /// The staged, validated and audited result model.
    pub model: Model,
    /// Input-model index of the replaced face.
    pub face: usize,
    /// "plane" | "cylinder".
    pub target_kind: &'static str,
    /// Loop vertices re-positioned onto the target (slide line ∩ target,
    /// nearest branch to the old position).
    pub retrimmed_vertices: usize,
    /// Edges re-authored: the replaced face's loop edges plus every edge
    /// incident to a moved vertex (neighbor/neighbor slide edges included).
    pub reauthored_edges: usize,
    /// Pcurves recomputed on the target and on the planar neighbors.
    pub recomputed_pcurves: usize,
    /// Conservative seed + second-order + result-expanded source region.
    pub dirty_faces: usize,
    /// Faces whose surface or pcurves actually changed; all others are reused.
    pub rebuilt_faces: usize,
    pub reused_faces: usize,
    /// Complete curve/pcurve correspondence for every replaced-face edge.
    pub correspondences: Vec<EdgeCorrespondenceCertificate>,
    /// Max sampled |surface(pcurve(t)) − edge(t)| residual over the
    /// re-authored target-face coedges (authoring evidence, independently
    /// enforced by `Model::validate`).
    pub max_surface_residual_mm: f64,
    pub volume_before_mm3: f64,
    pub volume_after_mm3: f64,
    /// Measured delta from `mass_properties` (after − before).
    pub delta_volume_actual_mm3: f64,
    /// Post-operation solid audit certificate (acceptance gate).
    pub audit: SolidAuditCertificate,
}

/// Bilinear planar patch frame + knot domains (the rigid UV map of
/// [`curve_to_uv`], scaled from the unit square into the actual domain).
#[derive(Clone, Copy, Debug)]
struct PlaneFrame {
    origin: [f64; 3],
    u: [f64; 3],
    v: [f64; 3],
    len_u: f64,
    len_v: f64,
    domain_u: [f64; 2],
    domain_v: [f64; 2],
}

fn knot_domain(knots: &[f64], degree: usize) -> [f64; 2] {
    [knots[degree], knots[knots.len() - 1 - degree]]
}

fn plane_frame(surface: &Surface) -> Option<PlaneFrame> {
    if surface.degree_u != 1
        || surface.degree_v != 1
        || surface.control_points.len() != 2
        || surface.control_points[0].len() != 2
    {
        return None;
    }
    let cp = |i: usize, j: usize| {
        let p = &surface.control_points[i][j];
        [p[0], p[1], p[2]]
    };
    let origin = cp(0, 0);
    let du = sub(cp(1, 0), origin);
    let dv = sub(cp(0, 1), origin);
    let (len_u, len_v) = (norm(du), norm(dv));
    if !(len_u.is_finite() && len_v.is_finite() && len_u > 0. && len_v > 0.) {
        return None;
    }
    Some(PlaneFrame {
        origin,
        u: mul(du, 1. / len_u),
        v: mul(dv, 1. / len_v),
        len_u,
        len_v,
        domain_u: knot_domain(&surface.knots_u, surface.degree_u),
        domain_v: knot_domain(&surface.knots_v, surface.degree_v),
    })
}

impl PlaneFrame {
    fn equation(&self) -> PlaneEq {
        let n = cross(self.u, self.v);
        let normal = mul(n, 1. / norm(n));
        PlaneEq {
            normal,
            offset: dot(normal, self.origin),
        }
    }
    fn scale_uv(&self, normalized: [f64; 2]) -> [f64; 2] {
        [
            self.domain_u[0] + normalized[0] * (self.domain_u[1] - self.domain_u[0]),
            self.domain_v[0] + normalized[1] * (self.domain_v[1] - self.domain_v[0]),
        ]
    }
    fn in_domain(&self, uv: [f64; 2], eps: f64) -> bool {
        uv[0] >= self.domain_u[0] - eps
            && uv[0] <= self.domain_u[1] + eps
            && uv[1] >= self.domain_v[0] - eps
            && uv[1] <= self.domain_v[1] + eps
    }
}

/// Exact UV line between the endpoints of an oriented 3D curve, on the same
/// parameter domain. Exact for the iso-curve segments (cylinder) and affine
/// images (plane) authored here; `Model::validate` independently enforces
/// the sampled agreement.
fn uv_line(start: [f64; 2], end: [f64; 2], domain: [f64; 2]) -> Curve {
    Curve {
        degree: 1,
        knots: vec![domain[0], domain[0], domain[1], domain[1]],
        control_points: vec![start.to_vec(), end.to_vec()],
        weights: vec![1., 1.],
        periodic: false,
    }
}

/// Closest-point projection onto a curve: certified candidate isolation
/// (`project_curve_report`, nearest branch wins) + Gauss–Newton refinement
/// to parameter precision.
fn project_parameter(curve: &Curve, point: [f64; 3]) -> Result<f64> {
    let report = project_curve_report(curve, &point.to_vec(), None)?;
    let best = report
        .candidates
        .iter()
        .min_by(|a, b| a.distance_upper.total_cmp(&b.distance_upper))
        .ok_or_else(|| {
            error(
                REPLACE_UNSUPPORTED,
                "Curve projection produced no candidate",
            )
        })?;
    let sampler = CurveSampler::new(curve)?;
    let domain = sampler.domain();
    let mut t = (0.5 * (best.parameter_interval[0] + best.parameter_interval[1]))
        .clamp(domain[0], domain[1]);
    let evaluated = sampler.point3(t)?;
    let delta = sub(evaluated, point);
    let mut best_distance2 = dot(delta, delta);
    // The certified intervals isolate stationary roots, but an interval's
    // midpoint is not necessarily its closest point. Add deterministic seeds
    // and choose by measured distance before local refinement.
    for i in 0..=64 {
        let candidate = domain[0] + (domain[1] - domain[0]) * i as f64 / 64.;
        let q = sampler.point3(candidate)?;
        let delta = sub(q, point);
        let distance2 = dot(delta, delta);
        if distance2 < best_distance2 {
            t = candidate;
            best_distance2 = distance2;
        }
    }
    for _ in 0..12 {
        let ev = sampler.evaluate(t)?;
        let q = [ev.point[0], ev.point[1], ev.point[2]];
        let d = sub(q, point);
        let distance2 = dot(d, d);
        if distance2 <= 1e-28 {
            break;
        }
        let deriv = match &ev.d1 {
            Some(d) if d.len() == 3 => [d[0], d[1], d[2]],
            _ => {
                let h = (domain[1] - domain[0]) * 1e-7;
                let ta = (t - h).max(domain[0]);
                let tb = (t + h).min(domain[1]);
                if tb - ta <= 0. {
                    break;
                }
                let a = sampler.point3(ta)?;
                let b = sampler.point3(tb)?;
                mul(sub(b, a), 1. / (tb - ta))
            }
        };
        let f = dot(d, deriv);
        let df = dot(deriv, deriv);
        if !df.is_finite() || df <= 1e-30 {
            break;
        }
        let step = f / df;
        // A full Newton step can overshoot on a rational arc. Backtrack and
        // accept only actual distance decreases, never a worse stationary end.
        let mut scale = 1.;
        let mut accepted = false;
        for _ in 0..16 {
            let candidate = (t - scale * step).clamp(domain[0], domain[1]);
            let q = sampler.point3(candidate)?;
            let delta = sub(q, point);
            if dot(delta, delta) <= distance2 {
                accepted = (candidate - t).abs() > 0.;
                t = candidate;
                break;
            }
            scale *= 0.5;
        }
        if !accepted || (scale * step).abs() <= 1e-14 * (domain[1] - domain[0]).abs().max(1.) {
            break;
        }
    }
    Ok(t)
}

/// Iso-v curve of a degree_v == 1 cylindrical patch at parameter v: control
/// points and weights are the exact rational blend of the two v rows, so the
/// curve shares the target's u parameterization (the key to exact pcurves).
fn iso_v_curve(surface: &Surface, v: f64) -> Result<Curve> {
    let dv = knot_domain(&surface.knots_v, surface.degree_v);
    let s = (v - dv[0]) / (dv[1] - dv[0]);
    let mut control_points = Vec::with_capacity(surface.control_points.len());
    let mut weights = Vec::with_capacity(surface.control_points.len());
    for (i, row) in surface.control_points.iter().enumerate() {
        if row.len() != 2 {
            return Err(error(
                REPLACE_UNSUPPORTED,
                "Cylinder target must be degree 1 in v",
            ));
        }
        let (w0, w1) = (surface.weights[i][0], surface.weights[i][1]);
        let w = (1. - s) * w0 + s * w1;
        let p = add(
            mul([row[0][0], row[0][1], row[0][2]], (1. - s) * w0 / w),
            mul([row[1][0], row[1][1], row[1][2]], s * w1 / w),
        );
        control_points.push(p.to_vec());
        weights.push(w);
    }
    Ok(Curve {
        degree: surface.degree_u,
        knots: surface.knots_u.clone(),
        control_points,
        weights,
        periodic: surface.periodic_u,
    })
}

/// Target geometry admitted in this iteration.
enum TargetGeom {
    Plane(PlaneFrame),
    Cylinder(CylinderTarget),
}

struct CylinderTarget {
    axis: Axis,
    radius: f64,
    /// Axial coordinate of the two v rows of the target patch.
    axial: [f64; 2],
    domain_u: [f64; 2],
    domain_v: [f64; 2],
}

impl CylinderTarget {
    fn v_of(&self, p: [f64; 3]) -> f64 {
        let z = dot(sub(p, self.axis.point), self.axis.direction);
        self.domain_v[0]
            + (z - self.axial[0]) / (self.axial[1] - self.axial[0])
                * (self.domain_v[1] - self.domain_v[0])
    }
    fn in_domain(&self, uv: [f64; 2], eps: f64) -> bool {
        uv[0] >= self.domain_u[0] - eps
            && uv[0] <= self.domain_u[1] + eps
            && uv[1] >= self.domain_v[0] - eps
            && uv[1] <= self.domain_v[1] + eps
    }
    /// Exact UV of a point on the cylinder, with an out-of-domain residual
    /// check (the nearest-candidate clamp must never smuggle an outside
    /// projection back into the patch).
    fn uv(&self, surface: &Surface, p: [f64; 3], linear: f64) -> Result<[f64; 2]> {
        let v = self.v_of(p);
        let master = iso_v_curve(surface, v)?;
        let u = project_parameter(&master, p)?;
        let uv = [u, v];
        let q = surface.evaluate(u, v)?.point;
        if dist([q[0], q[1], q[2]], p) > linear {
            return Err(error(
                REPLACE_OUT_OF_DOMAIN,
                format!(
                    "Projection residual {} leaves the target patch (uv {uv:?}, point {p:?}, projected {q:?})",
                    dist([q[0], q[1], q[2]], p)
                ),
            ));
        }
        Ok(uv)
    }
}

fn cylinder_target(
    surface: &Surface,
    class: &crate::analysis::surface_classify::SurfaceClassification,
) -> Result<CylinderTarget> {
    if surface.degree_v != 1
        || surface.control_points.first().map_or(0, Vec::len) != 2
        || surface
            .control_points
            .iter()
            .flatten()
            .any(|p| p.len() != 3)
    {
        return Err(error(
            REPLACE_UNSUPPORTED,
            "Cylinder target must have two degree-1 axial control-point rows",
        ));
    }
    let axis = class.axis.ok_or_else(|| {
        error(
            REPLACE_UNSUPPORTED,
            "Cylinder classification has no fitted axis",
        )
    })?;
    let radius = class
        .radius
        .filter(|r| r.is_finite() && *r > 0.)
        .ok_or_else(|| {
            error(
                REPLACE_UNSUPPORTED,
                "Cylinder classification has no positive radius",
            )
        })?;
    let axial = [0, 1].map(|column| {
        let (weighted, weight) = surface.control_points.iter().enumerate().fold(
            (0., 0.),
            |(weighted, weight), (row, points)| {
                let w = surface.weights[row][column];
                let p = &points[column];
                (
                    weighted + dot([p[0], p[1], p[2]], axis.direction) * w,
                    weight + w,
                )
            },
        );
        weighted / weight - dot(axis.point, axis.direction)
    });
    if axial.iter().any(|z| !z.is_finite()) || (axial[1] - axial[0]).abs() <= 1e-12 {
        return Err(error(
            REPLACE_UNSUPPORTED,
            "Cylinder target has a degenerate axial span",
        ));
    }
    Ok(CylinderTarget {
        axis,
        radius,
        axial,
        domain_u: knot_domain(&surface.knots_u, surface.degree_u),
        domain_v: knot_domain(&surface.knots_v, surface.degree_v),
    })
}

/// Line ∩ infinite cylinder; both roots, nearest-to-old selection happens
/// at the call site (branch by proximity to the old geometry).
fn line_cylinder(
    axis: &Axis,
    radius: f64,
    lp: [f64; 3],
    ld: [f64; 3],
) -> Option<([f64; 3], [f64; 3])> {
    let k = axis.direction;
    let d = sub(lp, axis.point);
    let d_perp = sub(d, mul(k, dot(d, k)));
    let l_perp = sub(ld, mul(k, dot(ld, k)));
    let a = dot(l_perp, l_perp);
    if a <= 1e-18 {
        return None;
    }
    let b = 2. * dot(d_perp, l_perp);
    let c = dot(d_perp, d_perp) - radius * radius;
    let mut disc = b * b - 4. * a * c;
    // A tangent can acquire a few negative ulps from the independently fitted
    // radius. Clamp only that rounding-scale band; a genuine miss still fails.
    let rounding =
        (64. * f64::EPSILON * (b * b + (4. * a * c).abs()).max(1.)).max(4. * a * radius * 1e-10);
    if disc < -rounding {
        return None;
    }
    if disc < 0. {
        disc = 0.;
    }
    let root = disc.sqrt();
    let t0 = (-b - root) / (2. * a);
    let t1 = (-b + root) / (2. * a);
    Some((add(lp, mul(ld, t0)), add(lp, mul(ld, t1))))
}

/// Pcurve control points may sit this far outside the patch domain (in UV
/// units) before the projection is refused; they are clamped back inside.
const PCURVE_DOMAIN_SLACK: f64 = 1e-7;
/// Work limit for the exact curve/pcurve identity predicate.
const EXACT_AGREEMENT_WORK: u64 = 100_000;
/// Cell limit for the bounded curve/pcurve agreement certificate.
const AGREEMENT_CELLS: usize = 4096;
/// Number of intervals sampled for the surface residual of one edge.
const RESIDUAL_SAMPLES: usize = 8;
/// Neighbor planes closer to parallel than this have no slide line.
const SLIDE_LINE_PARALLEL: f64 = 1e-9;
/// Slide lines closer to parallel to the target plane are refused.
const SLIDE_PLANE_PARALLEL: f64 = 1e-12;
/// `|cos|` band classifying a neighbor plane as ⟂ or ∥ to the cylinder axis.
const AXIS_ALIGNMENT: f64 = 1e-9;
/// Relative slack for the old-edge midpoint lying on the new arc.
const ARC_CONTAINMENT_SLACK: f64 = 1e-6;

/// Replace `face`'s geometry by `target` while preserving topology
/// (checklist 874). See the module-level notes in `direct_edit.rs` for the
/// supported subset and the honest refusals.
pub fn replace_face(
    model: &Model,
    face: usize,
    target: &Surface,
    budget: &Budget,
) -> Result<ReplaceFaceReport> {
    let snapshot = ModelSnapshot::new(model.clone())?;
    let source = snapshot.model();
    if face >= source.faces.len() {
        return Err(error(
            REPLACE_INVALID,
            format!("Face {face} is out of range"),
        ));
    }
    target.validate()?;
    let mut guard = budget.guard("replace-face");
    guard.check()?;
    let geom = replace_target_geom(source, face, target, budget)?;
    let mut replacement = FaceReplacement {
        source,
        face,
        target,
        geom: &geom,
        linear: source.tolerance_mm * 8.,
        guard: &mut guard,
    };
    let mut neighbors = replacement.neighbors()?;
    let substitute = replacement.retrim_vertices(&mut neighbors)?;
    let (affected, new_curves) = replacement.reauthor_edges(&neighbors, &substitute)?;

    // Staged mutation; the input model is never touched.
    let mut staged = source.clone();
    for (&v, &p) in &substitute {
        staged.vertices[v].point = p;
    }
    for (&e, curve) in &new_curves {
        staged.edges[e].curve = curve.clone();
    }
    staged.faces[face].surface = target.clone();
    let tally = replacement.recompute_pcurves(&neighbors, &affected, &new_curves, &mut staged)?;

    staged.validate()?;
    let audit = crate::solid_audit::audit_solid(&staged)?;
    let (volume_before, volume_after) = (volume_of(source)?, volume_of(&staged)?);
    let incremental = complete_incremental_rebuild(source, staged, &[face], budget)?;
    let (dirty_faces, rebuilt_faces, reused_faces) = (
        incremental.dirty.len(),
        incremental.rebuilt_faces,
        incremental.reused_faces,
    );
    guard.check()?;
    Ok(ReplaceFaceReport {
        model: incremental.model,
        face,
        target_kind: geom.kind(),
        retrimmed_vertices: substitute.len(),
        reauthored_edges: new_curves.len(),
        recomputed_pcurves: tally.recomputed,
        dirty_faces,
        rebuilt_faces,
        reused_faces,
        correspondences: tally.correspondences,
        max_surface_residual_mm: tally.max_residual,
        volume_before_mm3: volume_before,
        volume_after_mm3: volume_after,
        delta_volume_actual_mm3: volume_after - volume_before,
        audit,
    })
}

impl TargetGeom {
    fn kind(&self) -> &'static str {
        match self {
            TargetGeom::Plane(_) => "plane",
            TargetGeom::Cylinder(_) => "cylinder",
        }
    }
}

/// Admit the source face class and classify the target into its exact
/// geometry.
fn replace_target_geom(
    source: &Model,
    face: usize,
    target: &Surface,
    budget: &Budget,
) -> Result<TargetGeom> {
    let source_class = classify_surface(
        &source.faces[face].surface,
        source.tolerance_mm.max(1e-7),
        budget,
    )?;
    if !matches!(
        source_class.class,
        SurfaceClass::Plane | SurfaceClass::Freeform
    ) {
        return Err(error(
            REPLACE_UNSUPPORTED,
            format!(
                "Face {face} is {:?}; replace-face/1 accepts planar source faces and freeform source faces",
                source_class.class
            ),
        ));
    }
    let class = classify_surface(target, source.tolerance_mm.max(1e-7), budget)?;
    match class.class {
        SurfaceClass::Plane => {
            let frame = plane_frame(target).ok_or_else(|| {
                error(
                    REPLACE_UNSUPPORTED,
                    "Plane target must be a bilinear patch with an explicit UV frame",
                )
            })?;
            Ok(TargetGeom::Plane(frame))
        }
        SurfaceClass::Cylinder => Ok(TargetGeom::Cylinder(cylinder_target(target, &class)?)),
        other => Err(error(
            REPLACE_UNSUPPORTED,
            format!("Target is {other:?}; replace-face/1 admits plane and cylinder targets"),
        )),
    }
}

/// Loop structure of the replaced face and the (single, planar) neighbor
/// per loop edge.
struct ReplaceNeighbors {
    face_edges: BTreeSet<usize>,
    face_vertices: BTreeSet<usize>,
    /// Plane cache of every neighbor face consulted so far.
    planes: BTreeMap<usize, PlaneEq>,
    neighbor_of: BTreeMap<usize, usize>,
    frames: BTreeMap<usize, PlaneFrame>,
}

impl ReplaceNeighbors {
    fn plane(&mut self, model: &Model, f: usize) -> Result<PlaneEq> {
        if let Some(p) = self.planes.get(&f) {
            return Ok(*p);
        }
        let p = face_plane_fit(model, f).map_err(|_| {
            error(
                REPLACE_UNSUPPORTED,
                format!("Neighbor face {f} is not planar; replace-face/1 needs planar neighbors"),
            )
        })?;
        self.planes.insert(f, p);
        Ok(p)
    }

    fn cached_plane(&self, f: usize) -> Result<PlaneEq> {
        self.planes.get(&f).copied().ok_or_else(|| {
            error(
                REPLACE_INVALID,
                format!("Neighbor face {f} has no fitted plane"),
            )
        })
    }
}

/// Pcurve bookkeeping accumulated over every re-mapped coedge.
struct PcurveTally {
    recomputed: usize,
    correspondences: Vec<EdgeCorrespondenceCertificate>,
    max_residual: f64,
}

/// One face replacement in flight: the source model, the replaced face, the
/// target and its classified geometry.
struct FaceReplacement<'a> {
    source: &'a Model,
    face: usize,
    target: &'a Surface,
    geom: &'a TargetGeom,
    linear: f64,
    guard: &'a mut BudgetGuard,
}

impl FaceReplacement<'_> {
    fn neighbors(&mut self) -> Result<ReplaceNeighbors> {
        let source = self.source;
        let face = self.face;
        let adjacency = edge_face_adjacency(source, self.guard)?;
        let mut neighbors = ReplaceNeighbors {
            face_edges: BTreeSet::new(),
            face_vertices: BTreeSet::new(),
            planes: BTreeMap::new(),
            neighbor_of: BTreeMap::new(),
            frames: BTreeMap::new(),
        };
        for &ring in std::iter::once(&source.faces[face].outer).chain(&source.faces[face].holes) {
            for coedge in &source.loops[ring].coedges {
                neighbors.face_edges.insert(coedge.edge);
                neighbors
                    .face_vertices
                    .extend(source.edges[coedge.edge].vertices);
            }
        }
        let face_edges = neighbors.face_edges.clone();
        for e in face_edges {
            self.guard.tick()?;
            let mut other_iter = adjacency[e].iter().copied().filter(|&f| f != face);
            let (Some(n), None) = (other_iter.next(), other_iter.next()) else {
                let count = adjacency[e].iter().filter(|&&f| f != face).count();
                return Err(error(
                    REPLACE_UNSUPPORTED,
                    format!("Edge {e} has {count} neighboring faces; exactly one is supported"),
                ));
            };
            neighbors.neighbor_of.insert(e, n);
            neighbors.plane(source, n)?;
            let Some(frame) = plane_frame(&source.faces[n].surface) else {
                return Err(error(
                    REPLACE_UNSUPPORTED,
                    format!(
                        "Neighbor face {n} is not a bilinear planar patch; pcurve re-map is out of scope"
                    ),
                ));
            };
            neighbors.frames.insert(n, frame);
        }
        Ok(neighbors)
    }

    /// Re-trim vertices: slide line (the two neighbor planes) ∩ target,
    /// nearest branch to the old position.
    fn retrim_vertices(
        &mut self,
        neighbors: &mut ReplaceNeighbors,
    ) -> Result<BTreeMap<usize, [f64; 3]>> {
        let source = self.source;
        let face = self.face;
        let incidence = vertex_face_incidence(source);
        let mut substitute: BTreeMap<usize, [f64; 3]> = BTreeMap::new();
        let face_vertices = neighbors.face_vertices.clone();
        for v in face_vertices {
            self.guard.tick()?;
            let mut non_iter = incidence[v].iter().copied().filter(|&f| f != face);
            let (Some(n1), Some(n2), None) = (non_iter.next(), non_iter.next(), non_iter.next())
            else {
                let count = incidence[v].iter().filter(|&&f| f != face).count();
                return Err(error(
                    REPLACE_UNSUPPORTED,
                    format!("Vertex {v} touches {count} other faces; exactly two are supported"),
                ));
            };
            let p1 = neighbors.plane(source, n1)?;
            let p2 = neighbors.plane(source, n2)?;
            let Some((lp, ld)) = intersect_planes(p1, p2, SLIDE_LINE_PARALLEL) else {
                return Err(error(
                    REPLACE_UNSUPPORTED,
                    format!(
                        "Vertex {v}: neighbor planes are parallel; the slide line is undefined"
                    ),
                ));
            };
            let moved = self.slide_onto_target(v, lp, ld)?;
            if moved.iter().any(|c| !c.is_finite()) {
                return Err(error(
                    REPLACE_UNSUPPORTED,
                    format!("Vertex {v} re-trim produced a non-finite point"),
                ));
            }
            substitute.insert(v, moved);
        }
        Ok(substitute)
    }

    fn slide_onto_target(&self, v: usize, lp: [f64; 3], ld: [f64; 3]) -> Result<[f64; 3]> {
        let old = self.source.vertices[v].point;
        match self.geom {
            TargetGeom::Plane(frame) => line_plane(lp, ld, frame.equation(), SLIDE_PLANE_PARALLEL)
                .ok_or_else(|| {
                    error(
                        REPLACE_UNSUPPORTED,
                        format!("Vertex {v}: the slide line is parallel to the target plane"),
                    )
                }),
            TargetGeom::Cylinder(cyl) => {
                let Some((a, b)) = line_cylinder(&cyl.axis, cyl.radius, lp, ld) else {
                    return Err(error(
                        REPLACE_UNSUPPORTED,
                        format!("Vertex {v}: the slide line misses the target cylinder"),
                    ));
                };
                Ok(if dist(a, old) <= dist(b, old) { a } else { b })
            }
        }
    }

    /// Re-author every edge touched by a moved vertex: loop edges become
    /// target ∩ neighbor-plane sections, slide edges become lines between
    /// their (possibly moved) endpoints.
    fn reauthor_edges(
        &mut self,
        neighbors: &ReplaceNeighbors,
        substitute: &BTreeMap<usize, [f64; 3]>,
    ) -> Result<(BTreeSet<usize>, BTreeMap<usize, Curve>)> {
        let source = self.source;
        let affected: BTreeSet<usize> = source
            .edges
            .iter()
            .enumerate()
            .filter(|(_, edge)| edge.vertices.iter().any(|v| substitute.contains_key(v)))
            .map(|(e, _)| e)
            .collect();
        let mut new_curves: BTreeMap<usize, Curve> = BTreeMap::new();
        for &e in &affected {
            self.guard.tick()?;
            let edge = &source.edges[e];
            if edge.degenerate {
                return Err(error(
                    REPLACE_UNSUPPORTED,
                    format!("Edge {e} is degenerate; replace-face/1 does not re-trim pole edges"),
                ));
            }
            let at = |i: usize| {
                let v = edge.vertices[i];
                substitute
                    .get(&v)
                    .copied()
                    .unwrap_or(source.vertices[v].point)
            };
            let (a, b) = (at(0), at(1));
            let curve = if neighbors.face_edges.contains(&e) {
                self.loop_edge_curve(neighbors, e, a, b)?
            } else {
                if edge.curve.degree != 1 {
                    return Err(error(
                        REPLACE_UNSUPPORTED,
                        format!(
                            "Edge {e} is a curved slide edge; only straight slide edges are supported"
                        ),
                    ));
                }
                crate::line(a.to_vec(), b.to_vec())
            };
            new_curves.insert(e, curve);
        }
        Ok((affected, new_curves))
    }

    fn loop_edge_curve(
        &self,
        neighbors: &ReplaceNeighbors,
        e: usize,
        a: [f64; 3],
        b: [f64; 3],
    ) -> Result<Curve> {
        let n = *neighbors.neighbor_of.get(&e).ok_or_else(|| {
            error(
                REPLACE_INVALID,
                format!("Edge {e} has no recorded neighbor face"),
            )
        })?;
        match self.geom {
            TargetGeom::Plane(_) => Ok(crate::line(a.to_vec(), b.to_vec())),
            TargetGeom::Cylinder(cyl) => {
                let n_plane = neighbors.cached_plane(n)?;
                let align = dot(cyl.axis.direction, n_plane.normal).abs();
                if align >= 1. - AXIS_ALIGNMENT {
                    self.iso_arc(cyl, e, a, b)
                } else if align <= AXIS_ALIGNMENT {
                    // Plane ∥ axis: exact ruling.
                    Ok(crate::line(a.to_vec(), b.to_vec()))
                } else {
                    Err(error(
                        REPLACE_UNSUPPORTED,
                        format!(
                            "Edge {e}: oblique plane/cylinder section has no exact low-degree UV lift (see intersections::plane_cylinder); refused"
                        ),
                    ))
                }
            }
        }
    }

    /// Plane ⟂ axis: exact iso-v circle arc, trimmed out of the target's own
    /// iso-curve (exact pcurves follow).
    fn iso_arc(&self, cyl: &CylinderTarget, e: usize, a: [f64; 3], b: [f64; 3]) -> Result<Curve> {
        let edge = &self.source.edges[e];
        let v = cyl.v_of(a);
        let master = iso_v_curve(self.target, v)?;
        let ta = project_parameter(&master, a)?;
        let tb = project_parameter(&master, b)?;
        // Branch continuity with the old edge: the arc must contain the
        // projection of the old edge midpoint.
        let domain = edge.curve.domain();
        let mid = edge.curve.evaluate(0.5 * (domain[0] + domain[1]))?.point;
        let tm = project_parameter(&master, [mid[0], mid[1], mid[2]])?;
        let (lo, hi) = (ta.min(tb), ta.max(tb));
        let span = (master.domain()[1] - master.domain()[0]).abs().max(1.);
        if tm < lo - ARC_CONTAINMENT_SLACK * span || tm > hi + ARC_CONTAINMENT_SLACK * span {
            return Err(error(
                REPLACE_OUT_OF_DOMAIN,
                format!(
                    "Edge {e}: the required arc leaves the target patch (seam wrap is out of scope)"
                ),
            ));
        }
        let mut curve = master.trim(lo, hi)?;
        if ta > tb {
            curve = curve.reverse()?;
        }
        Ok(curve)
    }

    /// Pcurves on the target and on the planar neighbors; out-of-domain
    /// projections fail with the edge id before any validation attempt.
    fn recompute_pcurves(
        &mut self,
        neighbors: &ReplaceNeighbors,
        affected: &BTreeSet<usize>,
        new_curves: &BTreeMap<usize, Curve>,
        staged: &mut Model,
    ) -> Result<PcurveTally> {
        let source = self.source;
        let mut tally = PcurveTally {
            recomputed: 0,
            correspondences: Vec::new(),
            max_residual: 0.,
        };
        let target_sampler = SurfaceRefSampler::new(self.target)?;
        for (f, mface) in source.faces.iter().enumerate() {
            for &ring in std::iter::once(&mface.outer).chain(&mface.holes) {
                for (ci, coedge) in source.loops[ring].coedges.iter().enumerate() {
                    if !affected.contains(&coedge.edge) {
                        continue;
                    }
                    self.guard.tick()?;
                    let e = coedge.edge;
                    let mut curve = new_curves.get(&e).cloned().ok_or_else(|| {
                        error(
                            REPLACE_INVALID,
                            format!("Edge {e} has no re-authored curve"),
                        )
                    })?;
                    if coedge.reversed {
                        curve = curve.reverse()?;
                    }
                    let pcurve = if f == self.face {
                        self.target_pcurve(&curve, e)?
                    } else {
                        let frame = neighbors.frames.get(&f).ok_or_else(|| {
                            error(
                                REPLACE_UNSUPPORTED,
                                format!(
                                    "Face {f} shares a moved edge but has no bilinear UV frame"
                                ),
                            )
                        })?;
                        frame_pcurve(&curve, frame, |uv| {
                            format!(
                                "Edge {e} projection leaves neighbor face {f} patch domain (uv {uv:?})"
                            )
                        })?
                    };
                    let edited = &mut staged.loops[ring].coedges[ci];
                    *edited =
                        brep_topology::Coedge::new(edited.edge, edited.reversed, pcurve.clone());
                    tally.recomputed += 1;
                    if f == self.face {
                        let certificate = self.certify_edge(
                            staged,
                            e,
                            &pcurve,
                            coedge.reversed,
                            curve.domain(),
                            &target_sampler,
                        )?;
                        tally.max_residual = tally.max_residual.max(certificate.max_residual_mm);
                        tally.correspondences.push(certificate);
                    }
                }
            }
        }
        Ok(tally)
    }

    /// Pcurve of a re-authored edge on the target surface.
    fn target_pcurve(&self, curve: &Curve, e: usize) -> Result<Curve> {
        match self.geom {
            TargetGeom::Plane(frame) => frame_pcurve(curve, frame, |uv| {
                format!("Edge {e} projection leaves the target plane patch domain (uv {uv:?})")
            }),
            TargetGeom::Cylinder(cyl) => {
                let domain = curve.domain();
                let (p0, p1) = (
                    curve.evaluate(domain[0])?.point,
                    curve.evaluate(domain[1])?.point,
                );
                let lift = |p: &[f64]| {
                    cyl.uv(self.target, [p[0], p[1], p[2]], self.linear)
                        .map_err(|pr| {
                            error(
                                REPLACE_OUT_OF_DOMAIN,
                                format!(
                                    "Edge {e} projection leaves the target cylinder patch domain: {}",
                                    pr.message
                                ),
                            )
                        })
                };
                let uv0 = lift(&p0)?;
                let uv1 = lift(&p1)?;
                if !cyl.in_domain(uv0, PCURVE_DOMAIN_SLACK)
                    || !cyl.in_domain(uv1, PCURVE_DOMAIN_SLACK)
                {
                    return Err(error(
                        REPLACE_OUT_OF_DOMAIN,
                        format!(
                            "Edge {e} projection leaves the target cylinder patch domain (uv {uv0:?}..{uv1:?})"
                        ),
                    ));
                }
                let clamp = |uv: [f64; 2]| {
                    [
                        uv[0].clamp(cyl.domain_u[0], cyl.domain_u[1]),
                        uv[1].clamp(cyl.domain_v[0], cyl.domain_v[1]),
                    ]
                };
                Ok(uv_line(clamp(uv0), clamp(uv1), domain))
            }
        }
    }

    /// Certify the curve/pcurve correspondence of one replaced-face edge
    /// (exact identity first, bounded agreement otherwise) and measure its
    /// sampled surface residual.
    fn certify_edge(
        &self,
        staged: &Model,
        e: usize,
        pcurve: &Curve,
        reversed: bool,
        domain: [f64; 2],
        target_sampler: &SurfaceRefSampler,
    ) -> Result<EdgeCorrespondenceCertificate> {
        let edge_curve = &staged.edges[e].curve;
        let exact = nurbs_core::curve_surface_agreement::verify_exact(
            edge_curve,
            pcurve,
            self.target,
            reversed,
            EXACT_AGREEMENT_WORK,
        )?;
        let exact_equal = exact
            .as_ref()
            .is_some_and(|d| matches!(d.outcome, cad_predicates::BezierIdentity::Equal));
        let (is_exact, cells) = if exact_equal {
            (true, exact.as_ref().map_or(0, |d| d.work_used as usize))
        } else {
            let agreement = nurbs_core::curve_surface_agreement::verify(
                edge_curve,
                pcurve,
                self.target,
                reversed,
                self.linear,
                AGREEMENT_CELLS,
            )?;
            if agreement.status != nurbs_core::curve_surface_agreement::Status::WithinTolerance {
                return Err(error(
                    REPLACE_UNSUPPORTED,
                    format!(
                        "Edge {e} has no certified curve/pcurve correspondence ({:?}, witness {:?})",
                        agreement.status, agreement.witness
                    ),
                ));
            }
            (false, agreement.cells)
        };
        let pcurve_sampler = CurveSampler::new(pcurve)?;
        let edge_sampler = CurveSampler::new(edge_curve)?;
        let mut edge_residual = 0f64;
        for i in 0..=RESIDUAL_SAMPLES {
            let frac = i as f64 / RESIDUAL_SAMPLES as f64;
            let t = domain[0] + frac * (domain[1] - domain[0]);
            let uv = pcurve_sampler.point2(t)?;
            let sp = target_sampler.point(uv[0], uv[1])?;
            let te =
                domain[0] + (if reversed { 1. - frac } else { frac }) * (domain[1] - domain[0]);
            let q = edge_sampler.point3(te)?;
            edge_residual = edge_residual.max(dist(sp, q));
        }
        Ok(EdgeCorrespondenceCertificate {
            edge: e,
            exact: is_exact,
            agreement_cells: cells,
            max_residual_mm: edge_residual,
        })
    }
}

/// Map a 3D curve into a bilinear plane patch's UV frame, refusing control
/// points outside the patch domain (with the caller's message) and clamping
/// the rest into it.
fn frame_pcurve(
    curve: &Curve,
    frame: &PlaneFrame,
    out_of_domain: impl Fn([f64; 2]) -> String,
) -> Result<Curve> {
    let mut mapped = curve_to_uv(
        curve,
        frame.origin,
        frame.u,
        frame.v,
        frame.len_u,
        frame.len_v,
    )?;
    for p in &mut mapped.control_points {
        let uv = frame.scale_uv([p[0], p[1]]);
        if !frame.in_domain(uv, PCURVE_DOMAIN_SLACK) {
            return Err(error(REPLACE_OUT_OF_DOMAIN, out_of_domain(uv)));
        }
        *p = [
            uv[0].clamp(frame.domain_u[0], frame.domain_u[1]),
            uv[1].clamp(frame.domain_v[0], frame.domain_v[1]),
        ]
        .to_vec();
    }
    Ok(mapped)
}
