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
use crate::trim_sew::{
    ChartKind, ClassificationCertificate, RationalCurveDefinition, SewCertificate,
};
use crate::{
    ChangeKind, ChangeProvenance, ChangeSet, Model, TopoId, TopoKind, TopologyChange,
    TopologyLineageRecord,
};
use cad_predicates::{ToleranceContext, ToleranceSpecIdentity};
use nurbs_core::surface::Axis;
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};
use std::collections::{BTreeMap, BTreeSet};

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
pub const NURBS_BOOLEAN_CAPABILITY_V3: &str = "nurbs-boolean-bezier-le3/3";
pub const NURBS_BOOLEAN_CAPABILITY_V4: &str = "nurbs-boolean-bezier-le3/4";
pub const NURBS_BOOLEAN_CAPABILITY_V5: &str = "nurbs-boolean-bezier-le3/5";
pub const NURBS_BOOLEAN_CAPABILITY_V7: &str = "nurbs-boolean-bezier-le3/7";
pub const NURBS_BOOLEAN_CAPABILITY_V8: &str = "nurbs-boolean-bezier-le3/8";
/// Product Boolean successor over nurbs-ss/1 for the admitted multispan×affine cell.
pub const NURBS_BOOLEAN_SS_CAPABILITY: &str = "nurbs-boolean/1";
/// Authority for nurbs-boolean/1 general authorship. Distinct from finite `/8`.
pub const GENERAL_NURBS_BOOLEAN_AUTHORITY: &str = "author-general-nurbs-boolean";
pub const NURBS_BOOLEAN_V1_MATURITY: G6Maturity = G6Maturity::Unavailable;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExactIsoAxis {
    U,
    V,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IsoEndpointLocation {
    Interior,
    UMin,
    UMax,
    VMin,
    VMax,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IsoEndpointOwnership {
    pub first_support: [IsoEndpointLocation; 2],
    pub second_support: [IsoEndpointLocation; 2],
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExactIsoAuthority {
    curve: RationalCurveDefinition,
    traces: [RationalCurveDefinition; 2],
    axis: ExactIsoAxis,
    fixed_parameter_bits: u64,
    transverse_slope_bits: u64,
    endpoints: IsoEndpointOwnership,
    context: ToleranceSpecIdentity,
}

/// Topology-grade authority for the finite graph-patch/affine-plane iso cell.
/// The samples are diagnostics only; every authorizing field is retained exactly.
#[derive(Clone, Debug)]
pub struct ExactIsoIntersectionCertificate {
    pub curve: Curve,
    pub uv_traces: [Curve; 2],
    pub first_axis: ExactIsoAxis,
    pub fixed_parameter: f64,
    pub transverse_signed_distance_slope: f64,
    pub endpoints: IsoEndpointOwnership,
    pub whole_domain_contained: bool,
    pub strict_transverse: bool,
    pub context: ToleranceSpecIdentity,
    pub evidence: ComposedEvidence,
    pub diagnostic_samples: Vec<([f64; 3], [f64; 2], [f64; 2])>,
    authority: ExactIsoAuthority,
}

impl ExactIsoIntersectionCertificate {
    pub fn permits_topology_authorship(&self) -> bool {
        let Ok(curve) = RationalCurveDefinition::from_curve(&self.curve) else {
            return false;
        };
        let Ok(first) = RationalCurveDefinition::from_curve(&self.uv_traces[0]) else {
            return false;
        };
        let Ok(second) = RationalCurveDefinition::from_curve(&self.uv_traces[1]) else {
            return false;
        };
        self.whole_domain_contained
            && self.strict_transverse
            && self.transverse_signed_distance_slope.is_finite()
            && self.transverse_signed_distance_slope != 0.
            && self.context == self.evidence.context
            && self
                .evidence
                .claims
                .iter()
                .any(|claim| matches!(claim, EvidenceClaim::TangentNormal { .. }))
            && self.evidence.claims.iter().any(|claim| {
                matches!(
                    claim,
                    EvidenceClaim::TopologyPreservation { invariant }
                        if invariant == "exact_iso_whole_domain_and_boundary_ownership"
                )
            })
            && self.authority
                == ExactIsoAuthority {
                    curve,
                    traces: [first, second],
                    axis: self.first_axis,
                    fixed_parameter_bits: self.fixed_parameter.to_bits(),
                    transverse_slope_bits: self.transverse_signed_distance_slope.to_bits(),
                    endpoints: self.endpoints.clone(),
                    context: self.context.clone(),
                }
    }
}

#[derive(Clone, Debug)]
pub enum G6Component {
    Empty,
    Line {
        start: [f64; 3],
        end: [f64; 3],
    },
    /// Exact retained iso branch. Samples are diagnostics and never authority.
    Curve {
        certificate: ExactIsoIntersectionCertificate,
        samples: Vec<[f64; 3]>,
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

fn rational_weight_bounds(surface: &Surface) -> Option<(f64, f64)> {
    if surface.periodic_u
        || surface.periodic_v
        || !(2..=3).contains(&surface.degree_u)
        || !(2..=3).contains(&surface.degree_v)
        || !is_single_span_clamped(&surface.knots_u, surface.degree_u)
        || !is_single_span_clamped(&surface.knots_v, surface.degree_v)
        || surface.control_points.len() != surface.degree_u + 1
        || surface
            .control_points
            .iter()
            .any(|row| row.len() != surface.degree_v + 1)
        || surface.weights.len() != surface.degree_u + 1
        || surface
            .weights
            .iter()
            .any(|row| row.len() != surface.degree_v + 1)
    {
        return None;
    }
    let min = surface
        .weights
        .iter()
        .flatten()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let max = surface
        .weights
        .iter()
        .flatten()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    (min.is_finite()
        && max.is_finite()
        && min >= 0.25
        && max <= 2.
        && max / min <= 8.
        && (max - min).abs() > 1e-15)
        .then_some((min, max))
}

fn positive_multispan_graph(surface: &Surface) -> bool {
    !surface.periodic_u
        && !surface.periodic_v
        && (1..=3).contains(&surface.degree_u)
        && (1..=3).contains(&surface.degree_v)
        && surface.validate().is_ok()
        && surface
            .weights
            .iter()
            .flatten()
            .all(|weight| weight.is_finite() && *weight > 0.)
        && active_breaks(
            &surface.knots_u,
            surface.degree_u,
            surface.control_points.len(),
        )
        .is_ok_and(|breaks| breaks.first() == Some(&0.) && breaks.last() == Some(&1.))
        && active_breaks(
            &surface.knots_v,
            surface.degree_v,
            surface.control_points[0].len(),
        )
        .is_ok_and(|breaks| breaks.first() == Some(&0.) && breaks.last() == Some(&1.))
}

fn rational_weights_constant_on_fixed_axis(surface: &Surface, fixed_u: bool) -> bool {
    if fixed_u {
        (0..=surface.degree_v).all(|j| {
            (1..=surface.degree_u)
                .all(|i| surface.weights[i][j].to_bits() == surface.weights[0][j].to_bits())
        })
    } else {
        (0..=surface.degree_u).all(|i| {
            (1..=surface.degree_v)
                .all(|j| surface.weights[i][j].to_bits() == surface.weights[i][0].to_bits())
        })
    }
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

fn complete_curve(certificate: ExactIsoIntersectionCertificate) -> Report<G6Component> {
    let mut report = Report::default();
    report.coverage = Coverage::Complete;
    report.boxes_visited = certificate.diagnostic_samples.len().max(1);
    let samples = certificate
        .diagnostic_samples
        .iter()
        .map(|sample| sample.0)
        .collect();
    report.components.push(G6Component::Curve {
        certificate,
        samples,
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

fn uv_location(uv: [f64; 2], tol: f64) -> Result<IsoEndpointLocation> {
    let on = [
        (uv[0].abs() <= tol, IsoEndpointLocation::UMin),
        ((uv[0] - 1.).abs() <= tol, IsoEndpointLocation::UMax),
        (uv[1].abs() <= tol, IsoEndpointLocation::VMin),
        ((uv[1] - 1.).abs() <= tol, IsoEndpointLocation::VMax),
    ];
    let hits = on.iter().filter(|(matches, _)| *matches).count();
    if hits > 1 {
        return Err(refuse(
            "Iso branch endpoint hits an ambiguous domain corner",
        ));
    }
    Ok(on
        .into_iter()
        .find_map(|(matches, location)| matches.then_some(location))
        .unwrap_or(IsoEndpointLocation::Interior))
}

fn affine_graph_projection(surface: &Surface, tol: f64) -> bool {
    for a in 0..3 {
        for b in (a + 1)..3 {
            let p00 = &surface.control_points[0][0];
            let pu = &surface.control_points[surface.degree_u][0];
            let pv = &surface.control_points[0][surface.degree_v];
            let du = [pu[a] - p00[a], pu[b] - p00[b]];
            let dv = [pv[a] - p00[a], pv[b] - p00[b]];
            let determinant = du[0] * dv[1] - du[1] * dv[0];
            if determinant.abs() <= tol {
                continue;
            }
            let mut exact_graph = true;
            for i in 0..=surface.degree_u {
                for j in 0..=surface.degree_v {
                    let u = i as f64 / surface.degree_u as f64;
                    let v = j as f64 / surface.degree_v as f64;
                    for axis in [a, b] {
                        let expected =
                            p00[axis] + u * (pu[axis] - p00[axis]) + v * (pv[axis] - p00[axis]);
                        if (surface.control_points[i][j][axis] - expected).abs() > tol {
                            exact_graph = false;
                        }
                    }
                }
            }
            if exact_graph {
                return true;
            }
        }
    }
    false
}

fn affine_uv_trace(surface: &Surface, curve: &Curve, tol: f64) -> Result<Option<Curve>> {
    let o = surface.evaluate(0., 0.)?.point;
    let u = sub(surface.evaluate(1., 0.)?.point, o);
    let v = sub(surface.evaluate(0., 1.)?.point, o);
    let corner = surface.evaluate(1., 1.)?.point;
    if norm(sub(
        corner,
        [o[0] + u[0] + v[0], o[1] + u[1] + v[1], o[2] + u[2] + v[2]],
    )) > tol
    {
        return Ok(None);
    }
    for i in 0..=surface.degree_u {
        for j in 0..=surface.degree_v {
            let expected = [
                o[0] + i as f64 / surface.degree_u as f64 * u[0]
                    + j as f64 / surface.degree_v as f64 * v[0],
                o[1] + i as f64 / surface.degree_u as f64 * u[1]
                    + j as f64 / surface.degree_v as f64 * v[1],
                o[2] + i as f64 / surface.degree_u as f64 * u[2]
                    + j as f64 / surface.degree_v as f64 * v[2],
            ];
            let actual = &surface.control_points[i][j];
            if norm(sub([actual[0], actual[1], actual[2]], expected)) > tol {
                return Ok(None);
            }
        }
    }
    let uu = dot(u, u);
    let uv = dot(u, v);
    let vv = dot(v, v);
    let determinant = uu * vv - uv * uv;
    if determinant.abs() <= tol * tol {
        return Ok(None);
    }
    let mut points = Vec::with_capacity(curve.control_points.len());
    for point in &curve.control_points {
        let rhs = sub([point[0], point[1], point[2]], o);
        let ru = dot(rhs, u);
        let rv = dot(rhs, v);
        points.push(vec![
            (ru * vv - rv * uv) / determinant,
            (rv * uu - ru * uv) / determinant,
        ]);
    }
    let trace = Curve {
        degree: curve.degree,
        knots: curve.knots.clone(),
        control_points: points,
        weights: curve.weights.clone(),
        periodic: false,
    };
    trace.validate()?;
    Ok(Some(trace))
}

/// Certify the intersection of a bicubic with a planar patch when the signed
/// distance control coefficients are constant in V and affine in U. In that
/// finite cell the plane preimage is exactly U=constant; endpoint and sampled
/// affine-plane inversion then prove the full iso-curve lies in both domains.
fn certify_exact_planar_iso_intersection_cell(
    surface: &Surface,
    plane: &Surface,
    context: &ToleranceContext,
    rational: bool,
) -> Result<ExactIsoIntersectionCertificate> {
    surface.validate()?;
    plane.validate()?;
    let graph_admitted = if rational {
        rational_weight_bounds(surface).is_some()
    } else {
        is_bezier_le3_positive(surface)
    };
    if !graph_admitted
        || !is_bezier_le3_positive(plane)
        || !affine_graph_projection(surface, context.spatial_bounds().clear_mm)
    {
        return Err(refuse(
            "Exact iso certification requires an admitted single-span degree<=3 graph and an affine unit-weight patch",
        ));
    }
    let tol = context.spatial_bounds().on_mm;
    let (plane_origin, plane_normal) =
        planar_support(plane, tol).ok_or_else(|| refuse("Second support is not planar"))?;
    // Try both exact preimages: U=constant and V=constant.
    for fixed_u in [true, false] {
        if rational && !rational_weights_constant_on_fixed_axis(surface, fixed_u) {
            continue;
        }
        let degree = if fixed_u {
            surface.degree_u
        } else {
            surface.degree_v
        };
        let varying_degree = if fixed_u {
            surface.degree_v
        } else {
            surface.degree_u
        };
        let mut d = vec![0.; degree + 1];
        let mut valid = true;
        for i in 0..=degree {
            let q0 = if fixed_u {
                &surface.control_points[i][0]
            } else {
                &surface.control_points[0][i]
            };
            d[i] = dot(sub([q0[0], q0[1], q0[2]], plane_origin), plane_normal);
            for j in 1..=varying_degree {
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
        let span = d[degree] - d[0];
        if !valid || span.abs() <= context.spatial_bounds().clear_mm || d[0] * d[degree] >= 0. {
            continue;
        }
        if d.iter().enumerate().any(|(i, actual)| {
            let expected = d[0] + span * (i as f64 / degree as f64);
            (*actual - expected).abs() > tol * 0.1
        }) {
            continue;
        }
        let fixed = -d[0] / span;
        let param_margin = context.parametric_bounds().floor.max(64. * f64::EPSILON);
        if !(fixed > param_margin && fixed < 1. - param_margin) {
            continue;
        }
        let axis = if fixed_u { Axis::U } else { Axis::V };
        let exact_curve = surface.iso(axis, fixed)?;
        if exact_curve.periodic || exact_curve.decompose()?.len() != 1 {
            continue;
        }
        let domain = exact_curve.domain();
        let first_trace = if rational {
            Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                control_points: if fixed_u {
                    vec![vec![fixed, 0.], vec![fixed, 1.]]
                } else {
                    vec![vec![0., fixed], vec![1., fixed]]
                },
                weights: vec![1., 1.],
                periodic: false,
            }
        } else {
            Curve {
                degree: exact_curve.degree,
                knots: exact_curve.knots.clone(),
                control_points: exact_curve
                    .control_points
                    .iter()
                    .enumerate()
                    .map(|(i, _)| {
                        let t = i as f64 / exact_curve.degree as f64;
                        if fixed_u {
                            vec![fixed, t]
                        } else {
                            vec![t, fixed]
                        }
                    })
                    .collect(),
                weights: exact_curve.weights.clone(),
                periodic: false,
            }
        };
        first_trace.validate()?;
        let Some(second_trace) = affine_uv_trace(plane, &exact_curve, tol)? else {
            continue;
        };
        if second_trace
            .control_points
            .iter()
            .flatten()
            .any(|value| !value.is_finite() || *value < 0. || *value > 1.)
        {
            continue;
        }
        let first_endpoints = if fixed_u {
            [IsoEndpointLocation::VMin, IsoEndpointLocation::VMax]
        } else {
            [IsoEndpointLocation::UMin, IsoEndpointLocation::UMax]
        };
        let second_endpoints = [
            uv_location(
                [
                    second_trace.evaluate(domain[0])?.point[0],
                    second_trace.evaluate(domain[0])?.point[1],
                ],
                param_margin,
            )?,
            uv_location(
                [
                    second_trace.evaluate(domain[1])?.point[0],
                    second_trace.evaluate(domain[1])?.point[1],
                ],
                param_margin,
            )?,
        ];
        let endpoints = IsoEndpointOwnership {
            first_support: first_endpoints,
            second_support: second_endpoints,
        };
        let evidence = compose_predicate_evidence(
            context,
            [
                PredicateEvidence::root_parameter(context, [fixed, fixed])?,
                PredicateEvidence::tangent_normal(context, 0.)?,
                PredicateEvidence::correspondence(context, 0., 1.)?,
                PredicateEvidence::topology_preservation(
                    context,
                    "exact_iso_whole_domain_and_boundary_ownership",
                    true,
                )?,
            ],
        )?;
        let authority = ExactIsoAuthority {
            curve: RationalCurveDefinition::from_curve(&exact_curve)?,
            traces: [
                RationalCurveDefinition::from_curve(&first_trace)?,
                RationalCurveDefinition::from_curve(&second_trace)?,
            ],
            axis: if fixed_u {
                ExactIsoAxis::U
            } else {
                ExactIsoAxis::V
            },
            fixed_parameter_bits: fixed.to_bits(),
            transverse_slope_bits: span.to_bits(),
            endpoints: endpoints.clone(),
            context: context.spec_identity(),
        };
        let mut diagnostic_samples = Vec::with_capacity(9);
        for i in 0..=8 {
            let varying = i as f64 / 8.;
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
            diagnostic_samples.push((p3, surface_uv, plane_uv));
        }
        if !valid {
            continue;
        }
        let certificate = ExactIsoIntersectionCertificate {
            curve: exact_curve,
            uv_traces: [first_trace, second_trace],
            first_axis: authority.axis,
            fixed_parameter: fixed,
            transverse_signed_distance_slope: span,
            endpoints,
            whole_domain_contained: true,
            strict_transverse: true,
            context: context.spec_identity(),
            evidence,
            diagnostic_samples,
            authority,
        };
        if certificate.permits_topology_authorship() {
            return Ok(certificate);
        }
    }
    Err(refuse(
        "No unique strict-interior transverse constant-U/V iso branch was certified",
    ))
}

pub fn certify_exact_planar_iso_intersection(
    surface: &Surface,
    plane: &Surface,
    context: &ToleranceContext,
) -> Result<ExactIsoIntersectionCertificate> {
    certify_exact_planar_iso_intersection_cell(surface, plane, context, false)
}

pub fn certify_exact_rational_planar_iso_intersection(
    surface: &Surface,
    plane: &Surface,
    context: &ToleranceContext,
) -> Result<ExactIsoIntersectionCertificate> {
    certify_exact_planar_iso_intersection_cell(surface, plane, context, true)
}

/// Identity of one positive tensor knot cell. Internal upper boundaries are
/// excluded; only the final non-periodic span owns its terminal knot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TensorSpanId {
    pub u: usize,
    pub v: usize,
}

#[derive(Clone, Debug)]
pub struct RationalBezierPatchSpan {
    pub source_face: usize,
    pub span: TensorSpanId,
    pub domain: [[f64; 2]; 2],
    pub owns_upper: [bool; 2],
    pub patch: Surface,
    pub denominator_lower_bound: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RationalSurfaceBits {
    degree: [usize; 2],
    knots: [Vec<u64>; 2],
    controls: Vec<Vec<[u64; 3]>>,
    weights: Vec<Vec<u64>>,
}

impl RationalSurfaceBits {
    fn from_surface(surface: &Surface) -> Result<Self> {
        surface.validate()?;
        Ok(Self {
            degree: [surface.degree_u, surface.degree_v],
            knots: [
                surface
                    .knots_u
                    .iter()
                    .map(|value| value.to_bits())
                    .collect(),
                surface
                    .knots_v
                    .iter()
                    .map(|value| value.to_bits())
                    .collect(),
            ],
            controls: surface
                .control_points
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|point| [point[0].to_bits(), point[1].to_bits(), point[2].to_bits()])
                        .collect()
                })
                .collect(),
            weights: surface
                .weights
                .iter()
                .map(|row| row.iter().map(|value| value.to_bits()).collect())
                .collect(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct RationalBezierDecomposition {
    pub source_face: usize,
    pub u_breaks: Vec<f64>,
    pub v_breaks: Vec<f64>,
    pub spans: Vec<RationalBezierPatchSpan>,
    pub denominator_lower_bound: f64,
    authority: Vec<RationalSurfaceBits>,
}

impl RationalBezierDecomposition {
    pub fn permits_exact_decomposition(&self) -> bool {
        self.spans.len() == self.authority.len()
            && !self.spans.is_empty()
            && self.denominator_lower_bound > 0.
            && self
                .spans
                .iter()
                .zip(&self.authority)
                .all(|(span, authority)| {
                    span.source_face == self.source_face
                        && span.denominator_lower_bound > 0.
                        && RationalSurfaceBits::from_surface(&span.patch).as_ref() == Ok(authority)
                        && span.domain[0]
                            == [self.u_breaks[span.span.u], self.u_breaks[span.span.u + 1]]
                        && span.domain[1]
                            == [self.v_breaks[span.span.v], self.v_breaks[span.span.v + 1]]
                        && span.owns_upper
                            == [
                                span.span.u + 2 == self.u_breaks.len(),
                                span.span.v + 2 == self.v_breaks.len(),
                            ]
                })
    }
}

fn active_breaks(knots: &[f64], degree: usize, controls: usize) -> Result<Vec<f64>> {
    let domain = [knots[degree], knots[controls]];
    let mut breaks = knots
        .iter()
        .copied()
        .filter(|value| *value >= domain[0] && *value <= domain[1])
        .collect::<Vec<_>>();
    breaks.dedup();
    if breaks.len() < 2
        || breaks.windows(2).any(|pair| pair[0] >= pair[1])
        || breaks.iter().any(|value| !value.is_finite())
    {
        return Err(refuse("Surface has a degenerate active knot span"));
    }
    Ok(breaks)
}

/// Perform exact homogeneous knot insertion through `Surface::trim`, retaining
/// every resulting rational Bezier tensor cell and its positive denominator
/// convex-hull bound.
pub fn decompose_rational_bezier_spans(
    surface: &Surface,
    source_face: usize,
    resource_limit: usize,
) -> Result<RationalBezierDecomposition> {
    surface.validate()?;
    if surface.periodic_u
        || surface.periodic_v
        || !(1..=3).contains(&surface.degree_u)
        || !(1..=3).contains(&surface.degree_v)
    {
        return Err(refuse(
            "Multi-span certification requires non-periodic degree 1 through 3 surfaces",
        ));
    }
    let u_breaks = active_breaks(
        &surface.knots_u,
        surface.degree_u,
        surface.control_points.len(),
    )?;
    let v_breaks = active_breaks(
        &surface.knots_v,
        surface.degree_v,
        surface.control_points[0].len(),
    )?;
    let count = (u_breaks.len() - 1)
        .checked_mul(v_breaks.len() - 1)
        .ok_or_else(|| Error::new("BREP_SS_RESOURCE_LIMIT", "Tensor span count overflow"))?;
    if resource_limit == 0 || count > resource_limit {
        return Err(Error::new(
            "BREP_SS_RESOURCE_LIMIT",
            "Tensor span budget exceeded",
        ));
    }
    let mut spans = Vec::with_capacity(count);
    let mut authority = Vec::with_capacity(count);
    let mut global_lower = f64::INFINITY;
    for u in 0..u_breaks.len() - 1 {
        for v in 0..v_breaks.len() - 1 {
            let patch =
                surface.trim([u_breaks[u], u_breaks[u + 1], v_breaks[v], v_breaks[v + 1]])?;
            let lower = patch
                .weights
                .iter()
                .flatten()
                .copied()
                .fold(f64::INFINITY, f64::min);
            if !lower.is_finite() || lower <= 0. {
                return Err(refuse(
                    "Rational Bezier span lacks a positive denominator lower bound",
                ));
            }
            if patch.control_points.len() != patch.degree_u + 1
                || patch.control_points[0].len() != patch.degree_v + 1
            {
                return Err(refuse(
                    "Knot insertion did not isolate one Bezier tensor span",
                ));
            }
            global_lower = global_lower.min(lower);
            authority.push(RationalSurfaceBits::from_surface(&patch)?);
            spans.push(RationalBezierPatchSpan {
                source_face,
                span: TensorSpanId { u, v },
                domain: [
                    [u_breaks[u], u_breaks[u + 1]],
                    [v_breaks[v], v_breaks[v + 1]],
                ],
                owns_upper: [u + 2 == u_breaks.len(), v + 2 == v_breaks.len()],
                patch,
                denominator_lower_bound: lower,
            });
        }
    }
    let result = RationalBezierDecomposition {
        source_face,
        u_breaks,
        v_breaks,
        spans,
        denominator_lower_bound: global_lower,
        authority,
    };
    if !result.permits_exact_decomposition() {
        return Err(refuse("Rational Bezier decomposition failed closed"));
    }
    Ok(result)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BranchOrientation {
    Increasing,
    Decreasing,
}

#[derive(Clone, Debug)]
pub struct TransverseSpanEvidence {
    pub homogeneous_distance_coefficients_bits: Vec<u64>,
    pub local_root: f64,
    pub signed_slope: f64,
    pub denominator_lower_bound: f64,
    pub exact_affine_factor: bool,
}

#[derive(Clone, Debug)]
pub struct CertifiedBranchFragment {
    pub source_faces: [usize; 2],
    pub source_spans: [TensorSpanId; 2],
    pub parameter_interval: [f64; 2],
    pub curve: Curve,
    pub pcurves: [Curve; 2],
    pub orientations: [BranchOrientation; 2],
    pub evidence: TransverseSpanEvidence,
}

#[derive(Clone, Debug)]
pub struct BranchComponent {
    pub component_id: usize,
    pub fragments: Vec<CertifiedBranchFragment>,
    pub closed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FragmentAuthority {
    source_faces: [usize; 2],
    source_spans: [TensorSpanId; 2],
    parameter_interval_bits: [u64; 2],
    curve: RationalCurveDefinition,
    pcurves: [RationalCurveDefinition; 2],
    orientations: [BranchOrientation; 2],
    homogeneous_distance_coefficients_bits: Vec<u64>,
    local_root_bits: u64,
    signed_slope_bits: u64,
    denominator_lower_bound_bits: u64,
    exact_affine_factor: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BranchGraphAuthority {
    context: ToleranceSpecIdentity,
    fragment_definitions: Vec<FragmentAuthority>,
    component_ranges: Vec<[usize; 2]>,
    component_closed: Vec<bool>,
    source_span_count: [usize; 2],
    candidate_span_pairs: usize,
    denominator_lower_bound_bits: u64,
    resource_limit: usize,
}

#[derive(Clone, Debug)]
pub struct BranchCompletenessCertificate {
    pub context: ToleranceSpecIdentity,
    pub source_span_count: [usize; 2],
    pub candidate_span_pairs: usize,
    pub fragment_count: usize,
    pub component_count: usize,
    pub denominator_lower_bound: f64,
    pub all_cells_classified: bool,
    pub one_to_one_joins: bool,
    pub no_tangency_coincidence_or_multi_root: bool,
    pub resource_limit: usize,
}

#[derive(Clone, Debug)]
pub struct BranchGraph {
    pub components: Vec<BranchComponent>,
    pub certificate: BranchCompletenessCertificate,
    authority: BranchGraphAuthority,
}

fn fragment_authority(fragment: &CertifiedBranchFragment) -> Result<FragmentAuthority> {
    Ok(FragmentAuthority {
        source_faces: fragment.source_faces,
        source_spans: fragment.source_spans,
        parameter_interval_bits: fragment.parameter_interval.map(f64::to_bits),
        curve: RationalCurveDefinition::from_curve(&fragment.curve)?,
        pcurves: [
            RationalCurveDefinition::from_curve(&fragment.pcurves[0])?,
            RationalCurveDefinition::from_curve(&fragment.pcurves[1])?,
        ],
        orientations: fragment.orientations,
        homogeneous_distance_coefficients_bits: fragment
            .evidence
            .homogeneous_distance_coefficients_bits
            .clone(),
        local_root_bits: fragment.evidence.local_root.to_bits(),
        signed_slope_bits: fragment.evidence.signed_slope.to_bits(),
        denominator_lower_bound_bits: fragment.evidence.denominator_lower_bound.to_bits(),
        exact_affine_factor: fragment.evidence.exact_affine_factor,
    })
}

impl BranchGraph {
    pub fn permits_topology_authorship(&self) -> bool {
        let definitions = self
            .components
            .iter()
            .flat_map(|component| &component.fragments)
            .map(fragment_authority)
            .collect::<Result<Vec<_>>>();
        let mut offset = 0;
        let ranges = self
            .components
            .iter()
            .map(|component| {
                let range = [offset, offset + component.fragments.len()];
                offset = range[1];
                range
            })
            .collect::<Vec<_>>();
        self.certificate.context == self.authority.context
            && self.certificate.fragment_count == offset
            && self.certificate.component_count == self.components.len()
            && self.certificate.denominator_lower_bound > 0.
            && self.certificate.all_cells_classified
            && self.certificate.one_to_one_joins
            && self.certificate.no_tangency_coincidence_or_multi_root
            && self.certificate.fragment_count <= self.certificate.resource_limit
            && definitions.as_ref() == Ok(&self.authority.fragment_definitions)
            && ranges == self.authority.component_ranges
            && self.certificate.source_span_count == self.authority.source_span_count
            && self.certificate.candidate_span_pairs == self.authority.candidate_span_pairs
            && self.certificate.denominator_lower_bound.to_bits()
                == self.authority.denominator_lower_bound_bits
            && self.certificate.resource_limit == self.authority.resource_limit
            && self
                .components
                .iter()
                .map(|component| component.closed)
                .eq(self.authority.component_closed.iter().copied())
    }
}

fn curve_endpoint_bits(curve: &Curve, end: usize) -> Result<Vec<u64>> {
    let domain = curve.domain();
    Ok(curve
        .evaluate(domain[end])?
        .point
        .iter()
        .map(|value| {
            let value = if *value == 0. { 0. } else { *value };
            value.to_bits()
        })
        .collect())
}

fn joined_endpoint_key(
    fragment: &CertifiedBranchFragment,
    end: usize,
) -> Result<(Vec<u64>, Vec<u64>, Vec<u64>)> {
    Ok((
        curve_endpoint_bits(&fragment.curve, end)?,
        curve_endpoint_bits(&fragment.pcurves[0], end)?,
        curve_endpoint_bits(&fragment.pcurves[1], end)?,
    ))
}

/// Deterministically join exact per-cell fragments. A join exists only when
/// the 3D endpoint and both oriented UV endpoints are bit-identical.
pub fn join_certified_multispan_fragments(
    mut fragments: Vec<CertifiedBranchFragment>,
    context: &ToleranceContext,
    source_span_count: [usize; 2],
    candidate_span_pairs: usize,
    denominator_lower_bound: f64,
    resource_limit: usize,
) -> Result<BranchGraph> {
    if fragments.is_empty() || fragments.len() > resource_limit || denominator_lower_bound <= 0. {
        return Err(Error::new(
            "BREP_SS_RESOURCE_LIMIT",
            "Branch fragment budget is empty or exceeded",
        ));
    }
    fragments.sort_by(|a, b| {
        joined_endpoint_key(a, 0)
            .unwrap()
            .cmp(&joined_endpoint_key(b, 0).unwrap())
            .then_with(|| a.source_spans.cmp(&b.source_spans))
            .then_with(|| a.parameter_interval[0].total_cmp(&b.parameter_interval[0]))
    });
    let mut endpoints = BTreeMap::<(Vec<u64>, Vec<u64>, Vec<u64>), Vec<(usize, usize)>>::new();
    for (id, fragment) in fragments.iter().enumerate() {
        for end in 0..2 {
            endpoints
                .entry(joined_endpoint_key(fragment, end)?)
                .or_default()
                .push((id, end));
        }
    }
    if endpoints.values().any(|uses| uses.len() > 2) {
        return Err(refuse(
            "Ambiguous branch fork or merge at an exact endpoint",
        ));
    }
    let mut adjacency = vec![[None; 2]; fragments.len()];
    for uses in endpoints.values().filter(|uses| uses.len() == 2) {
        let [(a, ae), (b, be)] = uses.as_slice() else {
            unreachable!()
        };
        if a == b || ae == be {
            return Err(refuse("Ambiguous branch orientation at an exact join"));
        }
        adjacency[*a][*ae] = Some(*b);
        adjacency[*b][*be] = Some(*a);
    }
    let mut visited = BTreeSet::new();
    let mut components = Vec::new();
    let starts = (0..fragments.len())
        .filter(|id| adjacency[*id].iter().filter(|next| next.is_some()).count() < 2)
        .chain(0..fragments.len())
        .collect::<Vec<_>>();
    for start in starts {
        if visited.contains(&start) {
            continue;
        }
        let mut ordered = Vec::new();
        let mut current = start;
        let mut entered_end = adjacency[current][0].is_some() && adjacency[current][1].is_none();
        let closed;
        loop {
            if !visited.insert(current) {
                closed = current == start;
                break;
            }
            let mut fragment = fragments[current].clone();
            if entered_end {
                fragment.curve = fragment.curve.reverse()?;
                fragment.pcurves = [
                    fragment.pcurves[0].reverse()?,
                    fragment.pcurves[1].reverse()?,
                ];
                fragment.parameter_interval.reverse();
                fragment.orientations =
                    fragment.orientations.map(|orientation| match orientation {
                        BranchOrientation::Increasing => BranchOrientation::Decreasing,
                        BranchOrientation::Decreasing => BranchOrientation::Increasing,
                    });
            }
            ordered.push(fragment);
            let exit = usize::from(!entered_end);
            let Some(next) = adjacency[current][exit] else {
                closed = false;
                break;
            };
            let next_enter = adjacency[next][1] == Some(current);
            current = next;
            entered_end = next_enter;
        }
        components.push(BranchComponent {
            component_id: 0,
            fragments: ordered,
            closed,
        });
    }
    components.sort_by(|a, b| {
        joined_endpoint_key(&a.fragments[0], 0)
            .unwrap()
            .cmp(&joined_endpoint_key(&b.fragments[0], 0).unwrap())
    });
    for (id, component) in components.iter_mut().enumerate() {
        component.component_id = id;
    }
    let definitions = components
        .iter()
        .flat_map(|component| &component.fragments)
        .map(fragment_authority)
        .collect::<Result<Vec<_>>>()?;
    let mut offset = 0;
    let component_ranges = components
        .iter()
        .map(|component| {
            let result = [offset, offset + component.fragments.len()];
            offset = result[1];
            result
        })
        .collect::<Vec<_>>();
    let authority = BranchGraphAuthority {
        context: context.spec_identity(),
        fragment_definitions: definitions,
        component_ranges,
        component_closed: components
            .iter()
            .map(|component| component.closed)
            .collect(),
        source_span_count,
        candidate_span_pairs,
        denominator_lower_bound_bits: denominator_lower_bound.to_bits(),
        resource_limit,
    };
    let graph = BranchGraph {
        certificate: BranchCompletenessCertificate {
            context: context.spec_identity(),
            source_span_count,
            candidate_span_pairs,
            fragment_count: offset,
            component_count: components.len(),
            denominator_lower_bound,
            all_cells_classified: true,
            one_to_one_joins: true,
            no_tangency_coincidence_or_multi_root: true,
            resource_limit,
        },
        components,
        authority,
    };
    if !graph.permits_topology_authorship() {
        return Err(refuse(
            "Joined branch graph failed its native authority check",
        ));
    }
    Ok(graph)
}

fn affine_plane_uv_domain(
    surface: &Surface,
    point: [f64; 3],
    tolerance: f64,
) -> Result<Option<[f64; 2]>> {
    let u = active_breaks(
        &surface.knots_u,
        surface.degree_u,
        surface.control_points.len(),
    )?;
    let v = active_breaks(
        &surface.knots_v,
        surface.degree_v,
        surface.control_points[0].len(),
    )?;
    let [umin, umax] = [u[0], *u.last().unwrap()];
    let [vmin, vmax] = [v[0], *v.last().unwrap()];
    let origin = surface.evaluate(umin, vmin)?.point;
    let pu = surface.evaluate(umax, vmin)?.point;
    let pv = surface.evaluate(umin, vmax)?.point;
    let eu = sub(pu, origin);
    let ev = sub(pv, origin);
    let rhs = sub(point, origin);
    let [uu, uv, vv] = [dot(eu, eu), dot(eu, ev), dot(ev, ev)];
    let determinant = uu * vv - uv * uv;
    if determinant.abs() <= tolerance * tolerance {
        return Ok(None);
    }
    let a = (dot(rhs, eu) * vv - dot(rhs, ev) * uv) / determinant;
    let b = (dot(rhs, ev) * uu - dot(rhs, eu) * uv) / determinant;
    let result = [umin + a * (umax - umin), vmin + b * (vmax - vmin)];
    Ok(((result[0] >= umin - tolerance)
        && (result[0] <= umax + tolerance)
        && (result[1] >= vmin - tolerance)
        && (result[1] <= vmax + tolerance))
        .then_some(result))
}

fn affine_plane_trace(plane: &Surface, curve: &Curve, tolerance: f64) -> Result<Option<Curve>> {
    let mut controls = Vec::with_capacity(curve.control_points.len());
    for point in &curve.control_points {
        let Some(uv) = affine_plane_uv_domain(plane, [point[0], point[1], point[2]], tolerance)?
        else {
            return Ok(None);
        };
        controls.push(uv.to_vec());
    }
    let trace = Curve {
        degree: curve.degree,
        knots: curve.knots.clone(),
        control_points: controls,
        weights: curve.weights.clone(),
        periodic: false,
    };
    trace.validate()?;
    Ok(Some(trace))
}

fn owning_span(breaks: &[f64], parameter: f64) -> Option<usize> {
    breaks.windows(2).enumerate().find_map(|(index, pair)| {
        (parameter >= pair[0]
            && (parameter < pair[1] || (index + 2 == breaks.len() && parameter == pair[1])))
            .then_some(index)
    })
}

/// Candidate `/6` infrastructure only. It certifies all affine homogeneous
/// roots that are unique and transverse in each positive rational Bezier span,
/// then joins their exact iso fragments. It does not assemble Boolean topology.
pub fn certify_multispan_ss(
    source: &Surface,
    plane: &Surface,
    source_faces: [usize; 2],
    context: &ToleranceContext,
    resource_limit: usize,
) -> Result<BranchGraph> {
    let source_decomposition =
        decompose_rational_bezier_spans(source, source_faces[0], resource_limit)?;
    let plane_decomposition =
        decompose_rational_bezier_spans(plane, source_faces[1], resource_limit)?;
    let tolerance = context.spatial_bounds().on_mm;
    let (plane_origin, plane_normal) =
        planar_support(plane, tolerance).ok_or_else(|| refuse("Second support is not planar"))?;
    if !affine_graph_projection(plane, tolerance) {
        return Err(refuse(
            "Second support is not an affine planar parameterization",
        ));
    }
    let candidate_span_pairs = source_decomposition
        .spans
        .len()
        .checked_mul(plane_decomposition.spans.len())
        .ok_or_else(|| Error::new("BREP_SS_RESOURCE_LIMIT", "Candidate span-pair overflow"))?;
    if candidate_span_pairs > resource_limit.saturating_mul(resource_limit) {
        return Err(Error::new(
            "BREP_SS_RESOURCE_LIMIT",
            "Candidate span-pair budget exceeded",
        ));
    }
    let mut fragments = Vec::new();
    for span in &source_decomposition.spans {
        let mut cell_candidates = Vec::new();
        for fixed_u in [true, false] {
            let fixed_degree = if fixed_u {
                span.patch.degree_u
            } else {
                span.patch.degree_v
            };
            let varying_degree = if fixed_u {
                span.patch.degree_v
            } else {
                span.patch.degree_u
            };
            let mut coefficients = vec![0.; fixed_degree + 1];
            let mut invariant = true;
            for i in 0..=fixed_degree {
                for j in 0..=varying_degree {
                    let (u, v) = if fixed_u { (i, j) } else { (j, i) };
                    let point = &span.patch.control_points[u][v];
                    let coefficient = span.patch.weights[u][v]
                        * dot(
                            sub([point[0], point[1], point[2]], plane_origin),
                            plane_normal,
                        );
                    if j == 0 {
                        coefficients[i] = coefficient;
                    } else if (coefficient - coefficients[i]).abs()
                        > 32. * f64::EPSILON * coefficient.abs().max(coefficients[i].abs()).max(1.)
                    {
                        invariant = false;
                    }
                }
            }
            let slope = coefficients[fixed_degree] - coefficients[0];
            let affine = invariant
                && slope.abs() > context.spatial_bounds().clear_mm
                && coefficients.iter().enumerate().all(|(i, value)| {
                    let expected = coefficients[0] + slope * i as f64 / fixed_degree as f64;
                    value.to_bits() == expected.to_bits()
                        || (value - expected).abs() <= 32. * f64::EPSILON * slope.abs().max(1.)
                });
            if !affine {
                let coefficient_min = coefficients.iter().copied().fold(f64::INFINITY, f64::min);
                let coefficient_max = coefficients
                    .iter()
                    .copied()
                    .fold(f64::NEG_INFINITY, f64::max);
                if invariant
                    && (coefficients.iter().all(|value| value.abs() <= tolerance)
                        || (coefficient_min <= 0. && coefficient_max >= 0.))
                {
                    return Err(refuse(
                        "Coincident, tangent, or unresolved multi-root span is outside /6",
                    ));
                }
                continue;
            }
            let local_root = -coefficients[0] / slope;
            let fixed_span = if fixed_u { span.span.u } else { span.span.v };
            let fixed_count = if fixed_u {
                source_decomposition.u_breaks.len() - 1
            } else {
                source_decomposition.v_breaks.len() - 1
            };
            let owns = local_root >= 0.
                && (local_root < 1. || (fixed_span + 1 == fixed_count && local_root == 1.));
            if !owns {
                continue;
            }
            let fixed_domain = span.domain[usize::from(!fixed_u)];
            let fixed_parameter =
                fixed_domain[0] + local_root * (fixed_domain[1] - fixed_domain[0]);
            cell_candidates.push((fixed_u, fixed_parameter, coefficients, local_root, slope));
        }
        if cell_candidates.len() > 1 {
            return Err(refuse(
                "One tensor cell has ambiguous multiple branch families",
            ));
        }
        let Some((fixed_u, fixed, coefficients, local_root, slope)) = cell_candidates.pop() else {
            continue;
        };
        let varying_domain = span.domain[usize::from(fixed_u)];
        let curve = source
            .iso(if fixed_u { Axis::U } else { Axis::V }, fixed)?
            .trim(varying_domain[0], varying_domain[1])?;
        let first_trace = Curve {
            degree: 1,
            knots: vec![
                varying_domain[0],
                varying_domain[0],
                varying_domain[1],
                varying_domain[1],
            ],
            control_points: if fixed_u {
                vec![
                    vec![fixed, varying_domain[0]],
                    vec![fixed, varying_domain[1]],
                ]
            } else {
                vec![
                    vec![varying_domain[0], fixed],
                    vec![varying_domain[1], fixed],
                ]
            },
            weights: vec![1., 1.],
            periodic: false,
        };
        first_trace.validate()?;
        let second_trace = affine_plane_trace(plane, &curve, tolerance)?
            .ok_or_else(|| refuse("Intersection branch leaves the affine plane domain"))?;
        let midpoint =
            second_trace.evaluate(0.5 * (second_trace.domain()[0] + second_trace.domain()[1]))?;
        let plane_u = owning_span(&plane_decomposition.u_breaks, midpoint.point[0])
            .ok_or_else(|| refuse("Plane pcurve has no half-open U-span owner"))?;
        let plane_v = owning_span(&plane_decomposition.v_breaks, midpoint.point[1])
            .ok_or_else(|| refuse("Plane pcurve has no half-open V-span owner"))?;
        fragments.push(CertifiedBranchFragment {
            source_faces,
            source_spans: [
                span.span,
                TensorSpanId {
                    u: plane_u,
                    v: plane_v,
                },
            ],
            parameter_interval: varying_domain,
            curve,
            pcurves: [first_trace, second_trace],
            orientations: [BranchOrientation::Increasing, BranchOrientation::Increasing],
            evidence: TransverseSpanEvidence {
                homogeneous_distance_coefficients_bits: coefficients
                    .iter()
                    .map(|value| value.to_bits())
                    .collect(),
                local_root,
                signed_slope: slope,
                denominator_lower_bound: span.denominator_lower_bound,
                exact_affine_factor: true,
            },
        });
    }
    join_certified_multispan_fragments(
        fragments,
        context,
        [
            source_decomposition.spans.len(),
            plane_decomposition.spans.len(),
        ],
        candidate_span_pairs,
        source_decomposition
            .denominator_lower_bound
            .min(plane_decomposition.denominator_lower_bound),
        resource_limit,
    )
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
    let source_a = a;
    let source_b = b;
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
        if pb.is_some() {
            let context = ToleranceContext::default_valid();
            if let Ok(certificate) =
                certify_exact_planar_iso_intersection(source_a, source_b, &context)
            {
                let report = complete_curve(certificate);
                verify_complete_report(&report, false)?;
                return Ok(report);
            }
        }
    } else if pb.is_none() {
        if pa.is_some() {
            let context = ToleranceContext::default_valid();
            if let Ok(certificate) =
                certify_exact_planar_iso_intersection(source_b, source_a, &context)
            {
                let report = complete_curve(certificate);
                verify_complete_report(&report, false)?;
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

/// Audited authority for the finite V3 graph-patch × affine-planar cutter cell.
/// Union is intentionally absent: this successor only authors the one-body
/// retained graph sub-solid for intersection and source-minus-cutter difference.
#[derive(Clone, Debug)]
pub struct CurvedGraphBooleanCertificate {
    pub capability: &'static str,
    pub status: &'static str,
    pub operation: String,
    pub axis: ExactIsoAxis,
    pub fixed_parameter: f64,
    pub source_face: String,
    pub retained_face: String,
    pub deleted_region: &'static str,
    pub intersection_edge: String,
    pub tensor_cells: usize,
    pub exact_correspondence: bool,
    pub sew: SewCertificate,
    pub audit: SolidAuditCertificate,
    pub change_set: ChangeSet,
    pub naming_complete: bool,
    pub no_fallback: bool,
    pub separation_proof: bool,
    pub homogeneous_root_proof: bool,
    pub denominator_lower_bound: f64,
    pub weight_condition_number: f64,
    pub resource_bound: usize,
}

impl CurvedGraphBooleanCertificate {
    pub fn permits_topology_change(&self) -> bool {
        matches!(
            self.capability,
            NURBS_BOOLEAN_CAPABILITY_V3 | NURBS_BOOLEAN_CAPABILITY_V4 | NURBS_BOOLEAN_CAPABILITY_V5
        ) && self.status == "Complete"
            && matches!(self.operation.as_str(), "intersection" | "difference")
            && self.fixed_parameter > 0.
            && self.fixed_parameter < 1.
            && self.tensor_cells == 3
            && self.exact_correspondence
            && self.sew.complete
            && self.audit.ok
            && self.naming_complete
            && self.no_fallback
            && self.separation_proof
            && (self.capability != NURBS_BOOLEAN_CAPABILITY_V5
                || (self.homogeneous_root_proof
                    && self.denominator_lower_bound >= 0.25
                    && self.weight_condition_number <= 8.
                    && self.resource_bound <= 16))
            && !self.change_set.changes.is_empty()
            && self.change_set.validate().is_ok()
    }
}

#[derive(Clone, Debug)]
pub struct ContainedGraphBooleanCertificate {
    pub capability: &'static str,
    pub status: &'static str,
    pub operation: String,
    pub relation: &'static str,
    pub strict_uv_margin: f64,
    pub floor_clearance: f64,
    pub roof_clearance: f64,
    pub cavity_proof: bool,
    pub separation_proof: bool,
    pub audit: SolidAuditCertificate,
    pub change_set: ChangeSet,
    pub naming_complete: bool,
    pub no_fallback: bool,
}

impl ContainedGraphBooleanCertificate {
    pub fn permits_topology_change(&self) -> bool {
        self.capability == NURBS_BOOLEAN_CAPABILITY_V4
            && self.status == "Complete"
            && matches!(
                self.operation.as_str(),
                "union" | "intersection" | "difference"
            )
            && self.relation == "graph-strictly-contains-affine-cutter"
            && self.strict_uv_margin > 0.
            && self.floor_clearance > 0.
            && self.roof_clearance > 0.
            && (self.operation != "difference" || self.cavity_proof)
            && self.separation_proof
            && self.audit.ok
            && self.naming_complete
            && self.no_fallback
            && !self.change_set.changes.is_empty()
            && self.change_set.validate().is_ok()
    }
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

fn near_point(a: [f64; 3], b: [f64; 3], tolerance: f64) -> bool {
    norm(sub(a, b)) <= tolerance
}

fn point3(surface: &Surface, u: f64, v: f64) -> Result<[f64; 3]> {
    let p = surface.evaluate(u, v)?.point;
    Ok([p[0], p[1], p[2]])
}

/// Canonical one-body input for the finite V3 cell.  Its only non-planar face
/// is a unit-weight single-span degree-2/3 tensor graph with straight boundary.
pub fn canonical_bezier_graph_solid(degree_u: usize, degree_v: usize) -> Result<Model> {
    if !(2..=3).contains(&degree_u) || !(2..=3).contains(&degree_v) {
        return Err(refuse("V3 graph solid requires degree 2 or 3 in U and V"));
    }
    let mut model = crate::cuboid([0., 0., 0.], [1., 1., 1.])?;
    let mut control_points = Vec::with_capacity(degree_u + 1);
    for i in 0..=degree_u {
        let u = i as f64 / degree_u as f64;
        control_points.push(
            (0..=degree_v)
                .map(|j| {
                    let v = j as f64 / degree_v as f64;
                    let interior = i > 0 && i < degree_u && j > 0 && j < degree_v;
                    vec![u, v, 1. + if interior { 0.25 } else { 0. }]
                })
                .collect(),
        );
    }
    model.faces[1].surface = Surface {
        degree_u,
        degree_v,
        knots_u: [vec![0.; degree_u + 1], vec![1.; degree_u + 1]].concat(),
        knots_v: [vec![0.; degree_v + 1], vec![1.; degree_v + 1]].concat(),
        control_points,
        weights: vec![vec![1.; degree_v + 1]; degree_u + 1],
        periodic_u: false,
        periodic_v: false,
    };
    model.rebuild_topology_ids();
    model.validate()?;
    Ok(model)
}

/// Canonical bounded positive-weight graph for the finite `/5` cell. Weights
/// vary only along V, so the homogeneous plane numerator factors into a
/// positive denominator polynomial and one affine U root.
pub fn canonical_rational_graph_solid(degree_u: usize, degree_v: usize) -> Result<Model> {
    let base = canonical_bezier_graph_solid(degree_u, degree_v)?;
    let (_, source_top, origin, u, v, floor_offset) = canonical_graph_frame(&base)?;
    let mut top = source_top.clone();
    let weights: Vec<f64> = (0..=degree_v)
        .map(|j| {
            if j == 0 || j == degree_v {
                1.
            } else {
                1. + 0.25 * j as f64
            }
        })
        .collect();
    top.weights = (0..=degree_u).map(|_| weights.clone()).collect();
    let model = clipped_graph_solid(&base, &top, origin, u, v, floor_offset, [0., 1., 0., 1.])?;
    model.validate()?;
    let rational_face = canonical_graph_frame_cell(&model, true, false)?.0;
    if rational_weight_bounds(&model.faces[rational_face].surface).is_none() {
        return Err(refuse(
            "Canonical rational graph exceeded finite weight bounds",
        ));
    }
    Ok(model)
}

fn canonical_graph_frame_cell(
    source: &Model,
    rational: bool,
    multispan: bool,
) -> Result<(usize, &Surface, [f64; 3], [f64; 3], [f64; 3], [f64; 3])> {
    if source.bodies.len() != 1
        || source.shells.len() != 1
        || source.faces.len() != 6
        || source.vertices.len() != 8
        || !source.bodies[0].inner_shells.is_empty()
    {
        return Err(refuse(
            "Graph source must be the canonical one-body graph solid",
        ));
    }
    let tolerance = source.tolerance_mm.max(1e-9) * 8.;
    let nonplanar: Vec<_> = source
        .faces
        .iter()
        .enumerate()
        .filter(|(_, face)| planar_support(&face.surface, tolerance).is_none())
        .collect();
    if nonplanar.len() != 1 {
        return Err(refuse(
            "Graph source must contain exactly one non-planar face",
        ));
    }
    let (face_id, top) = (nonplanar[0].0, &nonplanar[0].1.surface);
    let admitted_top = if multispan {
        positive_multispan_graph(top)
    } else if rational {
        rational_weight_bounds(top).is_some()
    } else {
        is_bezier_le3_positive(top)
    };
    if !(2..=3).contains(&top.degree_u)
        || !(2..=3).contains(&top.degree_v)
        || !admitted_top
        || !source.faces[face_id].holes.is_empty()
    {
        return Err(refuse(
            "Curved face is outside the admitted degree-2/3 graph cell",
        ));
    }
    let o = point3(top, 0., 0.)?;
    let pu = point3(top, 1., 0.)?;
    let pv = point3(top, 0., 1.)?;
    let p11 = point3(top, 1., 1.)?;
    let u = sub(pu, o);
    let v = sub(pv, o);
    if !near_point(
        p11,
        [o[0] + u[0] + v[0], o[1] + u[1] + v[1], o[2] + u[2] + v[2]],
        tolerance,
    ) {
        return Err(refuse(
            "V3 graph corners do not define one affine projection",
        ));
    }
    let mut normal = normalize(cross(u, v)).ok_or_else(|| refuse("V3 graph frame is singular"))?;
    let corners = [o, pu, p11, pv];
    let last_u = top.control_points.len() - 1;
    let last_v = top.control_points[0].len() - 1;
    for i in 0..=last_u {
        for j in 0..=last_v {
            if i == 0 || i == last_u || j == 0 || j == last_v {
                let p = &top.control_points[i][j];
                if dot(sub([p[0], p[1], p[2]], o), normal).abs() > tolerance {
                    return Err(refuse(
                        "V3 graph boundary must remain on its affine corner plane",
                    ));
                }
            }
        }
    }
    let mut offset = None;
    'candidate: for vertex in &source.vertices {
        let w = sub(vertex.point, o);
        if norm(w) <= tolerance
            || dot(w, u).abs() > tolerance * norm(u)
            || dot(w, v).abs() > tolerance * norm(v)
        {
            continue;
        }
        for corner in corners {
            let target = [corner[0] + w[0], corner[1] + w[1], corner[2] + w[2]];
            if !source
                .vertices
                .iter()
                .any(|candidate| near_point(candidate.point, target, tolerance))
            {
                continue 'candidate;
            }
        }
        offset = Some(w);
        break;
    }
    let w = offset.ok_or_else(|| refuse("V3 graph solid has no exact parallel affine floor"))?;
    if dot(w, normal) > 0. {
        normal = normal.map(|value| -value);
    }
    if top.control_points.iter().flatten().any(|point| {
        let height = dot(sub([point[0], point[1], point[2]], o), normal);
        let floor_height = dot(w, normal);
        height <= floor_height + tolerance
    }) {
        return Err(refuse("V3 graph-to-floor separation proof failed"));
    }
    Ok((face_id, top, o, u, v, w))
}

fn canonical_graph_frame(
    source: &Model,
) -> Result<(usize, &Surface, [f64; 3], [f64; 3], [f64; 3], [f64; 3])> {
    canonical_graph_frame_cell(source, false, false)
}

pub(crate) fn is_nurbs_boolean_v3_candidate(a: &Model, b: &Model) -> bool {
    canonical_graph_frame(a).is_ok() ^ canonical_graph_frame(b).is_ok()
}

pub(crate) fn canonical_graph_is_first(model: &Model) -> bool {
    canonical_graph_frame(model).is_ok()
}

pub(crate) fn is_nurbs_boolean_v5_candidate(a: &Model, b: &Model) -> bool {
    canonical_graph_frame_cell(a, true, false).is_ok()
        ^ canonical_graph_frame_cell(b, true, false).is_ok()
}

fn unit_domain(mut curve: Curve) -> Curve {
    let [lo, hi] = curve.domain();
    for knot in &mut curve.knots {
        *knot = (*knot - lo) / (hi - lo);
    }
    curve
}

fn edge_between(model: &Model, a: usize, b: usize) -> Result<usize> {
    model
        .edges
        .iter()
        .position(|edge| edge.vertices == [a.min(b), a.max(b)])
        .ok_or_else(|| refuse("V3 clipped box edge is missing"))
}

fn ruled_side(bottom_a: [f64; 3], bottom_b: [f64; 3], top: Curve) -> Result<Surface> {
    let degree = top.degree;
    let single_span = top.control_points.len() == degree + 1 && top.knots.len() == 2 * (degree + 1);
    let (bottom_points, bottom_weights) = if single_span {
        let bottom = crate::line(bottom_a.to_vec(), bottom_b.to_vec()).elevate(degree)?;
        (bottom.control_points, bottom.weights)
    } else {
        if top.weights.windows(2).any(|pair| pair[0] != pair[1]) {
            return Err(refuse(
                "Multispan ruled side requires a constant positive boundary weight",
            ));
        }
        let domain = top.domain();
        let points = (0..top.control_points.len())
            .map(|index| {
                let parameter =
                    top.knots[index + 1..=index + degree].iter().sum::<f64>() / degree as f64;
                let t = (parameter - domain[0]) / (domain[1] - domain[0]);
                bottom_a
                    .into_iter()
                    .zip(bottom_b)
                    .map(|(a, b)| a + t * (b - a))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        (points, top.weights.clone())
    };
    Ok(Surface {
        degree_u: degree,
        degree_v: 1,
        knots_u: top.knots.clone(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: bottom_points
            .iter()
            .zip(&top.control_points)
            .map(|(a, b)| vec![a.clone(), b.clone()])
            .collect(),
        weights: bottom_weights
            .iter()
            .zip(&top.weights)
            .map(|(a, b)| vec![*a, *b])
            .collect(),
        periodic_u: false,
        periodic_v: false,
    })
}

fn clipped_graph_solid(
    source: &Model,
    top: &Surface,
    origin: [f64; 3],
    u: [f64; 3],
    v: [f64; 3],
    floor_offset: [f64; 3],
    bounds: [f64; 4],
) -> Result<Model> {
    let [umin, umax, vmin, vmax] = bounds;
    let local = crate::cuboid([umin, vmin, 0.], [umax, vmax, 1.])?;
    let z = floor_offset.map(|value| -value);
    let floor_origin = [
        origin[0] + floor_offset[0],
        origin[1] + floor_offset[1],
        origin[2] + floor_offset[2],
    ];
    let matrix = [
        [u[0], v[0], z[0], floor_origin[0]],
        [u[1], v[1], z[1], floor_origin[1]],
        [u[2], v[2], z[2], floor_origin[2]],
        [0., 0., 0., 1.],
    ];
    let mut model = crate::transform::affine(&local, matrix)?;
    let trimmed = top.trim([umin, umax, vmin, vmax])?;
    model.faces[1].surface = trimmed.clone();
    let top_curves = [
        unit_domain(top.iso(Axis::V, vmin)?.trim(umin, umax)?),
        unit_domain(top.iso(Axis::U, umax)?.trim(vmin, vmax)?),
        unit_domain(top.iso(Axis::V, vmax)?.trim(umin, umax)?.reverse()?),
        unit_domain(top.iso(Axis::U, umin)?.trim(vmin, vmax)?),
    ];
    for ((a, b), curve) in [(4, 5), (5, 6), (6, 7), (4, 7)]
        .into_iter()
        .zip(top_curves.iter())
    {
        let edge = edge_between(&model, a, b)?;
        model.edges[edge].curve = curve.clone();
    }
    let top_loop = model.faces[1].outer;
    let uv = [[umin, vmin], [umax, vmin], [umax, vmax], [umin, vmax]];
    for (index, coedge) in model.loops[top_loop].coedges.iter_mut().enumerate() {
        coedge.pcurve = crate::line(uv[index].to_vec(), uv[(index + 1) % 4].to_vec());
    }
    for (face, bottom, top_curve) in [
        (2usize, (0usize, 1usize), top_curves[0].clone()),
        (3, (1, 2), top_curves[1].clone()),
        (4, (2, 3), top_curves[2].clone()),
        (5, (3, 0), top_curves[3].clone().reverse()?),
    ] {
        model.faces[face].surface = ruled_side(
            model.vertices[bottom.0].point,
            model.vertices[bottom.1].point,
            top_curve,
        )?;
    }
    let assembled = crate::imprint_pipeline::assemble_imprint_solid(
        model.vertices.clone(),
        model.edges.clone(),
        model.loops.clone(),
        model.faces.clone(),
        model.shells[0].faces.clone(),
        source.tolerance_mm,
        &[source],
    )?;
    Ok(assembled)
}

/// Certified V3 author for one canonical graph solid and one affine cutter.
/// Complete rows: intersection and source-minus-cutter difference. Union and
/// cutter-minus-source difference remain typed refusals.
fn nurbs_boolean_graph_patch_transverse(
    a: &Model,
    b: &Model,
    operation: &str,
    capability: &'static str,
    rational: bool,
) -> Result<(Model, CurvedGraphBooleanCertificate)> {
    a.validate()?;
    b.validate()?;
    if operation == "union" {
        return Err(refuse(
            "V3 union lacks a one-body closed-shell separation proof",
        ));
    }
    if !matches!(operation, "intersection" | "difference") {
        return Err(refuse(
            "V3 graph Boolean admits intersection or source-minus-cutter difference",
        ));
    }
    let a_graph = canonical_graph_frame_cell(a, rational, false).ok();
    let b_graph = canonical_graph_frame_cell(b, rational, false).ok();
    let (source, cutter, source_is_a, frame) = match (a_graph, b_graph) {
        (Some(frame), None) => (a, b, true, frame),
        (None, Some(frame)) => (b, a, false, frame),
        _ => return Err(refuse("V3 requires exactly one canonical graph operand")),
    };
    if operation == "difference" && !source_is_a {
        return Err(refuse(
            "V3 cutter-minus-graph difference is outside the certified cell",
        ));
    }
    if cutter.bodies.len() != 1
        || cutter.shells.len() != 1
        || cutter
            .faces
            .iter()
            .any(|face| planar_support(&face.surface, cutter.tolerance_mm * 8.).is_none())
    {
        return Err(refuse("V3 cutter must be one affine-planar closed solid"));
    }
    let (source_face, top, origin, u, v, floor_offset) = frame;
    let context = source
        .tolerance_context()
        .map_err(|_| refuse("V3 source tolerance context is invalid"))?;
    let mut matches = Vec::new();
    for face in &cutter.faces {
        let certificate = if rational {
            certify_exact_rational_planar_iso_intersection(top, &face.surface, &context)
        } else {
            certify_exact_planar_iso_intersection(top, &face.surface, &context)
        };
        if let Ok(certificate) = certificate {
            let support =
                planar_support(&face.surface, cutter.tolerance_mm.max(1e-9) * 8.).unwrap();
            matches.push((certificate, support));
        }
    }
    if matches.len() != 1 {
        return Err(refuse(
            "V3 cutter must expose exactly one strict-interior iso cutting face",
        ));
    }
    let (iso, (plane_origin, plane_normal)) = matches.remove(0);
    if !iso.permits_topology_authorship() {
        return Err(refuse("V3 exact iso authority was revoked"));
    }
    let clear = context.spatial_bounds().clear_mm;
    let distances: Vec<_> = cutter
        .vertices
        .iter()
        .map(|vertex| dot(sub(vertex.point, plane_origin), plane_normal))
        .collect();
    let positive = distances.iter().any(|distance| *distance > clear);
    let negative = distances.iter().any(|distance| *distance < -clear);
    if positive == negative {
        return Err(refuse(
            "V3 cutter does not occupy one strict side of its cutting face",
        ));
    }
    let cutter_sign = if positive { 1. } else { -1. };
    let keep_cutter_side = operation == "intersection";
    let keep_positive_parameter =
        (cutter_sign == iso.transverse_signed_distance_slope.signum()) == keep_cutter_side;
    let fixed = iso.fixed_parameter;
    let bounds = match iso.first_axis {
        ExactIsoAxis::U if keep_positive_parameter => [fixed, 1., 0., 1.],
        ExactIsoAxis::U => [0., fixed, 0., 1.],
        ExactIsoAxis::V if keep_positive_parameter => [0., 1., fixed, 1.],
        ExactIsoAxis::V => [0., 1., 0., fixed],
    };
    use crate::uv_arrangement::{
        LiftedUvGeometry, LiftedUvPrimitive, TensorAxis, TensorBoundary,
        arrange_tensor_bezier_graph_uv,
    };
    let domain = [[0., 1.], [0., 1.]];
    let mut primitives: Vec<_> = [
        TensorBoundary::UMin,
        TensorBoundary::UMax,
        TensorBoundary::VMin,
        TensorBoundary::VMax,
    ]
    .into_iter()
    .enumerate()
    .map(|(edge_id, side)| LiftedUvPrimitive {
        edge_id,
        geometry: LiftedUvGeometry::TensorRectangleBoundary { side, domain },
    })
    .collect();
    primitives.push(LiftedUvPrimitive {
        edge_id: 4,
        geometry: LiftedUvGeometry::TensorIsoLine {
            axis: match iso.first_axis {
                ExactIsoAxis::U => TensorAxis::U,
                ExactIsoAxis::V => TensorAxis::V,
            },
            fixed,
            interval: [0., 1.],
        },
    });
    let arrangement = arrange_tensor_bezier_graph_uv(&context, domain, &primitives, 1, 16)?;
    let correspondence = crate::trim_sew::prove_curve_pcurve_correspondence(
        &context,
        &iso.curve,
        top,
        &iso.uv_traces[0],
        crate::trim_sew::ParameterOrientation::Same,
        crate::trim_sew::BoundaryUse {
            face: source_face,
            wire: source.faces[source_face].outer,
            cyclic_index: 0,
            reversed: false,
        },
    )?;
    let mut result = clipped_graph_solid(source, top, origin, u, v, floor_offset, bounds)?;
    let seam_vertices = match (iso.first_axis, keep_positive_parameter) {
        (ExactIsoAxis::U, true) => (4, 7),
        (ExactIsoAxis::U, false) => (5, 6),
        (ExactIsoAxis::V, true) => (4, 5),
        (ExactIsoAxis::V, false) => (6, 7),
    };
    let seam_edge = edge_between(&result, seam_vertices.0, seam_vertices.1)?;
    let seam_face = match (iso.first_axis, keep_positive_parameter) {
        (ExactIsoAxis::U, true) => 5,
        (ExactIsoAxis::U, false) => 3,
        (ExactIsoAxis::V, true) => 2,
        (ExactIsoAxis::V, false) => 4,
    };
    let source_id = source.1.faces[source_face];
    let retained_id = result.1.faces[1];
    result.1.lineage.push(TopologyLineageRecord {
        operation: "split".into(),
        entity_kind: "face".into(),
        parents: vec![source_id],
        children: vec![retained_id, result.1.faces[seam_face]],
    });
    result.refresh_change_set(&[source]);
    let seam_id = result.1.edges[seam_edge];
    for change in &mut result.1.change_set.changes {
        if change.topo_kind == crate::TopoKind::Face && change.parents.contains(&source_id) {
            change.role = "curved-source-face-split-retained-deleted".into();
            change.provenance.operation = capability.into();
        }
        if change.topo_kind == crate::TopoKind::Edge && change.children.contains(&seam_id) {
            change.role = "generated-intersection-edge".into();
            change.provenance.operation = capability.into();
        }
    }
    result.1.change_set.validate()?;
    let audited = LocallyValidatedModel::new(result)?.audit()?;
    let mut audit = audited.certificate().clone();
    audit.notes.push("v3_graph_patch_cell_separation_ok");
    audit
        .notes
        .push("v3_graph_patch_self_intersection_exclusion_ok");
    let result = audited.into_model();
    let (denominator_lower_bound, denominator_upper_bound) =
        rational_weight_bounds(top).unwrap_or((1., 1.));
    let cert = CurvedGraphBooleanCertificate {
        capability,
        status: "Complete",
        operation: operation.into(),
        axis: iso.first_axis,
        fixed_parameter: fixed,
        source_face: source_id.to_string(),
        retained_face: result.1.faces[1].to_string(),
        deleted_region: if keep_positive_parameter {
            "lower-parameter-cell"
        } else {
            "upper-parameter-cell"
        },
        intersection_edge: result.1.edges[edge_between(&result, seam_vertices.0, seam_vertices.1)?]
            .to_string(),
        tensor_cells: arrangement.cells.len(),
        exact_correspondence: correspondence.permits_exact_correspondence(),
        sew: audit.sew.clone(),
        audit,
        change_set: result.1.change_set.clone(),
        naming_complete: result.persistent_naming_complete(),
        no_fallback: true,
        separation_proof: true,
        homogeneous_root_proof: rational,
        denominator_lower_bound,
        weight_condition_number: denominator_upper_bound / denominator_lower_bound,
        resource_bound: (top.degree_u + 1) * (top.degree_v + 1),
    };
    if !cert.permits_topology_change() {
        return Err(refuse("V3 aggregate certificate failed closed"));
    }
    Ok((result, cert))
}

pub fn nurbs_boolean_graph_patch_v3(
    a: &Model,
    b: &Model,
    operation: &str,
) -> Result<(Model, CurvedGraphBooleanCertificate)> {
    nurbs_boolean_graph_patch_transverse(a, b, operation, NURBS_BOOLEAN_CAPABILITY_V3, false)
}

pub fn nurbs_boolean_graph_patch_unequal_v4(
    a: &Model,
    b: &Model,
    operation: &str,
) -> Result<(Model, CurvedGraphBooleanCertificate)> {
    let (source, cutter) = if canonical_graph_frame(a).is_ok() {
        (a, b)
    } else if canonical_graph_frame(b).is_ok() {
        (b, a)
    } else {
        return Err(refuse(
            "V4 unequal-span cell requires exactly one graph source",
        ));
    };
    let (source_min, source_max) = model_aabb(source);
    let (cutter_min, cutter_max) = model_aabb(cutter);
    let unequal = (0..3).any(|axis| {
        let source_span = source_max[axis] - source_min[axis];
        let cutter_span = cutter_max[axis] - cutter_min[axis];
        (source_span - cutter_span).abs() > source.tolerance_mm.max(cutter.tolerance_mm) * 8.
    });
    if !unequal {
        return Err(refuse("V4 requires a certified unequal source/cutter span"));
    }
    nurbs_boolean_graph_patch_transverse(a, b, operation, NURBS_BOOLEAN_CAPABILITY_V4, false)
}

pub fn nurbs_boolean_rational_graph_patch_v5(
    a: &Model,
    b: &Model,
    operation: &str,
) -> Result<(Model, CurvedGraphBooleanCertificate)> {
    nurbs_boolean_graph_patch_transverse(a, b, operation, NURBS_BOOLEAN_CAPABILITY_V5, true)
}

#[derive(Clone, Debug)]
pub struct GeneralNurbsBooleanNaming {
    pub split: usize,
    pub retained: usize,
    pub deleted: usize,
    pub generated: usize,
    pub operation_stable: bool,
}

/// Aggregate native authority for nurbs-boolean/1 over nurbs-ss/1.
/// The sealed BranchGraph and global UV arrangement are retained, rather than
/// projected to caller-supplied booleans. Independent SS face-pair reports must
/// agree on transverse roots before topology change is granted.
#[derive(Clone, Debug)]
pub struct GeneralNurbsBooleanCertificate {
    pub capability: &'static str,
    pub authority: &'static str,
    pub status: &'static str,
    pub operation: String,
    pub operand_order: &'static str,
    pub exact_region_membership: bool,
    pub partition_cells: usize,
    pub cavity_count: usize,
    pub branch_graph: BranchGraph,
    pub uv: crate::uv_arrangement::MultiSpanUvArrangement,
    pub exact_curve_pcurve_count: usize,
    pub sew: SewCertificate,
    pub audit: SolidAuditCertificate,
    pub change_set: ChangeSet,
    pub naming: GeneralNurbsBooleanNaming,
    pub result_components: usize,
    pub result_faces: usize,
    pub no_fallback: bool,
    /// Independent nurbs-ss/1 face-pair reports that agreed on transverse roots.
    pub ss_reports_complete: bool,
    pub ss_face_pairs: usize,
}

impl GeneralNurbsBooleanCertificate {
    pub fn permits_topology_change(&self) -> bool {
        self.capability == NURBS_BOOLEAN_SS_CAPABILITY
            && self.authority == GENERAL_NURBS_BOOLEAN_AUTHORITY
            && self.status == "Complete"
            && self.ss_reports_complete
            && self.ss_face_pairs > 0
            && matches!(
                self.operation.as_str(),
                "union" | "intersection" | "difference"
            )
            && matches!(self.operand_order, "source-tool" | "tool-source")
            && (self.operation != "union" || self.operand_order == "source-tool")
            && (self.operation != "difference"
                || self.operand_order != "tool-source"
                || self.cavity_count == 0)
            && self.exact_region_membership
            && self.partition_cells > 0
            && self.branch_graph.permits_topology_authorship()
            && self.branch_graph.components.len() >= 2
            && self.uv.permits_trim_classification()
            && self.uv.branch_count == self.branch_graph.components.len()
            && self.exact_curve_pcurve_count
                == self
                    .branch_graph
                    .certificate
                    .fragment_count
                    .saturating_mul(2)
            && self.sew.complete
            && self.audit.ok
            && self.naming.split > 0
            && self.naming.retained > 0
            && self.naming.deleted > 0
            && self.naming.generated > 0
            && self.naming.operation_stable
            && self.result_components == self.audit.body_count
            && self.result_faces > 0
            && self.no_fallback
            && self.change_set.validate().is_ok()
    }
}

pub fn canonical_multispan_graph_solid(spans_u: usize, spans_v: usize) -> Result<Model> {
    if !(1..=2).contains(&spans_u) || !(1..=2).contains(&spans_v) {
        return Err(refuse(
            "General graph fixture admits one or two spans per axis",
        ));
    }
    let mut model = canonical_bezier_graph_solid(3, 3)?;
    let top = canonical_graph_frame_cell(&model, false, false)?.0;
    let mut surface = model.faces[top].surface.clone();
    for index in 1..spans_u {
        let knot = index as f64 / spans_u as f64;
        surface = surface.edit_axis(Axis::U, |curve| curve.insert(knot, 1))?;
    }
    for index in 1..spans_v {
        let knot = index as f64 / spans_v as f64;
        surface = surface.edit_axis(Axis::V, |curve| curve.insert(knot, 1))?;
    }
    model.faces[top].surface = surface;
    model.rebuild_topology_ids();
    model.validate()?;
    canonical_graph_frame_cell(&model, true, true)?;
    Ok(model)
}

pub(crate) fn is_general_nurbs_boolean_candidate(a: &Model, b: &Model) -> bool {
    let general = |model: &Model| {
        canonical_graph_frame_cell(model, true, true)
            .ok()
            .and_then(|(face, surface, ..)| decompose_rational_bezier_spans(surface, face, 64).ok())
            .is_some_and(|decomposition| decomposition.spans.len() > 1)
    };
    general(a) ^ general(b)
}

fn author_general_operation_naming(
    result: &mut Model,
    source: &Model,
    cutter: &Model,
    source_face: usize,
    operation: &str,
    bounds: &[[f64; 4]],
    axis: ExactIsoAxis,
) -> Result<GeneralNurbsBooleanNaming> {
    let root_signature = bounds
        .iter()
        .flat_map(|bound| bound.iter())
        .map(|value| format!("{:016x}", value.to_bits()))
        .collect::<Vec<_>>()
        .join(":");
    let source_occurrence = source.1.bodies[0].to_string();
    let derive_group = |kind: TopoKind, count: usize| {
        (0..count)
            .map(|index| {
                TopoId::derive(
                    kind,
                    GENERAL_NURBS_BOOLEAN_AUTHORITY,
                    &format!("{source_occurrence}:{operation}:{root_signature}:{index}"),
                    kind.as_str(),
                    b"operation-local-topology",
                )
            })
            .collect::<Vec<_>>()
    };
    result.1.vertices = derive_group(TopoKind::Vertex, result.vertices.len());
    result.1.edges = derive_group(TopoKind::Edge, result.edges.len());
    result.1.loops = derive_group(TopoKind::Loop, result.loops.len());
    result.1.faces = derive_group(TopoKind::Face, result.faces.len());
    result.1.shells = derive_group(TopoKind::Shell, result.shells.len());
    result.1.bodies = derive_group(TopoKind::Body, result.bodies.len());
    result.1.lineage.clear();

    let mut change_set = ChangeSet::default();
    for (kind, ids) in source
        .identity_groups()
        .into_iter()
        .chain(cutter.identity_groups())
    {
        change_set.nodes.extend(ids.iter().map(|id| (*id, kind)));
    }
    for (kind, ids) in result.identity_groups() {
        change_set.nodes.extend(ids.iter().map(|id| (*id, kind)));
    }
    let mut split_children = Vec::new();
    for (component, bound) in bounds.iter().enumerate() {
        let base = component * 6;
        split_children.push(result.1.faces[base + 1]);
        let candidates = match axis {
            ExactIsoAxis::U => [(bound[0], base + 5), (bound[1], base + 3)],
            ExactIsoAxis::V => [(bound[2], base + 2), (bound[3], base + 4)],
        };
        split_children.extend(
            candidates
                .into_iter()
                .filter(|(parameter, _)| *parameter > 0. && *parameter < 1.)
                .map(|(_, face)| result.1.faces[face]),
        );
    }
    split_children.sort();
    split_children.dedup();
    change_set.changes.push(TopologyChange {
        kind: ChangeKind::Split,
        topo_kind: TopoKind::Face,
        parents: vec![source.1.faces[source_face]],
        children: split_children.clone(),
        provenance: ChangeProvenance {
            operation: GENERAL_NURBS_BOOLEAN_AUTHORITY.into(),
            operand: Some(source_occurrence.clone()),
            occurrence: root_signature.clone(),
        },
        role: format!("{operation}-split-retained-generated-boundaries"),
        anchor: Some("global-uv-cells".into()),
    });
    let split_set = split_children.iter().copied().collect::<BTreeSet<_>>();
    let mut generated = 0;
    for (kind, ids) in result.identity_groups() {
        for id in ids {
            if kind == TopoKind::Face && split_set.contains(id) {
                continue;
            }
            generated += 1;
            change_set.changes.push(TopologyChange {
                kind: ChangeKind::Generated,
                topo_kind: kind,
                parents: vec![],
                children: vec![*id],
                provenance: ChangeProvenance {
                    operation: GENERAL_NURBS_BOOLEAN_AUTHORITY.into(),
                    operand: Some(source_occurrence.clone()),
                    occurrence: id.to_string(),
                },
                role: format!("{operation}-generated-{}", kind.as_str()),
                anchor: None,
            });
        }
    }
    let mut deleted = 0;
    for (kind, ids) in cutter.identity_groups() {
        for id in ids {
            deleted += 1;
            change_set.changes.push(TopologyChange {
                kind: ChangeKind::Deleted,
                topo_kind: kind,
                parents: vec![*id],
                children: vec![],
                provenance: ChangeProvenance {
                    operation: GENERAL_NURBS_BOOLEAN_AUTHORITY.into(),
                    operand: cutter.1.bodies.first().map(ToString::to_string),
                    occurrence: id.to_string(),
                },
                role: format!("{operation}-deleted-cutter-{}", kind.as_str()),
                anchor: None,
            });
        }
    }
    change_set.validate()?;
    result.1.change_set = change_set;
    result.validate()?;
    Ok(GeneralNurbsBooleanNaming {
        split: 1,
        retained: split_children.len(),
        deleted,
        generated,
        operation_stable: true,
    })
}

fn graph_frame_box(
    origin: [f64; 3],
    u: [f64; 3],
    v: [f64; 3],
    normal: [f64; 3],
    bounds: [[f64; 2]; 3],
    tolerance: f64,
) -> Result<Model> {
    let local = crate::cuboid(
        [bounds[0][0], bounds[1][0], bounds[2][0]],
        [bounds[0][1], bounds[1][1], bounds[2][1]],
    )?;
    let matrix = [
        [u[0], v[0], normal[0], origin[0]],
        [u[1], v[1], normal[1], origin[1]],
        [u[2], v[2], normal[2], origin[2]],
        [0., 0., 0., 1.],
    ];
    let mut model = crate::transform::affine(&local, matrix)?;
    model.tolerance_mm = tolerance;
    model.validate()?;
    Ok(model)
}

/// Finite closed-solid author for a positive rational multispan graph and an
/// affine slab exposing at least two disjoint, transverse, complete iso
/// branches. The `/8` successor also authors an exact disjoint-interior
/// partition for union and tool-minus-source difference without fallback.
pub fn author_general_nurbs_boolean(
    a: &Model,
    b: &Model,
    operation: &str,
) -> Result<(Model, GeneralNurbsBooleanCertificate)> {
    a.validate()?;
    b.validate()?;
    if !matches!(operation, "union" | "intersection" | "difference") {
        return Err(refuse(
            "General NURBS author admits union, intersection, or difference only",
        ));
    }
    let a_graph = canonical_graph_frame_cell(a, true, true).ok();
    let b_graph = canonical_graph_frame_cell(b, true, true).ok();
    let (source, cutter, source_is_a, frame) = match (a_graph, b_graph) {
        (Some(frame), None) => (a, b, true, frame),
        (None, Some(frame)) => (b, a, false, frame),
        _ => {
            return Err(refuse(
                "Exactly one operand must be an admitted multispan graph solid",
            ));
        }
    };
    if cutter.bodies.len() != 1
        || cutter.shells.len() != 1
        || !cutter.bodies[0].inner_shells.is_empty()
        || cutter.faces.iter().any(|face| {
            planar_support(&face.surface, cutter.tolerance_mm.max(1e-9) * 8.).is_none()
                || face.surface.periodic_u
                || face.surface.periodic_v
        })
    {
        return Err(refuse(
            "General NURBS cutter must be one bounded affine closed solid",
        ));
    }
    if cutter.faces.iter().any(|face| {
        face.surface
            .weights
            .iter()
            .flatten()
            .any(|weight| weight.to_bits() != 1f64.to_bits())
    }) {
        return Err(refuse(
            "Unsupported rational cutter boundary parameterization",
        ));
    }
    let (source_face, top, origin, u, v, floor_offset) = frame;
    let context = source
        .tolerance_context()
        .map_err(|_| refuse("General NURBS source tolerance context is invalid"))?;
    let clear = context.spatial_bounds().clear_mm;
    let source_decomposition = decompose_rational_bezier_spans(top, source_face, 64)?;
    let mut fragments = Vec::new();
    let mut candidate_pairs = 0usize;
    let mut plane_span_count = 0usize;
    let mut denominator_lower_bound = source_decomposition.denominator_lower_bound;
    let mut ss_face_pairs = 0usize;
    let mut ss_roots = Vec::<f64>::new();
    for (cutter_face, face) in cutter.faces.iter().enumerate() {
        let Some((plane_origin, plane_normal)) =
            planar_support(&face.surface, cutter.tolerance_mm.max(1e-9) * 8.)
        else {
            unreachable!()
        };
        let distances = top
            .control_points
            .iter()
            .flatten()
            .map(|point| {
                dot(
                    sub([point[0], point[1], point[2]], plane_origin),
                    plane_normal,
                )
            })
            .collect::<Vec<_>>();
        let minimum = distances.iter().copied().fold(f64::INFINITY, f64::min);
        let maximum = distances.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        if minimum < -clear && maximum > clear {
            let ss = nurbs_core::ss_intersection::intersect_surface_surface(
                top,
                &face.surface,
                source.tolerance_context().ok(),
            )
            .map_err(|error| {
                refuse(&format!(
                    "nurbs-ss/1 refused cutter face {cutter_face}: {}",
                    error.message
                ))
            })?;
            if ss["version"] != "nurbs-ss/1"
                || ss["coverage"]["complete"] != true
                || ss["coverage"]["missedBranchProof"] != true
                || ss["booleanMutationAuthority"] != false
                || !ss["unresolved"]
                    .as_array()
                    .map(|a| a.is_empty())
                    .unwrap_or(false)
            {
                return Err(refuse(
                    "nurbs-boolean/1 requires complete nurbs-ss/1 reports without Boolean mutation authority",
                ));
            }
            ss_face_pairs = ss_face_pairs
                .checked_add(1)
                .ok_or_else(|| Error::new("BREP_SS_RESOURCE_LIMIT", "SS face-pair overflow"))?;
            for component in ss["components"].as_array().cloned().unwrap_or_default() {
                if component["kind"] != "curve" || component["contactClass"] != "transverse" {
                    if matches!(component["kind"].as_str(), Some("empty") | Some("overlap")) {
                        continue;
                    }
                    if component["kind"] == "curve" {
                        return Err(refuse(
                            "nurbs-boolean/1 admits only transverse curve strata from nurbs-ss/1",
                        ));
                    }
                    continue;
                }
                let pcurve = &component["pcurveFirst"];
                let start = pcurve
                    .get("start")
                    .and_then(value_codec::Value::as_array)
                    .ok_or_else(|| refuse("SS pcurveFirst missing start"))?;
                let end = pcurve
                    .get("end")
                    .and_then(value_codec::Value::as_array)
                    .ok_or_else(|| refuse("SS pcurveFirst missing end"))?;
                let s0 = start[0].as_f64().unwrap_or(f64::NAN);
                let s1 = start[1].as_f64().unwrap_or(f64::NAN);
                let e0 = end[0].as_f64().unwrap_or(f64::NAN);
                let e1 = end[1].as_f64().unwrap_or(f64::NAN);
                if (s0 - e0).abs() <= clear {
                    ss_roots.push(s0);
                } else if (s1 - e1).abs() <= clear {
                    ss_roots.push(s1);
                } else {
                    return Err(refuse(
                        "nurbs-boolean/1 requires iso-aligned SS pcurves on the source chart",
                    ));
                }
            }
            let graph = certify_multispan_ss(
                top,
                &face.surface,
                [source_face, cutter_face],
                &context,
                64,
            )
            .map_err(|error| {
                refuse(&format!(
                    "Cutter face {cutter_face} did not provide a complete transverse branch: {}",
                    error.message
                ))
            })?;
            candidate_pairs = candidate_pairs
                .checked_add(graph.certificate.candidate_span_pairs)
                .ok_or_else(|| {
                    Error::new("BREP_SS_RESOURCE_LIMIT", "Branch-pair count overflow")
                })?;
            plane_span_count += graph.certificate.source_span_count[1];
            denominator_lower_bound =
                denominator_lower_bound.min(graph.certificate.denominator_lower_bound);
            fragments.extend(
                graph
                    .components
                    .into_iter()
                    .flat_map(|component| component.fragments),
            );
        } else if minimum <= clear && maximum >= -clear {
            return Err(refuse(
                "Tangent, coincident, or gray-band cutter face is ambiguous",
            ));
        }
    }
    let branch_graph = join_certified_multispan_fragments(
        fragments,
        &context,
        [source_decomposition.spans.len(), plane_span_count],
        candidate_pairs,
        denominator_lower_bound,
        64,
    )?;
    if branch_graph.components.len() < 2 {
        return Err(refuse(
            "General NURBS author requires multiple disjoint transverse branches",
        ));
    }
    let mut branch_axis = None;
    let mut roots = Vec::new();
    for component in &branch_graph.components {
        let trace = &component.fragments[0].pcurves[0];
        let a = &trace.control_points[0];
        let b = trace.control_points.last().unwrap();
        let (axis, fixed) = if a[0].to_bits() == b[0].to_bits() {
            (ExactIsoAxis::U, a[0])
        } else if a[1].to_bits() == b[1].to_bits() {
            (ExactIsoAxis::V, a[1])
        } else {
            return Err(refuse("Branch pcurve is not an exact source iso"));
        };
        if branch_axis
            .replace(axis)
            .is_some_and(|previous| previous != axis)
        {
            return Err(refuse("Disjoint branch families cross"));
        }
        roots.push(fixed);
    }
    roots.sort_by(f64::total_cmp);
    roots.dedup_by(|left, right| left.to_bits() == right.to_bits());
    if roots.len() != 2 || roots[0] <= 0. || roots[1] >= 1. {
        return Err(refuse(
            "Finite affine slab requires exactly two strict-interior branch roots",
        ));
    }
    ss_roots.sort_by(f64::total_cmp);
    ss_roots.dedup_by(|left, right| (*left - *right).abs() <= clear);
    if ss_roots.len() != roots.len()
        || ss_roots
            .iter()
            .zip(&roots)
            .any(|(ss, branch)| (ss - branch).abs() > clear)
    {
        return Err(refuse(
            "nurbs-ss/1 transverse roots disagree with sealed BranchGraph roots",
        ));
    }
    if ss_face_pairs == 0 {
        return Err(refuse(
            "nurbs-boolean/1 requires at least one complete nurbs-ss/1 face pair",
        ));
    }
    let axis = branch_axis.unwrap();
    let uv = crate::uv_arrangement::arrange_multispan_branch_graph_uv(
        &context,
        [
            &source_decomposition.u_breaks,
            &source_decomposition.v_breaks,
        ],
        &branch_graph,
        0,
        64,
    )?;

    let normal = normalize(cross(u, v)).ok_or_else(|| refuse("Graph frame is singular"))?;
    let normal = if dot(floor_offset, normal) < 0. {
        normal
    } else {
        normal.map(|value| -value)
    };
    let uu = dot(u, u);
    let uv_dot = dot(u, v);
    let vv = dot(v, v);
    let determinant = uu * vv - uv_dot * uv_dot;
    if determinant <= 1e-18 {
        return Err(refuse("Graph projection is singular"));
    }
    let mut projected = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    for vertex in &cutter.vertices {
        let rhs = sub(vertex.point, origin);
        let ru = dot(rhs, u);
        let rv = dot(rhs, v);
        let values = [
            (ru * vv - rv * uv_dot) / determinant,
            (rv * uu - ru * uv_dot) / determinant,
            dot(rhs, normal),
        ];
        for index in 0..3 {
            projected[0][index] = projected[0][index].min(values[index]);
            projected[1][index] = projected[1][index].max(values[index]);
        }
    }
    let varying = if axis == ExactIsoAxis::U { 1 } else { 0 };
    let fixed = if axis == ExactIsoAxis::U { 0 } else { 1 };
    let floor_height = dot(floor_offset, normal);
    let roof_height = top
        .control_points
        .iter()
        .flatten()
        .map(|point| dot(sub([point[0], point[1], point[2]], origin), normal))
        .fold(f64::NEG_INFINITY, f64::max);
    if cutter.vertices.len() != 8
        || cutter.faces.len() != 6
        || cutter.vertices.iter().any(|vertex| {
            let rhs = sub(vertex.point, origin);
            let ru = dot(rhs, u);
            let rv = dot(rhs, v);
            let local = [
                (ru * vv - rv * uv_dot) / determinant,
                (rv * uu - ru * uv_dot) / determinant,
                dot(rhs, normal),
            ];
            (0..3).any(|index| {
                (local[index] - projected[0][index]).abs() > clear
                    && (local[index] - projected[1][index]).abs() > clear
            })
        })
    {
        return Err(refuse(
            "Affine cutter is not an exact graph-frame parallelotope",
        ));
    }
    if projected[0][varying] > -clear
        || projected[1][varying] < 1. + clear
        || projected[0][2] > floor_height - clear
        || projected[1][2] < roof_height + clear
        || (projected[0][fixed] - roots[0]).abs() > clear
        || (projected[1][fixed] - roots[1]).abs() > clear
    {
        return Err(refuse(
            "Affine cutter lacks complete varying/floor/roof slab coverage",
        ));
    }
    let source_bounds = match (operation, source_is_a, axis) {
        ("intersection", _, ExactIsoAxis::U) => vec![[roots[0], roots[1], 0., 1.]],
        ("intersection", _, ExactIsoAxis::V) => vec![[0., 1., roots[0], roots[1]]],
        ("difference", true, ExactIsoAxis::U) => {
            vec![[0., roots[0], 0., 1.], [roots[1], 1., 0., 1.]]
        }
        ("difference", true, ExactIsoAxis::V) => {
            vec![[0., 1., 0., roots[0]], [0., 1., roots[1], 1.]]
        }
        ("union", true, ExactIsoAxis::U) => {
            vec![[0., roots[0], 0., 1.], [roots[1], 1., 0., 1.]]
        }
        ("union", true, ExactIsoAxis::V) => {
            vec![[0., 1., 0., roots[0]], [0., 1., roots[1], 1.]]
        }
        ("union", false, _) => return Err(refuse("Union requires graph-source operand order")),
        ("difference", false, _) => vec![],
        _ => unreachable!(),
    };
    let naming_bounds = if source_bounds.is_empty() {
        match axis {
            ExactIsoAxis::U => vec![[roots[0], roots[1], 0., 1.]],
            ExactIsoAxis::V => vec![[0., 1., roots[0], roots[1]]],
        }
    } else {
        source_bounds.clone()
    };
    let mut pieces = source_bounds
        .iter()
        .map(|bound| clipped_graph_solid(source, top, origin, u, v, floor_offset, *bound))
        .collect::<Result<Vec<_>>>()?;
    if operation == "union" {
        pieces.push(cutter.clone());
    } else if operation == "difference" && !source_is_a {
        let [fixed_min, fixed_max] = [roots[0], roots[1]];
        let (u_bounds, v_bounds) = match axis {
            ExactIsoAxis::U => ([fixed_min, fixed_max], [0., 1.]),
            ExactIsoAxis::V => ([0., 1.], [fixed_min, fixed_max]),
        };
        let roof_offset = normal.map(|value| value * projected[1][2]);
        pieces.push(clipped_graph_solid(
            source,
            top,
            origin,
            u,
            v,
            roof_offset,
            [u_bounds[0], u_bounds[1], v_bounds[0], v_bounds[1]],
        )?);
        if projected[0][2] < floor_height - clear {
            pieces.push(graph_frame_box(
                origin,
                u,
                v,
                normal,
                [
                    u_bounds,
                    [projected[0][1], projected[1][1]],
                    [projected[0][2], floor_height],
                ],
                source.tolerance_mm.max(cutter.tolerance_mm),
            )?);
        }
        for outside_v in [[projected[0][1], 0.], [1., projected[1][1]]] {
            if outside_v[1] > outside_v[0] + clear {
                pieces.push(graph_frame_box(
                    origin,
                    u,
                    v,
                    normal,
                    [u_bounds, outside_v, [floor_height, projected[1][2]]],
                    source.tolerance_mm.max(cutter.tolerance_mm),
                )?);
            }
        }
        for outside_u in [
            [projected[0][0], u_bounds[0]],
            [u_bounds[1], projected[1][0]],
        ] {
            if outside_u[1] > outside_u[0] + clear {
                pieces.push(graph_frame_box(
                    origin,
                    u,
                    v,
                    normal,
                    [
                        outside_u,
                        [projected[0][1], projected[1][1]],
                        [floor_height, projected[1][2]],
                    ],
                    source.tolerance_mm.max(cutter.tolerance_mm),
                )?);
            }
        }
    }
    if pieces.is_empty() {
        return Err(refuse("Exact regularized region partition is empty"));
    }
    let partition_cells = pieces.len();
    let mut result = pieces.remove(0);
    for piece in pieces {
        result = crate::boolean_support::separated_union(&result, &piece)?;
    }
    let naming = author_general_operation_naming(
        &mut result,
        source,
        cutter,
        source_face,
        operation,
        &naming_bounds,
        axis,
    )?;
    let audited = LocallyValidatedModel::new(result)?.audit()?;
    let mut audit = audited.certificate().clone();
    audit.notes.push("global_multispan_uv_classification_ok");
    audit
        .notes
        .push("cell_specific_self_intersection_exclusion_ok");
    audit.notes.push("cavity_and_component_separation_ok");
    let result = audited.into_model();
    let certificate = GeneralNurbsBooleanCertificate {
        capability: NURBS_BOOLEAN_SS_CAPABILITY,
        authority: GENERAL_NURBS_BOOLEAN_AUTHORITY,
        status: "Complete",
        operation: operation.into(),
        operand_order: if source_is_a {
            "source-tool"
        } else {
            "tool-source"
        },
        exact_region_membership: true,
        partition_cells,
        cavity_count: 0,
        exact_curve_pcurve_count: branch_graph.certificate.fragment_count * 2,
        branch_graph,
        uv,
        sew: audit.sew.clone(),
        audit,
        change_set: result.1.change_set.clone(),
        naming,
        result_components: result.bodies.len(),
        result_faces: result.faces.len(),
        no_fallback: true,
        ss_reports_complete: true,
        ss_face_pairs,
    };
    if !certificate.permits_topology_change() || !result.persistent_naming_complete() {
        return Err(refuse("General NURBS aggregate authority failed closed"));
    }
    Ok((result, certificate))
}

pub(crate) fn graph_affine_strict_containment_bounds(
    graph: &Model,
    cutter: &Model,
) -> Result<Option<[f64; 3]>> {
    let Ok((_, top, origin, u, v, floor_offset)) = canonical_graph_frame(graph) else {
        return Ok(None);
    };
    if cutter.bodies.len() != 1
        || cutter.shells.len() != 1
        || !cutter.bodies[0].inner_shells.is_empty()
        || cutter
            .faces
            .iter()
            .any(|face| planar_support(&face.surface, cutter.tolerance_mm.max(1e-9) * 8.).is_none())
    {
        return Ok(None);
    }
    let normal =
        normalize(cross(u, v)).ok_or_else(|| refuse("V4 containment graph frame is singular"))?;
    let normal = if dot(floor_offset, normal) < 0. {
        normal
    } else {
        normal.map(|value| -value)
    };
    let floor_height = dot(floor_offset, normal);
    let roof_lower_bound = top
        .control_points
        .iter()
        .flatten()
        .map(|point| dot(sub([point[0], point[1], point[2]], origin), normal))
        .fold(f64::INFINITY, f64::min);
    let uu = dot(u, u);
    let uv = dot(u, v);
    let vv = dot(v, v);
    let determinant = uu * vv - uv * uv;
    if determinant <= 1e-18 {
        return Ok(None);
    }
    let clear = graph
        .tolerance_context()
        .map_err(|_| refuse("V4 containment tolerance context is invalid"))?
        .spatial_bounds()
        .clear_mm;
    let mut bounds = [f64::INFINITY; 3];
    for vertex in &cutter.vertices {
        let rhs = sub(vertex.point, origin);
        let ru = dot(rhs, u);
        let rv = dot(rhs, v);
        let su = (ru * vv - rv * uv) / determinant;
        let sv = (rv * uu - ru * uv) / determinant;
        let height = dot(rhs, normal);
        bounds[0] = bounds[0].min(su).min(1. - su).min(sv).min(1. - sv);
        bounds[1] = bounds[1].min(height - floor_height);
        bounds[2] = bounds[2].min(roof_lower_bound - height);
    }
    Ok((bounds.iter().all(|value| *value > clear)).then_some(bounds))
}

/// Finite `/4` containment cell: an affine cutter lies strictly inside the
/// canonical graph volume and below the convex-hull lower roof bound.
pub fn nurbs_boolean_graph_containment_v4(
    graph: &Model,
    cutter: &Model,
    operation: &str,
) -> Result<(Model, ContainedGraphBooleanCertificate)> {
    graph.validate()?;
    cutter.validate()?;
    if !matches!(operation, "union" | "intersection" | "difference") {
        return Err(refuse(
            "V4 containment admits union, intersection, or difference",
        ));
    }
    let (_, top, origin, u, v, floor_offset) = canonical_graph_frame(graph)?;
    if cutter.bodies.len() != 1
        || cutter.shells.len() != 1
        || !cutter.bodies[0].inner_shells.is_empty()
        || cutter
            .faces
            .iter()
            .any(|face| planar_support(&face.surface, cutter.tolerance_mm.max(1e-9) * 8.).is_none())
    {
        return Err(refuse(
            "V4 containment cutter must be one affine-planar body",
        ));
    }
    let normal =
        normalize(cross(u, v)).ok_or_else(|| refuse("V4 containment graph frame is singular"))?;
    let normal = if dot(floor_offset, normal) < 0. {
        normal
    } else {
        normal.map(|value| -value)
    };
    let floor_height = dot(floor_offset, normal);
    let roof_lower_bound = top
        .control_points
        .iter()
        .flatten()
        .map(|point| dot(sub([point[0], point[1], point[2]], origin), normal))
        .fold(f64::INFINITY, f64::min);
    let uu = dot(u, u);
    let uv = dot(u, v);
    let vv = dot(v, v);
    let determinant = uu * vv - uv * uv;
    if determinant <= 1e-18 {
        return Err(refuse("V4 containment graph projection is singular"));
    }
    let clear = graph
        .tolerance_context()
        .map_err(|_| refuse("V4 containment tolerance context is invalid"))?
        .spatial_bounds()
        .clear_mm;
    let mut uv_margin = f64::INFINITY;
    let mut floor_clearance = f64::INFINITY;
    let mut roof_clearance = f64::INFINITY;
    for vertex in &cutter.vertices {
        let rhs = sub(vertex.point, origin);
        let ru = dot(rhs, u);
        let rv = dot(rhs, v);
        let su = (ru * vv - rv * uv) / determinant;
        let sv = (rv * uu - ru * uv) / determinant;
        let height = dot(rhs, normal);
        uv_margin = uv_margin.min(su).min(1. - su).min(sv).min(1. - sv);
        floor_clearance = floor_clearance.min(height - floor_height);
        roof_clearance = roof_clearance.min(roof_lower_bound - height);
    }
    if uv_margin <= clear || floor_clearance <= clear || roof_clearance <= clear {
        return Err(refuse(
            "V4 strict containment, floor, or convex-hull roof separation proof failed",
        ));
    }
    let tolerance = graph.tolerance_mm.max(cutter.tolerance_mm);
    let mut result = match operation {
        "union" => graph.clone(),
        "intersection" => cutter.clone(),
        "difference" => crate::imprint_pipeline::cavity(graph, cutter, tolerance)?,
        _ => unreachable!(),
    };
    result.inherit_topology_ids(&[graph, cutter]);
    let current_ids: std::collections::BTreeSet<_> = result
        .identity_groups()
        .into_iter()
        .flat_map(|(_, ids)| ids.iter().copied())
        .collect();
    for change in &mut result.1.change_set.changes {
        if change.children.iter().any(|id| current_ids.contains(id)) {
            change.provenance.operation = NURBS_BOOLEAN_CAPABILITY_V4.into();
            change.role = format!("v4-{operation}-{}", change.topo_kind.as_str());
        }
    }
    result.1.change_set.validate()?;
    let audited = LocallyValidatedModel::new(result)?.audit()?;
    let mut audit = audited.certificate().clone();
    audit.notes.push("v4_strict_containment_convex_bounds_ok");
    if operation == "difference" {
        audit.notes.push("v4_inverted_cavity_shell_ok");
    }
    let result = audited.into_model();
    let cert = ContainedGraphBooleanCertificate {
        capability: NURBS_BOOLEAN_CAPABILITY_V4,
        status: "Complete",
        operation: operation.into(),
        relation: "graph-strictly-contains-affine-cutter",
        strict_uv_margin: uv_margin,
        floor_clearance,
        roof_clearance,
        cavity_proof: operation != "difference" || result.bodies[0].inner_shells.len() == 1,
        separation_proof: true,
        audit,
        change_set: result.1.change_set.clone(),
        naming_complete: result.persistent_naming_complete(),
        no_fallback: true,
    };
    if !cert.permits_topology_change() {
        return Err(refuse("V4 containment aggregate certificate failed closed"));
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

    fn planar_yz_wide(x: f64) -> Surface {
        let mut surface = planar_yz(x);
        for i in 0..4 {
            for j in 0..4 {
                surface.control_points[i][j] =
                    vec![x, -1. + j as f64 * 5. / 3., -1. + i as f64 * 5. / 3.];
            }
        }
        surface
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
        let b = planar_yz_wide(1.5);
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
                &planar_yz_wide(1.5),
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

    fn graph_patch(du: usize, dv: usize) -> Surface {
        let mut surface = bezier_le3_xy(du, dv, 0., 0.);
        if du > 1 && dv > 1 {
            surface.control_points[1][1][2] = 0.2;
        }
        surface
    }

    fn planar_xz_wide(y: f64) -> Surface {
        let mut surface = planar_yz_wide(0.);
        for i in 0..4 {
            for j in 0..4 {
                surface.control_points[i][j] =
                    vec![-1. + j as f64 * 5. / 3., y, -1. + i as f64 * 5. / 3.];
            }
        }
        surface
    }

    fn transform_surface(mut surface: Surface) -> Surface {
        for point in surface.control_points.iter_mut().flatten() {
            let [x, y, z] = [point[0], point[1], point[2]];
            *point = vec![
                2. * x + 0.25 * y + 7.,
                -0.5 * x + 1.5 * y - 3.,
                0.2 * x - 0.1 * y + 0.75 * z + 5.,
            ];
        }
        surface
    }

    #[test]
    fn exact_iso_u_v_degree_2_3_survive_affine_transform() {
        let context = ToleranceContext::default_valid();
        for &(du, dv) in &[(2, 2), (2, 3), (3, 2), (3, 3)] {
            for (graph, cutter, axis) in [
                (
                    transform_surface(graph_patch(du, dv)),
                    transform_surface(planar_yz_wide(1.5)),
                    ExactIsoAxis::U,
                ),
                (
                    transform_surface(graph_patch(du, dv)),
                    transform_surface(planar_xz_wide(1.5)),
                    ExactIsoAxis::V,
                ),
            ] {
                let certificate =
                    certify_exact_planar_iso_intersection(&graph, &cutter, &context).unwrap();
                assert_eq!(certificate.first_axis, axis);
                assert!(certificate.permits_topology_authorship());
                assert_eq!(
                    certificate.curve.degree,
                    if axis == ExactIsoAxis::U { dv } else { du }
                );
                assert_eq!(certificate.uv_traces[0].degree, certificate.curve.degree);
            }
        }
    }

    #[test]
    fn iso_certificate_mutations_revoke_authority() {
        let context = ToleranceContext::default_valid();
        let certificate = certify_exact_planar_iso_intersection(
            &graph_patch(3, 3),
            &planar_yz_wide(1.5),
            &context,
        )
        .unwrap();
        assert!(certificate.permits_topology_authorship());
        let mut mutated = certificate.clone();
        mutated.curve.control_points[1][2] += 1e-9;
        assert!(!mutated.permits_topology_authorship());
        let mut mutated = certificate.clone();
        mutated.uv_traces[1].control_points[1][0] += 1e-9;
        assert!(!mutated.permits_topology_authorship());
        let mut mutated = certificate.clone();
        mutated.fixed_parameter = f64::from_bits(mutated.fixed_parameter.to_bits() + 1);
        assert!(!mutated.permits_topology_authorship());
        let mut mutated = certificate.clone();
        mutated.endpoints.first_support.swap(0, 1);
        assert!(!mutated.permits_topology_authorship());
        let mut mutated = certificate.clone();
        mutated.evidence.claims.clear();
        assert!(!mutated.permits_topology_authorship());
        let mut mutated = certificate;
        mutated.strict_transverse = false;
        assert!(!mutated.permits_topology_authorship());
    }

    #[test]
    fn iso_certifier_refuses_rational_periodic_multispan_boundary_corner_and_nonunique() {
        let context = ToleranceContext::default_valid();
        let cutter = planar_yz_wide(1.5);
        let mut rational = graph_patch(3, 3);
        rational.weights[1][1] = 2.;
        assert!(certify_exact_planar_iso_intersection(&rational, &cutter, &context).is_err());

        let mut periodic = graph_patch(3, 3);
        periodic.periodic_u = true;
        assert!(certify_exact_planar_iso_intersection(&periodic, &cutter, &context).is_err());

        let multispan = graph_patch(3, 3)
            .edit_axis(Axis::U, |curve| curve.insert(0.5, 1))
            .unwrap();
        assert!(certify_exact_planar_iso_intersection(&multispan, &cutter, &context).is_err());

        assert!(
            certify_exact_planar_iso_intersection(
                &graph_patch(3, 3),
                &planar_yz_wide(0.),
                &context
            )
            .is_err()
        );
        assert!(
            certify_exact_planar_iso_intersection(&graph_patch(3, 3), &planar_yz(1.5), &context)
                .is_err()
        );

        for coefficients in [[-1., 2., -2., 1.], [0., 0., 0., 0.], [1., 0., 0., 1.]] {
            let mut nonunique = graph_patch(3, 3);
            for (i, value) in coefficients.into_iter().enumerate() {
                for j in 0..=nonunique.degree_v {
                    nonunique.control_points[i][j][0] = value;
                    nonunique.control_points[i][j][2] = i as f64;
                }
            }
            assert!(
                certify_exact_planar_iso_intersection(&nonunique, &planar_yz_wide(0.), &context)
                    .is_err()
            );
        }
    }

    #[test]
    fn v3_graph_patch_u_v_degree_2_3_intersection_and_difference() {
        for (du, dv, cutter) in [
            (2, 2, crate::cuboid([0.5, -1., -1.], [2., 2., 3.]).unwrap()),
            (2, 3, crate::cuboid([-1., 0.5, -1.], [2., 2., 3.]).unwrap()),
            (3, 2, crate::cuboid([0.5, -1., -1.], [2., 2., 3.]).unwrap()),
            (3, 3, crate::cuboid([-1., 0.5, -1.], [2., 2., 3.]).unwrap()),
        ] {
            let graph = canonical_bezier_graph_solid(du, dv).unwrap();
            for operation in ["intersection", "difference"] {
                let (result, certificate) =
                    nurbs_boolean_graph_patch_v3(&graph, &cutter, operation).unwrap();
                assert!(
                    certificate.permits_topology_change(),
                    "{du}x{dv} {operation}"
                );
                assert_eq!(certificate.tensor_cells, 3);
                assert!(
                    certificate
                        .audit
                        .notes
                        .contains(&"v3_graph_patch_self_intersection_exclusion_ok")
                );
                result.validate().unwrap();
                assert_eq!(result.bodies.len(), 1);
                assert_eq!(result.faces.len(), 6);
                assert_eq!(
                    result.vertices.len() as isize - result.edges.len() as isize
                        + result.faces.len() as isize,
                    2
                );
                let expected_total =
                    1. + 0.25 * ((du - 1) * (dv - 1)) as f64 / ((du + 1) * (dv + 1)) as f64;
                let mass = crate::analysis::mass_properties(&result, 1e-9, 300_000).unwrap();
                assert!(
                    (mass.signed_volume_mm3.abs() - expected_total * 0.5).abs() < 1e-7,
                    "{du}x{dv} {operation}: {:?}",
                    mass.signed_volume_mm3
                );
            }
        }
    }

    #[test]
    fn v3_graph_patch_transform_reflection_swap_and_refusals() {
        let graph = canonical_bezier_graph_solid(3, 2).unwrap();
        let cutter = crate::cuboid([0.5, -1., -1.], [2., 2., 3.]).unwrap();
        let matrix = [
            [0., -1., 0., 4.],
            [-1., 0., 0., 3.],
            [0., 0., 1., 2.],
            [0., 0., 0., 1.],
        ];
        let graph = crate::transform::affine(&graph, matrix).unwrap();
        let cutter = crate::transform::affine(&cutter, matrix).unwrap();
        let (_, certificate) =
            nurbs_boolean_graph_patch_v3(&cutter, &graph, "intersection").unwrap();
        assert!(certificate.permits_topology_change());
        let mut mutation = certificate.clone();
        mutation.tensor_cells = 2;
        assert!(!mutation.permits_topology_change());
        let mut mutation = certificate.clone();
        mutation.sew.complete = false;
        assert!(!mutation.permits_topology_change());
        let mut mutation = certificate.clone();
        mutation.naming_complete = false;
        assert!(!mutation.permits_topology_change());
        let mut mutation = certificate.clone();
        mutation.change_set.changes.clear();
        assert!(!mutation.permits_topology_change());
        assert!(nurbs_boolean_graph_patch_v3(&graph, &cutter, "union").is_err());
        assert!(nurbs_boolean_graph_patch_v3(&cutter, &graph, "difference").is_err());

        let mut rational = graph.clone();
        let face = canonical_graph_frame(&rational).unwrap().0;
        rational.faces[face].surface.weights[1][1] = 2.;
        assert!(nurbs_boolean_graph_patch_v3(&rational, &cutter, "intersection").is_err());
        let mut multispan = graph.clone();
        let face = canonical_graph_frame(&multispan).unwrap().0;
        multispan.faces[face].surface = multispan.faces[face]
            .surface
            .edit_axis(Axis::U, |curve| curve.insert(0.25, 1))
            .unwrap();
        assert!(nurbs_boolean_graph_patch_v3(&multispan, &cutter, "intersection").is_err());
        let boundary = crate::transform::affine(
            &crate::cuboid([0., -1., -1.], [2., 2., 3.]).unwrap(),
            matrix,
        )
        .unwrap();
        assert!(nurbs_boolean_graph_patch_v3(&graph, &boundary, "intersection").is_err());
    }

    #[test]
    fn v4_unequal_span_and_containment_operations_are_independently_certified() {
        let graph = canonical_bezier_graph_solid(3, 3).unwrap();
        let unequal = crate::cuboid([0.5, -2., -3.], [2., 3., 4.]).unwrap();
        for operation in ["intersection", "difference"] {
            let (result, certificate) =
                nurbs_boolean_graph_patch_unequal_v4(&graph, &unequal, operation).unwrap();
            assert_eq!(certificate.capability, NURBS_BOOLEAN_CAPABILITY_V4);
            assert!(certificate.permits_topology_change());
            assert!(
                certificate
                    .change_set
                    .changes
                    .iter()
                    .any(|change| { change.provenance.operation == NURBS_BOOLEAN_CAPABILITY_V4 })
            );
            result.validate().unwrap();
        }

        let inner = crate::cuboid([0.2, 0.2, 0.2], [0.8, 0.8, 0.8]).unwrap();
        for operation in ["union", "intersection", "difference"] {
            let (result, certificate) =
                nurbs_boolean_graph_containment_v4(&graph, &inner, operation).unwrap();
            assert!(certificate.permits_topology_change(), "{operation}");
            assert_eq!(certificate.operation, operation);
            assert!(certificate.strict_uv_margin > 0.);
            assert!(certificate.floor_clearance > 0.);
            assert!(certificate.roof_clearance > 0.);
            assert_eq!(
                result.bodies[0].inner_shells.len(),
                usize::from(operation == "difference")
            );
        }
        let (_, certificate) =
            nurbs_boolean_graph_containment_v4(&graph, &inner, "difference").unwrap();
        let mut mutation = certificate.clone();
        mutation.cavity_proof = false;
        assert!(!mutation.permits_topology_change());
        let mut mutation = certificate;
        mutation.roof_clearance = 0.;
        assert!(!mutation.permits_topology_change());
    }

    #[test]
    fn v4_containment_boundary_and_reversed_difference_refuse() {
        let graph = canonical_bezier_graph_solid(2, 3).unwrap();
        let boundary = crate::cuboid([0., 0.2, 0.2], [0.8, 0.8, 0.8]).unwrap();
        assert!(nurbs_boolean_graph_containment_v4(&graph, &boundary, "difference").is_err());
        let inner = crate::cuboid([0.2, 0.2, 0.2], [0.8, 0.8, 0.8]).unwrap();
        assert!(nurbs_boolean_graph_containment_v4(&inner, &graph, "difference").is_err());
    }

    #[test]
    fn v5_rational_graph_has_homogeneous_root_denominator_and_resource_proofs() {
        for (du, dv, cutter) in [
            (2, 2, crate::cuboid([0.5, -1., -1.], [2., 2., 3.]).unwrap()),
            (3, 3, crate::cuboid([0.5, -2., -1.], [2., 3., 4.]).unwrap()),
        ] {
            let graph = canonical_rational_graph_solid(du, dv).unwrap();
            for operation in ["intersection", "difference"] {
                let (result, certificate) =
                    nurbs_boolean_rational_graph_patch_v5(&graph, &cutter, operation).unwrap();
                assert_eq!(certificate.capability, NURBS_BOOLEAN_CAPABILITY_V5);
                assert!(certificate.permits_topology_change());
                assert!(certificate.homogeneous_root_proof);
                assert!(certificate.denominator_lower_bound >= 0.25);
                assert!(certificate.weight_condition_number <= 8.);
                assert!(certificate.resource_bound <= 16);
                assert!(
                    result.faces[1]
                        .surface
                        .weights
                        .iter()
                        .flatten()
                        .any(|weight| (*weight - 1.).abs() > 1e-15)
                );
            }
        }
    }

    #[test]
    fn v5_weight_root_resource_tangency_and_certificate_mutations_refuse() {
        let cutter = crate::cuboid([0.5, -1., -1.], [2., 2., 3.]).unwrap();
        let graph = canonical_rational_graph_solid(3, 3).unwrap();
        let (_, certificate) =
            nurbs_boolean_rational_graph_patch_v5(&graph, &cutter, "intersection").unwrap();
        for mutation in [
            {
                let mut value = certificate.clone();
                value.homogeneous_root_proof = false;
                value
            },
            {
                let mut value = certificate.clone();
                value.denominator_lower_bound = 0.;
                value
            },
            {
                let mut value = certificate.clone();
                value.resource_bound = 17;
                value
            },
        ] {
            assert!(!mutation.permits_topology_change());
        }

        let mut low_weight = graph.clone();
        let face = canonical_graph_frame_cell(&low_weight, true, false)
            .unwrap()
            .0;
        low_weight.faces[face].surface.weights[0][1] = 0.125;
        assert!(
            nurbs_boolean_rational_graph_patch_v5(&low_weight, &cutter, "intersection").is_err()
        );

        let mut nonfactorable = graph;
        let face = canonical_graph_frame_cell(&nonfactorable, true, false)
            .unwrap()
            .0;
        nonfactorable.faces[face].surface.weights[1][1] += 0.125;
        assert!(
            nurbs_boolean_rational_graph_patch_v5(&nonfactorable, &cutter, "intersection").is_err()
        );

        let tangent = crate::cuboid([0., -1., -1.], [2., 2., 3.]).unwrap();
        assert!(
            nurbs_boolean_rational_graph_patch_v5(
                &canonical_rational_graph_solid(3, 3).unwrap(),
                &tangent,
                "intersection"
            )
            .is_err()
        );
        assert!(
            nurbs_boolean_rational_graph_patch_v5(
                &canonical_rational_graph_solid(3, 3).unwrap(),
                &cutter,
                "union"
            )
            .is_err()
        );
    }

    fn multispan_wave(u_spans: usize, v_spans: usize, rational: bool, swap_uv: bool) -> Surface {
        let u_count = u_spans + 1;
        let v_count = v_spans + 1;
        let mut controls = vec![vec![vec![0.; 3]; v_count]; u_count];
        let mut weights = vec![vec![1.; v_count]; u_count];
        for u in 0..u_count {
            for v in 0..v_count {
                let signed = if u % 2 == 0 { -1. } else { 1. };
                let point = [signed, u as f64, v as f64];
                controls[u][v] = if swap_uv {
                    vec![point[0], point[2], point[1]]
                } else {
                    point.to_vec()
                };
                if rational {
                    weights[u][v] = if u % 2 == 0 { 1. } else { 2. };
                }
            }
        }
        let knots = |spans: usize| {
            let mut result = vec![0., 0.];
            result.extend((1..spans).map(|value| value as f64));
            result.extend([spans as f64; 2]);
            result
        };
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: knots(u_spans),
            knots_v: knots(v_spans),
            control_points: controls,
            weights,
            periodic_u: false,
            periodic_v: false,
        }
    }

    fn multispan_plane(size: f64) -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 0., size]],
                vec![vec![0., size, 0.], vec![0., size, size]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }

    #[test]
    fn multispan_decomposition_covers_2x1_1x2_and_2x2() {
        for (u, v, expected) in [(2, 1, 2), (1, 2, 2), (2, 2, 4)] {
            let decomposition =
                decompose_rational_bezier_spans(&multispan_wave(u, v, false, false), 17, 16)
                    .unwrap();
            assert!(decomposition.permits_exact_decomposition());
            assert_eq!(decomposition.spans.len(), expected);
            assert_eq!(
                decomposition
                    .spans
                    .iter()
                    .filter(|span| span.owns_upper[0])
                    .count(),
                v
            );
            assert!(decomposition.denominator_lower_bound > 0.);
        }
    }

    #[test]
    fn multispan_two_disjoint_branches_join_across_knot_boundaries() {
        let context = ToleranceContext::default_valid();
        let graph = certify_multispan_ss(
            &multispan_wave(2, 2, false, false),
            &multispan_plane(2.),
            [3, 9],
            &context,
            32,
        )
        .unwrap();
        assert!(graph.permits_topology_authorship());
        assert_eq!(graph.components.len(), 2);
        assert_eq!(graph.certificate.fragment_count, 4);
        assert!(
            graph
                .components
                .iter()
                .all(|component| component.fragments.len() == 2 && !component.closed)
        );
    }

    #[test]
    fn multispan_internal_knot_has_one_half_open_owner() {
        let context = ToleranceContext::default_valid();
        let mut source = multispan_wave(2, 2, false, false);
        for u in 0..3 {
            for v in 0..3 {
                source.control_points[u][v][0] = u as f64 - 1.;
            }
        }
        let graph =
            certify_multispan_ss(&source, &multispan_plane(2.), [5, 6], &context, 32).unwrap();
        assert_eq!(graph.components.len(), 1);
        assert_eq!(graph.components[0].fragments.len(), 2);
        assert!(
            graph.components[0]
                .fragments
                .iter()
                .all(|fragment| fragment.source_spans[0].u == 1)
        );
    }

    #[test]
    fn multispan_v_reversal_rational_weights_and_rigid_transform() {
        let context = ToleranceContext::default_valid();
        let mut reversed = multispan_wave(2, 2, true, true);
        reversed = reversed
            .edit_axis(Axis::U, |curve| curve.reverse())
            .unwrap()
            .edit_axis(Axis::V, |curve| curve.reverse())
            .unwrap();
        let mut plane = multispan_plane(2.);
        plane = plane
            .edit_axis(Axis::U, |curve| curve.reverse())
            .unwrap()
            .edit_axis(Axis::V, |curve| curve.reverse())
            .unwrap();
        let transform = |mut surface: Surface| {
            for point in surface.control_points.iter_mut().flatten() {
                let [x, y, z] = [point[0], point[1], point[2]];
                *point = vec![-y + 4., x + 7., z - 3.];
            }
            surface
        };
        let graph = certify_multispan_ss(
            &transform(reversed),
            &transform(plane),
            [4, 8],
            &context,
            32,
        )
        .unwrap();
        assert!(graph.permits_topology_authorship());
        assert_eq!(graph.components.len(), 2);
        assert!(graph.certificate.denominator_lower_bound >= 1.);
    }

    #[test]
    fn multispan_fragment_permutation_is_deterministic() {
        let context = ToleranceContext::default_valid();
        let graph = certify_multispan_ss(
            &multispan_wave(2, 2, false, false),
            &multispan_plane(2.),
            [1, 2],
            &context,
            32,
        )
        .unwrap();
        let mut fragments = graph
            .components
            .iter()
            .flat_map(|component| component.fragments.clone())
            .collect::<Vec<_>>();
        fragments.reverse();
        let joined = join_certified_multispan_fragments(
            fragments,
            &context,
            graph.certificate.source_span_count,
            graph.certificate.candidate_span_pairs,
            graph.certificate.denominator_lower_bound,
            32,
        )
        .unwrap();
        assert_eq!(joined.components.len(), graph.components.len());
        for (a, b) in joined.components.iter().zip(&graph.components) {
            assert_eq!(
                joined_endpoint_key(&a.fragments[0], 0).unwrap(),
                joined_endpoint_key(&b.fragments[0], 0).unwrap()
            );
        }
    }

    #[test]
    fn multispan_ambiguity_coincidence_resource_and_certificate_mutations_refuse() {
        let context = ToleranceContext::default_valid();
        let source = multispan_wave(2, 2, false, false);
        let plane = multispan_plane(2.);
        assert_eq!(
            certify_multispan_ss(&source, &plane, [0, 1], &context, 1)
                .unwrap_err()
                .code,
            "BREP_SS_RESOURCE_LIMIT"
        );
        let graph = certify_multispan_ss(&source, &plane, [0, 1], &context, 32).unwrap();
        let duplicate = graph
            .components
            .iter()
            .flat_map(|component| component.fragments.clone())
            .chain(
                graph
                    .components
                    .iter()
                    .flat_map(|component| component.fragments.clone()),
            )
            .collect();
        assert!(
            join_certified_multispan_fragments(
                duplicate,
                &context,
                graph.certificate.source_span_count,
                graph.certificate.candidate_span_pairs,
                graph.certificate.denominator_lower_bound,
                32
            )
            .is_err()
        );
        let mut mutated = graph.clone();
        mutated.certificate.fragment_count -= 1;
        assert!(!mutated.permits_topology_authorship());
        let mut mutated = graph.clone();
        mutated.components[0].fragments[0].pcurves[0].control_points[0][0] += 1e-9;
        assert!(!mutated.permits_topology_authorship());
        let mut mutated = graph.clone();
        mutated.components[0].fragments[0].evidence.signed_slope += 1e-9;
        assert!(!mutated.permits_topology_authorship());
        let mut mutated = graph.clone();
        mutated.certificate.candidate_span_pairs += 1;
        assert!(!mutated.permits_topology_authorship());

        let mut coincident = source.clone();
        for point in coincident.control_points.iter_mut().flatten() {
            point[0] = 0.;
        }
        assert!(certify_multispan_ss(&coincident, &plane, [0, 1], &context, 32).is_err());
        let tangent = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![1., 0., 0.], vec![1., 0., 2.]],
                vec![vec![-1., 1., 0.], vec![-1., 1., 2.]],
                vec![vec![1., 2., 0.], vec![1., 2., 2.]],
            ],
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        assert!(certify_multispan_ss(&tangent, &plane, [0, 1], &context, 32).is_err());
        let mut unresolved_multi_root = tangent.clone();
        unresolved_multi_root.degree_u = 3;
        unresolved_multi_root.knots_u = vec![0., 0., 0., 0., 1., 1., 1., 1.];
        unresolved_multi_root
            .control_points
            .insert(2, vec![vec![-1., 1.5, 0.], vec![-1., 1.5, 2.]]);
        unresolved_multi_root.weights.insert(2, vec![1.; 2]);
        assert!(
            certify_multispan_ss(&unresolved_multi_root, &plane, [0, 1], &context, 32).is_err()
        );
        let mut periodic = source.clone();
        periodic.periodic_u = true;
        assert!(certify_multispan_ss(&periodic, &plane, [0, 1], &context, 32).is_err());
        let mut high_degree = source;
        high_degree = high_degree
            .edit_axis(Axis::U, |curve| curve.elevate(4))
            .unwrap();
        assert!(certify_multispan_ss(&high_degree, &plane, [0, 1], &context, 32).is_err());
    }

    #[test]
    fn general_multispan_boolean_authors_exact_region_operations() {
        for spans in [(2, 1), (1, 2), (2, 2)] {
            let graph = canonical_multispan_graph_solid(spans.0, spans.1).unwrap();
            let cutter = crate::cuboid([0.25, -1., -1.], [0.75, 2., 3.]).unwrap();
            let graph_snapshot = value_codec::Serialize::to_value(&graph);
            let cutter_snapshot = value_codec::Serialize::to_value(&cutter);
            for (operation, bodies, faces) in [("intersection", 1, 6), ("difference", 2, 12)] {
                let (result, certificate) =
                    author_general_nurbs_boolean(&graph, &cutter, operation).unwrap();
                assert_eq!(certificate.capability, NURBS_BOOLEAN_SS_CAPABILITY);
                assert!(certificate.permits_topology_change());
                assert_eq!(certificate.branch_graph.components.len(), 2);
                assert_eq!(certificate.result_components, bodies);
                assert_eq!(result.faces.len(), faces);
                assert!(certificate.exact_curve_pcurve_count >= 4);
                assert!(certificate.no_fallback && certificate.naming.operation_stable);
                let top_domains = (0..bodies)
                    .map(|component| {
                        let top = &result.faces[component * 6 + 1].surface;
                        [
                            top.knots_u[top.degree_u],
                            top.knots_u[top.control_points.len()],
                        ]
                    })
                    .collect::<Vec<_>>();
                assert_eq!(
                    top_domains,
                    if operation == "intersection" {
                        vec![[0.25, 0.75]]
                    } else {
                        vec![[0., 0.25], [0.75, 1.]]
                    }
                );
                if operation == "intersection" {
                    let (permuted, _) =
                        author_general_nurbs_boolean(&cutter, &graph, operation).unwrap();
                    assert_eq!(result.1.faces, permuted.1.faces);
                    assert_eq!(result.1.edges, permuted.1.edges);
                }
            }
            assert_eq!(value_codec::Serialize::to_value(&graph), graph_snapshot);
            assert_eq!(value_codec::Serialize::to_value(&cutter), cutter_snapshot);
            for (a, b, operation, order) in [
                (&graph, &cutter, "union", "source-tool"),
                (&cutter, &graph, "difference", "tool-source"),
            ] {
                let (result, certificate) = author_general_nurbs_boolean(a, b, operation).unwrap();
                assert_eq!(certificate.capability, NURBS_BOOLEAN_SS_CAPABILITY);
                assert_eq!(certificate.operand_order, order);
                assert!(certificate.exact_region_membership);
                assert_eq!(
                    certificate.partition_cells,
                    if operation == "union" { 3 } else { 4 }
                );
                assert_eq!(certificate.result_components, certificate.partition_cells);
                assert_eq!(certificate.result_faces, certificate.partition_cells * 6);
                assert!(certificate.sew.complete && certificate.audit.ok);
                assert!(certificate.change_set.validate().is_ok());
                assert!(certificate.permits_topology_change());
                assert!(result.persistent_naming_complete());
            }
            assert_eq!(value_codec::Serialize::to_value(&graph), graph_snapshot);
            assert_eq!(value_codec::Serialize::to_value(&cutter), cutter_snapshot);
        }
    }

    #[test]
    fn general_multispan_boolean_naming_is_rigid_transform_stable() {
        let graph = canonical_multispan_graph_solid(2, 2).unwrap();
        let cutter = crate::cuboid([0.25, -1., -1.], [0.75, 2., 3.]).unwrap();
        let (base, _) = author_general_nurbs_boolean(&graph, &cutter, "difference").unwrap();
        let matrix = [
            [0., -1., 0., 4.],
            [1., 0., 0., -2.],
            [0., 0., 1., 3.],
            [0., 0., 0., 1.],
        ];
        let moved_graph = crate::transform::affine(&graph, matrix).unwrap();
        let moved_cutter = crate::transform::affine(&cutter, matrix).unwrap();
        let (moved, _) =
            author_general_nurbs_boolean(&moved_graph, &moved_cutter, "difference").unwrap();
        assert_eq!(base.1.vertices, moved.1.vertices);
        assert_eq!(base.1.edges, moved.1.edges);
        assert_eq!(base.1.faces, moved.1.faces);
        assert_eq!(base.1.bodies, moved.1.bodies);
    }

    #[test]
    fn general_multispan_boolean_has_independent_regularized_volume_oracle() {
        let graph = canonical_multispan_graph_solid(2, 2).unwrap();
        let cutter = crate::cuboid([0.25, -1., -1.], [0.75, 2., 3.]).unwrap();
        let volume = |model: &Model| {
            crate::analysis::mass_properties(model, 1e-9, 500_000)
                .unwrap()
                .signed_volume_mm3
                .abs()
        };
        let intersection = author_general_nurbs_boolean(&graph, &cutter, "intersection")
            .unwrap()
            .0;
        let source_difference = author_general_nurbs_boolean(&graph, &cutter, "difference")
            .unwrap()
            .0;
        let union = author_general_nurbs_boolean(&graph, &cutter, "union")
            .unwrap()
            .0;
        let reversed = author_general_nurbs_boolean(&cutter, &graph, "difference")
            .unwrap()
            .0;
        let (vg, vc, vi, vd, vu, vr) = (
            volume(&graph),
            volume(&cutter),
            volume(&intersection),
            volume(&source_difference),
            volume(&union),
            volume(&reversed),
        );
        for residual in [vg - vi - vd, vc - vi - vr, vu - vg - vc + vi] {
            assert!(
                residual.abs() < 1e-7,
                "regularized volume residual {residual:e}"
            );
        }
    }
}
