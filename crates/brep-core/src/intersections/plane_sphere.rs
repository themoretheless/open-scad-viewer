//! Analytic plane/sphere intersection for a canonical planar patch operand
//! and a canonical stereographic sphere solid.
//!
//! The plane operand is a standalone finite rectangular planar patch: a
//! one-face body-less model (a single open shell — bodies require closed
//! shells) whose single face is an exact bilinear affine surface over the
//! unit square with unit weights, exact unit-square boundary trims and four
//! corner vertices — the same surface construction a `cuboid` face uses,
//! admitted here as a rigidly placeable free patch. A
//! face embedded in a closed solid is not accepted: the operand must be the
//! patch alone. The sphere operand must be the exact canonical stereographic
//! sphere of `analytic::sphere` (recognized by `sphere_sphere::recognize`).
//! Anything else is an explicit `UnsupportedSurface` region — this cell
//! never falls back to numerical surface/surface subdivision. Center
//! distance classification uses an outward binary64 band widened by both
//! recognition deviations: a plane clearly missing the sphere resolves
//! empty, a distance within the band of the radius reports
//! `TangencyOrMultipleRoot` (a tangent plane is never a point component),
//! and a clear crossing yields the exact rational circle of radius
//! sqrt(r^2 - d^2). The circle is clipped by the patch's finite rectangular
//! domain with exact trim parameters (circle/rectangle clipping in the
//! patch's UV coordinates, where the circle is an axis-aligned ellipse
//! because the patch is rectangular); a circle fully inside the domain
//! yields the full circle, boundary crossings yield clipped arcs, and a
//! tangency to a domain edge stays unresolved. Lifts: per-patch UV
//! circles/lines on the sphere via `sphere_sphere::lift` (full circle) or
//! `sphere_sphere::lift_clipped` (clipped arcs, with exact endpoint
//! inversion), and exact rational quadratic ellipse arcs in the plane UV.
//! Nothing here authorizes a topology change.
use super::sphere_sphere::{self, CanonicalSphere, RECOGNITION, SpherePatchCircle};
use super::*;
use crate::Model;

const TAU: f64 = std::f64::consts::TAU;
const QUARTER: f64 = std::f64::consts::FRAC_PI_2;

/// A recognized canonical planar patch: an exact affine rectangular surface
/// `P(u,v) = origin + u*U + v*V` over `(u,v) in [0,1]^2`, with U and V
/// orthogonal within the recognition band.
#[derive(Clone, Debug)]
pub(crate) struct CanonicalPlane {
    /// Surface control point [0][0]: the (0,0) corner.
    pub(crate) origin: [f64; 3],
    /// Edge vector to the (1,0) corner.
    pub(crate) u: [f64; 3],
    /// Edge vector to the (0,1) corner.
    pub(crate) v: [f64; 3],
    /// Unit normal u x v / |u x v|.
    pub(crate) normal: [f64; 3],
    pub(crate) u_len: f64,
    pub(crate) v_len: f64,
    /// Observed structural deviation bound; feeds the outward classification.
    pub(crate) error: f64,
}

/// Degree-1 pcurve exactly from `from` to `to` with unit knots/weights.
fn unit_edge(curve: &Curve, from: [f64; 2], to: [f64; 2]) -> bool {
    curve.degree == 1
        && curve.knots == [0., 0., 1., 1.]
        && curve.control_points == [from.to_vec(), to.to_vec()]
        && curve.weights == [1., 1.]
}

