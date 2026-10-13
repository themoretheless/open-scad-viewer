use super::*;

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
pub(super) struct ExactIsoAuthority {
    pub(super) curve: RationalCurveDefinition,
    pub(super) traces: [RationalCurveDefinition; 2],
    pub(super) axis: ExactIsoAxis,
    pub(super) fixed_parameter_bits: u64,
    pub(super) transverse_slope_bits: u64,
    pub(super) endpoints: IsoEndpointOwnership,
    pub(super) context: ToleranceSpecIdentity,
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
    pub(super) authority: ExactIsoAuthority,
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

pub(super) fn is_uniform_bicubic_positive(surface: &Surface) -> bool {
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

pub(super) fn rational_weights_constant_on_fixed_axis(surface: &Surface, fixed_u: bool) -> bool {
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

pub(super) fn bbox(s: &Surface) -> ([f64; 3], [f64; 3]) {
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
pub use crate::vec3::{cross, dot, norm, normalize, sub};

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

pub(super) fn complete_empty() -> Report<G6Component> {
    Report {
        components: vec![G6Component::Empty],
        boxes_visited: 1,
        coverage: Coverage::Complete,
        ..Report::default()
    }
}

pub(super) fn complete_line(start: [f64; 3], end: [f64; 3]) -> Report<G6Component> {
    Report {
        components: vec![G6Component::Line { start, end }],
        boxes_visited: 1,
        coverage: Coverage::Complete,
        ..Report::default()
    }
}

pub(super) fn complete_curve(certificate: ExactIsoIntersectionCertificate) -> Report<G6Component> {
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

pub(super) fn affine_plane_uv(surface: &Surface, p: [f64; 3], tol: f64) -> Result<Option<[f64; 2]>> {
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

pub(super) fn uv_location(uv: [f64; 2], tol: f64) -> Result<IsoEndpointLocation> {
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

pub(super) fn affine_graph_projection(surface: &Surface, tol: f64) -> bool {
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

pub(super) fn affine_uv_trace(surface: &Surface, curve: &Curve, tol: f64) -> Result<Option<Curve>> {
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
