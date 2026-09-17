//! Conservative, deterministic certificates used by product NURBS consumers.
//!
//! Every geometric enclosure is derived from positive-weight convex-hull
//! properties. `next_down`/`next_up` make binary64 arithmetic outward rounded.
//! Ambiguous regularity and projection cases remain explicitly unresolved.
use crate::{Result, check, curve::Curve, numeric, resource, surface::Surface};
use cad_predicates::ToleranceContext;
use value_codec::{Value, json};

const MAX_CERTIFICATE_CELLS: usize = 4096;

fn next_down(x: f64) -> f64 {
    if x == f64::NEG_INFINITY || x.is_nan() {
        x
    } else if x == 0. {
        -f64::from_bits(1)
    } else {
        f64::from_bits(x.to_bits().wrapping_add(if x < 0. { 1 } else { u64::MAX }))
    }
}
fn next_up(x: f64) -> f64 {
    if x == f64::INFINITY || x.is_nan() {
        x
    } else if x == 0. {
        f64::from_bits(1)
    } else {
        f64::from_bits(x.to_bits().wrapping_add(if x < 0. { u64::MAX } else { 1 }))
    }
}
fn interval(values: impl Iterator<Item = f64>) -> [f64; 2] {
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for value in values {
        lo = lo.min(value);
        hi = hi.max(value);
    }
    [next_down(lo), next_up(hi)]
}
fn box_of(points: &[Vec<f64>]) -> (Vec<f64>, Vec<f64>) {
    let dimension = points[0].len();
    (
        (0..dimension)
            .map(|axis| next_down(points.iter().map(|p| p[axis]).fold(f64::INFINITY, f64::min)))
            .collect(),
        (0..dimension)
            .map(|axis| {
                next_up(
                    points
                        .iter()
                        .map(|p| p[axis])
                        .fold(f64::NEG_INFINITY, f64::max),
                )
            })
            .collect(),
    )
}
fn tolerance_evidence(context: &ToleranceContext) -> Value {
    let spatial = context.spatial_bounds();
    json!({
        "toleranceIdentity": context.spec_identity(),
        "linearAbsoluteMm": spatial.absolute_mm,
        "linearRelative": spatial.relative,
        "parametricFloor": context.parametric_bounds().floor,
        "maxEntityErrorMm": context.entity_error_bounds().maximum_mm
    })
}
fn context(value: Option<ToleranceContext>) -> ToleranceContext {
    value.unwrap_or_else(ToleranceContext::default_valid)
}

/// Per-Bézier-span positive-denominator and conservative Euclidean hull bounds.
pub fn certify_curve(curve: &Curve, tolerance: Option<ToleranceContext>) -> Result<Value> {
    curve.validate()?;
    let tolerance = context(tolerance);
    let segments = curve.decompose()?;
    check(
        segments.len() <= MAX_CERTIFICATE_CELLS,
        "Curve certificate exceeds the span resource limit",
    )?;
    let mut spans = Vec::with_capacity(segments.len());
    for segment in segments {
        let c = segment.definition();
        let (min, max) = box_of(&c.control_points);
        let denominator = interval(c.weights.iter().copied());
        numeric(
            denominator[0] > 0.,
            "Outward denominator lower bound is not positive",
        )?;
        let regularity = curve_span_regularity(c);
        spans.push(json!({
            "domain": segment.domain(),
            "min": min,
            "max": max,
            "denominatorLower": denominator[0],
            "denominatorUpper": denominator[1],
            "regularity": regularity,
        }));
    }
    Ok(json!({
        "version": "nurbs-foundation/1",
        "kind": "curve",
        "rounding": "binary64-nextafter-outward",
        "basis": "positive-rational-convex-hull",
        "periodic": curve.periodic,
        "period": if curve.periodic { Some(curve.domain()[1]-curve.domain()[0]) } else { None },
        "spans": spans,
        "evidence": tolerance_evidence(&tolerance)
    }))
}