/// Recognizes a canonical planar patch model: one body, one open shell, one
/// bilinear affine face over the unit square with exact unit weights, the
/// four unit-square boundary trims each exactly once, four corner vertices
/// matching the surface corners, an exactly-affine fourth corner, and edge
/// vectors orthogonal within the recognition band. Rigid affine placement is
/// admitted; anything else returns `None`.
pub(crate) fn recognize_plane(model: &Model) -> Result<Option<CanonicalPlane>> {
    model.validate()?;
    // Bodies require closed shells, so the freestanding patch is a single
    // open shell with no body at all.
    if !model.bodies.is_empty()
        || model.shells.len() != 1
        || model.faces.len() != 1
        || model.vertices.len() != 4
        || model.edges.len() != 4
        || model.loops.len() != 1
    {
        return Ok(None);
    }
    let shell = &model.shells[0];
    if shell.closed || shell.faces.len() != 1 {
        return Ok(None);
    }
    let face = &model.faces[0];
    let surface = &face.surface;
    if surface.degree_u != 1
        || surface.degree_v != 1
        || surface.periodic_u
        || surface.periodic_v
        || surface.knots_u != [0., 0., 1., 1.]
        || surface.knots_v != [0., 0., 1., 1.]
        || surface.control_points.len() != 2
        || surface.control_points.iter().any(|row| row.len() != 2)
        || surface
            .control_points
            .iter()
            .flatten()
            .any(|p| p.len() != 3)
        || surface.weights != [[1., 1.], [1., 1.]]
        || !face.holes.is_empty()
    {
        return Ok(None);
    }
    let boundary = [
        ([0., 0.], [1., 0.]),
        ([1., 0.], [1., 1.]),
        ([1., 1.], [0., 1.]),
        ([0., 1.], [0., 0.]),
    ];
    let loop_ = &model.loops[face.outer];
    if loop_.coedges.len() != 4 {
        return Ok(None);
    }
    let mut seen = [false; 4];
    for coedge in &loop_.coedges {
        let mut hit = false;
        for (k, &(a, b)) in boundary.iter().enumerate() {
            if !seen[k] && unit_edge(&coedge.pcurve, a, b) {
                seen[k] = true;
                hit = true;
                break;
            }
        }
        if !hit {
            return Ok(None);
        }
    }
    if seen.into_iter().any(|hit| !hit) {
        return Ok(None);
    }
    let p = &surface.control_points;
    let p00 = point3(&p[0][0]);
    let p10 = point3(&p[1][0]);
    let p01 = point3(&p[0][1]);
    let p11 = point3(&p[1][1]);
    let u = sub(p10, p00);
    let v = sub(p01, p00);
    let u_len = u[0].hypot(u[1]).hypot(u[2]);
    let v_len = v[0].hypot(v[1]).hypot(v[2]);
    if !(1e-5..=1e6).contains(&u_len) || !(1e-5..=1e6).contains(&v_len) {
        return Ok(None);
    }
    let scale = u_len.max(v_len);
    let mut error: f64 = 0.;
    // Affine fourth corner: p11 == p00 + u + v.
    let deviation = {
        let d = [
            p11[0] - (p00[0] + u[0] + v[0]),
            p11[1] - (p00[1] + u[1] + v[1]),
            p11[2] - (p00[2] + u[2] + v[2]),
        ];
        d[0].hypot(d[1]).hypot(d[2])
    };
    if !deviation.is_finite() || deviation > RECOGNITION * scale + 1e-12 {
        return Ok(None);
    }
    error = error.max(deviation);
    // Rectangularity: the edge vectors must be orthogonal within the band.
    let skew = dot(u, v);
    let skew_model = skew.abs() / u_len.min(v_len);
    if !skew_model.is_finite() || skew_model > RECOGNITION * scale {
        return Ok(None);
    }
    error = error.max(skew_model);
    let normal = cross(u, v);
    let n_len = normal[0].hypot(normal[1]).hypot(normal[2]);
    if !(n_len > 0.) || !n_len.is_finite() {
        return Ok(None);
    }
    let normal = normal.map(|x| x / n_len);
    // Every vertex coincides with one surface corner, each exactly once.
    let corners = [p00, p10, p11, p01];
    let mut used = [false; 4];
    for vertex in &model.vertices {
        let mut hit = false;
        for (k, corner) in corners.iter().enumerate() {
            let d = sub(vertex.point, *corner);
            let deviation = d[0].hypot(d[1]).hypot(d[2]);
            if !used[k] && deviation <= RECOGNITION * scale {
                used[k] = true;
                hit = true;
                error = error.max(deviation);
                break;
            }
        }
        if !hit {
            return Ok(None);
        }
    }
    if used.into_iter().any(|hit| !hit) {
        return Ok(None);
    }
    Ok(Some(CanonicalPlane {
        origin: p00,
        u,
        v,
        normal,
        u_len,
        v_len,
        error,
    }))
}

/// One plane patch's share of an intersection curve in that patch's UV. The
/// canonical plane operand has exactly one face, so `patch` is always 0.
#[derive(Clone, Debug)]
pub struct PlanePatchCurve {
    /// Face index in the source plane model.
    pub patch: usize,
    /// Exact 2D rational quadratic arcs (or degree-1 segments) in patch UV.
    pub arcs: Vec<Curve>,
}

/// A 2D half-plane constraint `normal . x <= offset` with its own outward
/// classification band (in the 2D coordinate units of the clipped space).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Halfplane2 {
    pub(crate) normal: [f64; 2],
    pub(crate) offset: f64,
    pub(crate) band: f64,
}

/// Result of clipping an ellipse by a set of half-planes.
#[derive(Clone, Debug)]
pub(crate) enum EllipseClip {
    /// Provably outside at least one half-plane beyond its band.
    Empty,
    /// No cuts: strictly inside every half-plane.
    Full,
    /// Swept eccentric-angle intervals (start, end), `end > start`, each
    /// strictly inside all half-planes.
    Arcs(Vec<(f64, f64)>),
}

