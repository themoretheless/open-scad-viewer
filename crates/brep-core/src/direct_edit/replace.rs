use super::*;
use crate::analysis::surface_classify::classify_surface;
use nurbs_core::curve::Curve;
use nurbs_core::foundation::project_curve_report;
use nurbs_core::surface::{Surface, SurfaceSampler};

/// Outcome of one successful face replacement.
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
        uv[0] >= self.domain_u[0] - eps && uv[0] <= self.domain_u[1] + eps
            && uv[1] >= self.domain_v[0] - eps && uv[1] <= self.domain_v[1] + eps
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
        .ok_or_else(|| error(REPLACE_UNSUPPORTED, "Curve projection produced no candidate"))?;
    let domain = curve.domain();
    let mut t = (0.5 * (best.parameter_interval[0] + best.parameter_interval[1]))
        .clamp(domain[0], domain[1]);
    let evaluated = curve.evaluate(t)?.point;
    let delta = sub([evaluated[0], evaluated[1], evaluated[2]], point);
    let mut best_distance2 = dot(delta, delta);
    // The certified intervals isolate stationary roots, but an interval's
    // midpoint is not necessarily its closest point. Add deterministic seeds
    // and choose by measured distance before local refinement.
    for i in 0..=64 {
        let candidate = domain[0] + (domain[1] - domain[0]) * i as f64 / 64.;
        let q = curve.evaluate(candidate)?.point;
        let delta = sub([q[0], q[1], q[2]], point);
        let distance2 = dot(delta, delta);
        if distance2 < best_distance2 {
            t = candidate;
            best_distance2 = distance2;
        }
    }
    for _ in 0..12 {
        let ev = curve.evaluate(t)?;
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
                let a = curve.evaluate(ta)?.point;
                let b = curve.evaluate(tb)?.point;
                mul(sub([b[0], b[1], b[2]], [a[0], a[1], a[2]]), 1. / (tb - ta))
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
            let q = curve.evaluate(candidate)?.point;
            let delta = sub([q[0], q[1], q[2]], point);
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
    #[allow(dead_code)]
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
        uv[0] >= self.domain_u[0] - eps && uv[0] <= self.domain_u[1] + eps
            && uv[1] >= self.domain_v[0] - eps && uv[1] <= self.domain_v[1] + eps
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
                format!("Projection residual {} leaves the target patch (uv {uv:?}, point {p:?}, projected {q:?})", dist([q[0], q[1], q[2]], p)),
            ));
        }
        Ok(uv)
    }
}