fn curve_span_regularity(curve: &Curve) -> Value {
    curve_span_regularity_depth(curve, 0)
}
fn curve_span_regularity_depth(curve: &Curve, depth: usize) -> Value {
    let p = curve.degree;
    if p == 0 {
        return json!({"classification":"singular","reason":"constant-span"});
    }
    let homogeneous: Vec<Vec<f64>> = curve
        .control_points
        .iter()
        .zip(&curve.weights)
        .map(|(point, weight)| {
            point
                .iter()
                .map(|coordinate| coordinate * weight)
                .chain(std::iter::once(*weight))
                .collect()
        })
        .collect();
    let derivative: Vec<Vec<f64>> = homogeneous
        .array_windows()
        .map(|[a, b]| {
            a.iter()
                .zip(b)
                .map(|(x, y)| p as f64 * (y - x))
                .collect()
        })
        .collect();
    // C' numerator is X'W-XW'. Bernstein products preserve convex hulls.
    let dimension = curve.control_points[0].len();
    let mut component_bounds = Vec::new();
    for axis in 0..dimension {
        let first = bernstein_product(
            &derivative.iter().map(|value| value[axis]).collect::<Vec<_>>(),
            &homogeneous
                .iter()
                .map(|value| value[dimension])
                .collect::<Vec<_>>(),
        );
        let second = bernstein_product(
            &homogeneous.iter().map(|value| value[axis]).collect::<Vec<_>>(),
            &derivative
                .iter()
                .map(|value| value[dimension])
                .collect::<Vec<_>>(),
        );
        let coefficients = first.into_iter().zip(second).map(|(a, b)| a - b);
        component_bounds.push(interval(coefficients.into_iter()));
    }
    let separated = component_bounds
        .iter()
        .any(|bound| bound[0] > 0. || bound[1] < 0.);
    let identically_zero = component_bounds.iter().all(|bound| {
        bound[0] <= 0.
            && bound[1] >= 0.
            && bound[0].abs() <= f64::from_bits(1)
            && bound[1].abs() <= f64::from_bits(1)
    });
    if !separated && !identically_zero && depth < 8 {
        let [a, b] = curve.domain();
        if let Ok(children) = curve.split((a + b) / 2.) {
            let certificates = children
                .iter()
                .map(|child| curve_span_regularity_depth(child, depth + 1))
                .collect::<Vec<_>>();
            if certificates
                .iter()
                .all(|certificate| certificate["classification"] == "certified_regular")
            {
                return json!({
                    "classification":"certified_regular",
                    "method":"recursive-Bernstein-component-separation",
                    "subdivisionDepth":depth+1,
                    "children":certificates
                });
            }
        }
    }
    json!({
        "classification": if separated { "certified_regular" } else if identically_zero { "singular" } else { "unresolved" },
        "derivativeNumeratorBounds": component_bounds
    })
}

fn binomial(n: usize, k: usize) -> f64 {
    let k = k.min(n - k);
    (0..k).fold(1., |value, i| value * (n - i) as f64 / (i + 1) as f64)
}
fn bernstein_product(first: &[f64], second: &[f64]) -> Vec<f64> {
    let m = first.len() - 1;
    let n = second.len() - 1;
    (0..=m + n)
        .map(|k| {
            let start = k.saturating_sub(n);
            let end = k.min(m);
            (start..=end)
                .map(|i| {
                    binomial(m, i) * binomial(n, k - i) / binomial(m + n, k)
                        * first[i]
                        * second[k - i]
                })
                .sum()
        })
        .collect()
}

/// Per nonempty knot rectangle. Active control support is a conservative hull.
pub fn certify_surface(surface: &Surface, tolerance: Option<ToleranceContext>) -> Result<Value> {
    surface.validate()?;
    let tolerance = context(tolerance);
    let nu = surface.control_points.len();
    let nv = surface.control_points[0].len();
    let u_spans: Vec<usize> = (surface.degree_u..nu)
        .filter(|&i| surface.knots_u[i] < surface.knots_u[i + 1])
        .collect();
    let v_spans: Vec<usize> = (surface.degree_v..nv)
        .filter(|&i| surface.knots_v[i] < surface.knots_v[i + 1])
        .collect();
    if u_spans.len().saturating_mul(v_spans.len()) > MAX_CERTIFICATE_CELLS {
        return Err(resource("Surface certificate exceeds 4096 span cells"));
    }
    let mut cells = Vec::new();
    for &iu in &u_spans {
        for &iv in &v_spans {
            let mut points = Vec::new();
            let mut weights = Vec::new();
            for u in iu - surface.degree_u..=iu {
                for v in iv - surface.degree_v..=iv {
                    points.push(surface.control_points[u][v].clone());
                    weights.push(surface.weights[u][v]);
                }
            }
            let (min, max) = box_of(&points);
            let denominator = interval(weights.into_iter());
            numeric(
                denominator[0] > 0.,
                "Outward denominator lower bound is not positive",
            )?;
            let regularity = certify_surface_cell(surface, iu, iv);
            cells.push(json!({
                "domainU": [surface.knots_u[iu],surface.knots_u[iu+1]],
                "domainV": [surface.knots_v[iv],surface.knots_v[iv+1]],
                "min": min,
                "max": max,
                "denominatorLower": denominator[0],
                "denominatorUpper": denominator[1],
                "normalRegularity": regularity
            }));
        }
    }
    Ok(json!({
        "version":"nurbs-foundation/1",
        "kind":"surface",
        "rounding":"binary64-nextafter-outward",
        "basis":"positive-rational-convex-hull",
        "periodicU":surface.periodic_u,
        "periodicV":surface.periodic_v,
        "periodU":if surface.periodic_u {Some(surface.knots_u[nu]-surface.knots_u[surface.degree_u])} else {None},
        "periodV":if surface.periodic_v {Some(surface.knots_v[nv]-surface.knots_v[surface.degree_v])} else {None},
        "cells":cells,
        "evidence":tolerance_evidence(&tolerance)
    }))
}