/// Exact ellipse/half-plane clipping by eccentric angle. The ellipse is
/// `p(phi) = center + cos(phi) w1 + sin(phi) w2`. Each half-plane cuts the
/// ellipse where `f0 + P cos(phi) + Q sin(phi) = 0`, solved exactly as
/// `phi = atan2(Q, P) +- acos(-f0 / hypot(P, Q))`. A maximum or minimum of
/// the constraint function within the half-plane's band of the boundary is
/// a tangency and is reported through the flag, never clipped through.
/// Surviving spans are decided by exact midpoint evaluation; nothing is
/// merged across gaps.
pub(crate) fn clip_ellipse(
    center: [f64; 2],
    w1: [f64; 2],
    w2: [f64; 2],
    halfplanes: &[Halfplane2],
) -> (EllipseClip, bool) {
    let point = |phi: f64| {
        [
            center[0] + phi.cos() * w1[0] + phi.sin() * w2[0],
            center[1] + phi.cos() * w1[1] + phi.sin() * w2[1],
        ]
    };
    let mut cuts: Vec<f64> = Vec::new();
    let mut tangent = false;
    for hp in halfplanes {
        let f0 = hp.normal[0] * center[0] + hp.normal[1] * center[1] - hp.offset;
        let p = hp.normal[0] * w1[0] + hp.normal[1] * w1[1];
        let q = hp.normal[0] * w2[0] + hp.normal[1] * w2[1];
        let s = p.hypot(q);
        if !f0.is_finite() || !s.is_finite() {
            return (EllipseClip::Empty, true);
        }
        if !(s > 0.) {
            // The constraint is constant on the ellipse (degenerate input).
            if f0 > hp.band {
                return (EllipseClip::Empty, tangent);
            }
            if f0.abs() <= hp.band {
                tangent = true;
            }
            continue;
        }
        if f0 - s > hp.band {
            // Provably violated everywhere: the ellipse misses this region.
            return (EllipseClip::Empty, tangent);
        }
        if f0 + s <= -hp.band {
            // Provably inside everywhere: the constraint never binds.
            continue;
        }
        if (f0 + s).abs() <= hp.band || (f0 - s).abs() <= hp.band {
            // An extremum of the constraint function sits within the band of
            // the boundary line: a tangency, never clipped through.
            tangent = true;
            continue;
        }
        let delta = q.atan2(p);
        let alpha = (-f0 / s).clamp(-1., 1.).acos();
        cuts.push((delta + alpha).rem_euclid(TAU));
        cuts.push((delta - alpha).rem_euclid(TAU));
    }
    if cuts.is_empty() {
        return (EllipseClip::Full, tangent);
    }
    cuts.sort_by(f64::total_cmp);
    cuts.dedup_by(|a, b| (*a - *b).abs() <= 1e-12);
    let inside = |phi: f64| {
        let q = point(phi);
        halfplanes.iter().all(|hp| {
            let scale = hp.normal[0].abs() * (center[0].abs() + w1[0].abs() + w2[0].abs())
                + hp.normal[1].abs() * (center[1].abs() + w1[1].abs() + w2[1].abs())
                + hp.offset.abs()
                + 1.;
            hp.normal[0] * q[0] + hp.normal[1] * q[1] - hp.offset <= 1e-12 * scale
        })
    };
    let mut arcs: Vec<(f64, f64)> = Vec::new();
    for i in 0..cuts.len() {
        let start = cuts[i];
        let end = if i + 1 < cuts.len() {
            cuts[i + 1]
        } else {
            cuts[0] + TAU
        };
        if end - start > 1e-14 && inside((start + end) / 2.) {
            arcs.push((start, end));
        }
    }
    if arcs.is_empty() {
        (EllipseClip::Empty, tangent)
    } else {
        (EllipseClip::Arcs(arcs), tangent)
    }
}

/// Exact rational multi-span sweep of the conic
/// `p(phi) = center + cos(phi) w1 + sin(phi) w2` over `[start, end]`, split
/// into arcs of at most 90 degrees with weights cos(half-angle). Knots are
/// `0..=pieces` with doubled interior knots, matching `circle_curve` when
/// w1/w2 are perpendicular equal-length vectors and the sweep is 0..TAU.
pub(crate) fn conic_sweep(
    center: [f64; 3],
    w1: [f64; 3],
    w2: [f64; 3],
    start: f64,
    end: f64,
) -> Curve {
    let pieces = ((end - start) / QUARTER).ceil().max(1.) as usize;
    let mut knots = vec![0., 0., 0.];
    let mut control_points = Vec::with_capacity(2 * pieces + 1);
    let mut weights = Vec::with_capacity(2 * pieces + 1);
    let point = |phi: f64| {
        std::array::from_fn::<f64, 3, _>(|k| center[k] + phi.cos() * w1[k] + phi.sin() * w2[k])
            .to_vec()
    };
    for i in 0..pieces {
        let a0 = start + (end - start) * i as f64 / pieces as f64;
        let a1 = start + (end - start) * (i + 1) as f64 / pieces as f64;
        let half = (a1 - a0) / 2.;
        let weight = half.cos();
        let middle = (a0 + a1) / 2.;
        let shoulder = std::array::from_fn::<f64, 3, _>(|k| {
            center[k] + (middle.cos() / weight) * w1[k] + (middle.sin() / weight) * w2[k]
        })
        .to_vec();
        if i == 0 {
            control_points.push(point(a0));
            weights.push(1.);
        }
        control_points.push(shoulder);
        weights.push(weight);
        control_points.push(point(a1));
        weights.push(1.);
        if i + 1 < pieces {
            knots.extend_from_slice(&[(i + 1) as f64, (i + 1) as f64]);
        }
    }
    knots.extend_from_slice(&[pieces as f64, pieces as f64, pieces as f64]);
    Curve {
        degree: 2,
        knots,
        control_points,
        weights,
        periodic: false,
    }
}

/// Exact rational quadratic arcs (single-span, knots [0,0,0,1,1,1]) covering
/// the 2D conic sweep over `[start, end]`, matching `circle_arcs` for a
/// circular w1/w2 pair.
pub(crate) fn conic_arcs_2d(
    center: [f64; 2],
    w1: [f64; 2],
    w2: [f64; 2],
    start: f64,
    end: f64,
) -> Vec<Curve> {
    let pieces = ((end - start) / QUARTER).ceil().max(1.) as usize;
    (0..pieces)
        .map(|i| {
            let a0 = start + (end - start) * i as f64 / pieces as f64;
            let a1 = start + (end - start) * (i + 1) as f64 / pieces as f64;
            let half = (a1 - a0) / 2.;
            let weight = half.cos();
            let point = |phi: f64| {
                [
                    center[0] + phi.cos() * w1[0] + phi.sin() * w2[0],
                    center[1] + phi.cos() * w1[1] + phi.sin() * w2[1],
                ]
                .to_vec()
            };
            let middle = (a0 + a1) / 2.;
            let shoulder = [
                center[0] + (middle.cos() / weight) * w1[0] + (middle.sin() / weight) * w2[0],
                center[1] + (middle.cos() / weight) * w1[1] + (middle.sin() / weight) * w2[1],
            ]
            .to_vec();
            Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: vec![point(a0), shoulder, point(a1)],
                weights: vec![1., weight, 1.],
                periodic: false,
            }
        })
        .collect()
}

