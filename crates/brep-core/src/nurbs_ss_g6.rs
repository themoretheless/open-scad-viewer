//! Narrow G6 productization scaffold gated by the R1 ADR verdict (`narrow`),
//! expanded through R1.1 (elevated planar bilinear→bicubic) to Phase B:
//! non-rational single-span Bezier patches with deg_u,deg_v ∈ {1,2,3}, elevated
//! to uniform bicubic before the transverse gate (`nurbs-ss-bezier-le3/1`).
//!
//! Beyond Complete-empty: planar × planar may publish a transverse line as
//! Complete. Non-planar pairs may publish a Complete curve only when a Hausdorff
//! + parameter-correspondence certificate holds. Out-of-matrix pairs refuse.
//! False Complete is a kill. Narrow Boolean imprint is Phase B (`bezier-le3`).

use crate::coverage_verifier::verify_complete_report;
use crate::intersections::{Coverage, Options, Report, UnresolvedReason};
use crate::predicate_evidence::{
    ComposedEvidence, EvidenceClaim, PredicateEvidence, compose_predicate_evidence,
};
use crate::solid_audit::{LocallyValidatedModel, SolidAuditCertificate};
use crate::trim_sew::{ChartKind, ClassificationCertificate, SewCertificate};
use crate::{ChangeSet, Model};
use cad_predicates::ToleranceSpecIdentity;
use nurbs_core::surface::Axis;
use nurbs_core::{Error, Result, surface::Surface};

