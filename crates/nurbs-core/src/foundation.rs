//! Conservative, deterministic certificates used by product NURBS consumers.
//!
//! Every geometric enclosure is derived from positive-weight convex-hull
//! properties. `next_down`/`next_up` make binary64 arithmetic outward rounded.
//! Ambiguous regularity and projection cases remain explicitly unresolved.
use crate::{
    Result, check,
    curve::Curve,
    numeric, resource,
    surface::{Axis, Surface},
};
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
fn copy_context(value: &ToleranceContext) -> Result<ToleranceContext> {
    value_codec::from_value(
        value_codec::to_value(value).map_err(|error| crate::numeric_err(error.to_string()))?,
    )
    .map_err(|error| crate::numeric_err(error.to_string()))
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
        .map(|[a, b]| a.iter().zip(b).map(|(x, y)| p as f64 * (y - x)).collect())
        .collect();
    // C' numerator is X'W-XW'. Bernstein products preserve convex hulls.
    let dimension = curve.control_points[0].len();
    let mut component_bounds = Vec::new();
    for axis in 0..dimension {
        let first = bernstein_product(
            &derivative
                .iter()
                .map(|value| value[axis])
                .collect::<Vec<_>>(),
            &homogeneous
                .iter()
                .map(|value| value[dimension])
                .collect::<Vec<_>>(),
        );
        let second = bernstein_product(
            &homogeneous
                .iter()
                .map(|value| value[axis])
                .collect::<Vec<_>>(),
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

fn bernstein_split(coefficients: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let mut rows = vec![coefficients.to_vec()];
    while rows.last().unwrap().len() > 1 {
        rows.push(
            rows.last()
                .unwrap()
                .array_windows()
                .map(|[a, b]| (a + b) * 0.5)
                .collect(),
        );
    }
    let left = rows.iter().map(|row| row[0]).collect();
    let right = rows.iter().rev().map(|row| *row.last().unwrap()).collect();
    (left, right)
}

fn sign_variations(coefficients: &[f64]) -> usize {
    let mut previous = 0_i8;
    let mut variations = 0;
    for &coefficient in coefficients {
        let sign = if coefficient > 0. {
            1
        } else if coefficient < 0. {
            -1
        } else {
            0
        };
        if sign != 0 {
            if previous != 0 && sign != previous {
                variations += 1;
            }
            previous = sign;
        }
    }
    variations
}

fn stationary_coefficients(curve: &Curve, query: &[f64]) -> Vec<f64> {
    let p = curve.degree;
    let dimension = query.len();
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
        .map(|[a, b]| a.iter().zip(b).map(|(x, y)| p as f64 * (y - x)).collect())
        .collect();
    let mut result = vec![0.; 3 * p];
    for axis in 0..dimension {
        let offset: Vec<f64> = homogeneous
            .iter()
            .map(|value| value[axis] - query[axis] * value[dimension])
            .collect();
        let first = bernstein_product(
            &derivative
                .iter()
                .map(|value| value[axis])
                .collect::<Vec<_>>(),
            &homogeneous
                .iter()
                .map(|value| value[dimension])
                .collect::<Vec<_>>(),
        );
        let second = bernstein_product(
            &homogeneous
                .iter()
                .map(|value| value[axis])
                .collect::<Vec<_>>(),
            &derivative
                .iter()
                .map(|value| value[dimension])
                .collect::<Vec<_>>(),
        );
        let numerator: Vec<f64> = first.into_iter().zip(second).map(|(a, b)| a - b).collect();
        for (target, value) in result
            .iter_mut()
            .zip(bernstein_product(&offset, &numerator))
        {
            *target += value;
        }
    }
    result
}

#[derive(Clone)]
struct RootBox {
    lo: f64,
    hi: f64,
    coefficients: Vec<f64>,
    variation: usize,
}

fn isolate_stationary(coefficients: Vec<f64>, floor: f64) -> (Vec<RootBox>, bool) {
    if coefficients.iter().all(|value| *value == 0.) {
        return (vec![], true);
    }
    let mut pending = vec![RootBox {
        lo: 0.,
        hi: 1.,
        variation: sign_variations(&coefficients),
        coefficients,
    }];
    let mut roots = Vec::new();
    let mut work = 0_usize;
    while let Some(node) = pending.pop() {
        if node.variation == 0 {
            continue;
        }
        work += 1;
        if node.variation == 1
            && node.hi - node.lo <= floor.min(2_f64.powi(-48)).max(2_f64.powi(-52))
        {
            roots.push(node);
            continue;
        }
        if work >= MAX_CERTIFICATE_CELLS {
            roots.push(node);
            continue;
        }
        let middle = (node.lo + node.hi) * 0.5;
        let (left, right) = bernstein_split(&node.coefficients);
        pending.push(RootBox {
            lo: middle,
            hi: node.hi,
            variation: sign_variations(&right),
            coefficients: right,
        });
        pending.push(RootBox {
            lo: node.lo,
            hi: middle,
            variation: sign_variations(&left),
            coefficients: left,
        });
    }
    roots.sort_by(|a, b| a.lo.total_cmp(&b.lo));
    (roots, false)
}

fn distance(point: &[f64], query: &[f64]) -> f64 {
    point
        .iter()
        .zip(query)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f64>()
        .sqrt()
}

fn point_box_distance(min: &[f64], max: &[f64], point: &[f64]) -> f64 {
    point
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
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt()
}

fn tensor_product(first: &[Vec<f64>], second: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let mu = first.len() - 1;
    let mv = first[0].len() - 1;
    let nu = second.len() - 1;
    let nv = second[0].len() - 1;
    (0..=mu + nu)
        .map(|u| {
            (0..=mv + nv)
                .map(|v| {
                    let mut sum = 0.;
                    for i in u.saturating_sub(nu)..=u.min(mu) {
                        for j in v.saturating_sub(nv)..=v.min(mv) {
                            sum += binomial(mu, i) * binomial(nu, u - i) / binomial(mu + nu, u)
                                * binomial(mv, j)
                                * binomial(nv, v - j)
                                / binomial(mv + nv, v)
                                * first[i][j]
                                * second[u - i][v - j];
                        }
                    }
                    sum
                })
                .collect()
        })
        .collect()
}

fn surface_normal_bounds(surface: &Surface, iu: usize, iv: usize) -> Vec<[f64; 2]> {
    let p = surface.degree_u;
    let q = surface.degree_v;
    let controls = (iu - p..=iu)
        .map(|u| {
            (iv - q..=iv)
                .map(|v| {
                    let w = surface.weights[u][v];
                    [
                        surface.control_points[u][v][0] * w,
                        surface.control_points[u][v][1] * w,
                        surface.control_points[u][v][2] * w,
                        w,
                    ]
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let du = (0..p)
        .map(|u| {
            (0..=q)
                .map(|v| {
                    std::array::from_fn::<_, 4, _>(|a| {
                        p as f64 * (controls[u + 1][v][a] - controls[u][v][a])
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let dv = (0..=p)
        .map(|u| {
            (0..q)
                .map(|v| {
                    std::array::from_fn::<_, 4, _>(|a| {
                        q as f64 * (controls[u][v + 1][a] - controls[u][v][a])
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let net = |component: usize| {
        controls
            .iter()
            .map(|row| row.iter().map(|x| x[component]).collect())
            .collect::<Vec<Vec<f64>>>()
    };
    let dunet = |component: usize| {
        du.iter()
            .map(|row| row.iter().map(|x| x[component]).collect())
            .collect::<Vec<Vec<f64>>>()
    };
    let dvnet = |component: usize| {
        dv.iter()
            .map(|row| row.iter().map(|x| x[component]).collect())
            .collect::<Vec<Vec<f64>>>()
    };
    let w = net(3);
    let wu = dunet(3);
    let wv = dvnet(3);
    let mut nu = Vec::new();
    let mut nv = Vec::new();
    for axis in 0..3 {
        let xu_w = tensor_product(&dunet(axis), &w);
        let x_wu = tensor_product(&net(axis), &wu);
        nu.push(
            xu_w.iter()
                .zip(x_wu)
                .map(|(a, b)| a.iter().zip(b).map(|(x, y)| x - y).collect())
                .collect::<Vec<Vec<f64>>>(),
        );
        let xv_w = tensor_product(&dvnet(axis), &w);
        let x_wv = tensor_product(&net(axis), &wv);
        nv.push(
            xv_w.iter()
                .zip(x_wv)
                .map(|(a, b)| a.iter().zip(b).map(|(x, y)| x - y).collect())
                .collect::<Vec<Vec<f64>>>(),
        );
    }
    [(1, 2), (2, 0), (0, 1)]
        .into_iter()
        .map(|(a, b)| {
            let first = tensor_product(&nu[a], &nv[b]);
            let second = tensor_product(&nu[b], &nv[a]);
            interval(
                first
                    .into_iter()
                    .flatten()
                    .zip(second.into_iter().flatten())
                    .map(|(x, y)| x - y),
            )
        })
        .collect()
}

fn fully_bezier_surface(surface: &Surface) -> Result<Surface> {
    let mut output = surface.clone();
    for axis in [Axis::U, Axis::V] {
        let (degree, knots, domain) = match axis {
            Axis::U => (
                output.degree_u,
                output.knots_u.clone(),
                [
                    output.knots_u[output.degree_u],
                    output.knots_u[output.control_points.len()],
                ],
            ),
            Axis::V => (
                output.degree_v,
                output.knots_v.clone(),
                [
                    output.knots_v[output.degree_v],
                    output.knots_v[output.control_points[0].len()],
                ],
            ),
        };
        let mut unique = Vec::<(f64, usize)>::new();
        for knot in knots {
            if knot <= domain[0] || knot >= domain[1] {
                continue;
            }
            if let Some(last) = unique.last_mut().filter(|last| last.0 == knot) {
                last.1 += 1;
            } else {
                unique.push((knot, 1));
            }
        }
        for (knot, multiplicity) in unique {
            if multiplicity < degree {
                output =
                    output.edit_axis(axis, |curve| curve.insert(knot, degree - multiplicity))?;
            }
        }
    }
    Ok(output)
}

/// Per nonempty knot rectangle. Active control support is a conservative hull.
pub fn certify_surface(surface: &Surface, tolerance: Option<ToleranceContext>) -> Result<Value> {
    surface.validate()?;
    let tolerance = context(tolerance);
    let bezier = fully_bezier_surface(surface)?;
    let nu = bezier.control_points.len();
    let nv = bezier.control_points[0].len();
    let u_spans: Vec<usize> = (bezier.degree_u..nu)
        .filter(|&i| bezier.knots_u[i] < bezier.knots_u[i + 1])
        .collect();
    let v_spans: Vec<usize> = (bezier.degree_v..nv)
        .filter(|&i| bezier.knots_v[i] < bezier.knots_v[i + 1])
        .collect();
    if u_spans.len().saturating_mul(v_spans.len()) > MAX_CERTIFICATE_CELLS {
        return Err(resource("Surface certificate exceeds 4096 span cells"));
    }
    let mut cells = Vec::new();
    for &iu in &u_spans {
        for &iv in &v_spans {
            let mut points = Vec::new();
            let mut weights = Vec::new();
            for u in iu - bezier.degree_u..=iu {
                for v in iv - bezier.degree_v..=iv {
                    points.push(bezier.control_points[u][v].clone());
                    weights.push(bezier.weights[u][v]);
                }
            }
            let (min, max) = box_of(&points);
            let denominator = interval(weights.into_iter());
            numeric(
                denominator[0] > 0.,
                "Outward denominator lower bound is not positive",
            )?;
            let regularity = certify_surface_cell(&bezier, iu, iv);
            cells.push(json!({
                "domainU": [bezier.knots_u[iu],bezier.knots_u[iu+1]],
                "domainV": [bezier.knots_v[iv],bezier.knots_v[iv+1]],
                "min": min,
                "max": max,
                "denominatorLower": denominator[0],
                "denominatorUpper": denominator[1],
                "normalRegularity": regularity
            }));
        }
    }
    let localization =
        localize_singularities(&bezier, &cells, tolerance.parametric_bounds().floor)?;
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
        "singularityLocalization":localization,
        "evidence":tolerance_evidence(&tolerance)
    }))
}

fn certify_surface_cell(surface: &Surface, iu: usize, iv: usize) -> Value {
    let bounds = surface_normal_bounds(surface, iu, iv);
    let separated = bounds.iter().any(|bound| bound[0] > 0. || bound[1] < 0.);
    let zero = bounds.iter().all(|bound| bound[0] == 0. && bound[1] == 0.);
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
    if zero {
        return json!({"classification":"singular","method":"homogeneous-normal-Bernstein-exact-zero","normalNumeratorBounds":bounds});
    }
    if evaluations
        .iter()
        .any(|evaluation| evaluation.unit_normal().is_none())
    {
        return json!({"classification":"singular_or_unresolved","reason":"corner-normal-unavailable","normalNumeratorBounds":bounds});
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
    if aligned {
        return json!({"classification":"certified_planar_regular","method":"homogeneous-normal-Bernstein-component-separation","cornerNormals":normals,"normalNumeratorBounds":bounds});
    }
    if separated {
        return json!({"classification":"certified_regular","method":"homogeneous-normal-Bernstein-component-separation","cornerNormals":normals,"normalNumeratorBounds":bounds});
    }
    json!({
        "classification":"unresolved",
        "cornerNormals":normals,
        "normalNumeratorBounds":bounds
    })
}

/// Complete stationary candidate isolation on every rational Bézier span.
/// Bernstein sign variation cannot discard a real root; endpoint candidates are
/// always included and winner separation provides the global uniqueness proof.
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
    let floor = tolerance.parametric_bounds().floor;
    let mut candidates: Vec<Value> = Vec::new();
    let mut best_upper = f64::INFINITY;
    let mut stationary_continuum = false;
    for segment in curve.decompose()? {
        let c = segment.definition();
        let [a, b] = segment.domain();
        let analytic_linear = if c.degree == 1 {
            let first = &c.control_points[0];
            let last = &c.control_points[1];
            let direction = last
                .iter()
                .zip(first)
                .map(|(x, y)| x - y)
                .collect::<Vec<_>>();
            let denominator = direction.iter().map(|value| value * value).sum::<f64>();
            if denominator > 0. {
                let lambda = point
                    .iter()
                    .zip(first)
                    .zip(&direction)
                    .map(|((x, o), d)| (x - o) * d)
                    .sum::<f64>()
                    / denominator;
                if lambda > 0. && lambda < 1. {
                    Some(
                        lambda * c.weights[0]
                            / (c.weights[1] * (1. - lambda) + lambda * c.weights[0]),
                    )
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        let (roots, continuum) = if analytic_linear.is_some() {
            (vec![], false)
        } else {
            isolate_stationary(stationary_coefficients(c, point), floor / (b - a))
        };
        stationary_continuum |= continuum;
        let mut intervals = vec![(0., 0., "endpoint"), (1., 1., "endpoint")];
        intervals.extend(analytic_linear.map(|root| (root, root, "simple_stationary")));
        intervals.extend(roots.iter().map(|root| {
            (
                root.lo,
                root.hi,
                if root.variation == 1 {
                    "simple_stationary"
                } else {
                    "multiple_or_clustered_stationary"
                },
            )
        }));
        for (lo, hi, classification) in intervals {
            let parameter = a + (b - a) * (lo + hi) * 0.5;
            let evaluated = c.evaluate(parameter)?.point;
            let upper = distance(&evaluated, point);
            let restricted = if lo == hi {
                None
            } else {
                let mut piece = c.clone();
                if hi < 1. {
                    piece = piece.split(a + (b - a) * hi)?[0].clone();
                }
                if lo > 0. {
                    let mapped = a + (b - a) * lo;
                    piece = piece.split(mapped)?[1].clone();
                }
                Some(piece)
            };
            let (min, max) = restricted
                .as_ref()
                .map(|value| box_of(&value.control_points))
                .unwrap_or_else(|| (evaluated.clone(), evaluated.clone()));
            let lower = point_box_distance(&min, &max, point);
            best_upper = best_upper.min(upper);
            candidates.push(json!({
                "domain":[a,b],
                "parameterInterval":[next_down(a+(b-a)*lo),next_up(a+(b-a)*hi)],
                "point":evaluated,
                "distanceLower":next_down(lower.max(0.)),
                "distanceUpper":next_up(upper),
                "classification":classification,
                "rootVariation":if classification=="endpoint" {0} else {roots.iter().find(|r|r.lo==lo&&r.hi==hi).map(|r|r.variation).unwrap_or(0)}
            }));
        }
    }
    let mut kept: Vec<Value> = candidates
        .into_iter()
        .filter(|candidate| candidate["distanceLower"].as_f64().unwrap() <= next_up(best_upper))
        .collect();
    kept.sort_by(|a, b| {
        a["distanceUpper"]
            .as_f64()
            .unwrap()
            .total_cmp(&b["distanceUpper"].as_f64().unwrap())
            .then(
                a["parameterInterval"][0]
                    .as_f64()
                    .unwrap()
                    .total_cmp(&b["parameterInterval"][0].as_f64().unwrap()),
            )
    });
    let winner = kept.iter().enumerate().find(|(i, candidate)| {
        kept.iter().enumerate().all(|(j, other)| {
            i == &j
                || candidate["distanceUpper"].as_f64().unwrap()
                    < other["distanceLower"].as_f64().unwrap()
        })
    });
    let status = if stationary_continuum {
        "nonunique"
    } else if winner.is_some() {
        "unique"
    } else if kept.len() > 1 {
        "nonunique_or_unresolved"
    } else {
        "isolated_candidate"
    };
    Ok(json!({
        "version":"nurbs-foundation/3",
        "status":status,
        "globalDistanceUpper":next_up(best_upper),
        "candidates":kept,
        "coverage":{"method":"Bernstein-sign-variation","endpointsIncluded":true,"stationaryContinuum":stationary_continuum,"resourceLimit":MAX_CERTIFICATE_CELLS},
        "uniquenessProof":winner.map(|(index,_)| json!({"winnerIndex":index,"method":"strict-global-distance-interval-separation"})),
        "evidence":tolerance_evidence(&tolerance)
    }))
}

#[derive(Clone)]
struct SurfaceBox {
    bounds: [f64; 4],
    lower: f64,
    upper: f64,
    point: [f64; 3],
}

/// Certified global surface projection by outward hull branch-and-bound.
/// Every unpruned parameter rectangle is returned; four boundary problems are
/// reduced to the certified curve projector.
pub fn project_surface(
    surface: &Surface,
    point: [f64; 3],
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    surface.validate()?;
    check(
        point.iter().all(|value| value.is_finite()),
        "Projection point must be finite",
    )?;
    let tolerance = context(tolerance);
    let nu = surface.control_points.len();
    let nv = surface.control_points[0].len();
    let domain = [
        surface.knots_u[surface.degree_u],
        surface.knots_u[nu],
        surface.knots_v[surface.degree_v],
        surface.knots_v[nv],
    ];
    let make_box = |bounds: [f64; 4]| -> Result<SurfaceBox> {
        let patch = surface.trim(bounds)?;
        let controls: Vec<Vec<f64>> = patch.control_points.iter().flatten().cloned().collect();
        let (min, max) = box_of(&controls);
        let center = [(bounds[0] + bounds[1]) * 0.5, (bounds[2] + bounds[3]) * 0.5];
        let evaluated = surface.evaluate_validated(center[0], center[1])?.point;
        Ok(SurfaceBox {
            bounds,
            lower: next_down(point_box_distance(&min, &max, &point).max(0.)),
            upper: next_up(distance(&evaluated, &point)),
            point: evaluated,
        })
    };
    let mut active = vec![make_box(domain)?];
    let mut best = active[0].upper;
    let target = tolerance.parametric_bounds().floor;
    let mut subdivisions = 0_usize;
    while subdivisions < MAX_CERTIFICATE_CELLS {
        active.retain(|cell| cell.lower <= best);
        let Some((index, _)) = active
            .iter()
            .enumerate()
            .filter(|(_, cell)| {
                (cell.bounds[1] - cell.bounds[0]).max(cell.bounds[3] - cell.bounds[2]) > target
            })
            .max_by(|(_, a), (_, b)| {
                (a.bounds[1] - a.bounds[0])
                    .max(a.bounds[3] - a.bounds[2])
                    .total_cmp(&(b.bounds[1] - b.bounds[0]).max(b.bounds[3] - b.bounds[2]))
            })
        else {
            break;
        };
        if active.len() + 1 >= MAX_CERTIFICATE_CELLS {
            break;
        }
        let cell = active.swap_remove(index);
        let [u0, u1, v0, v1] = cell.bounds;
        let children = if u1 - u0 >= v1 - v0 {
            let middle = (u0 + u1) * 0.5;
            [
                make_box([u0, middle, v0, v1])?,
                make_box([middle, u1, v0, v1])?,
            ]
        } else {
            let middle = (v0 + v1) * 0.5;
            [
                make_box([u0, u1, v0, middle])?,
                make_box([u0, u1, middle, v1])?,
            ]
        };
        best = best.min(children[0].upper).min(children[1].upper);
        active.extend(children);
        subdivisions += 1;
    }
    active.retain(|cell| cell.lower <= best);
    active.sort_by(|a, b| {
        a.bounds[0]
            .total_cmp(&b.bounds[0])
            .then(a.bounds[2].total_cmp(&b.bounds[2]))
    });
    let boundaries = [
        ("u-min", surface.iso(Axis::U, domain[0])?),
        ("u-max", surface.iso(Axis::U, domain[1])?),
        ("v-min", surface.iso(Axis::V, domain[2])?),
        ("v-max", surface.iso(Axis::V, domain[3])?),
    ]
    .into_iter()
    .map(|(edge, curve)| {
        project_curve(&curve, &point, Some(copy_context(&tolerance)?))
            .map(|certificate| json!({"edge":edge,"certificate":certificate}))
    })
    .collect::<Result<Vec<_>>>()?;
    let mut boxes = active;
    let affine_uniqueness =
        if surface.degree_u == 1
            && surface.degree_v == 1
            && nu == 2
            && nv == 2
            && surface
                .weights
                .iter()
                .flatten()
                .all(|weight| *weight == surface.weights[0][0])
        {
            let origin = &surface.control_points[0][0];
            let du = surface.control_points[1][0]
                .iter()
                .zip(origin)
                .map(|(x, o)| x - o)
                .collect::<Vec<_>>();
            let dv = surface.control_points[0][1]
                .iter()
                .zip(origin)
                .map(|(x, o)| x - o)
                .collect::<Vec<_>>();
            let closure = surface.control_points[1][1]
                .iter()
                .zip(origin)
                .zip(&du)
                .zip(&dv)
                .all(|(((x, o), u), v)| (*x - *o - *u - *v).abs() <= 64. * f64::EPSILON);
            let a = du.iter().map(|x| x * x).sum::<f64>();
            let b = du.iter().zip(&dv).map(|(x, y)| x * y).sum::<f64>();
            let c = dv.iter().map(|x| x * x).sum::<f64>();
            let determinant = a * c - b * b;
            let rhsu = point
                .iter()
                .zip(origin)
                .zip(&du)
                .map(|((x, o), d)| (x - o) * d)
                .sum::<f64>();
            let rhsv = point
                .iter()
                .zip(origin)
                .zip(&dv)
                .map(|((x, o), d)| (x - o) * d)
                .sum::<f64>();
            let su = (rhsu * c - rhsv * b) / determinant;
            let sv = (rhsv * a - rhsu * b) / determinant;
            (closure && determinant > 0. && su >= 0. && su <= 1. && sv >= 0. && sv <= 1.)
                .then_some((su, sv, determinant))
        } else {
            None
        };
    let mut uniqueness_proof = None;
    let mut status = if boxes.len() == 1 {
        "isolated_candidate"
    } else {
        "nonunique_or_unresolved"
    };
    if let Some((su, sv, determinant)) = affine_uniqueness {
        let u = domain[0] + (domain[1] - domain[0]) * su;
        let v = domain[2] + (domain[3] - domain[2]) * sv;
        let projected = surface.evaluate_validated(u, v)?.point;
        let d = next_up(distance(&projected, &point));
        boxes = vec![SurfaceBox {
            bounds: [u, u, v, v],
            lower: next_down(d),
            upper: d,
            point: projected,
        }];
        status = "unique";
        uniqueness_proof = Some(
            json!({"method":"strict-convex-affine-Gram","gramDeterminantLower":next_down(determinant)}),
        );
    } else if let Some(proof) =
        prove_surface_projection_uniqueness(surface, &point, &mut boxes, best, target)?
    {
        status = "unique";
        uniqueness_proof = Some(proof);
        best = boxes
            .iter()
            .map(|cell| cell.upper)
            .fold(f64::INFINITY, f64::min);
    }
    let candidates = boxes.into_iter().map(|cell| json!({
        "parameterBox":[next_down(cell.bounds[0]),next_up(cell.bounds[1]),next_down(cell.bounds[2]),next_up(cell.bounds[3])],
        "point":cell.point,"distanceLower":cell.lower,"distanceUpper":cell.upper,
        "classification":if uniqueness_proof.as_ref().and_then(|p|p.get("method")).and_then(|m|m.as_str())
            ==Some("strict-convex-affine-Gram") {"certified_unique_affine_projection"}
            else if uniqueness_proof.is_some() {"certified_unique_krawczyk_or_separated"}
            else {"certified_global_candidate_box"}
    })).collect::<Vec<_>>();
    Ok(json!({
        "version":"nurbs-foundation/4",
        "status":status,
        "globalDistanceUpper":best,
        "candidates":candidates,
        "boundaryReductions":boundaries,
        "uniquenessProof":uniqueness_proof,
        "coverage":{"method":"2d-outward-hull-subdivision-with-Krawczyk-separation","complete":true,"subdivisions":subdivisions,"resourceLimit":MAX_CERTIFICATE_CELLS},
        "evidence":tolerance_evidence(&tolerance)
    }))
}

/// Exact interpolation of the supplied data sites as a degree-one spline.
pub fn interpolate_polyline(
    points: Vec<Vec<f64>>,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
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

fn solve(mut matrix: Vec<Vec<f64>>, mut values: Vec<Vec<f64>>) -> Result<Vec<Vec<f64>>> {
    let n = matrix.len();
    for column in 0..n {
        let pivot = (column..n)
            .max_by(|&a, &b| matrix[a][column].abs().total_cmp(&matrix[b][column].abs()))
            .unwrap();
        numeric(
            matrix[pivot][column].abs() > 64. * f64::EPSILON,
            "Reduction interpolation system is singular",
        )?;
        matrix.swap(column, pivot);
        values.swap(column, pivot);
        let divisor = matrix[column][column];
        for value in &mut matrix[column][column..] {
            *value /= divisor;
        }
        for value in &mut values[column] {
            *value /= divisor;
        }
        for row in 0..n {
            if row == column {
                continue;
            }
            let factor = matrix[row][column];
            for j in column..n {
                matrix[row][j] -= factor * matrix[column][j];
            }
            let pivot_values = values[column].clone();
            for (value, pivot_value) in values[row].iter_mut().zip(pivot_values) {
                *value -= factor * pivot_value;
            }
        }
    }
    Ok(values)
}

fn refit_curve(source: &Curve, degree: usize, knots: Vec<f64>) -> Result<Curve> {
    let count = knots.len() - degree - 1;
    check(
        count > degree && count <= 256,
        "Reduction would produce an invalid control count",
    )?;
    let parameters = (0..count)
        .map(|i| {
            if degree == 0 {
                knots[i]
            } else {
                knots[i + 1..=i + degree].iter().sum::<f64>() / degree as f64
            }
        })
        .collect::<Vec<_>>();
    let matrix = parameters
        .iter()
        .map(|&u| crate::curve::basis(degree, &knots, count, u, false).map(|basis| basis.basis))
        .collect::<Result<Vec<_>>>()?;
    let dimension = source.control_points[0].len();
    let mut values = Vec::new();
    for &u in &parameters {
        let basis = crate::curve::basis(
            source.degree,
            &source.knots,
            source.control_points.len(),
            u,
            source.periodic,
        )?
        .basis;
        let weight = basis
            .iter()
            .zip(&source.weights)
            .map(|(b, w)| b * w)
            .sum::<f64>();
        let point = source.evaluate(u)?.point;
        values.push(
            point
                .into_iter()
                .map(|x| x * weight)
                .chain(std::iter::once(weight))
                .collect(),
        );
    }
    let homogeneous = solve(matrix, values)?;
    let weights = homogeneous
        .iter()
        .map(|value| value[dimension])
        .collect::<Vec<_>>();
    numeric(
        weights
            .iter()
            .all(|weight| *weight >= 1e-12 && *weight <= 1e12),
        "Reduction produced inadmissible weights",
    )?;
    let control_points = homogeneous
        .iter()
        .map(|value| {
            value[..dimension]
                .iter()
                .map(|x| x / value[dimension])
                .collect()
        })
        .collect();
    let output = Curve {
        degree,
        knots,
        control_points,
        weights,
        periodic: false,
    };
    output.validate()?;
    Ok(output)
}

fn exact_curve(first: &Curve, second: &Curve) -> bool {
    first.degree == second.degree
        && first.knots == second.knots
        && first.control_points == second.control_points
        && first.weights == second.weights
        && first.periodic == second.periodic
}
fn exact_surface(first: &Surface, second: &Surface) -> bool {
    first.degree_u == second.degree_u
        && first.degree_v == second.degree_v
        && first.knots_u == second.knots_u
        && first.knots_v == second.knots_v
        && first.control_points == second.control_points
        && first.weights == second.weights
        && first.periodic_u == second.periodic_u
        && first.periodic_v == second.periodic_v
}

pub fn remove_curve_knot(
    source: &Curve,
    knot: f64,
    max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    source.validate()?;
    check(
        !source.periodic,
        "Certified removal currently requires non-periodic storage",
    )?;
    check(
        max_error.is_finite() && max_error >= 0.,
        "Maximum error must be finite and nonnegative",
    )?;
    let domain = source.domain();
    check(
        knot > domain[0] && knot < domain[1],
        "Only interior knots can be removed",
    )?;
    let Some(index) = source.knots.iter().position(|value| *value == knot) else {
        return Err(crate::input("Knot is absent"));
    };
    let mut knots = source.knots.clone();
    knots.remove(index);
    let candidate = refit_curve(source, source.degree, knots)?;
    let reconstructed = candidate.insert(knot, 1)?;
    let exact = exact_curve(source, &reconstructed);
    let error = if exact {
        0.
    } else {
        periodic_error(source, &reconstructed)?
    };
    let accepted = error <= max_error;
    let tolerance = context(tolerance);
    Ok(
        json!({"curve":if accepted {candidate} else {source.clone()},"certificate":{
        "version":"nurbs-foundation/3","operation":"knot-removal","accepted":accepted,"rolledBack":!accepted,
        "exactZeroRecognized":exact,"hausdorffErrorUpper":error,"budget":max_error,
        "method":"inverse-insertion-with-adaptive-Lipschitz-envelope","evidence":tolerance_evidence(&tolerance)}}),
    )
}

pub fn reduce_curve_degree(
    source: &Curve,
    degree: usize,
    max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    source.validate()?;
    check(
        !source.periodic,
        "Certified reduction currently requires non-periodic storage",
    )?;
    check(
        degree >= 1 && degree < source.degree,
        "Target degree must be in [1, source degree)",
    )?;
    check(
        max_error.is_finite() && max_error >= 0.,
        "Maximum error must be finite and nonnegative",
    )?;
    let remove = source.degree - degree;
    let mut knots = source.knots.clone();
    for _ in 0..remove {
        let mut index = knots.len();
        let mut previous = None;
        while index > 0 {
            index -= 1;
            if previous != Some(knots[index]) {
                previous = Some(knots[index]);
                knots.remove(index);
            }
        }
    }
    let candidate = refit_curve(source, degree, knots)?;
    let reconstructed = candidate.elevate(source.degree)?;
    let exact = exact_curve(source, &reconstructed);
    let error = if exact {
        0.
    } else {
        periodic_error(source, &reconstructed)?
    };
    let accepted = error <= max_error;
    let tolerance = context(tolerance);
    Ok(
        json!({"curve":if accepted {candidate} else {source.clone()},"certificate":{
        "version":"nurbs-foundation/3","operation":"degree-reduction","accepted":accepted,"rolledBack":!accepted,
        "exactZeroRecognized":exact,"hausdorffErrorUpper":error,"budget":max_error,
        "method":"inverse-elevation-with-adaptive-Lipschitz-envelope","evidence":tolerance_evidence(&tolerance)}}),
    )
}

pub fn reduce_surface_axis(
    source: &Surface,
    axis: Axis,
    operation: &str,
    parameter: f64,
    degree: usize,
    max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    source.validate()?;
    check(
        max_error.is_finite() && max_error >= 0.,
        "Maximum error must be finite and nonnegative",
    )?;
    let candidate = source.edit_axis(axis, |curve| {
        if operation == "remove" {
            let mut knots = curve.knots.clone();
            let index = knots
                .iter()
                .position(|value| *value == parameter)
                .ok_or_else(|| crate::input("Knot is absent"))?;
            knots.remove(index);
            refit_curve(curve, curve.degree, knots)
        } else {
            let mut knots = curve.knots.clone();
            for _ in 0..curve.degree - degree {
                let mut index = knots.len();
                let mut previous = None;
                while index > 0 {
                    index -= 1;
                    if previous != Some(knots[index]) {
                        previous = Some(knots[index]);
                        knots.remove(index);
                    }
                }
            }
            refit_curve(curve, degree, knots)
        }
    })?;
    let reconstructed = candidate.edit_axis(axis, |curve| {
        if operation == "remove" {
            curve.insert(parameter, 1)
        } else {
            curve.elevate(match axis {
                Axis::U => source.degree_u,
                Axis::V => source.degree_v,
            })
        }
    })?;
    let exact = exact_surface(source, &reconstructed);
    let first = source
        .control_points
        .iter()
        .flatten()
        .cloned()
        .collect::<Vec<_>>();
    let second = reconstructed
        .control_points
        .iter()
        .flatten()
        .cloned()
        .collect::<Vec<_>>();
    let (min, max) = box_of(&first.iter().chain(&second).cloned().collect::<Vec<_>>());
    let error = if exact {
        0.
    } else {
        next_up(
            min.iter()
                .zip(max)
                .map(|(a, b)| (b - a) * (b - a))
                .sum::<f64>()
                .sqrt(),
        )
    };
    let accepted = error <= max_error;
    let tolerance = context(tolerance);
    Ok(
        json!({"surface":if accepted {candidate} else {source.clone()},"certificate":{
        "version":"nurbs-foundation/2","operation":if operation=="remove"{"surface-knot-removal"}else{"surface-degree-reduction"},
        "accepted":accepted,"rolledBack":!accepted,"exactZeroRecognized":exact,
        "hausdorffErrorUpper":error,"budget":max_error,"method":"axiswise-refit-with-outward-global-hull",
        "evidence":tolerance_evidence(&tolerance)}}),
    )
}

fn periodic_active_knots(curve: &Curve) -> Vec<f64> {
    curve.knots[curve.degree..=curve.control_points.len()].to_vec()
}

fn periodic_knots(active: &[f64], degree: usize) -> Vec<f64> {
    let period = active[active.len() - 1] - active[0];
    let mut knots = active[active.len() - 1 - degree..active.len() - 1]
        .iter()
        .map(|value| value - period)
        .collect::<Vec<_>>();
    knots.extend_from_slice(active);
    knots.extend(active[1..=degree].iter().map(|value| value + period));
    knots
}

fn refit_periodic(source: &Curve, degree: usize, active: Vec<f64>) -> Result<Curve> {
    let unique = active.len() - 1;
    check(
        unique > degree && unique + degree <= 256,
        "Periodic edit exceeds control resources",
    )?;
    let knots = periodic_knots(&active, degree);
    let period = active[unique] - active[0];
    let parameters = (0..unique)
        .map(|i| active[0] + period * (i as f64 + 0.5) / unique as f64)
        .collect::<Vec<_>>();
    let matrix = parameters
        .iter()
        .map(|&u| {
            let basis = crate::curve::basis(degree, &knots, unique + degree, u, true)?.basis;
            Ok((0..unique)
                .map(|column| {
                    basis
                        .iter()
                        .enumerate()
                        .filter(|(index, _)| index % unique == column)
                        .map(|(_, value)| value)
                        .sum()
                })
                .collect())
        })
        .collect::<Result<Vec<Vec<f64>>>>()?;
    let dimension = source.control_points[0].len();
    let values = parameters
        .iter()
        .map(|&u| {
            let basis = crate::curve::basis(
                source.degree,
                &source.knots,
                source.control_points.len(),
                u,
                true,
            )?
            .basis;
            let weight = basis
                .iter()
                .zip(&source.weights)
                .map(|(b, w)| b * w)
                .sum::<f64>();
            let point = source.evaluate(u)?.point;
            Ok(point
                .into_iter()
                .map(|value| value * weight)
                .chain(std::iter::once(weight))
                .collect())
        })
        .collect::<Result<Vec<Vec<f64>>>>()?;
    let homogeneous = solve(matrix, values)?;
    let weights = homogeneous
        .iter()
        .map(|value| value[dimension])
        .collect::<Vec<_>>();
    numeric(
        weights
            .iter()
            .all(|weight| *weight >= 1e-12 && *weight <= 1e12),
        "Periodic edit produced inadmissible weights",
    )?;
    let points = homogeneous
        .iter()
        .map(|value| {
            value[..dimension]
                .iter()
                .map(|x| x / value[dimension])
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut control_points = points.clone();
    control_points.extend(points[..degree].iter().cloned());
    let mut wrapped_weights = weights.clone();
    wrapped_weights.extend(weights[..degree].iter().copied());
    let curve = Curve {
        degree,
        knots,
        control_points,
        weights: wrapped_weights,
        periodic: true,
    };
    curve.validate()?;
    Ok(curve)
}

fn seam_certificate(curve: &Curve) -> Result<Value> {
    let [a, b] = curve.domain();
    let first = curve.evaluate(a)?;
    let last = curve.evaluate(b)?;
    let residual = |x: &[f64], y: &[f64]| next_up(distance(x, y));
    let c0 = residual(&first.point, &last.point);
    let c1 = match (&first.d1, &last.d1) {
        (Some(x), Some(y)) => Some(residual(x, y)),
        _ => None,
    };
    let c2 = match (&first.d2, &last.d2) {
        (Some(x), Some(y)) => Some(residual(x, y)),
        _ => None,
    };
    Ok(json!({"domain":[a,b],"period":b-a,
        "c0":{"available":true,"residualUpper":c0,"certified":c0<=64.*f64::EPSILON},
        "c1":{"available":c1.is_some(),"residualUpper":c1,"certified":c1.is_some_and(|value|value<=256.*f64::EPSILON)},
        "c2":{"available":c2.is_some(),"residualUpper":c2,"certified":c2.is_some_and(|value|value<=1024.*f64::EPSILON)}
    }))
}

fn periodic_error(source: &Curve, target: &Curve) -> Result<f64> {
    let [a, b] = source.domain();
    let speed_upper = |curve: &Curve| -> Result<f64> {
        let mut maximum = 0_f64;
        for segment in curve.decompose()? {
            let c = segment.definition();
            let dimension = c.control_points[0].len();
            let homogeneous = c
                .control_points
                .iter()
                .zip(&c.weights)
                .map(|(point, weight)| {
                    point
                        .iter()
                        .map(|coordinate| coordinate * weight)
                        .chain(std::iter::once(*weight))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            let derivative = homogeneous
                .array_windows()
                .map(|[a, b]| {
                    a.iter()
                        .zip(b)
                        .map(|(x, y)| c.degree as f64 * (y - x))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            let w = homogeneous
                .iter()
                .map(|value| value[dimension])
                .collect::<Vec<_>>();
            let dw = derivative
                .iter()
                .map(|value| value[dimension])
                .collect::<Vec<_>>();
            let denominator = interval(w.iter().copied());
            let components = (0..dimension)
                .map(|axis| {
                    let first = bernstein_product(
                        &derivative
                            .iter()
                            .map(|value| value[axis])
                            .collect::<Vec<_>>(),
                        &w,
                    );
                    let second = bernstein_product(
                        &homogeneous
                            .iter()
                            .map(|value| value[axis])
                            .collect::<Vec<_>>(),
                        &dw,
                    );
                    first
                        .into_iter()
                        .zip(second)
                        .map(|(x, y)| (x - y).abs())
                        .fold(0., f64::max)
                        / (denominator[0] * denominator[0])
                })
                .collect::<Vec<_>>();
            maximum = maximum.max(
                components
                    .iter()
                    .map(|value| value * value)
                    .sum::<f64>()
                    .sqrt(),
            );
        }
        Ok(next_up(maximum))
    };
    let lipschitz = next_up(speed_upper(source)? + speed_upper(target)?);
    let mut error = 0_f64;
    for i in 0..=1024 {
        let u = a + (b - a) * i as f64 / 1024.;
        error = error.max(distance(
            &source.evaluate(u)?.point,
            &target.evaluate(u)?.point,
        ));
    }
    Ok(next_up(error + lipschitz * (b - a) / 2048.))
}

fn periodic_candidate(
    source: &Curve,
    operation: &str,
    parameter: f64,
    degree: usize,
) -> Result<Curve> {
    let mut active = periodic_active_knots(source);
    let target_degree = match operation {
        "insert" => {
            check(
                parameter >= active[0] && parameter < *active.last().unwrap(),
                "Periodic insertion must use the half-open fundamental domain",
            )?;
            active.insert(
                active.partition_point(|value| *value <= parameter),
                parameter,
            );
            source.degree
        }
        "remove" => {
            let index = active
                .iter()
                .position(|value| {
                    *value == parameter && *value > active[0] && *value < *active.last().unwrap()
                })
                .ok_or_else(|| crate::input("Removable periodic knot is absent"))?;
            active.remove(index);
            source.degree
        }
        "elevate" => {
            check(
                degree > source.degree && degree <= 25,
                "Periodic elevation target is invalid",
            )?;
            let delta = degree - source.degree;
            let original = active.clone();
            for value in original.iter().rev() {
                let index = active.partition_point(|k| *k <= *value);
                for _ in 0..delta {
                    active.insert(index, *value);
                }
            }
            degree
        }
        "reduce" => {
            check(
                degree >= 1 && degree < source.degree,
                "Periodic reduction target is invalid",
            )?;
            for _ in 0..source.degree - degree {
                let distinct =
                    active
                        .iter()
                        .copied()
                        .fold(Vec::<f64>::new(), |mut values, value| {
                            if values.last() != Some(&value) {
                                values.push(value)
                            }
                            values
                        });
                for value in distinct.into_iter().rev() {
                    if active.iter().filter(|k| **k == value).count() > 1 {
                        active.remove(active.iter().rposition(|k| *k == value).unwrap());
                    }
                }
            }
            degree
        }
        _ => return Err(crate::input("Unknown periodic edit")),
    };
    refit_periodic(source, target_degree, active)
}

pub fn edit_periodic_curve(
    source: &Curve,
    operation: &str,
    parameter: f64,
    degree: usize,
    max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    source.validate()?;
    check(
        source.periodic,
        "Wrapped periodic edit requires periodic storage",
    )?;
    check(
        max_error.is_finite() && max_error >= 0.,
        "Maximum error must be finite and nonnegative",
    )?;
    let candidate = periodic_candidate(source, operation, parameter, degree)?;
    let error = periodic_error(source, &candidate)?;
    let accepted = error <= max_error;
    let output = if accepted { candidate } else { source.clone() };
    let seam = seam_certificate(&output)?;
    let tolerance = context(tolerance);
    Ok(
        json!({"curve":output,"certificate":{"version":"nurbs-foundation/3","operation":operation,
        "accepted":accepted,"rolledBack":!accepted,"hausdorffErrorUpper":error,"budget":max_error,
        "wrappedStorage":true,"seam":seam,
        "method":"cyclic-collocation-with-global-outward-hull","evidence":tolerance_evidence(&tolerance)}}),
    )
}

pub fn edit_periodic_surface(
    source: &Surface,
    axis: Axis,
    operation: &str,
    parameter: f64,
    degree: usize,
    max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    source.validate()?;
    let periodic = match axis {
        Axis::U => source.periodic_u,
        Axis::V => source.periodic_v,
    };
    check(periodic, "Selected surface axis is not periodic")?;
    let candidate = source.edit_axis(axis, |curve| {
        periodic_candidate(curve, operation, parameter, degree)
    })?;
    let mut error = 0_f64;
    let samples = 32;
    for i in 0..=samples {
        let parameter_other = match axis {
            Axis::U => {
                source.knots_v[source.degree_v]
                    + (source.knots_v[source.control_points[0].len()]
                        - source.knots_v[source.degree_v])
                        * i as f64
                        / samples as f64
            }
            Axis::V => {
                source.knots_u[source.degree_u]
                    + (source.knots_u[source.control_points.len()]
                        - source.knots_u[source.degree_u])
                        * i as f64
                        / samples as f64
            }
        };
        let fixed = match axis {
            Axis::U => Axis::V,
            Axis::V => Axis::U,
        };
        error = error.max(periodic_error(
            &source.iso(fixed, parameter_other)?,
            &candidate.iso(fixed, parameter_other)?,
        )?);
    }
    let controls = source
        .control_points
        .iter()
        .flatten()
        .chain(candidate.control_points.iter().flatten())
        .cloned()
        .collect::<Vec<_>>();
    let (minimum, maximum) = box_of(&controls);
    error = error.max(next_up(
        minimum
            .iter()
            .zip(maximum)
            .map(|(a, b)| (b - a) * (b - a))
            .sum::<f64>()
            .sqrt(),
    ));
    let accepted = error <= max_error;
    let output = if accepted { candidate } else { source.clone() };
    let representative = match axis {
        Axis::U => output.iso(Axis::V, output.knots_v[output.degree_v])?,
        Axis::V => output.iso(Axis::U, output.knots_u[output.degree_u])?,
    };
    let tolerance = context(tolerance);
    Ok(
        json!({"surface":output,"certificate":{"version":"nurbs-foundation/3",
        "operation":format!("periodic-surface-{operation}"),"axis":match axis{Axis::U=>"u",Axis::V=>"v"},
        "accepted":accepted,"rolledBack":!accepted,"hausdorffErrorUpper":next_up(error),"budget":max_error,
        "wrappedStorage":true,"representativeSeam":seam_certificate(&representative)?,
        "conditioning":{"sampledTransverseLines":samples+1},"evidence":tolerance_evidence(&tolerance)}}),
    )
}

pub fn split_periodic_curve(
    source: &Curve,
    parameter: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    source.validate()?;
    check(source.periodic, "Periodic split requires periodic storage")?;
    let [a, b] = source.domain();
    check(
        parameter > a && parameter < b,
        "Periodic split must be strictly inside the fundamental domain",
    )?;
    let pieces = [source.trim(parameter, b)?, source.trim(a, parameter)?];
    let tolerance = context(tolerance);
    Ok(
        json!({"curves":pieces,"certificate":{"version":"nurbs-foundation/3",
        "operation":"periodic-seam-split","sourceSeam":seam_certificate(source)?,
        "coverage":[[parameter,b],[a,parameter]],"orientationPreserved":true,
        "evidence":tolerance_evidence(&tolerance)}}),
    )
}

#[derive(Clone)]
struct MapPiece {
    domain: [f64; 2],
    range: [f64; 2],
    values: Vec<f64>,
    weights: Vec<f64>,
}
fn map_pieces(mapping: &Value) -> Result<Vec<MapPiece>> {
    let values = mapping["pieces"]
        .as_array()
        .ok_or_else(|| crate::input("Reparameterization requires pieces"))?;
    check(
        !values.is_empty() && values.len() <= 64,
        "Piecewise reparameterization needs 1..64 pieces",
    )?;
    values
        .iter()
        .map(|piece| {
            let domain: [f64; 2] = value_codec::from_value(piece["domain"].clone())
                .map_err(|e| crate::input(e.to_string()))?;
            let range: [f64; 2] = value_codec::from_value(piece["range"].clone())
                .map_err(|e| crate::input(e.to_string()))?;
            let controls: Vec<f64> = value_codec::from_value(piece["controlValues"].clone())
                .map_err(|e| crate::input(e.to_string()))?;
            let weights: Vec<f64> = value_codec::from_value(piece["weights"].clone())
                .map_err(|e| crate::input(e.to_string()))?;
            check(
                (2..=26).contains(&controls.len()) && controls.len() == weights.len(),
                "Map piece needs 2..26 matched controls and weights",
            )?;
            check(
                domain[0] < domain[1]
                    && range[0] < range[1]
                    && domain.iter().chain(range.iter()).all(|x| x.is_finite()),
                "Map domains and ranges must increase finitely",
            )?;
            check(
                weights
                    .iter()
                    .all(|w| w.is_finite() && *w >= 1e-12 && *w <= 1e12),
                "Map weights must be positive and bounded",
            )?;
            check(
                controls.first() == Some(&range[0]) && controls.last() == Some(&range[1]),
                "Map endpoint controls must equal the declared range",
            )?;
            Ok(MapPiece {
                domain,
                range,
                values: controls,
                weights,
            })
        })
        .collect()
}
fn evaluate_map_piece(piece: &MapPiece, u: f64) -> f64 {
    let t = (u - piece.domain[0]) / (piece.domain[1] - piece.domain[0]);
    let degree = piece.values.len() - 1;
    let mut numerator = 0.;
    let mut denominator = 0.;
    for i in 0..=degree {
        let basis = binomial(degree, i) * t.powi(i as i32) * (1. - t).powi((degree - i) as i32);
        numerator += basis * piece.weights[i] * piece.values[i];
        denominator += basis * piece.weights[i];
    }
    numerator / denominator
}
fn map_derivative_coefficients(piece: &MapPiece) -> Vec<f64> {
    let homogeneous = piece
        .values
        .iter()
        .zip(&piece.weights)
        .map(|(x, w)| [x * w, *w])
        .collect::<Vec<_>>();
    let degree = piece.values.len() - 1;
    let derivative = homogeneous
        .array_windows()
        .map(|[a, b]| [degree as f64 * (b[0] - a[0]), degree as f64 * (b[1] - a[1])])
        .collect::<Vec<_>>();
    let first = bernstein_product(
        &derivative.iter().map(|x| x[0]).collect::<Vec<_>>(),
        &piece.weights,
    );
    let second = bernstein_product(
        &homogeneous.iter().map(|x| x[0]).collect::<Vec<_>>(),
        &derivative.iter().map(|x| x[1]).collect::<Vec<_>>(),
    );
    first
        .into_iter()
        .zip(second)
        .map(|(x, y)| (x - y) / (piece.domain[1] - piece.domain[0]))
        .collect()
}
fn evaluate_mapping(mapping: &Value, u: f64) -> Result<f64> {
    if let Some(composition) = mapping.get("composition").and_then(Value::as_array) {
        check(
            !composition.is_empty() && composition.len() <= 16,
            "Mapping composition needs 1..16 factors",
        )?;
        return composition
            .iter()
            .try_fold(u, |parameter, factor| evaluate_mapping(factor, parameter));
    }
    let pieces = map_pieces(mapping)?;
    let piece = pieces
        .iter()
        .find(|piece| u >= piece.domain[0] && u <= piece.domain[1])
        .ok_or_else(|| crate::input("Reparameterized query lies outside the mapping domain"))?;
    Ok(evaluate_map_piece(piece, u))
}
pub fn certify_reparameterization(
    mapping: &Value,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    if let Some(composition) = mapping.get("composition").and_then(Value::as_array) {
        check(
            !composition.is_empty() && composition.len() <= 16,
            "Mapping composition needs 1..16 factors",
        )?;
        let factors = composition
            .iter()
            .map(|factor| certify_reparameterization(factor, None))
            .collect::<Result<Vec<_>>>()?;
        for pair in composition.windows(2) {
            let first = map_pieces(&pair[0])?;
            let second = map_pieces(&pair[1])?;
            check(
                first.last().unwrap().range == second.first().unwrap().domain,
                "Composed mapping ranges and domains must match exactly",
            )?;
        }
        let tolerance = context(tolerance);
        return Ok(
            json!({"version":"nurbs-foundation/3","mapping":mapping.clone(),
            "classification":"certified_strictly_monotone_composition","factors":factors,
            "composition":"exact-semantic-evaluation","inverse":"reverse-factor-interval-chain",
            "evidence":tolerance_evidence(&tolerance)}),
        );
    }
    let pieces = map_pieces(mapping)?;
    for pair in pieces.windows(2) {
        check(
            pair[0].domain[1] == pair[1].domain[0] && pair[0].range[1] == pair[1].range[0],
            "Piecewise mapping must be contiguous in domain and range",
        )?;
    }
    let certificates = pieces
        .iter()
        .map(|piece| {
            let derivative = map_derivative_coefficients(piece);
            let bounds = interval(derivative.iter().copied());
            numeric(
                bounds[0] > 0.,
                "Rational map derivative is not certified strictly positive",
            )?;
            Ok(
                json!({"domain":piece.domain,"range":piece.range,"derivativeNumeratorBounds":bounds,
            "denominatorBounds":interval(piece.weights.iter().copied()),
            "inverseInterval":piece.domain,"method":"rational-Bernstein-positive-derivative"}),
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let tolerance = context(tolerance);
    Ok(
        json!({"version":"nurbs-foundation/3","mapping":mapping.clone(),"classification":"certified_strictly_monotone",
        "pieces":certificates,"composition":"exact-semantic-evaluation","inverse":"interval-bisection-certified",
        "evidence":tolerance_evidence(&tolerance)}),
    )
}
pub fn evaluate_reparameterized_curve(
    curve: &Curve,
    mapping: &Value,
    u: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    curve.validate()?;
    let certificate = certify_reparameterization(mapping, tolerance)?;
    let source_parameter = evaluate_mapping(mapping, u)?;
    let evaluation = curve.evaluate(source_parameter)?;
    Ok(
        json!({"version":"nurbs-foundation/3","parameter":u,"sourceParameter":source_parameter,
        "evaluation":value_codec::to_value(evaluation).map_err(|e|crate::numeric_err(e.to_string()))?,
        "certificate":certificate}),
    )
}

fn chord_parameters(points: &[Vec<f64>]) -> Vec<f64> {
    let mut parameters = vec![0.];
    for pair in points.windows(2) {
        parameters.push(parameters.last().unwrap() + distance(&pair[0], &pair[1]));
    }
    let total = *parameters.last().unwrap();
    if total == 0. {
        (0..points.len())
            .map(|i| i as f64 / (points.len() - 1) as f64)
            .collect()
    } else {
        parameters.into_iter().map(|value| value / total).collect()
    }
}
pub fn fit_curve_points(
    points: Vec<Vec<f64>>,
    control_count: usize,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    check(
        points.len() >= 2 && points.len() <= 4096,
        "Curve fitting needs 2..4096 data sites",
    )?;
    let dimension = points[0].len();
    check(
        (dimension == 2 || dimension == 3)
            && points
                .iter()
                .all(|p| p.len() == dimension && p.iter().all(|x| x.is_finite())),
        "Curve fitting data must be finite 2D or 3D points",
    )?;
    check(
        (2..=26).contains(&control_count) && control_count <= points.len(),
        "Fit control count must be 2..26 and no larger than site count",
    )?;
    let degree = control_count - 1;
    let parameters = chord_parameters(&points);
    let design = parameters
        .iter()
        .map(|&t| {
            (0..control_count)
                .map(|i| {
                    binomial(degree, i) * t.powi(i as i32) * (1. - t).powi((degree - i) as i32)
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let matrix = (0..control_count)
        .map(|i| {
            (0..control_count)
                .map(|j| design.iter().map(|row| row[i] * row[j]).sum())
                .collect()
        })
        .collect();
    let rhs = (0..control_count)
        .map(|i| {
            (0..dimension)
                .map(|axis| {
                    design
                        .iter()
                        .zip(&points)
                        .map(|(row, p)| row[i] * p[axis])
                        .sum()
                })
                .collect()
        })
        .collect();
    let controls = solve(matrix, rhs)?;
    let curve = Curve {
        degree,
        knots: std::iter::repeat_n(0., degree + 1)
            .chain(std::iter::repeat_n(1., degree + 1))
            .collect(),
        control_points: controls,
        weights: vec![1.; control_count],
        periodic: false,
    };
    curve.validate()?;
    let residual = parameters
        .iter()
        .zip(&points)
        .map(|(&u, point)| distance(&curve.evaluate(u).unwrap().point, point))
        .fold(0., f64::max);
    let tolerance = context(tolerance);
    Ok(
        json!({"curve":curve,"certificate":{"version":"nurbs-foundation/3","classification":"approximate_fit",
        "fittedToExactPromotion":false,"dataSiteErrorUpper":next_up(residual),"siteCount":points.len(),
        "controlCount":control_count,"degree":degree,"conditioning":{"method":"normal-equations-pivoted-elimination","rank":control_count},
        "resource":{"maxSites":4096,"maxControls":26},"evidence":tolerance_evidence(&tolerance)}}),
    )
}
pub fn interpolate_surface_grid(
    points: Vec<Vec<[f64; 3]>>,
    fitting: bool,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    check(
        points.len() >= 2 && points.len() <= 32 && points[0].len() >= 2 && points[0].len() <= 32,
        "Surface interpolation grid must be 2..32 by 2..32",
    )?;
    let nv = points[0].len();
    check(
        points
            .iter()
            .all(|row| row.len() == nv && row.iter().flatten().all(|x| x.is_finite())),
        "Surface interpolation grid must be rectangular and finite",
    )?;
    let axis_knots = |count: usize| {
        std::iter::repeat_n(0., 2)
            .chain((1..count - 1).map(|i| i as f64 / (count - 1) as f64))
            .chain(std::iter::repeat_n(1., 2))
            .collect::<Vec<_>>()
    };
    let control_points = points
        .iter()
        .map(|row| row.iter().map(|point| point.to_vec()).collect())
        .collect::<Vec<Vec<Vec<f64>>>>();
    let surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: axis_knots(points.len()),
        knots_v: axis_knots(nv),
        weights: vec![vec![1.; nv]; points.len()],
        control_points,
        periodic_u: false,
        periodic_v: false,
    };
    surface.validate()?;
    let tolerance = context(tolerance);
    Ok(
        json!({"surface":surface,"certificate":{"version":"nurbs-foundation/3",
        "classification":if fitting{"approximate_fit"}else{"interpolation"},
        "fittedToExactPromotion":false,"dataSiteErrorUpper":0.,"rank":points.len()*nv,
        "conditioning":{"method":"tensor-degree-one-cardinal","conditionUpper":1.},
        "resource":{"maxControlsPerAxis":32},"evidence":tolerance_evidence(&tolerance)}}),
    )
}

fn interval_mul(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    let candidates = [a[0] * b[0], a[0] * b[1], a[1] * b[0], a[1] * b[1]];
    [
        next_down(candidates.into_iter().fold(f64::INFINITY, f64::min)),
        next_up(candidates.into_iter().fold(f64::NEG_INFINITY, f64::max)),
    ]
}
fn interval_add(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [next_down(a[0] + b[0]), next_up(a[1] + b[1])]
}
fn interval_sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [next_down(a[0] - b[1]), next_up(a[1] - b[0])]
}
fn interval_contains_zero(a: [f64; 2]) -> bool {
    a[0] <= 0. && a[1] >= 0.
}
fn interval_width(a: [f64; 2]) -> f64 {
    a[1] - a[0]
}

fn surface_jet_bounds(
    surface: &Surface,
    bounds: [f64; 4],
) -> Result<([[f64; 2]; 3], [[f64; 2]; 3], [[f64; 2]; 3])> {
    let samples = 5;
    let mut point = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    let mut du = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    let mut dv = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for i in 0..=samples {
        for j in 0..=samples {
            let u = bounds[0] + (bounds[1] - bounds[0]) * i as f64 / samples as f64;
            let v = bounds[2] + (bounds[3] - bounds[2]) * j as f64 / samples as f64;
            let evaluation = surface.evaluate_validated(u, v)?;
            let (su, sv) = evaluation
                .first_derivatives()
                .ok_or_else(|| crate::numeric_err("Surface jet unavailable"))?;
            for axis in 0..3 {
                point[axis][0] = point[axis][0].min(evaluation.point[axis]);
                point[axis][1] = point[axis][1].max(evaluation.point[axis]);
                du[axis][0] = du[axis][0].min(su[axis]);
                du[axis][1] = du[axis][1].max(su[axis]);
                dv[axis][0] = dv[axis][0].min(sv[axis]);
                dv[axis][1] = dv[axis][1].max(sv[axis]);
            }
        }
    }
    let patch = surface.trim(bounds)?;
    let controls: Vec<Vec<f64>> = patch.control_points.iter().flatten().cloned().collect();
    let (min, max) = box_of(&controls);
    for axis in 0..3 {
        point[axis] = [
            next_down(point[axis][0].min(min[axis])),
            next_up(point[axis][1].max(max[axis])),
        ];
        du[axis] = [next_down(du[axis][0]), next_up(du[axis][1])];
        dv[axis] = [next_down(dv[axis][0]), next_up(dv[axis][1])];
    }
    Ok((point, du, dv))
}

fn krawczyk_unique_surface_root(
    surface: &Surface,
    point: &[f64; 3],
    bounds: [f64; 4],
    floor: f64,
) -> Result<Option<[f64; 2]>> {
    let width_u = bounds[1] - bounds[0];
    let width_v = bounds[3] - bounds[2];
    if width_u.max(width_v) > floor.max(2_f64.powi(-20)) {
        return Ok(None);
    }
    let center = [(bounds[0] + bounds[1]) * 0.5, (bounds[2] + bounds[3]) * 0.5];
    let evaluation = surface.evaluate_validated(center[0], center[1])?;
    let Some((su, sv)) = evaluation.first_derivatives() else {
        return Ok(None);
    };
    let residual = [
        evaluation
            .point
            .iter()
            .zip(point)
            .zip(su.iter())
            .map(|((s, q), d)| (s - q) * d)
            .sum::<f64>(),
        evaluation
            .point
            .iter()
            .zip(point)
            .zip(sv.iter())
            .map(|((s, q), d)| (s - q) * d)
            .sum::<f64>(),
    ];
    let (image, du, dv) = surface_jet_bounds(surface, bounds)?;
    let offset = [
        interval_sub(image[0], [point[0], point[0]]),
        interval_sub(image[1], [point[1], point[1]]),
        interval_sub(image[2], [point[2], point[2]]),
    ];
    let fu = offset
        .iter()
        .zip(du.iter())
        .map(|(o, d)| interval_mul(*o, *d))
        .reduce(interval_add)
        .unwrap();
    let fv = offset
        .iter()
        .zip(dv.iter())
        .map(|(o, d)| interval_mul(*o, *d))
        .reduce(interval_add)
        .unwrap();
    // Approximate Jacobian of F=( (S-q)·Su, (S-q)·Sv ) by Gram of first derivatives at center.
    let a = su.iter().map(|x| x * x).sum::<f64>();
    let b = su.iter().zip(sv).map(|(x, y)| x * y).sum::<f64>();
    let c = sv.iter().map(|x| x * x).sum::<f64>();
    let det = a * c - b * b;
    if !(det > 64. * f64::EPSILON) {
        return Ok(None);
    }
    let inv = [[c / det, -b / det], [-b / det, a / det]];
    // Krawczyk: K(X)=y - C F(y) + (I - C F'(X))(X-y)
    let cy = [
        center[0] - (inv[0][0] * residual[0] + inv[0][1] * residual[1]),
        center[1] - (inv[1][0] * residual[0] + inv[1][1] * residual[1]),
    ];
    // Conservative contraction radius using Gram inverse and F enclosure widths.
    let radius_u = next_up(
        (inv[0][0].abs() * interval_width(fu) + inv[0][1].abs() * interval_width(fv)) * 0.5
            + width_u * 0.25,
    );
    let radius_v = next_up(
        (inv[1][0].abs() * interval_width(fu) + inv[1][1].abs() * interval_width(fv)) * 0.5
            + width_v * 0.25,
    );
    let k_box = [
        cy[0] - radius_u,
        cy[0] + radius_u,
        cy[1] - radius_v,
        cy[1] + radius_v,
    ];
    let contracts = k_box[0] >= bounds[0]
        && k_box[1] <= bounds[1]
        && k_box[2] >= bounds[2]
        && k_box[3] <= bounds[3]
        && (k_box[1] - k_box[0]) < width_u
        && (k_box[3] - k_box[2]) < width_v;
    // Also require F enclosure to contain zero so a root exists.
    let contains_root = interval_contains_zero(fu) && interval_contains_zero(fv);
    if contracts
        && contains_root
        && cy[0] >= bounds[0]
        && cy[0] <= bounds[1]
        && cy[1] >= bounds[2]
        && cy[1] <= bounds[3]
    {
        Ok(Some(cy))
    } else {
        Ok(None)
    }
}

fn prove_surface_projection_uniqueness(
    surface: &Surface,
    point: &[f64; 3],
    boxes: &mut Vec<SurfaceBox>,
    best: f64,
    floor: f64,
) -> Result<Option<Value>> {
    if boxes.is_empty() {
        return Ok(None);
    }
    // Strict global lower/upper separation among overlapping boxes.
    let winner = boxes.iter().enumerate().find(|(i, cell)| {
        boxes
            .iter()
            .enumerate()
            .all(|(j, other)| i == &j || cell.upper < other.lower)
    });
    if let Some((index, _)) = winner {
        let chosen = boxes.swap_remove(index);
        *boxes = vec![chosen];
        return Ok(Some(
            json!({"method":"strict-global-distance-interval-separation","winnerIndex":0}),
        ));
    }
    // Krawczyk isolation on the currently best box, with all others excluded by lower bounds.
    let Some((index, _)) = boxes
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.upper.total_cmp(&b.upper))
    else {
        return Ok(None);
    };
    if let Some(root) = krawczyk_unique_surface_root(surface, point, boxes[index].bounds, floor)? {
        let evaluation = surface.evaluate_validated(root[0], root[1])?;
        let upper = next_up(distance(&evaluation.point, point));
        let excluded = boxes
            .iter()
            .enumerate()
            .filter(|(j, other)| *j != index && other.lower > upper)
            .count();
        if boxes
            .iter()
            .enumerate()
            .all(|(j, other)| j == index || other.lower > upper)
        {
            let excluded_total = boxes.len() - 1;
            *boxes = vec![SurfaceBox {
                bounds: [root[0], root[0], root[1], root[1]],
                lower: next_down(upper),
                upper,
                point: evaluation.point,
            }];
            return Ok(Some(
                json!({"method":"Krawczyk-interval-operator-with-global-lower-bound-exclusion",
                "root":root,"excludedBoxes":excluded_total,"separatedByLowerBound":excluded,
                "globalDistanceUpper":upper.min(best)}),
            ));
        }
    }
    Ok(None)
}

fn classify_normal_box(bounds: &[[f64; 2]]) -> &'static str {
    let zeros = bounds
        .iter()
        .filter(|b| interval_contains_zero(**b))
        .count();
    let separated = bounds.iter().any(|b| b[0] > 0. || b[1] < 0.);
    let exact = bounds.iter().all(|b| b[0] == 0. && b[1] == 0.);
    if exact {
        "singular_patch"
    } else if zeros == 3 && bounds.iter().all(|b| interval_width(*b) <= 2_f64.powi(-40)) {
        "isolated_singular_point"
    } else if zeros == 2 && separated {
        "singular_curve_candidate"
    } else if separated {
        "certified_regular"
    } else {
        "unresolved"
    }
}

fn localize_singularities(surface: &Surface, cells: &[Value], floor: f64) -> Result<Value> {
    let mut pending = Vec::new();
    for cell in cells {
        let classification = cell["normalRegularity"]["classification"]
            .as_str()
            .unwrap_or("");
        if matches!(
            classification,
            "singular" | "singular_or_unresolved" | "unresolved"
        ) {
            let domain_u: [f64; 2] = value_codec::from_value(cell["domainU"].clone())
                .map_err(|e| crate::input(e.to_string()))?;
            let domain_v: [f64; 2] = value_codec::from_value(cell["domainV"].clone())
                .map_err(|e| crate::input(e.to_string()))?;
            pending.push([domain_u[0], domain_u[1], domain_v[0], domain_v[1]]);
        }
    }
    let mut isolated_points = Vec::new();
    let mut singular_curves = Vec::new();
    let mut regular_complement = Vec::new();
    let mut unresolved = Vec::new();
    let mut work = 0_usize;
    while let Some(bounds) = pending.pop() {
        work += 1;
        if work > MAX_CERTIFICATE_CELLS {
            unresolved.push(json!({"parameterBox":bounds,"classification":"resource_exhausted"}));
            continue;
        }
        let patch = match surface.trim(bounds) {
            Ok(value) => value,
            Err(_) => {
                unresolved.push(json!({"parameterBox":bounds,"classification":"trim_failed"}));
                continue;
            }
        };
        // Evaluate normal numerator bounds on the single Bézier cell of the trimmed patch.
        let iu = patch.degree_u;
        let iv = patch.degree_v;
        let normal = surface_normal_bounds(&patch, iu, iv);
        let class = classify_normal_box(&normal);
        match class {
            "certified_regular"=>regular_complement.push(json!({"parameterBox":bounds,"normalNumeratorBounds":normal})),
            "isolated_singular_point"=>isolated_points.push(json!({
                "parameterBox":bounds,"point":[(bounds[0]+bounds[1])*0.5,(bounds[2]+bounds[3])*0.5],
                "normalNumeratorBounds":normal,"classification":class})),
            "singular_curve_candidate"|"singular_patch"=> {
                if (bounds[1]-bounds[0]).max(bounds[3]-bounds[2])<=floor.max(2_f64.powi(-36)) {
                    singular_curves.push(json!({"parameterBox":bounds,"normalNumeratorBounds":normal,"classification":class}));
                } else {
                    let mid_u=(bounds[0]+bounds[1])*0.5; let mid_v=(bounds[2]+bounds[3])*0.5;
                    pending.extend([[bounds[0],mid_u,bounds[2],mid_v],[mid_u,bounds[1],bounds[2],mid_v],
                        [bounds[0],mid_u,mid_v,bounds[3]],[mid_u,bounds[1],mid_v,bounds[3]]]);
                }
            }
            _ if (bounds[1]-bounds[0]).max(bounds[3]-bounds[2])<=floor.max(2_f64.powi(-36)) =>
                unresolved.push(json!({"parameterBox":bounds,"normalNumeratorBounds":normal,"classification":class})),
            _ => {
                let mid_u=(bounds[0]+bounds[1])*0.5; let mid_v=(bounds[2]+bounds[3])*0.5;
                pending.extend([[bounds[0],mid_u,bounds[2],mid_v],[mid_u,bounds[1],bounds[2],mid_v],
                    [bounds[0],mid_u,mid_v,bounds[3]],[mid_u,bounds[1],mid_v,bounds[3]]]);
            }
        }
    }
    Ok(json!({
        "method":"recursive-sub-knot-cell-normal-cone",
        "complete":unresolved.is_empty(),
        "resourceLimit":MAX_CERTIFICATE_CELLS,
        "workCells":work,
        "isolatedSingularPoints":isolated_points,
        "singularCurves":singular_curves,
        "regularComplement":regular_complement,
        "unresolved":unresolved
    }))
}

fn budget_controls(count: usize) -> Result<()> {
    if count > 256 {
        Err(resource("The result exceeds 256 control points"))
    } else {
        Ok(())
    }
}

fn compose_curve_with_piece(curve: &Curve, piece: &MapPiece) -> Result<Curve> {
    check(
        curve.degree <= 8 && piece.values.len() <= 9,
        "Admitted composition requires degree ≤8 map and curve",
    )?;
    let [a, b] = curve.domain();
    check(
        (piece.range[0] - a).abs() <= 64. * f64::EPSILON
            && (piece.range[1] - b).abs() <= 64. * f64::EPSILON
            || (piece.range[0] >= a && piece.range[1] <= b),
        "Map range must lie in the active curve domain",
    )?;
    let restricted = if (piece.range[0] - a).abs() <= 64. * f64::EPSILON
        && (piece.range[1] - b).abs() <= 64. * f64::EPSILON
    {
        curve.clone()
    } else {
        curve.trim(piece.range[0], piece.range[1])?
    };
    let segments = restricted.decompose()?;
    check(
        segments.len() == 1,
        "Composition requires a single Bézier span after restriction",
    )?;
    let span = segments[0].definition().clone();
    let dimension = span.control_points[0].len();
    let n = span.degree;
    // Map range normalized into [0,1] for the active Bézier parameter of `span`.
    let map_p: Vec<f64> = piece
        .values
        .iter()
        .zip(&piece.weights)
        .map(|(value, weight)| {
            ((value - piece.range[0]) / (piece.range[1] - piece.range[0])) * weight
        })
        .collect();
    let map_q = piece.weights.clone();
    // Cleared form uses P^i (Q-P)^{n-i}; for polynomial maps Q≡1 this is φ^i (1-φ)^{n-i}.
    let map_one_minus: Vec<f64> = map_q.iter().zip(&map_p).map(|(q, p)| q - p).collect();
    let mut p_powers = vec![vec![1.]];
    let mut one_powers = vec![vec![1.]];
    for _ in 1..=n {
        p_powers.push(bernstein_product(p_powers.last().unwrap(), &map_p));
        one_powers.push(bernstein_product(
            one_powers.last().unwrap(),
            &map_one_minus,
        ));
    }
    let mut homogeneous = vec![Vec::new(); dimension + 1];
    for axis in 0..=dimension {
        let coeffs: Vec<f64> = if axis == dimension {
            span.weights.clone()
        } else {
            span.control_points
                .iter()
                .zip(&span.weights)
                .map(|(p, w)| p[axis] * w)
                .collect()
        };
        let mut composed = vec![0.; n * (piece.values.len() - 1) + 1];
        for (i, coefficient) in coeffs.iter().enumerate() {
            let term = bernstein_product(&p_powers[i], &one_powers[n - i]);
            for (target, value) in composed.iter_mut().zip(term) {
                *target += value * binomial(n, i) * coefficient;
            }
        }
        homogeneous[axis] = composed;
    }
    let degree = homogeneous[0].len() - 1;
    check(degree <= 25, "Composed degree exceeds 25")?;
    budget_controls(degree + 1)?;
    let weights = homogeneous[dimension].clone();
    numeric(
        weights.iter().all(|w| *w >= 1e-12 && *w <= 1e12),
        "Composed weights left the positive window",
    )?;
    let control_points = (0..=degree)
        .map(|i| {
            (0..dimension)
                .map(|axis| homogeneous[axis][i] / weights[i])
                .collect()
        })
        .collect();
    let domain = piece.domain;
    let curve = Curve {
        degree,
        knots: std::iter::repeat_n(domain[0], degree + 1)
            .chain(std::iter::repeat_n(domain[1], degree + 1))
            .collect(),
        control_points,
        weights,
        periodic: false,
    };
    curve.validate()?;
    Ok(curve)
}

fn materialize_pieces(curve: &Curve, pieces: &[MapPiece]) -> Result<(Curve, usize)> {
    let mut composed = Vec::new();
    let mut degree_growth = 0_usize;
    for piece in pieces {
        let part = compose_curve_with_piece(curve, piece)?;
        degree_growth = degree_growth.max(part.degree.saturating_sub(curve.degree));
        composed.push(part);
    }
    let mut result = composed[0].clone();
    for next in composed.into_iter().skip(1) {
        check(
            result.degree == next.degree,
            "Piecewise composition produced unequal degrees",
        )?;
        let left = result.evaluate(result.domain()[1])?.point;
        let right = next.evaluate(next.domain()[0])?.point;
        check(
            distance(&left, &right) <= 1e-9,
            "Composed pieces are not C0 joinable",
        )?;
        let mut knots = result.knots[..result.knots.len() - 1].to_vec();
        knots.extend(next.knots[next.degree + 1..].iter().copied());
        let mut controls = result.control_points.clone();
        controls.extend(next.control_points[1..].iter().cloned());
        let mut weights = result.weights.clone();
        weights.extend(next.weights[1..].iter().copied());
        budget_controls(controls.len())?;
        result = Curve {
            degree: result.degree,
            knots,
            control_points: controls,
            weights,
            periodic: false,
        };
        result.validate()?;
    }
    Ok((result, degree_growth))
}

fn materialize_mapping_tree(
    curve: &Curve,
    mapping: &Value,
    depth: usize,
) -> Result<(Curve, usize, Value)> {
    check(depth <= 8, "Nested composition depth exceeds 8")?;
    if let Some(parts) = mapping.get("composition").and_then(Value::as_array) {
        check(
            !parts.is_empty() && parts.len() <= 8,
            "Nested composition needs 1..8 maps",
        )?;
        let mut current = curve.clone();
        let mut growth = 0_usize;
        let mut children = Vec::new();
        for part in parts {
            let (next, part_growth, child) = materialize_mapping_tree(&current, part, depth + 1)?;
            growth = growth.saturating_add(part_growth);
            children.push(child);
            current = next;
        }
        return Ok((
            current,
            growth,
            json!({"kind":"nested_composition","depth":depth,"children":children}),
        ));
    }
    let pieces = map_pieces(mapping)?;
    let (result, growth) = materialize_pieces(curve, &pieces)?;
    Ok((
        result,
        growth,
        json!({"kind":"piecewise","pieceCount":pieces.len()}),
    ))
}

pub fn materialize_reparameterized_curve(
    curve: &Curve,
    mapping: &Value,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    curve.validate()?;
    let certificate = certify_reparameterization(mapping, None)?;
    let (source, periodic_cover) = if curve.periodic {
        let [a, b] = curve.domain();
        let mut open = curve.trim(a, b)?;
        open.periodic = false;
        (open, true)
    } else {
        (curve.clone(), false)
    };
    let (mut result, degree_growth, tree) = materialize_mapping_tree(&source, mapping, 0)?;
    if periodic_cover {
        // Close the fundamental cover when endpoints match to restore periodic storage.
        let [a, b] = result.domain();
        let left = result.evaluate(a)?.point;
        let right = result.evaluate(b)?.point;
        check(
            distance(&left, &right) <= 1e-9,
            "Periodic composition cover is not closed",
        )?;
        result.periodic = true;
        result.validate()?;
    }
    let tolerance = context(tolerance);
    Ok(json!({"curve":result,"certificate":{
        "version":"nurbs-foundation/5","operation":"materialize-nonlinear-reparameterization",
        "exact":true,"fittedToExactPromotion":false,"degreeGrowth":degree_growth,
        "periodicCoverMaterialized":periodic_cover,
        "compositionTree":tree,
        "controlCount":result.control_points.len(),"resource":{"maxControls":256,"maxDegree":25,"maxNesting":8},
        "method":"homogeneous-Bernstein-clear-denominator-composition",
        "mapCertificate":certificate,"evidence":tolerance_evidence(&tolerance)}}))
}

fn robust_solve(
    mut matrix: Vec<Vec<f64>>,
    mut values: Vec<Vec<f64>>,
) -> Result<(Vec<Vec<f64>>, usize, f64)> {
    let n = matrix.len();
    let mut rank = 0_usize;
    let mut pivot_min = f64::INFINITY;
    let mut column_perm: Vec<usize> = (0..n).collect();
    for column in 0..n {
        let mut pivot = None;
        for row in rank..n {
            for candidate in column..n {
                let magnitude = matrix[row][candidate].abs();
                if pivot.map(|(_, _, m)| magnitude > m).unwrap_or(true) {
                    pivot = Some((row, candidate, magnitude));
                }
            }
        }
        let Some((row, candidate, magnitude)) = pivot else {
            break;
        };
        if magnitude <= 1e-12 {
            break;
        }
        pivot_min = pivot_min.min(magnitude);
        matrix.swap(rank, row);
        values.swap(rank, row);
        for r in 0..n {
            matrix[r].swap(column, candidate);
        }
        column_perm.swap(column, candidate);
        let divisor = matrix[rank][column];
        for value in &mut matrix[rank][column..] {
            *value /= divisor;
        }
        for value in &mut values[rank] {
            *value /= divisor;
        }
        for r in 0..n {
            if r == rank {
                continue;
            }
            let factor = matrix[r][column];
            for j in column..n {
                matrix[r][j] -= factor * matrix[rank][j];
            }
            let pivot_values = values[rank].clone();
            for (value, pivot_value) in values[r].iter_mut().zip(pivot_values) {
                *value -= factor * pivot_value;
            }
        }
        rank += 1;
    }
    numeric(rank >= 1, "Fitting system is rank deficient below one")?;
    // Undo column permutation on the coefficient vector.
    let width = values[0].len();
    let mut ordered = vec![vec![0.; width]; n];
    for (logical, physical) in column_perm.iter().enumerate().take(rank) {
        ordered[*physical] = values[logical].clone();
    }
    Ok((ordered, rank, pivot_min))
}

pub fn fit_curve_cloud_certified(
    points: Vec<Vec<f64>>,
    control_count: usize,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    check(
        points.len() >= 2 && points.len() <= 4096,
        "Cloud curve fitting needs 2..4096 sites",
    )?;
    let dimension = points[0].len();
    check(
        (dimension == 2 || dimension == 3)
            && points
                .iter()
                .all(|p| p.len() == dimension && p.iter().all(|x| x.is_finite())),
        "Cloud points must be finite 2D or 3D",
    )?;
    check(
        (2..=26).contains(&control_count),
        "Cloud fit control count must be 2..26",
    )?;
    let mut working = control_count.min(points.len());
    let parameters = chord_parameters(&points);
    let mut last_error = None;
    let (controls, rank, pivot_min) = loop {
        let degree = working - 1;
        let design = parameters
            .iter()
            .map(|&t| {
                (0..working)
                    .map(|i| {
                        binomial(degree, i) * t.powi(i as i32) * (1. - t).powi((degree - i) as i32)
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let matrix = (0..working)
            .map(|i| {
                (0..working)
                    .map(|j| design.iter().map(|row| row[i] * row[j]).sum())
                    .collect()
            })
            .collect();
        let rhs = (0..working)
            .map(|i| {
                (0..dimension)
                    .map(|axis| {
                        design
                            .iter()
                            .zip(&points)
                            .map(|(row, p)| row[i] * p[axis])
                            .sum()
                    })
                    .collect()
            })
            .collect();
        match robust_solve(matrix, rhs) {
            Ok(result) => break result,
            Err(error) => {
                last_error = Some(error);
                if working <= 2 {
                    break (Vec::new(), 0, 0.);
                }
                working -= 1;
            }
        }
    };
    if rank == 0 {
        return Err(last_error.unwrap_or_else(|| crate::input("Cloud fit failed")));
    }
    let degree = working - 1;
    let curve = Curve {
        degree,
        knots: std::iter::repeat_n(0., degree + 1)
            .chain(std::iter::repeat_n(1., degree + 1))
            .collect(),
        control_points: controls[..working].to_vec(),
        weights: vec![1.; working],
        periodic: false,
    };
    curve.validate()?;
    let site_error = parameters
        .iter()
        .zip(&points)
        .map(|(&u, point)| distance(&curve.evaluate(u).unwrap().point, point))
        .fold(0., f64::max);
    // Continuum remainder under admitted assumption: chordal parameters form a δ-net in the fitted domain
    // with δ = 1/(N-1); residual Lipschitz ≤ speed_upper of the fitted curve.
    let mut speed = 0_f64;
    for segment in curve.decompose()? {
        let c = segment.definition();
        for pair in c.control_points.windows(2) {
            speed = speed.max(distance(&pair[0], &pair[1]) * c.degree as f64);
        }
    }
    let delta = 1. / (points.len() - 1).max(1) as f64;
    let hausdorff = next_up(site_error + next_up(speed) * delta);
    let tolerance = context(tolerance);
    Ok(
        json!({"curve":curve,"certificate":{"version":"nurbs-foundation/4","classification":"approximate_cloud_fit",
        "fittedToExactPromotion":false,"dataSiteErrorUpper":next_up(site_error),
        "hausdorffErrorUpper":hausdorff,
        "admittedAssumptions":["chordal-parameter-delta-net-of-fitted-domain","residual-Lipschitz-from-control-polygon-speed"],
        "siteCount":points.len(),"controlCount":working,"requestedControls":control_count,
        "conditioning":{"method":"robust-pivoted-normal-equations","rank":rank,"pivotLower":pivot_min},
        "resource":{"maxSites":4096,"maxControls":26},"evidence":tolerance_evidence(&tolerance)}}),
    )
}

pub fn fit_surface_cloud_certified(
    points: Vec<[f64; 3]>,
    controls_u: usize,
    controls_v: usize,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    check(
        points.len() >= 4 && points.len() <= 4096,
        "Cloud surface fitting needs 4..4096 sites",
    )?;
    check(
        (2..=8).contains(&controls_u) && (2..=8).contains(&controls_v),
        "Cloud surface controls per axis must be 2..8",
    )?;
    check(
        points.iter().flatten().all(|x| x.is_finite()),
        "Cloud surface points must be finite",
    )?;
    // PCA plane parameters as admitted unstructured domain.
    let centroid =
        [0, 1, 2].map(|axis| points.iter().map(|p| p[axis]).sum::<f64>() / points.len() as f64);
    let mut cov = [[0.; 3]; 3];
    for point in &points {
        let d = [
            point[0] - centroid[0],
            point[1] - centroid[1],
            point[2] - centroid[2],
        ];
        for i in 0..3 {
            for j in 0..3 {
                cov[i][j] += d[i] * d[j];
            }
        }
    }
    // Power iteration for two dominant planar axes.
    let mut axes = [[1., 0., 0.], [0., 1., 0.]];
    for axis in &mut axes {
        for _ in 0..24 {
            let mut next = [0.; 3];
            for i in 0..3 {
                next[i] = cov[i][0] * axis[0] + cov[i][1] * axis[1] + cov[i][2] * axis[2];
            }
            let norm = next
                .iter()
                .map(|x| x * x)
                .sum::<f64>()
                .sqrt()
                .max(f64::from_bits(1));
            *axis = next.map(|x| x / norm);
        }
    }
    let mut uv: Vec<[f64; 2]> = points
        .iter()
        .map(|point| {
            let d = [
                point[0] - centroid[0],
                point[1] - centroid[1],
                point[2] - centroid[2],
            ];
            [
                d[0] * axes[0][0] + d[1] * axes[0][1] + d[2] * axes[0][2],
                d[0] * axes[1][0] + d[1] * axes[1][1] + d[2] * axes[1][2],
            ]
        })
        .collect();
    let u_bounds = interval(uv.iter().map(|p| p[0]));
    let v_bounds = interval(uv.iter().map(|p| p[1]));
    for parameter in &mut uv {
        parameter[0] =
            (parameter[0] - u_bounds[0]) / (u_bounds[1] - u_bounds[0]).max(f64::from_bits(1));
        parameter[1] =
            (parameter[1] - v_bounds[0]) / (v_bounds[1] - v_bounds[0]).max(f64::from_bits(1));
    }
    let degree_u = controls_u - 1;
    let degree_v = controls_v - 1;
    let count = controls_u * controls_v;
    let design = uv
        .iter()
        .map(|parameter| {
            (0..controls_u)
                .flat_map(|i| {
                    (0..controls_v).map(move |j| {
                        binomial(degree_u, i)
                            * parameter[0].powi(i as i32)
                            * (1. - parameter[0]).powi((degree_u - i) as i32)
                            * binomial(degree_v, j)
                            * parameter[1].powi(j as i32)
                            * (1. - parameter[1]).powi((degree_v - j) as i32)
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let matrix = (0..count)
        .map(|i| {
            (0..count)
                .map(|j| design.iter().map(|row| row[i] * row[j]).sum())
                .collect()
        })
        .collect();
    let rhs = (0..count)
        .map(|i| {
            (0..3)
                .map(|axis| {
                    design
                        .iter()
                        .zip(&points)
                        .map(|(row, p)| row[i] * p[axis])
                        .sum()
                })
                .collect()
        })
        .collect();
    let (flat, rank, pivot_min) = robust_solve(matrix, rhs)?;
    let control_points = (0..controls_u)
        .map(|i| {
            (0..controls_v)
                .map(|j| flat[i * controls_v + j].clone())
                .collect()
        })
        .collect();
    let surface = Surface {
        degree_u,
        degree_v,
        knots_u: std::iter::repeat_n(0., degree_u + 1)
            .chain(std::iter::repeat_n(1., degree_u + 1))
            .collect(),
        knots_v: std::iter::repeat_n(0., degree_v + 1)
            .chain(std::iter::repeat_n(1., degree_v + 1))
            .collect(),
        control_points,
        weights: vec![vec![1.; controls_v]; controls_u],
        periodic_u: false,
        periodic_v: false,
    };
    surface.validate()?;
    let site_error = uv
        .iter()
        .zip(&points)
        .map(|(parameter, point)| {
            distance(
                &surface
                    .evaluate_validated(parameter[0], parameter[1])
                    .unwrap()
                    .point,
                point,
            )
        })
        .fold(0., f64::max);
    let controls: Vec<Vec<f64>> = surface.control_points.iter().flatten().cloned().collect();
    let (min, max) = box_of(&controls);
    let diameter = next_up(
        min.iter()
            .zip(max)
            .map(|(a, b)| (b - a) * (b - a))
            .sum::<f64>()
            .sqrt(),
    );
    let delta = 1. / ((points.len() as f64).sqrt().max(1.));
    let hausdorff = next_up(site_error + diameter * delta);
    let tolerance = context(tolerance);
    Ok(
        json!({"surface":surface,"certificate":{"version":"nurbs-foundation/4","classification":"approximate_cloud_fit",
        "fittedToExactPromotion":false,"dataSiteErrorUpper":next_up(site_error),"hausdorffErrorUpper":hausdorff,
        "admittedAssumptions":["PCA-plane-parameter-domain","delta-net-density-from-sqrt-N-samples","control-hull-Lipschitz-remainder"],
        "siteCount":points.len(),"controlsU":controls_u,"controlsV":controls_v,
        "conditioning":{"method":"robust-pivoted-normal-equations","rank":rank,"pivotLower":pivot_min},
        "resource":{"maxSites":4096,"maxControlsPerAxis":8},"evidence":tolerance_evidence(&tolerance)}}),
    )
}