#[derive(Clone, Debug)]
pub enum PlaneSphereComponent {
    /// Clear crossing: the exact circle (full, or clipped to the finite
    /// plane domain with exact trim parameters) plus both UV lifts.
    Circle {
        /// Exact rational quadratic sweep: the full circle (four 90-degree
        /// arcs, weights cos(pi/4), knots 0..=4) when `full`, else the
        /// clipped arc pieces over the swept interval.
        curve: Curve,
        center: [f64; 3],
        radius: f64,
        /// Unit plane normal (the circle plane normal).
        normal: [f64; 3],
        /// True when the circle lies fully inside the plane patch domain.
        full: bool,
        plane_uv: Vec<PlanePatchCurve>,
        sphere_uv: Vec<SpherePatchCircle>,
        /// Worst residual over curve samples against the sphere equation and
        /// the plane equation.
        max_sample_residual: f64,
    },
}

/// Worst |.|p|-r|. and |n.(p-o)| over `count` samples of the curve domain.
fn circle_residuals(
    curve: &Curve,
    sphere: &CanonicalSphere,
    plane: &CanonicalPlane,
    count: usize,
) -> Result<f64> {
    let [lo, hi] = curve.domain();
    let mut worst = 0_f64;
    for i in 0..count {
        let t = lo + (hi - lo) * i as f64 / (count - 1) as f64;
        let p = curve.evaluate(t)?.point;
        let p = [p[0], p[1], p[2]];
        let d = sub(p, sphere.center);
        let sphere_residual = (d[0].hypot(d[1]).hypot(d[2]) - sphere.radius).abs();
        let plane_residual = dot(plane.normal, sub(p, plane.origin)).abs();
        worst = worst.max(sphere_residual).max(plane_residual);
    }
    Ok(worst)
}

/// Analytic plane/sphere intersection of a canonical planar patch and a
/// canonical sphere solid. Non-canonical operands are explicit
/// `UnsupportedSurface` regions, never a numerical fallback; the tangency
/// band (|d| == r) and any tangency to the patch's rectangular domain
/// boundary stay unresolved — tangent contacts are never guessed.
pub fn intersect_plane_sphere(
    plane_model: &Model,
    sphere_model: &Model,
    options: Options,
) -> Result<Report<PlaneSphereComponent>> {
    let options = options.validate()?;
    let _ = options;
    let mut report = Report::default();
    let domain = vec![0., 1., 0., 1., 0., 1., 0., 1.];
    let (Some(plane), Some(sphere)) = (
        recognize_plane(plane_model)?,
        sphere_sphere::recognize(sphere_model)?,
    ) else {
        report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    };
    report.boxes_visited = 1;
    let d = dot(plane.normal, sub(sphere.center, plane.origin));
    let scale = plane.u_len
        + plane.v_len
        + sphere.radius
        + sub(sphere.center, plane.origin)
            .iter()
            .map(|v| v.abs())
            .fold(0., f64::max)
        + 1.;
    // Outward binary64 classification band: both recognition deviations plus
    // a rounding allowance on the distance arithmetic and the radii.
    let band = plane.error + sphere.error + 16. * f64::EPSILON * scale;
    let ad = d.abs();
    let d_lo = (ad - band).max(0.);
    let d_hi = ad + band;
    let r = sphere.radius;
    // Miss: the whole distance band clears the radius.
    if d_lo > r + band {
        return Ok(report);
    }
    // Tangency, or a band straddling it: never a point component.
    if d_hi >= (r - band).max(0.) {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    let rho2 = r * r - d * d;
    if !rho2.is_finite() || rho2 <= 0. {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    let rho = rho2.sqrt();
    let middle = std::array::from_fn(|k| sphere.center[k] - d * plane.normal[k]);
    // In-plane orthonormal basis aligned with the patch edges, so the circle
    // is an axis-aligned ellipse in patch UV.
    let e1 = plane.u.map(|x| x / plane.u_len);
    let e2 = plane.v.map(|x| x / plane.v_len);
    let rel = sub(middle, plane.origin);
    let uc = dot(rel, plane.u) / (plane.u_len * plane.u_len);
    let vc = dot(rel, plane.v) / (plane.v_len * plane.v_len);
    let w1 = [rho / plane.u_len, 0.];
    let w2 = [0., rho / plane.v_len];
    let rect = [
        Halfplane2 {
            normal: [-1., 0.],
            offset: 0.,
            band: band / plane.u_len,
        },
        Halfplane2 {
            normal: [1., 0.],
            offset: 1.,
            band: band / plane.u_len,
        },
        Halfplane2 {
            normal: [0., -1.],
            offset: 0.,
            band: band / plane.v_len,
        },
        Halfplane2 {
            normal: [0., 1.],
            offset: 1.,
            band: band / plane.v_len,
        },
    ];
    let (clip, tangent) = clip_ellipse([uc, vc], w1, w2, &rect);
    if tangent {
        // The circle touches the patch boundary within the band: unresolved,
        // alongside any resolved arc components (house tangency discipline).
        report.unresolved(domain.clone(), UnresolvedReason::TangencyOrMultipleRoot);
    }
    let point_at = |phi: f64| {
        std::array::from_fn::<f64, 3, _>(|k| {
            middle[k] + rho * (phi.cos() * e1[k] + phi.sin() * e2[k])
        })
    };
    let we1 = e1.map(|x| x * rho);
    let we2 = e2.map(|x| x * rho);
    match clip {
        EllipseClip::Empty => {}
        EllipseClip::Full if !tangent => {
            let curve = conic_sweep(middle, we1, we2, 0., TAU);
            let sphere_uv = sphere_sphere::lift(&sphere, plane.normal, middle);
            let plane_uv = vec![PlanePatchCurve {
                patch: 0,
                arcs: conic_arcs_2d([uc, vc], w1, w2, 0., TAU),
            }];
            let max_sample_residual = circle_residuals(&curve, &sphere, &plane, 16)?;
            report.components.push(PlaneSphereComponent::Circle {
                curve,
                center: middle,
                radius: rho,
                normal: plane.normal,
                full: true,
                plane_uv,
                sphere_uv,
                max_sample_residual,
            });
        }
        EllipseClip::Arcs(intervals) => {
            for (a, b) in intervals {
                let curve = conic_sweep(middle, we1, we2, a, b);
                let sphere_uv =
                    sphere_sphere::lift_clipped(&sphere, plane.normal, middle, &[(a, b)], point_at);
                let plane_uv = vec![PlanePatchCurve {
                    patch: 0,
                    arcs: conic_arcs_2d([uc, vc], w1, w2, a, b),
                }];
                let max_sample_residual = circle_residuals(&curve, &sphere, &plane, 9)?;
                report.components.push(PlaneSphereComponent::Circle {
                    curve,
                    center: middle,
                    radius: rho,
                    normal: plane.normal,
                    full: false,
                    plane_uv,
                    sphere_uv,
                    max_sample_residual,
                });
            }
        }
        // Tangency within the band: the full circle is withheld; the
        // unresolved region above carries the contact.
        EllipseClip::Full => {}
    }
    Ok(report)
}

impl value_codec::Serialize for PlanePatchCurve {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({"patch":self.patch,"arcs":self.arcs})
    }
}
impl value_codec::Serialize for PlaneSphereComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Circle {
                curve,
                center,
                radius,
                normal,
                full,
                plane_uv,
                sphere_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"circle","curve":curve,"center":center,"radius":radius,
                "normal":normal,"full":full,"planeUv":plane_uv,"sphereUv":sphere_uv,
                "maxSampleResidual":max_sample_residual}),
        }
    }
}