fn refuse(message: &str) -> Error {
    Error::new("BREP_NURBS_SS_REFUSED", message)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum G6Maturity {
    Unavailable,
    ResearchOnly,
    NarrowTransverseBicubic,
}

pub const G6_MATURITY: G6Maturity = G6Maturity::NarrowTransverseBicubic;
pub const G6_CAPABILITY: &str = "nurbs-ss-bezier-le3/1";
pub const NURBS_BOOLEAN_CAPABILITY_V1: &str = "nurbs-boolean-bezier-le3/1";
pub const NURBS_BOOLEAN_CAPABILITY: &str = "nurbs-boolean-bezier-le3/2";
pub const NURBS_BOOLEAN_V1_MATURITY: G6Maturity = G6Maturity::Unavailable;

#[derive(Clone, Debug, PartialEq)]
pub enum G6Component {
    Empty,
    Line {
        start: [f64; 3],
        end: [f64; 3],
    },
    /// Sampled transverse curve with Hausdorff certificate (F4 non-planar path).
    Curve {
        samples: Vec<[f64; 3]>,
        hausdorff_bound: f64,
        param_pairs: Vec<[f64; 4]>,
    },
}

fn is_uniform_bicubic_positive(surface: &Surface) -> bool {
    surface.degree_u == 3 && surface.degree_v == 3 && is_bezier_le3_positive(surface)
}

fn is_single_span_clamped(knots: &[f64], degree: usize) -> bool {
    let n = degree + 1;
    knots.len() == 2 * n
        && knots[..n]
            .iter()
            .all(|k| k.is_finite() && (*k - 0.).abs() <= 1e-15)
        && knots[n..]
            .iter()
            .all(|k| k.is_finite() && (*k - 1.).abs() <= 1e-15)
}

/// Phase B positive cell: non-periodic, unit-weight, Bezier deg∈{1,2,3}×{1,2,3}.
fn is_bezier_le3_positive(surface: &Surface) -> bool {
    if surface.periodic_u || surface.periodic_v {
        return false;
    }
    if !(1..=3).contains(&surface.degree_u) || !(1..=3).contains(&surface.degree_v) {
        return false;
    }
    let nu = surface.degree_u + 1;
    let nv = surface.degree_v + 1;
    if surface.control_points.len() != nu
        || surface.control_points.iter().any(|row| row.len() != nv)
        || surface.weights.len() != nu
        || surface.weights.iter().any(|row| row.len() != nv)
    {
        return false;
    }
    if surface
        .weights
        .iter()
        .flatten()
        .any(|w| !w.is_finite() || (*w - 1.).abs() > 1e-15)
    {
        return false;
    }
    is_single_span_clamped(&surface.knots_u, surface.degree_u)
        && is_single_span_clamped(&surface.knots_v, surface.degree_v)
}

pub(crate) fn is_nurbs_boolean_candidate(a: &Model, b: &Model) -> bool {
    a.faces
        .iter()
        .chain(&b.faces)
        .all(|face| is_bezier_le3_positive(&face.surface))
}

fn bbox(s: &Surface) -> ([f64; 3], [f64; 3]) {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for p in s.control_points.iter().flatten() {
        for i in 0..3 {
            min[i] = min[i].min(p[i]);
            max[i] = max[i].max(p[i]);
        }
    }
    (min, max)
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(a: [f64; 3]) -> f64 {
    a[0].hypot(a[1]).hypot(a[2])
}
fn normalize(a: [f64; 3]) -> Option<[f64; 3]> {
    let n = norm(a);
    if !n.is_finite() || n == 0. {
        None
    } else {
        Some([a[0] / n, a[1] / n, a[2] / n])
    }
}

/// Recover a supporting plane when all control points are coplanar.
fn planar_support(surface: &Surface, tol: f64) -> Option<([f64; 3], [f64; 3])> {
    let pts: Vec<[f64; 3]> = surface
        .control_points
        .iter()
        .flatten()
        .filter_map(|p| {
            if p.len() >= 3 {
                Some([p[0], p[1], p[2]])
            } else {
                None
            }
        })
        .collect();
    if pts.len() < 3 {
        return None;
    }
    let o = pts[0];
    let mut normal = None;
    for i in 1..pts.len() {
        for j in (i + 1)..pts.len() {
            let n = cross(sub(pts[i], o), sub(pts[j], o));
            if norm(n) > tol {
                normal = normalize(n);
                break;
            }
        }
        if normal.is_some() {
            break;
        }
    }
    let n = normal?;
    for p in &pts {
        if dot(sub(*p, o), n).abs() > tol {
            return None;
        }
    }
    Some((o, n))
}

fn complete_empty() -> Report<G6Component> {
    let mut report = Report::default();
    report.coverage = Coverage::Complete;
    report.boxes_visited = 1;
    report.components.push(G6Component::Empty);
    report
}

fn complete_line(start: [f64; 3], end: [f64; 3]) -> Report<G6Component> {
    let mut report = Report::default();
    report.coverage = Coverage::Complete;
    report.boxes_visited = 1;
    report.components.push(G6Component::Line { start, end });
    report
}

fn complete_curve(
    samples: Vec<[f64; 3]>,
    hausdorff_bound: f64,
    param_pairs: Vec<[f64; 4]>,
) -> Report<G6Component> {
    let mut report = Report::default();
    report.coverage = Coverage::Complete;
    report.boxes_visited = samples.len().max(1);
    report.components.push(G6Component::Curve {
        samples,
        hausdorff_bound,
        param_pairs,
    });
    report
}

fn affine_plane_uv(surface: &Surface, p: [f64; 3], tol: f64) -> Result<Option<[f64; 2]>> {
    let point3 = |u: f64, v: f64| -> Result<[f64; 3]> {
        let q = surface.evaluate(u, v)?.point;
        Ok([q[0], q[1], q[2]])
    };
    let o = point3(0., 0.)?;
    let eu = sub(point3(1., 0.)?, o);
    let ev = sub(point3(0., 1.)?, o);
    let rhs = sub(p, o);
    let uu = dot(eu, eu);
    let uv = dot(eu, ev);
    let vv = dot(ev, ev);
    let det = uu * vv - uv * uv;
    if det.abs() <= 1e-18 {
        return Ok(None);
    }
    let ru = dot(rhs, eu);
    let rv = dot(rhs, ev);
    let u = (ru * vv - rv * uv) / det;
    let v = (rv * uu - ru * uv) / det;
    if u < -tol || u > 1. + tol || v < -tol || v > 1. + tol {
        return Ok(None);
    }
    let uvp = [u.clamp(0., 1.), v.clamp(0., 1.)];
    let q = point3(uvp[0], uvp[1])?;
    if norm(sub(p, q)) > tol {
        return Ok(None);
    }
    Ok(Some(uvp))
}

/// Certify the intersection of a bicubic with a planar patch when the signed
/// distance control coefficients are constant in V and affine in U. In that
/// finite cell the plane preimage is exactly U=constant; endpoint and sampled
/// affine-plane inversion then prove the full iso-curve lies in both domains.
fn try_exact_planar_iso_curve(
    surface: &Surface,
    plane: &Surface,
    plane_origin: [f64; 3],
    plane_normal: [f64; 3],
    tol: f64,
    swapped: bool,
) -> Result<Option<Report<G6Component>>> {
    if surface.degree_u != 3 || surface.degree_v != 3 {
        return Ok(None);
    }
    // Try both exact preimages: U=constant and V=constant.
    for fixed_u in [true, false] {
        let mut d = [0.; 4];
        let mut valid = true;
        for i in 0..4 {
            let q0 = if fixed_u {
                &surface.control_points[i][0]
            } else {
                &surface.control_points[0][i]
            };
            d[i] = dot(sub([q0[0], q0[1], q0[2]], plane_origin), plane_normal);
            for j in 1..4 {
                let q = if fixed_u {
                    &surface.control_points[i][j]
                } else {
                    &surface.control_points[j][i]
                };
                let dj = dot(sub([q[0], q[1], q[2]], plane_origin), plane_normal);
                if (dj - d[i]).abs() > tol * 0.1 {
                    valid = false;
                    break;
                }
            }
            if !valid {
                break;
            }
        }
        let span = d[3] - d[0];
        if !valid || span.abs() <= tol || d[0] * d[3] > 0. {
            continue;
        }
        if d.iter().enumerate().any(|(i, actual)| {
            let expected = d[0] + span * (i as f64 / 3.);
            (*actual - expected).abs() > tol * 0.1
        }) {
            continue;
        }
        let fixed = (-d[0] / span).clamp(0., 1.);
        let mut samples = Vec::with_capacity(17);
        let mut param_pairs = Vec::with_capacity(17);
        let mut max_residual = 0_f64;
        for i in 0..=16 {
            let varying = i as f64 / 16.;
            let surface_uv = if fixed_u {
                [fixed, varying]
            } else {
                [varying, fixed]
            };
            let p = surface.evaluate(surface_uv[0], surface_uv[1])?.point;
            let p3 = [p[0], p[1], p[2]];
            let Some(plane_uv) = affine_plane_uv(plane, p3, tol)? else {
                valid = false;
                break;
            };
            let q = plane.evaluate(plane_uv[0], plane_uv[1])?.point;
            max_residual = max_residual.max(norm(sub(p3, [q[0], q[1], q[2]])));
            samples.push(p3);
            param_pairs.push(if swapped {
                [plane_uv[0], plane_uv[1], surface_uv[0], surface_uv[1]]
            } else {
                [surface_uv[0], surface_uv[1], plane_uv[0], plane_uv[1]]
            });
        }
        if !valid {
            continue;
        }
        let report = complete_curve(samples, max_residual.max(tol), param_pairs);
        verify_complete_report(&report, false)?;
        return Ok(Some(report));
    }
    Ok(None)
}

/// Fail-closed G6 slice: separated Complete-empty, exact planar intersections,
/// and exact plane preimages on admitted Bezier patches. Generic sampled
/// surface/surface proximity never publishes Complete.
pub fn narrow_transverse_bicubic(
    a: &Surface,
    b: &Surface,
    options: Options,
) -> Result<Report<G6Component>> {
    let options = options.validate()?;
    a.validate()?;
    b.validate()?;
    if G6_MATURITY == G6Maturity::Unavailable {
        return Err(refuse("G6 capability is Unavailable per R1 stop"));
    }
    let a_elev = as_uniform_bicubic(a);
    let b_elev = as_uniform_bicubic(b);
    let (Some(a), Some(b)) = (a_elev.as_ref(), b_elev.as_ref()) else {
        let mut report = Report::default();
        report.unresolved(
            [
                a.knots_u.first().copied().unwrap_or(0.),
                a.knots_u.last().copied().unwrap_or(1.),
                b.knots_u.first().copied().unwrap_or(0.),
                b.knots_u.last().copied().unwrap_or(1.),
            ],
            UnresolvedReason::UnsupportedSurface,
        );
        return Ok(report);
    };
    let (amin, amax) = bbox(a);
    let (bmin, bmax) = bbox(b);
    let separated = (0..3).any(|i| amax[i] < bmin[i] - 1e-9 || bmax[i] < amin[i] - 1e-9);
    if separated {
        return Ok(complete_empty());
    }

    let tol = options.distance_tolerance.max(1e-9);
    let same_cage =
        (0..3).all(|i| (amin[i] - bmin[i]).abs() <= tol && (amax[i] - bmax[i]).abs() <= tol);
    if same_cage {
        let mut report = Report::default();
        report.unresolved(
            vec![0., 1., 0., 1., 0., 1., 0., 1.],
            UnresolvedReason::NearCoincidence,
        );
        return Ok(report);
    }

    if let (Some((oa, na)), Some((ob, nb))) = (planar_support(a, tol), planar_support(b, tol)) {
        let parallel = norm(cross(na, nb)) <= 1e-9;
        if parallel {
            let d = dot(sub(ob, oa), na).abs();
            if d <= tol {
                let mut report = Report::default();
                report.unresolved(
                    vec![0., 1., 0., 1., 0., 1., 0., 1.],
                    UnresolvedReason::NearCoincidence,
                );
                return Ok(report);
            }
            let mut report = Report::default();
            report.unresolved(
                vec![0., 1., 0., 1., 0., 1., 0., 1.],
                UnresolvedReason::TangencyOrMultipleRoot,
            );
            return Ok(report);
        }
        let dir = normalize(cross(na, nb)).ok_or_else(|| refuse("Degenerate plane normals"))?;
        let center = [
            (amin[0].max(bmin[0]) + amax[0].min(bmax[0])) * 0.5,
            (amin[1].max(bmin[1]) + amax[1].min(bmax[1])) * 0.5,
            (amin[2].max(bmin[2]) + amax[2].min(bmax[2])) * 0.5,
        ];
        let da = -dot(sub(center, oa), na);
        let db = -dot(sub(center, ob), nb);
        let nn = dot(na, nb);
        let denom = 1. - nn * nn;
        if denom.abs() <= 1e-15 {
            let mut report = Report::default();
            report.unresolved(
                vec![0., 1., 0., 1., 0., 1., 0., 1.],
                UnresolvedReason::NearCoincidence,
            );
            return Ok(report);
        }
        let alpha = (da - db * nn) / denom;
        let beta = (db - da * nn) / denom;
        let point = [
            center[0] + alpha * na[0] + beta * nb[0],
            center[1] + alpha * na[1] + beta * nb[1],
            center[2] + alpha * na[2] + beta * nb[2],
        ];
        let extent = 0.5
            * ((amax[0] - amin[0])
                .hypot(amax[1] - amin[1])
                .hypot(amax[2] - amin[2])
                .max(1.));
        let start = [
            point[0] - dir[0] * extent,
            point[1] - dir[1] * extent,
            point[2] - dir[2] * extent,
        ];
        let end = [
            point[0] + dir[0] * extent,
            point[1] + dir[1] * extent,
            point[2] + dir[2] * extent,
        ];
        let report = complete_line(start, end);
        verify_complete_report(&report, false)?;
        return Ok(report);
    }

    let pa = planar_support(a, tol);
    let pb = planar_support(b, tol);
    if pa.is_none() {
        if let Some((origin, normal)) = pb {
            if let Some(report) = try_exact_planar_iso_curve(a, b, origin, normal, tol, false)? {
                return Ok(report);
            }
        }
    } else if pb.is_none() {
        if let Some((origin, normal)) = pa {
            if let Some(report) = try_exact_planar_iso_curve(b, a, origin, normal, tol, true)? {
                return Ok(report);
            }
        }
    }
    let mut report = Report::default();
    report.unresolved(
        vec![0., 1., 0., 1., 0., 1., 0., 1.],
        UnresolvedReason::NearCoincidence,
    );
    Ok(report)
}

/// Alias for [`narrow_transverse_bicubic`] under the Phase B capability id.
pub fn narrow_transverse_bezier_le3(
    a: &Surface,
    b: &Surface,
    options: Options,
) -> Result<Report<G6Component>> {
    narrow_transverse_bicubic(a, b, options)
}

/// Aggregate proof for the finite `/2` contacting-solid cell.
#[derive(Clone, Debug)]
pub struct NurbsBooleanImprintCertificate {
    pub capability: &'static str,
    pub complete: bool,
    pub boolean_imprint: bool,
    pub notes: Vec<&'static str>,
    pub curve_component: Option<G6Component>,
    pub operation: String,
    pub context: ToleranceSpecIdentity,
    pub evidence: ComposedEvidence,
    pub arrangements: Vec<ClassificationCertificate>,
    pub sew: SewCertificate,
    pub audit: SolidAuditCertificate,
    pub change_set: ChangeSet,
    pub no_healing: bool,
    pub no_curved_prism_authorship: bool,
}

impl NurbsBooleanImprintCertificate {
    pub fn permits_topology_change(&self) -> bool {
        self.capability == NURBS_BOOLEAN_CAPABILITY
            && self.complete
            && self.boolean_imprint
            && matches!(
                self.operation.as_str(),
                "union" | "difference" | "intersection"
            )
            && self.context == self.evidence.context
            && !self.arrangements.is_empty()
            && self
                .arrangements
                .iter()
                .all(|cert| cert.complete && cert.chart == ChartKind::PlanePoly)
            && self
                .evidence
                .claims
                .iter()
                .any(|claim| matches!(claim, EvidenceClaim::Positional { .. }))
            && self.evidence.claims.iter().any(|claim| {
                matches!(
                    claim,
                    EvidenceClaim::TopologyPreservation { invariant }
                        if invariant == "profile_arrangement_split_retrim_sew_audit"
                )
            })
            && self.sew.complete
            && self.sew.displacement_budget_ok
            && self.sew.matched == self.audit.sew.matched
            && self.sew.displacement_budget_ok == self.audit.sew.displacement_budget_ok
            && self.audit.ok
            && self.audit.sew.complete
            && self.no_healing
            && self.no_curved_prism_authorship
            && !self.change_set.changes.is_empty()
            && self.change_set.validate().is_ok()
    }
}

fn mix_pt(a: &[f64], b: &[f64], t: f64) -> Vec<f64> {
    a.iter()
        .zip(b.iter())
        .map(|(u, v)| (1. - t) * u + t * v)
        .collect()
}

/// Elevate an admitted Bezier-le3 patch (or already-uniform bicubic) to R1 bicubic.
/// Keeps the historical bilinear 1×1 elevation path; deg 2 and mixed use Bernstein
/// elevation on U then V until 3×3.
fn as_uniform_bicubic(surface: &Surface) -> Option<Surface> {
    if is_uniform_bicubic_positive(surface) {
        return Some(surface.clone());
    }
    if !is_bezier_le3_positive(surface) {
        return None;
    }
    if surface.degree_u == 1 && surface.degree_v == 1 {
        return elevate_bilinear_1x1(surface);
    }
    let elevated = surface
        .edit_axis(Axis::U, |c| c.elevate(3))
        .ok()?
        .edit_axis(Axis::V, |c| c.elevate(3))
        .ok()?;
    if is_uniform_bicubic_positive(&elevated) {
        Some(elevated)
    } else {
        None
    }
}

fn elevate_bilinear_1x1(surface: &Surface) -> Option<Surface> {
    let p00 = &surface.control_points[0][0];
    let p10 = &surface.control_points[0][1];
    let p01 = &surface.control_points[1][0];
    let p11 = &surface.control_points[1][1];
    let row = |a: &[f64], b: &[f64]| -> Vec<Vec<f64>> {
        vec![
            a.to_vec(),
            mix_pt(a, b, 1. / 3.),
            mix_pt(a, b, 2. / 3.),
            b.to_vec(),
        ]
    };
    let r0 = row(p00, p10);
    let r1 = row(p01, p11);
    let mut control_points = Vec::with_capacity(4);
    for j in 0..4 {
        let t = j as f64 / 3.;
        control_points.push((0..4).map(|i| mix_pt(&r0[i], &r1[i], t)).collect());
    }
    Some(Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points,
        weights: vec![vec![1.; 4]; 4],
        periodic_u: false,
        periodic_v: false,
    })
}

pub fn nurbs_boolean_transverse_bicubic(
    a: &Surface,
    b: &Surface,
    options: Options,
) -> Result<NurbsBooleanImprintCertificate> {
    let _ = narrow_transverse_bicubic(a, b, options)?;
    Err(refuse(
        "nurbs-boolean-bezier-le3/1 is Unavailable; use the finite /2 contacting-solid author",
    ))
}

fn model_aabb(model: &Model) -> ([f64; 3], [f64; 3]) {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for v in &model.vertices {
        for i in 0..3 {
            min[i] = min[i].min(v.point[i]);
            max[i] = max[i].max(v.point[i]);
        }
    }
    (min, max)
}

fn affine_planar_carrier(model: &Model) -> Result<Model> {
    let tolerance = model.tolerance_mm.max(1e-9);
    let mut carrier = model.clone();
    for (face_index, face) in model.faces.iter().enumerate() {
        let surface = &face.surface;
        if !is_bezier_le3_positive(surface) || planar_support(surface, tolerance).is_none() {
            return Err(refuse(
                "NURBS /2 requires non-periodic unit-weight single-span planar faces of degree <=3",
            ));
        }
        let du = surface.degree_u;
        let dv = surface.degree_v;
        let p00 = &surface.control_points[0][0];
        let p10 = &surface.control_points[du][0];
        let p01 = &surface.control_points[0][dv];
        let p11 = &surface.control_points[du][dv];
        for i in 0..=du {
            for j in 0..=dv {
                let u = i as f64 / du as f64;
                let v = j as f64 / dv as f64;
                let expected = (0..3)
                    .map(|axis| {
                        (1. - u) * (1. - v) * p00[axis]
                            + u * (1. - v) * p10[axis]
                            + (1. - u) * v * p01[axis]
                            + u * v * p11[axis]
                    })
                    .collect::<Vec<_>>();
                let actual = &surface.control_points[i][j];
                let residual = (0..3)
                    .map(|axis| (actual[axis] - expected[axis]).powi(2))
                    .sum::<f64>()
                    .sqrt();
                if residual > tolerance {
                    return Err(refuse(
                        "NURBS /2 refuses planar but non-affine Bezier parameterizations",
                    ));
                }
            }
        }
        carrier.faces[face_index].surface = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![p00.clone(), p01.clone()],
                vec![p10.clone(), p11.clone()],
            ],
            weights: vec![vec![1., 1.], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        };
    }
    carrier.validate()?;
    Ok(carrier)
}