/// Line ∩ infinite cylinder; both roots, nearest-to-old selection happens
/// at the call site (branch by proximity to the old geometry).
fn line_cylinder(axis: &Axis, radius: f64, lp: [f64; 3], ld: [f64; 3]) -> Option<([f64; 3], [f64; 3])> {
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
    let disc = b * b - 4. * a * c;
    if disc < 0. {
        return None;
    }
    let root = disc.sqrt();
    let t0 = (-b - root) / (2. * a);
    let t1 = (-b + root) / (2. * a);
    Some((add(lp, mul(ld, t0)), add(lp, mul(ld, t1))))
}

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
        return Err(error(REPLACE_INVALID, format!("Face {face} is out of range")));
    }
    target.validate()?;
    let linear = source.tolerance_mm * 8.;
    let mut guard = budget.guard("replace-face");
    guard.check()?;

    let source_class = classify_surface(
        &source.faces[face].surface,
        source.tolerance_mm.max(1e-7),
        budget,
    )?;
    if !matches!(source_class.class, SurfaceClass::Plane | SurfaceClass::Freeform) {
        return Err(error(
            REPLACE_UNSUPPORTED,
            format!("Face {face} is {:?}; replace-face/1 accepts planar source faces and freeform source faces", source_class.class),
        ));
    }
    let class = classify_surface(target, source.tolerance_mm.max(1e-7), budget)?;
    let geom = match class.class {
        SurfaceClass::Plane => {
            let frame = plane_frame(target).ok_or_else(|| {
                error(
                    REPLACE_UNSUPPORTED,
                    "Plane target must be a bilinear patch with an explicit UV frame",
                )
            })?;
            TargetGeom::Plane(frame)
        }
        SurfaceClass::Cylinder => {
            return Err(error(
                REPLACE_UNSUPPORTED,
                "Cylinder targets are deferred: replace-face/1 currently accepts only bilinear plane targets",
            ));
        }
        other => {
            return Err(error(
                REPLACE_UNSUPPORTED,
                format!("Target is {other:?}; replace-face/1 admits plane and cylinder targets"),
            ));
        }
    };

    // Loop structure of the replaced face and the (single, planar) neighbor
    // per loop edge.
    let adjacency = edge_face_adjacency(source, &mut guard)?;
    let incidence = vertex_face_incidence(source);
    let mut face_edges = BTreeSet::new();
    let mut face_vertices = BTreeSet::new();
    for &ring in std::iter::once(&source.faces[face].outer).chain(&source.faces[face].holes) {
        for coedge in &source.loops[ring].coedges {
            face_edges.insert(coedge.edge);
            for &v in &source.edges[coedge.edge].vertices {
                face_vertices.insert(v);
            }
        }
    }
    let mut planes: BTreeMap<usize, PlaneEq> = BTreeMap::new();
    let mut plane_of = |m: &Model, f: usize, planes: &mut BTreeMap<usize, PlaneEq>| -> Result<PlaneEq> {
        if let Some(p) = planes.get(&f) {
            return Ok(*p);
        }
        let p = face_plane_fit(m, f).map_err(|_| {
            error(
                REPLACE_UNSUPPORTED,
                format!("Neighbor face {f} is not planar; replace-face/1 needs planar neighbors"),
            )
        })?;
        planes.insert(f, p);
        Ok(p)
    };
    let mut neighbor_of: BTreeMap<usize, usize> = BTreeMap::new();
    let mut frames: BTreeMap<usize, PlaneFrame> = BTreeMap::new();
    for &e in &face_edges {
        guard.tick()?;
        let others: Vec<usize> = adjacency[e].iter().copied().filter(|&f| f != face).collect();
        if others.len() != 1 {
            return Err(error(
                REPLACE_UNSUPPORTED,
                format!("Edge {e} has {} neighboring faces; exactly one is supported", others.len()),
            ));
        }
        let n = others[0];
        neighbor_of.insert(e, n);
        plane_of(source, n, &mut planes)?;
        if let Some(frame) = plane_frame(&source.faces[n].surface) {
            frames.insert(n, frame);
        } else {
            return Err(error(
                REPLACE_UNSUPPORTED,
                format!("Neighbor face {n} is not a bilinear planar patch; pcurve re-map is out of scope"),
            ));
        }
    }

    // Re-trim vertices: slide line (the two neighbor planes) ∩ target,
    // nearest branch to the old position.
    let mut substitute: BTreeMap<usize, [f64; 3]> = BTreeMap::new();
    for &v in &face_vertices {
        guard.tick()?;
        let non: Vec<usize> = incidence[v].iter().copied().filter(|&f| f != face).collect();
        if non.len() != 2 {
            return Err(error(
                REPLACE_UNSUPPORTED,
                format!("Vertex {v} touches {} other faces; exactly two are supported", non.len()),
            ));
        }
        let p1 = plane_of(source, non[0], &mut planes)?;
        let p2 = plane_of(source, non[1], &mut planes)?;
        let Some((lp, ld)) = intersect_planes(p1, p2, 1e-9) else {
            return Err(error(
                REPLACE_UNSUPPORTED,
                format!("Vertex {v}: neighbor planes are parallel; the slide line is undefined"),
            ));
        };
        let old = source.vertices[v].point;
        let moved = match &geom {
            TargetGeom::Plane(frame) => line_plane(lp, ld, frame.equation(), 1e-12).ok_or_else(|| {
                error(
                    REPLACE_UNSUPPORTED,
                    format!("Vertex {v}: the slide line is parallel to the target plane"),
                )
            })?,
            TargetGeom::Cylinder(cyl) => {
                let Some((a, b)) = line_cylinder(&cyl.axis, cyl.radius, lp, ld) else {
                    return Err(error(
                        REPLACE_UNSUPPORTED,
                        format!("Vertex {v}: the slide line misses the target cylinder"),
                    ));
                };
                if dist(a, old) <= dist(b, old) { a } else { b }
            }
        };
        if moved.iter().any(|c| !c.is_finite()) {
            return Err(error(
                REPLACE_UNSUPPORTED,
                format!("Vertex {v} re-trim produced a non-finite point"),
            ));
        }
        substitute.insert(v, moved);
    }

    // Re-author every edge touched by a moved vertex: loop edges become
    // target ∩ neighbor-plane sections, slide edges become lines between
    // their (possibly moved) endpoints.
    let mut affected = BTreeSet::new();
    for (e, edge) in source.edges.iter().enumerate() {
        if edge.vertices.iter().any(|v| substitute.contains_key(v)) {
            affected.insert(e);
        }
    }
    let mut new_curves: BTreeMap<usize, Curve> = BTreeMap::new();
    for &e in &affected {
        guard.tick()?;
        let edge = &source.edges[e];
        if edge.degenerate {
            return Err(error(
                REPLACE_UNSUPPORTED,
                format!("Edge {e} is degenerate; replace-face/1 does not re-trim pole edges"),
            ));
        }
        let at = |i: usize| {
            let v = edge.vertices[i];
            substitute.get(&v).copied().unwrap_or(source.vertices[v].point)
        };
        let (a, b) = (at(0), at(1));
        let curve = if face_edges.contains(&e) {
            let n = neighbor_of[&e];
            match &geom {
                TargetGeom::Plane(_) => crate::line(a.to_vec(), b.to_vec()),
                TargetGeom::Cylinder(cyl) => {
                    let n_plane = planes[&n];
                    let align = dot(cyl.axis.direction, n_plane.normal).abs();
                    if align >= 1. - 1e-9 {
                        // Plane ⟂ axis: exact iso-v circle arc, trimmed out of
                        // the target's own iso-curve (exact pcurves follow).
                        let v = cyl.v_of(a);
                        let master = iso_v_curve(target, v)?;
                        let ta = project_parameter(&master, a)?;
                        let tb = project_parameter(&master, b)?;
                        // Branch continuity with the old edge: the arc must
                        // contain the projection of the old edge midpoint.
                        let domain = edge.curve.domain();
                        let mid = edge.curve.evaluate(0.5 * (domain[0] + domain[1]))?.point;
                        let tm = project_parameter(&master, [mid[0], mid[1], mid[2]])?;
                        let (lo, hi) = (ta.min(tb), ta.max(tb));
                        let span = (master.domain()[1] - master.domain()[0]).abs().max(1.);
                        if tm < lo - 1e-6 * span || tm > hi + 1e-6 * span {
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
                        curve
                    } else if align <= 1e-9 {
                        // Plane ∥ axis: exact ruling.
                        crate::line(a.to_vec(), b.to_vec())
                    } else {
                        return Err(error(
                            REPLACE_UNSUPPORTED,
                            format!(
                                "Edge {e}: oblique plane/cylinder section has no exact low-degree UV lift (see intersections::plane_cylinder); refused"
                            ),
                        ));
                    }
                }
            }
        } else {
            if edge.curve.degree != 1 {
                return Err(error(
                    REPLACE_UNSUPPORTED,
                    format!("Edge {e} is a curved slide edge; only straight slide edges are supported"),
                ));
            }
            crate::line(a.to_vec(), b.to_vec())
        };
        new_curves.insert(e, curve);
    }

    // Staged mutation; the input model is never touched.
    let mut staged = source.clone();
    for (&v, &p) in &substitute {
        staged.vertices[v].point = p;
    }
    for (&e, curve) in &new_curves {
        staged.edges[e].curve = curve.clone();
    }
    staged.faces[face].surface = target.clone();

    // Pcurves on the target and on the planar neighbors; out-of-domain
    // projections fail with the edge id before any validation attempt.
    let mut recomputed_pcurves = 0usize;
    let mut max_residual = 0f64;
    let target_sampler = SurfaceSampler::new(target)?;
    for (f, mface) in source.faces.iter().enumerate() {
        for &ring in std::iter::once(&mface.outer).chain(&mface.holes) {
            for ci in 0..source.loops[ring].coedges.len() {
                let coedge = source.loops[ring].coedges[ci].clone();
                if !affected.contains(&coedge.edge) {
                    continue;
                }
                guard.tick()?;
                let e = coedge.edge;
                let mut curve = new_curves[&e].clone();
                if coedge.reversed {
                    curve = curve.reverse()?;
                }
                let domain = curve.domain();
                let pcurve = if f == face {
                    match &geom {
                        TargetGeom::Plane(frame) => {
                            let mut mapped = curve_to_uv(
                                &curve, frame.origin, frame.u, frame.v, frame.len_u, frame.len_v,
                            )?;
                            for p in &mut mapped.control_points {
                                let uv = frame.scale_uv([p[0], p[1]]);
                                if !frame.in_domain(uv, 1e-7) {
                                    return Err(error(
                                        REPLACE_OUT_OF_DOMAIN,
                                        format!("Edge {e} projection leaves the target plane patch domain (uv {uv:?})"),
                                    ));
                                }
                                *p = uv.to_vec();
                            }
                            mapped
                        }
                        TargetGeom::Cylinder(cyl) => {
                            let p0 = curve.evaluate(domain[0])?.point;
                            let p1 = curve.evaluate(domain[1])?.point;
                            let uv0 = cyl.uv(target, [p0[0], p0[1], p0[2]], linear).map_err(|projection| {
                                error(
                                    REPLACE_OUT_OF_DOMAIN,
                                    format!("Edge {e} projection leaves the target cylinder patch domain: {}", projection.message),
                                )
                            })?;
                            let uv1 = cyl.uv(target, [p1[0], p1[1], p1[2]], linear).map_err(|projection| {
                                error(
                                    REPLACE_OUT_OF_DOMAIN,
                                    format!("Edge {e} projection leaves the target cylinder patch domain: {}", projection.message),
                                )
                            })?;
                            if !cyl.in_domain(uv0, 1e-7) || !cyl.in_domain(uv1, 1e-7) {
                                return Err(error(
                                    REPLACE_OUT_OF_DOMAIN,
                                    format!("Edge {e} projection leaves the target cylinder patch domain (uv {uv0:?}..{uv1:?})"),
                                ));
                            }
                            uv_line(uv0, uv1, domain)
                        }
                    }
                } else {
                    let frame = frames.get(&f).ok_or_else(|| {
                        error(
                            REPLACE_UNSUPPORTED,
                            format!("Face {f} shares a moved edge but has no bilinear UV frame"),
                        )
                    })?;
                    let mut mapped = curve_to_uv(
                        &curve, frame.origin, frame.u, frame.v, frame.len_u, frame.len_v,
                    )?;
                    for p in &mut mapped.control_points {
                        *p = frame.scale_uv([p[0], p[1]]).to_vec();
                    }
                    mapped
                };
                let edited = &mut staged.loops[ring].coedges[ci];
                edited.pcurve = pcurve.clone();
                *edited = brep_topology::Coedge::new(edited.edge, edited.reversed, pcurve.clone());
                recomputed_pcurves += 1;
                if f == face {
                    for i in 0..=8 {
                        let frac = i as f64 / 8.;
                        let t = domain[0] + frac * (domain[1] - domain[0]);
                        let uv = pcurve.evaluate(t)?.point;
                        let sp = target_sampler.evaluate(uv[0], uv[1])?.point;
                        let te = domain[0]
                            + if coedge.reversed { 1. - frac } else { frac }
                                * (domain[1] - domain[0]);
                        let q = staged.edges[e].curve.evaluate(te)?.point;
                        max_residual = max_residual.max(dist(
                            [sp[0], sp[1], sp[2]],
                            [q[0], q[1], q[2]],
                        ));
                    }
                }
            }
        }
    }

    staged.validate()?;
    let audit = crate::solid_audit::audit_solid(&staged)?;
    let volume_before = volume_of(source)?;
    let volume_after = volume_of(&staged)?;
    let dirty_faces = dirty_region_for_result(source, &staged, &[face], budget)?.len();
    validate_staged(source, &staged, &[face], budget)?;
    let rebuilt_faces = rebuilt_faces_in_place(source, &staged);
    guard.check()?;
    Ok(ReplaceFaceReport {
        model: staged,
        face,
        target_kind: match &geom {
            TargetGeom::Plane(_) => "plane",
            TargetGeom::Cylinder(_) => "cylinder",
        },
        retrimmed_vertices: substitute.len(),
        reauthored_edges: new_curves.len(),
        recomputed_pcurves,
        dirty_faces,
        rebuilt_faces,
        max_surface_residual_mm: max_residual,
        volume_before_mm3: volume_before,
        volume_after_mm3: volume_after,
        delta_volume_actual_mm3: volume_after - volume_before,
        audit,
    })
}