/// Builds the canonical planar patch operand: a one-face open model with an
/// exact bilinear affine surface over the unit square, unit-square boundary
/// trims and four corner vertices (the cuboid face construction, freestanding).
#[cfg(test)]
pub(crate) fn plane_patch(origin: [f64; 3], u: [f64; 3], v: [f64; 3]) -> Model {
    use crate::{Coedge, Edge, Face, FaceUse, Loop, Shell, TopologyIds, Vertex};
    let add = |a: [f64; 3], b: [f64; 3]| [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
    let corners = [origin, add(origin, u), add(add(origin, u), v), add(origin, v)];
    let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    let mut edges = Vec::new();
    let mut coedges = Vec::new();
    for i in 0..4 {
        let j = (i + 1) % 4;
        edges.push(Edge {
            degenerate: false,
            vertices: [i, j],
            curve: crate::line(corners[i].to_vec(), corners[j].to_vec()),
        });
        coedges.push(Coedge {
            edge: i,
            reversed: false,
            pcurve: crate::line(uv[i].to_vec(), uv[j].to_vec()),
        });
    }
    let mut model = Model(
        brep_topology::Model {
            vertices: corners.map(|point| Vertex { point }).to_vec(),
            edges,
            loops: vec![Loop { coedges }],
            faces: vec![Face {
                surface: Surface {
                    degree_u: 1,
                    degree_v: 1,
                    knots_u: vec![0., 0., 1., 1.],
                    knots_v: vec![0., 0., 1., 1.],
                    control_points: vec![
                        vec![corners[0].to_vec(), corners[3].to_vec()],
                        vec![corners[1].to_vec(), corners[2].to_vec()],
                    ],
                    weights: vec![vec![1., 1.], vec![1., 1.]],
                    periodic_u: false,
                    periodic_v: false,
                },
                outer: 0,
                holes: vec![],
            }],
            shells: vec![Shell {
                faces: vec![FaceUse {
                    face: 0,
                    reversed: false,
                }],
                closed: false,
            }],
            // A freestanding open patch: bodies require closed shells.
            bodies: vec![],
            tolerance_mm: 1e-7,
        },
        TopologyIds::default(),
    );
    model.rebuild_topology_ids();
    model.validate().unwrap();
    model
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::sphere_sphere::ARC_WEIGHT;

    fn translated(model: &Model, offset: [f64; 3]) -> Model {
        crate::transform::affine(
            model,
            [
                [1., 0., 0., offset[0]],
                [0., 1., 0., offset[1]],
                [0., 0., 1., offset[2]],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap()
    }
    fn rotated_translated(model: &Model, angle: f64, offset: [f64; 3]) -> Model {
        let (sin, cos) = angle.sin_cos();
        crate::transform::affine(
            model,
            [
                [1., 0., 0., offset[0]],
                [0., cos, -sin, offset[1]],
                [0., sin, cos, offset[2]],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap()
    }
    #[allow(clippy::type_complexity)]
    fn only_circle(
        report: &Report<PlaneSphereComponent>,
        full: bool,
    ) -> (
        &Curve,
        [f64; 3],
        f64,
        [f64; 3],
        &[PlanePatchCurve],
        &[SpherePatchCircle],
        f64,
    ) {
        assert!(report.unresolved.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        assert!(!report.permits_topology_change());
        let [PlaneSphereComponent::Circle {
            curve,
            center,
            radius,
            normal,
            full: is_full,
            plane_uv,
            sphere_uv,
            max_sample_residual,
        }] = &report.components[..]
        else {
            panic!("expected one circle component: {report:?}")
        };
        assert_eq!(*is_full, full);
        (
            curve,
            *center,
            *radius,
            *normal,
            plane_uv,
            sphere_uv,
            *max_sample_residual,
        )
    }
    fn sphere_residual(point: [f64; 3], center: [f64; 3], radius: f64) -> f64 {
        let d = sub(point, center);
        (d[0].hypot(d[1]).hypot(d[2]) - radius).abs()
    }
    /// Worst residual of lifted UV arcs on both operands, evaluated through
    /// their own surfaces, against the sphere and plane equations.
    fn uv_samples(
        plane_model: &Model,
        sphere_model: &Model,
        plane_uv: &[PlanePatchCurve],
        sphere_uv: &[SpherePatchCircle],
        sphere_def: ([f64; 3], f64),
        plane_def: ([f64; 3], [f64; 3]),
    ) -> f64 {
        let mut worst = 0_f64;
        let check = |p: [f64; 3]| {
            sphere_residual(p, sphere_def.0, sphere_def.1)
                .max(dot(plane_def.0, sub(p, plane_def.1)).abs())
        };
        for lift in plane_uv {
            let surface = &plane_model.faces[lift.patch].surface;
            for arc in &lift.arcs {
                for k in 0..=8 {
                    let uv = arc.evaluate(k as f64 / 8.).unwrap().point;
                    assert!((-1e-9..=1. + 1e-9).contains(&uv[0]), "{uv:?}");
                    assert!((-1e-9..=1. + 1e-9).contains(&uv[1]), "{uv:?}");
                    let p = surface
                        .evaluate(uv[0].clamp(0., 1.), uv[1].clamp(0., 1.))
                        .unwrap()
                        .point;
                    worst = worst.max(check([p[0], p[1], p[2]]));
                }
            }
        }
        for lift in sphere_uv {
            let surface = &sphere_model.faces[lift.patch].surface;
            for arc in &lift.arcs {
                for k in 0..=8 {
                    let uv = arc.evaluate(k as f64 / 8.).unwrap().point;
                    assert!(uv[0] >= -1e-9 && uv[1] >= -1e-9, "{uv:?}");
                    assert!(uv[0] * uv[0] + uv[1] * uv[1] <= 1. + 1e-9, "{uv:?}");
                    let p = surface
                        .evaluate(uv[0].clamp(0., 1.), uv[1].clamp(0., 1.))
                        .unwrap()
                        .point;
                    worst = worst.max(check([p[0], p[1], p[2]]));
                }
            }
        }
        worst
    }

    #[test]
    fn full_section_matches_the_sqrt_oracle() {
        // Plane z = 1, patch [-3,3]^2; sphere r = 2 at the origin: the exact
        // circle of radius sqrt(4 - 1) at z = 1, fully inside the patch.
        let plane = plane_patch([-3., -3., 1.], [6., 0., 0.], [0., 6., 0.]);
        let sphere = crate::analytic::sphere(2.).unwrap();
        let report = intersect_plane_sphere(&plane, &sphere, Options::default()).unwrap();
        let (curve, center, radius, normal, plane_uv, sphere_uv, sampled) =
            only_circle(&report, true);
        let oracle = (4_f64 - 1.).sqrt();
        assert!((radius - oracle).abs() <= 1e-12, "{radius} vs {oracle}");
        assert!(sub(center, [0., 0., 1.]).iter().all(|x| x.abs() <= 1e-12), "{center:?}");
        assert!(sub(normal, [0., 0., 1.]).iter().all(|x| x.abs() <= 1e-12), "{normal:?}");
        // Exact rational circle: four 90-degree arcs, weights cos(pi/4).
        assert_eq!(curve.degree, 2);
        assert_eq!(curve.knots, vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.]);
        assert_eq!(curve.control_points.len(), 9);
        assert_eq!(
            curve.weights,
            vec![1., ARC_WEIGHT, 1., ARC_WEIGHT, 1., ARC_WEIGHT, 1., ARC_WEIGHT, 1.]
        );
        // Sixteen sampled points satisfy both implicit equations.
        let mut worst = 0_f64;
        for i in 0..16 {
            let p = curve.evaluate(i as f64 / 4.).unwrap().point;
            let p = [p[0], p[1], p[2]];
            worst = worst
                .max(sphere_residual(p, [0., 0., 0.], 2.))
                .max((p[2] - 1.).abs());
        }
        assert!(worst <= 1e-12, "{worst}");
        assert!(sampled <= 1e-12, "{sampled}");
        // Plane UV: the exact UV circle of radius sqrt(3)/6 about [1/2, 1/2].
        assert_eq!(plane_uv.len(), 1);
        assert_eq!(plane_uv[0].patch, 0);
        assert_eq!(plane_uv[0].arcs.len(), 4);
        for arc in &plane_uv[0].arcs {
            assert_eq!(arc.degree, 2);
            assert_eq!(arc.weights, vec![1., ARC_WEIGHT, 1.]);
            let endpoint = &arc.control_points[0];
            let uvr = (endpoint[0] - 0.5).hypot(endpoint[1] - 0.5);
            assert!((uvr - oracle / 6.).abs() <= 1e-12, "{uvr}");
        }
        assert!(!sphere_uv.is_empty());
        let uv_worst = uv_samples(
            &plane,
            &sphere,
            plane_uv,
            sphere_uv,
            ([0., 0., 0.], 2.),
            ([0., 0., 1.], [0., 0., 1.]),
        );
        assert!(uv_worst <= 1e-9, "{uv_worst}");
        // Operand order is fixed: the sphere as plane operand refuses.
        let swapped = intersect_plane_sphere(&sphere, &plane, Options::default()).unwrap();
        assert_eq!(swapped.unresolved[0].reason, UnresolvedReason::UnsupportedSurface);
    }

    #[test]
    fn boundary_clipping_has_exact_trims_and_clipped_lifts() {
        // Plane z = 1, patch covering x in [0, 3], y in [-3, 3]: the circle of
        // radius sqrt(3) is clipped to its x >= 0 half, swept angle interval
        // [3pi/2, 5pi/2] in the (x, y) basis, endpoints (0, +-sqrt(3), 1).
        let plane = plane_patch([0., -3., 1.], [3., 0., 0.], [0., 6., 0.]);
        let sphere = crate::analytic::sphere(2.).unwrap();
        let report = intersect_plane_sphere(&plane, &sphere, Options::default()).unwrap();
        let (curve, center, radius, _, plane_uv, sphere_uv, sampled) =
            only_circle(&report, false);
        let oracle = 3_f64.sqrt();
        assert!((radius - oracle).abs() <= 1e-12);
        assert!(sub(center, [0., 0., 1.]).iter().all(|x| x.abs() <= 1e-12), "{center:?}");
        // Two 90-degree pieces over the half circle.
        assert_eq!(curve.knots, vec![0., 0., 0., 1., 1., 2., 2., 2.]);
        let start = curve.evaluate(0.).unwrap().point;
        let end = curve.evaluate(2.).unwrap().point;
        assert!(start[0].abs() <= 1e-12 && (start[1] + oracle).abs() <= 1e-12, "{start:?}");
        assert!((start[2] - 1.).abs() <= 1e-12);
        assert!(end[0].abs() <= 1e-12 && (end[1] - oracle).abs() <= 1e-12, "{end:?}");
        assert!((end[2] - 1.).abs() <= 1e-12);
        let mut worst = 0_f64;
        for i in 0..=16 {
            let p = curve.evaluate(i as f64 / 8.).unwrap().point;
            let p = [p[0], p[1], p[2]];
            assert!(p[0] >= -1e-12, "{p:?}");
            worst = worst
                .max(sphere_residual(p, [0., 0., 0.], 2.))
                .max((p[2] - 1.).abs());
        }
        assert!(worst <= 1e-12, "{worst}");
        assert!(sampled <= 1e-12, "{sampled}");
        // Plane UV: two clipped ellipse arcs meeting at the phi = 0 seam
        // (1/sqrt(3), 1/2); the free endpoints sit exactly on u = 0 at
        // v = 1/2 +- sqrt(3)/6.
        assert_eq!(plane_uv[0].arcs.len(), 2);
        let mut edge_v: Vec<f64> = Vec::new();
        let mut seam: Vec<[f64; 2]> = Vec::new();
        for arc in &plane_uv[0].arcs {
            for endpoint in [&arc.control_points[0], &arc.control_points[2]] {
                if endpoint[0].abs() <= 1e-12 {
                    edge_v.push(endpoint[1]);
                } else {
                    seam.push([endpoint[0], endpoint[1]]);
                }
            }
        }
        edge_v.sort_by(f64::total_cmp);
        let oracle_v = oracle / 6.;
        assert_eq!(edge_v.len(), 2, "{edge_v:?}");
        assert!((edge_v[0] - (0.5 - oracle_v)).abs() <= 1e-12, "{edge_v:?}");
        assert!((edge_v[1] - (0.5 + oracle_v)).abs() <= 1e-12, "{edge_v:?}");
        assert_eq!(seam.len(), 2, "{seam:?}");
        for p in &seam {
            assert!((p[0] - 1. / oracle).abs() <= 1e-12, "{p:?}");
            assert!((p[1] - 0.5).abs() <= 1e-12, "{p:?}");
        }
        // Sphere UV: clipped per-patch lifts evaluate onto both surfaces.
        assert!(!sphere_uv.is_empty());
        let uv_worst = uv_samples(
            &plane,
            &sphere,
            plane_uv,
            sphere_uv,
            ([0., 0., 0.], 2.),
            ([0., 0., 1.], [0., 0., 1.]),
        );
        assert!(uv_worst <= 1e-9, "{uv_worst}");
    }

    #[test]
    fn miss_tangency_and_band_classification() {
        let sphere = crate::analytic::sphere(2.).unwrap();
        let at = |z: f64| plane_patch([-1., -1., z], [2., 0., 0.], [0., 2., 0.]);
        // Miss: d = 5 > r, empty and resolved.
        let report = intersect_plane_sphere(&at(5.), &sphere, Options::default()).unwrap();
        assert!(report.components.is_empty() && report.unresolved.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
        // Exact tangency and within the outward band: never a point.
        for z in [2., 2. - 3e-14] {
            let report = intersect_plane_sphere(&at(z), &sphere, Options::default()).unwrap();
            assert!(report.components.is_empty(), "{z} {report:?}");
            assert_eq!(report.coverage, Coverage::Incomplete);
            assert_eq!(report.unresolved[0].reason, UnresolvedReason::TangencyOrMultipleRoot);
        }
        // Just clear of the band above: empty resolved; below: a small circle.
        let report =
            intersect_plane_sphere(&at(2. + 1e-12), &sphere, Options::default()).unwrap();
        assert!(report.components.is_empty() && report.unresolved.is_empty(), "{report:?}");
        let report =
            intersect_plane_sphere(&at(2. - 1e-12), &sphere, Options::default()).unwrap();
        let (_, _, radius, _, _, _, _) = only_circle(&report, true);
        // rho^2 = r^2 - d^2 loses digits to cancellation; compare squared.
        let d = 2. - 1e-12;
        let oracle = 4_f64 - d * d;
        assert!((radius * radius - oracle).abs() <= 2e-15, "{} vs {oracle}", radius * radius);
    }

    #[test]
    fn domain_edge_tangency_and_outside_cases() {
        let sphere = crate::analytic::sphere(2.).unwrap();
        // Circle of radius sqrt(3) at z = 1 tangent to the patch edge x = sqrt(3).
        let tangent = plane_patch([3_f64.sqrt(), -3., 1.], [3., 0., 0.], [0., 6., 0.]);
        let report = intersect_plane_sphere(&tangent, &sphere, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(report.unresolved[0].reason, UnresolvedReason::TangencyOrMultipleRoot);
        // Circle clearly outside the rectangle: empty and resolved.
        let outside = plane_patch([2.5, -3., 1.], [3., 0., 0.], [0., 6., 0.]);
        let report = intersect_plane_sphere(&outside, &sphere, Options::default()).unwrap();
        assert!(report.components.is_empty() && report.unresolved.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
        // Plane clear of the sphere in 3D regardless of patch size.
        let far = plane_patch([-10., -10., 4.5], [20., 0., 0.], [0., 20., 0.]);
        let report = intersect_plane_sphere(&far, &sphere, Options::default()).unwrap();
        assert!(report.components.is_empty() && report.unresolved.is_empty(), "{report:?}");
    }

    #[test]
    fn rigid_placement_keeps_the_exact_circle() {
        let angle = 0.5;
        let offset = [0.3, -0.2, 1.1];
        let plane = rotated_translated(
            &plane_patch([-3., -3., 1.], [6., 0., 0.], [0., 6., 0.]),
            angle,
            offset,
        );
        let sphere = rotated_translated(&crate::analytic::sphere(2.).unwrap(), angle, offset);
        let report = intersect_plane_sphere(&plane, &sphere, Options::default()).unwrap();
        let (curve, center, radius, normal, plane_uv, sphere_uv, sampled) =
            only_circle(&report, true);
        let (sin, cos) = angle.sin_cos();
        let placed = |p: [f64; 3]| {
            [
                p[0] + offset[0],
                cos * p[1] - sin * p[2] + offset[1],
                sin * p[1] + cos * p[2] + offset[2],
            ]
        };
        let placed_center = placed([0., 0., 1.]);
        let placed_normal = sub(placed([0., 0., 2.]), placed([0., 0., 1.]));
        assert!((radius - 3_f64.sqrt()).abs() <= 1e-12, "{radius}");
        assert!(sub(center, placed_center).iter().all(|x| x.abs() <= 1e-12), "{center:?}");
        assert!(sub(normal, placed_normal).iter().all(|x| x.abs() <= 1e-12), "{normal:?}");
        let mut worst = 0_f64;
        for i in 0..16 {
            let p = curve.evaluate(i as f64 / 4.).unwrap().point;
            let p = [p[0], p[1], p[2]];
            worst = worst
                .max(sphere_residual(p, offset, 2.))
                .max(dot(placed_normal, sub(p, placed_center)).abs());
        }
        assert!(worst <= 1e-12, "{worst}");
        assert!(sampled <= 1e-12, "{sampled}");
        let uv_worst = uv_samples(
            &plane,
            &sphere,
            plane_uv,
            sphere_uv,
            (offset, 2.),
            (placed_normal, placed([-3., -3., 1.])),
        );
        assert!(uv_worst <= 1e-9, "{uv_worst}");
    }

    #[test]
    fn noncanonical_operands_are_explicit_unsupported_regions() {
        let plane = plane_patch([-3., -3., 1.], [6., 0., 0.], [0., 6., 0.]);
        let sphere = crate::analytic::sphere(2.).unwrap();
        for (a, b) in [
            (plane.clone(), crate::analytic::cylinder(1., 2.).unwrap()),
            (plane.clone(), crate::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap()),
            (crate::analytic::cylinder(1., 2.).unwrap(), sphere.clone()),
            // A closed solid is not a planar patch even though it has faces.
            (crate::cuboid([0., 0., 0.], [4., 4., 1.]).unwrap(), sphere.clone()),
        ] {
            let report = intersect_plane_sphere(&a, &b, Options::default()).unwrap();
            assert!(report.components.is_empty(), "{report:?}");
            assert_eq!(report.coverage, Coverage::Incomplete);
            assert_eq!(report.unresolved.len(), 1);
            assert_eq!(report.unresolved[0].reason, UnresolvedReason::UnsupportedSurface);
            assert_eq!(report.unresolved[0].parameter_box, vec![0., 1., 0., 1., 0., 1., 0., 1.]);
            assert!(!report.permits_topology_change());
        }
        // A skewed (non-rectangular) affine patch is not canonical.
        let skewed = plane_patch([-3., -3., 1.], [6., 0., 0.], [3., 6., 0.]);
        let report = intersect_plane_sphere(&skewed, &sphere, Options::default()).unwrap();
        assert_eq!(report.unresolved[0].reason, UnresolvedReason::UnsupportedSurface);
        // A structurally perturbed patch (non-unit weight) no longer passes
        // model validation at all: the entry point refuses it outright.
        let mut perturbed = plane.clone();
        perturbed.faces[0].surface.weights[0][0] = 2.;
        assert!(intersect_plane_sphere(&perturbed, &sphere, Options::default()).is_err());
        // A valid canonical pair still resolves.
        let report = intersect_plane_sphere(&plane, &sphere, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
    }
}
