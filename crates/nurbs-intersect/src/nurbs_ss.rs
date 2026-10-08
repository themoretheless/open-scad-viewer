//! Narrow G6 productization scaffold gated by the R1 ADR verdict (`narrow`),
//! expanded through R1.1 (elevated planar bilinear→bicubic) to Phase B:
//! non-rational single-span Bezier patches with deg_u,deg_v ∈ {1,2,3}, elevated
//! to uniform bicubic before the transverse gate (`nurbs-ss-bezier-le3/1`).
//!
//! Beyond Complete-empty: planar × planar may publish a transverse line as
//! Complete. Non-planar pairs may publish a Complete curve only when a Hausdorff
//! + parameter-correspondence certificate holds. Out-of-matrix pairs refuse.

//!
//! False Complete is a kill. Narrow Boolean imprint is Phase B (`bezier-le3`).

use crate::coverage_verifier::verify_complete_report;
use crate::predicate_evidence::{
    ComposedEvidence, EvidenceClaim, PredicateEvidence, compose_predicate_evidence,
};
use crate::{Coverage, Options, RationalCurveDefinition, Report, UnresolvedReason};
use cad_predicates::{ToleranceContext, ToleranceSpecIdentity};
use nurbs_core::surface::Axis;
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};
use std::collections::{BTreeMap, BTreeSet};

#[doc(hidden)]
pub fn refuse(message: &str) -> Error {
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
        certificate: Box<ExactIsoIntersectionCertificate>,
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
#[doc(hidden)]
pub fn is_bezier_le3_positive(surface: &Surface) -> bool {
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

#[doc(hidden)]
pub fn rational_weight_bounds(surface: &Surface) -> Option<(f64, f64)> {
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

#[doc(hidden)]
pub fn positive_multispan_graph(surface: &Surface) -> bool {
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

#[doc(hidden)]
pub fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
#[doc(hidden)]
pub fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
#[doc(hidden)]
pub fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
#[doc(hidden)]
pub fn norm(a: [f64; 3]) -> f64 {
    a[0].hypot(a[1]).hypot(a[2])
}
#[doc(hidden)]
pub fn normalize(a: [f64; 3]) -> Option<[f64; 3]> {
    let n = norm(a);
    if !n.is_finite() || n == 0. {
        None
    } else {
        Some([a[0] / n, a[1] / n, a[2] / n])
    }
}

/// Recover a supporting plane when all control points are coplanar.
#[doc(hidden)]
pub fn planar_support(surface: &Surface, tol: f64) -> Option<([f64; 3], [f64; 3])> {
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
    Report {
        components: vec![G6Component::Empty],
        boxes_visited: 1,
        coverage: Coverage::Complete,
        ..Report::default()
    }
}

fn complete_line(start: [f64; 3], end: [f64; 3]) -> Report<G6Component> {
    Report {
        components: vec![G6Component::Line { start, end }],
        boxes_visited: 1,
        coverage: Coverage::Complete,
        ..Report::default()
    }
}

fn complete_curve(certificate: ExactIsoIntersectionCertificate) -> Report<G6Component> {
    let boxes_visited = certificate.diagnostic_samples.len().max(1);
    let samples = certificate
        .diagnostic_samples
        .iter()
        .map(|sample| sample.0)
        .collect();
    Report {
        components: vec![G6Component::Curve {
            certificate: Box::new(certificate),
            samples,
        }],
        boxes_visited,
        coverage: Coverage::Complete,
        ..Report::default()
    }
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
            for (i, coefficient_slot) in coefficients.iter_mut().enumerate() {
                for j in 0..=varying_degree {
                    let (u, v) = if fixed_u { (i, j) } else { (j, i) };
                    let point = &span.patch.control_points[u][v];
                    let coefficient = span.patch.weights[u][v]
                        * dot(
                            sub([point[0], point[1], point[2]], plane_origin),
                            plane_normal,
                        );
                    if j == 0 {
                        *coefficient_slot = coefficient;
                    } else if (coefficient - *coefficient_slot).abs()
                        > 32.
                            * f64::EPSILON
                            * coefficient.abs().max((*coefficient_slot).abs()).max(1.)
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
    } else if pb.is_none() && pa.is_some() {
        let context = ToleranceContext::default_valid();
        if let Ok(certificate) = certify_exact_planar_iso_intersection(source_b, source_a, &context)
        {
            let report = complete_curve(certificate);
            verify_complete_report(&report, false)?;
            return Ok(report);
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

fn mix_pt(a: &[f64], b: &[f64], t: f64) -> Vec<f64> {
    a.iter()
        .zip(b.iter())
        .map(|(u, v)| (1. - t) * u + t * v)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coverage_verifier::{verify_complete_report, verify_incomplete_refusal};

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
        for (i, row) in surface.control_points.iter_mut().take(4).enumerate() {
            for (j, point) in row.iter_mut().take(4).enumerate() {
                *point = vec![x, -1. + j as f64 * 5. / 3., -1. + i as f64 * 5. / 3.];
            }
        }
        surface
    }

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
        let iu = (du / 2).min(du);
        let iv = (dv / 2).min(dv);
        s.control_points[iu][iv][2] = 0.02;
        s
    }

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
        for (i, row) in surface.control_points.iter_mut().take(4).enumerate() {
            for (j, point) in row.iter_mut().take(4).enumerate() {
                *point = vec![-1. + j as f64 * 5. / 3., y, -1. + i as f64 * 5. / 3.];
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
        for (u, row) in source.control_points.iter_mut().enumerate().take(3) {
            for point in row.iter_mut().take(3) {
                point[0] = u as f64 - 1.;
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
}