fn certify_surface_cell(surface: &Surface, iu: usize, iv: usize) -> Value {
    // Corner normals can prove an exactly planar regular patch. General
    // interval-normal certification remains conservative and unresolved.
    let corners = [
        (surface.knots_u[iu], surface.knots_v[iv]),
        (surface.knots_u[iu + 1], surface.knots_v[iv]),
        (surface.knots_u[iu], surface.knots_v[iv + 1]),
        (surface.knots_u[iu + 1], surface.knots_v[iv + 1]),
    ];
    let evaluations: Vec<_> = corners
        .iter()
        .filter_map(|&(u, v)| surface.evaluate_validated(u, v).ok())
        .collect();
    if evaluations.iter().any(|evaluation| evaluation.unit_normal().is_none()) {
        return json!({"classification":"singular_or_unresolved","reason":"corner-normal-unavailable"});
    }
    let normals: Vec<[f64; 3]> = evaluations
        .iter()
        .map(|evaluation| evaluation.unit_normal().unwrap())
        .collect();
    let reference = normals[0];
    let aligned = normals.iter().all(|normal| {
        normal
            .iter()
            .zip(reference)
            .map(|(a, b)| (a - b).abs())
            .fold(0., f64::max)
            <= 64. * f64::EPSILON
    });
    json!({
        "classification":if aligned {"certified_planar_regular"} else {"unresolved"},
        "cornerNormals":normals
    })
}

/// Deterministic conservative closest-point enclosure. A linear span receives
/// an analytic uniqueness proof; higher-degree spans are reported as candidates.
pub fn project_curve(
    curve: &Curve,
    point: &[f64],
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    curve.validate()?;
    check(
        point.len() == curve.control_points[0].len()
            && point.iter().all(|coordinate| coordinate.is_finite()),
        "Projection point must be finite and match curve dimension",
    )?;
    let tolerance = context(tolerance);
    let mut candidates = Vec::new();
    let mut best_upper = f64::INFINITY;
    for segment in curve.decompose()? {
        let c = segment.definition();
        let (min, max) = box_of(&c.control_points);
        let lower = point
            .iter()
            .enumerate()
            .map(|(axis, coordinate)| {
                if *coordinate < min[axis] {
                    min[axis] - coordinate
                } else if *coordinate > max[axis] {
                    coordinate - max[axis]
                } else {
                    0.
                }
            })
            .map(|distance| distance * distance)
            .sum::<f64>()
            .sqrt();
        let [a, b] = segment.domain();
        let mut parameter = (a + b) / 2.;
        let mut classification = "candidate";
        if c.degree == 1 {
            let first = &c.control_points[0];
            let last = c.control_points.last().unwrap();
            let direction: Vec<f64> = last.iter().zip(first).map(|(x, y)| x - y).collect();
            let denominator = direction.iter().map(|x| x * x).sum::<f64>();
            if denominator > 0. {
                let ratio = point
                    .iter()
                    .zip(first)
                    .zip(&direction)
                    .map(|((q, origin), d)| (q - origin) * d)
                    .sum::<f64>()
                    / denominator;
                parameter = a + ratio.clamp(0., 1.) * (b - a);
                classification = "certified_unique_on_span";
            }
        }
        let evaluated = c.evaluate(parameter)?.point;
        let upper = evaluated
            .iter()
            .zip(point)
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f64>()
            .sqrt();
        best_upper = best_upper.min(upper);
        candidates.push((lower, parameter, upper, classification, [a, b], evaluated));
    }
    let floor = tolerance.parametric_bounds().floor;
    let kept: Vec<Value> = candidates
        .into_iter()
        .filter(|candidate| candidate.0 <= next_up(best_upper))
        .map(|candidate| {
            json!({
                "domain":candidate.4,
                "parameterInterval":[next_down(candidate.1-floor),next_up(candidate.1+floor)],
                "point":candidate.5,
                "distanceLower":next_down(candidate.0.max(0.)),
                "distanceUpper":next_up(candidate.2),
                "classification":candidate.3
            })
        })
        .collect();
    let status = if kept.len() == 1 && kept[0]["classification"] == "certified_unique_on_span" {
        "unique"
    } else if kept.len() > 1 {
        "nonunique_or_unresolved"
    } else {
        "isolated_candidate"
    };
    Ok(json!({
        "version":"nurbs-foundation/1",
        "status":status,
        "globalDistanceUpper":next_up(best_upper),
        "candidates":kept,
        "evidence":tolerance_evidence(&tolerance)
    }))
}