fn strict_partial_contact(a: &Model, b: &Model) -> Result<()> {
    let (amin, amax) = model_aabb(a);
    let (bmin, bmax) = model_aabb(b);
    let tolerance = a.tolerance_mm.max(b.tolerance_mm).max(1e-9);
    let overlap = std::array::from_fn::<_, 3, _>(|axis| {
        amax[axis].min(bmax[axis]) - amin[axis].max(bmin[axis])
    });
    if overlap.iter().any(|width| *width <= tolerance * 8.) {
        return Err(refuse(
            "NURBS /2 requires strict positive-volume contact; tangent and gray-band contacts refuse",
        ));
    }
    let contains =
        |outer_min: [f64; 3], outer_max: [f64; 3], inner_min: [f64; 3], inner_max: [f64; 3]| {
            (0..3).all(|axis| {
                inner_min[axis] >= outer_min[axis] - tolerance
                    && inner_max[axis] <= outer_max[axis] + tolerance
            })
        };
    if contains(amin, amax, bmin, bmax) || contains(bmin, bmax, amin, amax) {
        return Err(refuse(
            "NURBS /2 admits partial contacting solids only; containment and identity remain unavailable",
        ));
    }
    Ok(())
}

/// `/2` successor cell: exact Boolean authorship for strict partial contact
/// between affine-planar Bezier profile prisms. Generic nonplanar contact,
/// empty algebra, containment, tangency, and fallback construction all refuse.
pub fn nurbs_boolean_imprint_solids(
    a: &Model,
    b: &Model,
    operation: &str,
) -> Result<(Model, NurbsBooleanImprintCertificate)> {
    a.validate()?;
    b.validate()?;
    if !matches!(operation, "union" | "difference" | "intersection") {
        return Err(refuse(
            "nurbs-boolean-bezier-le3/2 admits union, difference, or intersection only",
        ));
    }
    strict_partial_contact(a, b)?;
    let carrier_a = affine_planar_carrier(a)?;
    let carrier_b = affine_planar_carrier(b)?;
    let result =
        crate::profile_imprint::exact_planar_contact_boolean(&carrier_a, &carrier_b, operation)?;
    let audited = LocallyValidatedModel::new(result)?.audit()?;
    let audit = audited.certificate().clone();
    let result = audited.into_model();
    if result
        .faces
        .iter()
        .any(|face| !is_bezier_le3_positive(&face.surface))
    {
        return Err(refuse(
            "NURBS /2 result escaped the non-periodic unit-weight single-span degree<=3 cell",
        ));
    }
    let arrangements = (0..result.faces.len())
        .map(|face| crate::trim_sew::classify_face_outer_loop(&result, face, ChartKind::PlanePoly))
        .collect::<Result<Vec<_>>>()?;
    let context = result
        .tolerance_context()
        .map_err(|_| refuse("NURBS /2 result tolerance context is invalid"))?;
    let evidence = compose_predicate_evidence(
        &context,
        [
            PredicateEvidence::positional(&context, 0., 1.)?,
            PredicateEvidence::topology_preservation(
                &context,
                "profile_arrangement_split_retrim_sew_audit",
                true,
            )?,
        ],
    )?;
    let cert = NurbsBooleanImprintCertificate {
        capability: NURBS_BOOLEAN_CAPABILITY,
        complete: true,
        boolean_imprint: true,
        notes: vec![
            "finite_affine_planar_contact_cell",
            "exact_profile_arrangement",
            "split_retrim_sew_audit_complete",
        ],
        curve_component: None,
        operation: operation.into(),
        context: context.spec_identity(),
        evidence,
        arrangements,
        sew: audit.sew.clone(),
        audit,
        change_set: result.1.change_set.clone(),
        no_healing: true,
        no_curved_prism_authorship: true,
    };
    if !cert.permits_topology_change() {
        return Err(refuse(
            "NURBS Boolean certificate does not permit topology change",
        ));
    }
    Ok((result, cert))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coverage_verifier::{verify_complete_report, verify_incomplete_refusal};

    fn bicubic(z: f64, shift: f64) -> Surface {
        let row = |y: f64| {
            (0..4)
                .map(|i| vec![i as f64 + shift, y, z])
                .collect::<Vec<_>>()
        };
        Surface {
            degree_u: 3,
            degree_v: 3,
            knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![row(0.), row(1.), row(2.), row(3.)],
            weights: vec![vec![1.; 4]; 4],
            periodic_u: false,
            periodic_v: false,
        }
    }

    fn planar_yz(x: f64) -> Surface {
        let mut control_points = Vec::new();
        for iz in 0..4 {
            let z = iz as f64;
            control_points.push((0..4).map(|iy| vec![x, iy as f64, z]).collect());
        }
        Surface {
            degree_u: 3,
            degree_v: 3,
            knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points,
            weights: vec![vec![1.; 4]; 4],
            periodic_u: false,
            periodic_v: false,
        }
    }

    /// Bilinear planar XY rectangle (elevated to bicubic inside G6).
    fn bilinear_xy(z: f64) -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., z], vec![3., 0., z]],
                vec![vec![0., 3., z], vec![3., 3., z]],
            ],
            weights: vec![vec![1., 1.], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        }
    }

    fn bilinear_yz(x: f64) -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![x, 0., 0.], vec![x, 3., 0.]],
                vec![vec![x, 0., 3.], vec![x, 3., 3.]],
            ],
            weights: vec![vec![1., 1.], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        }
    }

    /// Planar bicubic in a 45° plane (z = x) for BC-01.
    fn planar_45() -> Surface {
        let mut control_points = Vec::new();
        for iu in 0..4 {
            let u = iu as f64;
            control_points.push(
                (0..4)
                    .map(|iv| {
                        let v = iv as f64;
                        vec![u, v, u]
                    })
                    .collect(),
            );
        }
        Surface {
            degree_u: 3,
            degree_v: 3,
            knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points,
            weights: vec![vec![1.; 4]; 4],
            periodic_u: false,
            periodic_v: false,
        }
    }

    fn bumped_xy() -> Surface {
        let mut s = bicubic(0., 0.);
        s.control_points[1][1][2] = 0.02;
        s
    }

    #[test]
    fn separated_bicubics_are_complete_empty() {
        let report =
            narrow_transverse_bicubic(&bicubic(0., 0.), &bicubic(0., 20.), Options::default())
                .unwrap();
        assert_eq!(report.coverage, Coverage::Complete);
        verify_complete_report(&report, true).unwrap();
        assert!(!report.permits_topology_change());
    }

    #[test]
    fn overlapping_bicubics_refuse_without_complete() {
        let report =
            narrow_transverse_bicubic(&bicubic(0., 0.), &bicubic(0., 0.5), Options::default())
                .unwrap();
        verify_incomplete_refusal(&report).unwrap();
    }

    #[test]
    fn degree_gt3_is_unsupported() {
        let mut s = bicubic(0., 0.);
        s.degree_u = 4;
        s.knots_u = vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.];
        s.control_points = vec![
            s.control_points[0].clone(),
            s.control_points[1].clone(),
            s.control_points[2].clone(),
            s.control_points[3].clone(),
            s.control_points[3].clone(),
        ];
        s.weights = vec![vec![1.; 4]; 5];
        let report = narrow_transverse_bicubic(&s, &bicubic(0., 20.), Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::Incomplete);
    }

    #[test]
    fn rational_weights_and_high_degree_refuse() {
        let mut s = bicubic(0., 0.);
        s.weights[0][0] = 2.;
        let report = narrow_transverse_bicubic(&s, &bicubic(0., 20.), Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::Incomplete);
    }

    #[test]
    fn periodic_bezier_le3_refuses() {
        let mut s = bicubic(0., 0.);
        s.periodic_u = true;
        assert!(!is_bezier_le3_positive(&s));
        assert!(as_uniform_bicubic(&s).is_none());
        assert!(
            narrow_transverse_bezier_le3(&s, &bicubic(0., 20.), Options::default()).is_err(),
            "invalid periodic storage must refuse before the product gate"
        );
    }

    #[test]
    fn g6_never_permits_topology_change() {
        let report =
            narrow_transverse_bicubic(&bicubic(0., 0.), &bicubic(0., 20.), Options::default())
                .unwrap();
        assert_eq!(report.coverage, Coverage::Complete);
        assert!(!report.permits_topology_change());
    }

    #[test]
    fn planar_transverse_bicubics_publish_complete_line() {
        let xy = bicubic(0., 0.);
        let yz = planar_yz(1.5);
        let report = narrow_transverse_bicubic(&xy, &yz, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::Complete);
        verify_complete_report(&report, false).unwrap();
        assert!(matches!(report.components[0], G6Component::Line { .. }));
        assert!(!report.permits_topology_change());
    }

    #[test]
    fn nurbs_boolean_imprint_on_planar_transverse() {
        let xy = bicubic(0., 0.);
        let yz = planar_yz(1.5);
        let error = nurbs_boolean_transverse_bicubic(&xy, &yz, Options::default()).unwrap_err();
        assert_eq!(error.code, "BREP_NURBS_SS_REFUSED");
        assert!(error.message.contains("finite /2 contacting-solid author"));
    }

    #[test]
    fn nurbs_boolean_refuses_out_of_matrix() {
        let mut s = bicubic(0., 0.);
        s.weights[0][0] = 2.;
        assert_eq!(
            nurbs_boolean_transverse_bicubic(&s, &bicubic(0., 20.), Options::default())
                .unwrap_err()
                .code,
            "BREP_NURBS_SS_REFUSED"
        );
    }

    #[test]
    fn nonplanar_bump_may_complete_or_typed_refuse() {
        let a = bumped_xy();
        let b = planar_yz(1.5);
        let report = narrow_transverse_bicubic(&a, &b, Options::default()).unwrap();
        if report.coverage == Coverage::Complete {
            verify_complete_report(&report, false).unwrap();
        } else {
            verify_incomplete_refusal(&report).unwrap();
        }
    }

    #[test]
    fn nurbs_solid_boolean_disjoint_cuboids_refuse_v2() {
        let a = crate::cuboid([0., 0., 0.], [2., 2., 2.]).unwrap();
        let b0 = crate::cuboid([0., 0., 0.], [2., 2., 2.]).unwrap();
        let b = crate::transform::affine(
            &b0,
            [
                [1., 0., 0., 10.],
                [0., 1., 0., 0.],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        assert_eq!(
            nurbs_boolean_imprint_solids(&a, &b, "union")
                .unwrap_err()
                .code,
            "BREP_NURBS_SS_REFUSED"
        );
    }

    #[test]
    fn nurbs_solid_boolean_contact_authors_topology() {
        let a = crate::cuboid([0., 0., 0.], [2., 2., 2.]).unwrap();
        let b0 = crate::cuboid([0., 0., 0.], [2., 2., 2.]).unwrap();
        let b = crate::transform::affine(
            &b0,
            [
                [1., 0., 0., 1.],
                [0., 1., 0., 0.],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let (model, cert) = nurbs_boolean_imprint_solids(&a, &b, "union").unwrap();
        assert!(cert.permits_topology_change());
        assert_eq!(cert.capability, "nurbs-boolean-bezier-le3/2");
        assert!(cert.audit.ok && cert.sew.complete);
        assert!(!cert.arrangements.is_empty());
        model.validate().unwrap();
    }

    #[test]
    fn ep01_elevated_planar_times_bicubic_complete_line() {
        let report =
            narrow_transverse_bicubic(&bilinear_xy(0.), &planar_yz(1.5), Options::default())
                .unwrap();
        assert_eq!(report.coverage, Coverage::Complete);
        verify_complete_report(&report, false).unwrap();
        assert!(matches!(report.components[0], G6Component::Line { .. }));
    }

    #[test]
    fn ep02_two_elevated_planar_transverse() {
        let report =
            narrow_transverse_bicubic(&bilinear_xy(0.), &bilinear_yz(1.5), Options::default())
                .unwrap();
        assert_eq!(report.coverage, Coverage::Complete);
        assert!(matches!(report.components[0], G6Component::Line { .. }));
    }

    #[test]
    fn bc01_planar_45_orientation_complete_line() {
        let xy = bicubic(0., 0.);
        let angled = planar_45();
        let report = narrow_transverse_bicubic(&xy, &angled, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::Complete);
        verify_complete_report(&report, false).unwrap();
        assert!(matches!(report.components[0], G6Component::Line { .. }));
    }

    #[test]
    fn bc02_nonplanar_bump_plane_publishes_certified_curve() {
        let a = bumped_xy();
        let b = planar_yz(1.5);
        let report = narrow_transverse_bicubic(&a, &b, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::Complete);
        verify_complete_report(&report, false).unwrap();
        assert!(matches!(report.components[0], G6Component::Curve { .. }));

        let error = nurbs_boolean_transverse_bicubic(&a, &b, Options::default()).unwrap_err();
        assert_eq!(error.code, "BREP_NURBS_SS_REFUSED");
        assert!(error.message.contains("finite /2 contacting-solid author"));
    }

    #[test]
    fn ep03_containment_remains_unavailable() {
        let a = crate::cuboid([0., 0., 0.], [4., 4., 4.]).unwrap();
        let b0 = crate::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap();
        let b = crate::transform::affine(
            &b0,
            [
                [1., 0., 0., 1.],
                [0., 1., 0., 1.],
                [0., 0., 1., 1.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        assert!(nurbs_boolean_imprint_solids(&a, &b, "difference").is_err());
    }

    #[test]
    fn bezier_le3_solid_boolean_all_operations_contact() {
        let outer = crate::cuboid([0., 0., 0.], [3., 3., 3.]).unwrap();
        let inner0 = crate::cuboid([0., 0., 0.], [3., 3., 3.]).unwrap();
        let inner = crate::transform::affine(
            &inner0,
            [
                [1., 0., 0., 1.],
                [0., 1., 0., 0.5],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        for operation in ["union", "difference", "intersection"] {
            let (model, cert) = nurbs_boolean_imprint_solids(&outer, &inner, operation).unwrap();
            assert!(cert.permits_topology_change(), "{operation}");
            model.validate().unwrap();
            assert_eq!(cert.operation, operation);
        }
    }

    #[test]
    fn bezier_le3_solid_boolean_refuses_any_out_of_matrix_face() {
        let mut a = crate::cuboid([0., 0., 0.], [4., 4., 4.]).unwrap();
        let b = crate::cuboid([1., 1., 1.], [2., 2., 2.]).unwrap();
        a.faces[0].surface = a.faces[0]
            .surface
            .edit_axis(Axis::U, |curve| curve.elevate(4))
            .unwrap();
        a.validate().unwrap();
        assert_eq!(
            nurbs_boolean_imprint_solids(&a, &b, "union")
                .unwrap_err()
                .code,
            "BREP_NURBS_SS_REFUSED"
        );
    }

    fn elevate_model_faces(mut model: Model) -> Model {
        for face in &mut model.faces {
            face.surface = face
                .surface
                .edit_axis(Axis::U, |curve| curve.elevate(3))
                .unwrap()
                .edit_axis(Axis::V, |curve| curve.elevate(3))
                .unwrap();
        }
        model.validate().unwrap();
        model
    }

    #[test]
    fn elevated_planar_contact_survives_transform_and_scale() {
        let a0 = crate::cuboid([0., 0., 0.], [2., 3., 2.]).unwrap();
        let b0 = crate::cuboid([0., 0., 0.], [2., 3., 2.]).unwrap();
        let matrix = [
            [2., 0., 0., 10.],
            [0., 1.5, 0., -4.],
            [0., 0., 0.5, 3.],
            [0., 0., 0., 1.],
        ];
        let a = elevate_model_faces(crate::transform::affine(&a0, matrix).unwrap());
        let shifted = crate::transform::affine(
            &b0,
            [
                [1., 0., 0., 1.],
                [0., 1., 0., 0.5],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let b = elevate_model_faces(crate::transform::affine(&shifted, matrix).unwrap());
        let (result, cert) = nurbs_boolean_imprint_solids(&a, &b, "intersection").unwrap();
        assert!(cert.permits_topology_change());
        assert!(
            result
                .faces
                .iter()
                .all(|face| is_bezier_le3_positive(&face.surface))
        );
        result.validate().unwrap();
        crate::operations::boolean(&a, &b, "union")
            .unwrap()
            .validate()
            .unwrap();
    }

    #[test]
    fn tangent_and_gray_band_contacts_refuse() {
        let a = crate::cuboid([0., 0., 0.], [2., 2., 2.]).unwrap();
        for shift in [2., 2. - 4e-7] {
            let b = crate::transform::affine(
                &crate::cuboid([0., 0., 0.], [2., 2., 2.]).unwrap(),
                [
                    [1., 0., 0., shift],
                    [0., 1., 0., 0.],
                    [0., 0., 1., 0.],
                    [0., 0., 0., 1.],
                ],
            )
            .unwrap();
            assert!(nurbs_boolean_imprint_solids(&a, &b, "union").is_err());
        }
    }

    #[test]
    fn unequal_extrusion_spans_refuse_without_stepped_fallback() {
        let a = crate::cuboid([0., 0., 0.], [2., 2., 2.]).unwrap();
        let b = crate::cuboid([1., 0.5, 0.5], [3., 2.5, 2.5]).unwrap();
        let error = nurbs_boolean_imprint_solids(&a, &b, "union").unwrap_err();
        assert_eq!(error.code, "BREP_PROFILE_IMPRINT_REFUSED");
        assert!(error.message.contains("stepped fallback is forbidden"));
    }

    #[test]
    fn aggregate_certificate_mutations_revoke_authority() {
        let a = crate::cuboid([0., 0., 0.], [2., 2., 2.]).unwrap();
        let b = crate::transform::affine(
            &a,
            [
                [1., 0., 0., 1.],
                [0., 1., 0., 0.5],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let (_, cert) = nurbs_boolean_imprint_solids(&a, &b, "difference").unwrap();
        assert!(cert.permits_topology_change());
        let mut mutated = cert.clone();
        mutated.no_healing = false;
        assert!(!mutated.permits_topology_change());
        let mut mutated = cert.clone();
        mutated.arrangements[0].complete = false;
        assert!(!mutated.permits_topology_change());
        let mut mutated = cert.clone();
        mutated.evidence.claims.clear();
        assert!(!mutated.permits_topology_change());
        let mut mutated = cert;
        mutated.change_set.changes.clear();
        assert!(!mutated.permits_topology_change());
    }

    #[test]
    fn nonplanar_contact_typed_refuses_without_curved_prism_fallback() {
        let mut a = elevate_model_faces(crate::cuboid([0., 0., 0.], [2., 2., 2.]).unwrap());
        a.faces[0].surface.control_points[1][1][2] += 0.01;
        a.validate().unwrap();
        let b = crate::transform::affine(
            &crate::cuboid([0., 0., 0.], [2., 2., 2.]).unwrap(),
            [
                [1., 0., 0., 1.],
                [0., 1., 0., 0.5],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let error = nurbs_boolean_imprint_solids(&a, &b, "union").unwrap_err();
        assert_eq!(error.code, "BREP_NURBS_SS_REFUSED");
    }

    fn bezier_le3_xy(du: usize, dv: usize, z: f64, shift: f64) -> Surface {
        let nu = du + 1;
        let nv = dv + 1;
        let mut control_points = Vec::with_capacity(nu);
        for iu in 0..nu {
            let u = iu as f64 * 3. / du.max(1) as f64 + shift;
            control_points.push(
                (0..nv)
                    .map(|iv| {
                        let v = iv as f64 * 3. / dv.max(1) as f64;
                        vec![u, v, z]
                    })
                    .collect(),
            );
        }
        Surface {
            degree_u: du,
            degree_v: dv,
            knots_u: [vec![0.; nu], vec![1.; nu]].concat(),
            knots_v: [vec![0.; nv], vec![1.; nv]].concat(),
            control_points,
            weights: vec![vec![1.; nv]; nu],
            periodic_u: false,
            periodic_v: false,
        }
    }

    fn bezier_le3_yz(du: usize, dv: usize, x: f64) -> Surface {
        let nu = du + 1;
        let nv = dv + 1;
        let mut control_points = Vec::with_capacity(nu);
        for iu in 0..nu {
            let y = iu as f64 * 3. / du.max(1) as f64;
            control_points.push(
                (0..nv)
                    .map(|iv| {
                        let z = iv as f64 * 3. / dv.max(1) as f64;
                        vec![x, y, z]
                    })
                    .collect(),
            );
        }
        Surface {
            degree_u: du,
            degree_v: dv,
            knots_u: [vec![0.; nu], vec![1.; nu]].concat(),
            knots_v: [vec![0.; nv], vec![1.; nv]].concat(),
            control_points,
            weights: vec![vec![1.; nv]; nu],
            periodic_u: false,
            periodic_v: false,
        }
    }

    fn bumped_bezier_xy(du: usize, dv: usize) -> Surface {
        let mut s = bezier_le3_xy(du, dv, 0., 0.);
        let iu = (du / 2).max(0).min(du);
        let iv = (dv / 2).max(0).min(dv);
        s.control_points[iu][iv][2] = 0.02;
        s
    }

    const LE3_DEGREE_PAIRS: &[(usize, usize)] = &[
        (1, 1),
        (1, 2),
        (1, 3),
        (2, 1),
        (2, 2),
        (2, 3),
        (3, 1),
        (3, 2),
        (3, 3),
    ];

    #[test]
    fn bezier_le3_matrix_separated_complete_empty() {
        for &(du, dv) in LE3_DEGREE_PAIRS {
            for &(eu, ev) in LE3_DEGREE_PAIRS {
                let report = narrow_transverse_bezier_le3(
                    &bezier_le3_xy(du, dv, 0., 0.),
                    &bezier_le3_xy(eu, ev, 0., 20.),
                    Options::default(),
                )
                .unwrap();
                assert_eq!(
                    report.coverage,
                    Coverage::Complete,
                    "empty expected for ({du}×{dv})×({eu}×{ev})"
                );
                verify_complete_report(&report, true).unwrap();
            }
        }
    }

    #[test]
    fn bezier_le3_matrix_planar_transverse_complete_line() {
        for &(du, dv) in LE3_DEGREE_PAIRS {
            for &(eu, ev) in LE3_DEGREE_PAIRS {
                let report = narrow_transverse_bezier_le3(
                    &bezier_le3_xy(du, dv, 0., 0.),
                    &bezier_le3_yz(eu, ev, 1.5),
                    Options::default(),
                )
                .unwrap();
                assert_eq!(
                    report.coverage,
                    Coverage::Complete,
                    "line expected for ({du}×{dv})×yz({eu}×{ev})"
                );
                verify_complete_report(&report, false).unwrap();
                assert!(matches!(report.components[0], G6Component::Line { .. }));
            }
        }
    }

    #[test]
    fn bezier_le3_nonplanar_certified_curve_matrix() {
        for &(du, dv) in &[(2usize, 2), (2, 3), (3, 3)] {
            let report = narrow_transverse_bezier_le3(
                &bumped_bezier_xy(du, dv),
                &bezier_le3_yz(3, 3, 1.5),
                Options::default(),
            )
            .unwrap();
            assert_eq!(report.coverage, Coverage::Complete, "degree {du}x{dv}");
            verify_complete_report(&report, false).unwrap();
            assert!(matches!(report.components[0], G6Component::Curve { .. }));
        }
    }

    #[test]
    fn is_bezier_le3_positive_accepts_mixed_degrees() {
        assert!(is_bezier_le3_positive(&bezier_le3_xy(2, 3, 0., 0.)));
        assert!(is_bezier_le3_positive(&bezier_le3_xy(1, 1, 0., 0.)));
        assert!(as_uniform_bicubic(&bezier_le3_xy(2, 3, 0., 0.)).is_some());
        assert!(as_uniform_bicubic(&bezier_le3_xy(2, 2, 0., 0.)).is_some());
    }
}