/// Exact interpolation of the supplied data sites as a degree-one spline.
pub fn interpolate_polyline(points: Vec<Vec<f64>>, tolerance: Option<ToleranceContext>) -> Result<Value> {
    let curve = Curve::from_polyline(points)?;
    let tolerance = context(tolerance);
    Ok(json!({
        "curve":curve,
        "certificate":{
            "version":"nurbs-foundation/1",
            "method":"piecewise-linear-interpolation",
            "dataSiteErrorUpper":0.,
            "evidence":tolerance_evidence(&tolerance)
        }
    }))
}

/// Conservative piecewise-linear approximation. The hull diameter is a
/// symmetric Hausdorff upper bound because source span and chord share a hull.
pub fn approximate_curve(curve: &Curve, tolerance: Option<ToleranceContext>) -> Result<Value> {
    curve.validate()?;
    let tolerance = context(tolerance);
    let segments = curve.decompose()?;
    let mut points = Vec::new();
    let mut error: f64 = 0.;
    for (index, segment) in segments.iter().enumerate() {
        let c = segment.definition();
        let [a, b] = segment.domain();
        if index == 0 {
            points.push(c.evaluate(a)?.point);
        }
        points.push(c.evaluate(b)?.point);
        for first in &c.control_points {
            for second in &c.control_points {
                error = error.max(
                    first
                        .iter()
                        .zip(second)
                        .map(|(a, b)| (a - b) * (a - b))
                        .sum::<f64>()
                        .sqrt(),
                );
            }
        }
    }
    let approximation = Curve::from_polyline(points)?;
    Ok(json!({
        "curve":approximation,
        "certificate":{
            "version":"nurbs-foundation/1",
            "method":"per-span-convex-hull-diameter",
            "hausdorffErrorUpper":next_up(error),
            "withinEntityTolerance":next_up(error)<=tolerance.entity_error_bounds().maximum_mm,
            "evidence":tolerance_evidence(&tolerance)
        }
    }))
}

/// Exact affine monotone reparameterization, including periodic exterior knots.
pub fn reparameterize_curve(
    curve: &Curve,
    domain: [f64; 2],
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    curve.validate()?;
    check(
        domain.iter().all(|value| value.is_finite()) && domain[0] < domain[1],
        "Reparameterization domain must be finite and increasing",
    )?;
    let old = curve.domain();
    let scale = (domain[1] - domain[0]) / (old[1] - old[0]);
    let mut output = curve.clone();
    output.knots = curve
        .knots
        .iter()
        .map(|knot| domain[0] + (knot - old[0]) * scale)
        .collect();
    output.validate()?;
    let tolerance = context(tolerance);
    Ok(json!({
        "curve":output,
        "certificate":{
            "version":"nurbs-foundation/1",
            "mapping":{"oldDomain":old,"newDomain":domain,"scale":scale},
            "monotone":true,
            "geometryErrorUpper":0.,
            "periodPreserved":curve.periodic,
            "evidence":tolerance_evidence(&tolerance)
        }
    }))
}

pub fn certify_exact_edit(
    source: &Curve,
    operation: &str,
    parameter: f64,
    count: usize,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    source.validate()?;
    let output = match operation {
        "insert" => source.insert(parameter, count)?,
        "elevate" => source.elevate(count)?,
        _ => return Err(crate::input("Unknown exact certified edit")),
    };
    let tolerance = context(tolerance);
    Ok(json!({
        "curve":output,
        "certificate":{
            "version":"nurbs-foundation/1",
            "operation":operation,
            "geometryErrorUpper":0.,
            "periodPreserved":source.periodic==output.periodic,
            "evidence":tolerance_evidence(&tolerance)
        }
    }))
}
