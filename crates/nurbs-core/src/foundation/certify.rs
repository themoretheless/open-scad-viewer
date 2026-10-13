use super::*;

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

pub(super) fn curve_span_regularity(curve: &Curve) -> Value {
    curve_span_regularity_depth(curve, 0)
}
pub(super) fn curve_span_regularity_depth(curve: &Curve, depth: usize) -> Value {
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

pub(super) fn binomial(n: usize, k: usize) -> f64 {
    let k = k.min(n - k);
    (0..k).fold(1., |value, i| value * (n - i) as f64 / (i + 1) as f64)
}
pub(super) fn bernstein_product(first: &[f64], second: &[f64]) -> Vec<f64> {
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

pub(super) fn bernstein_split(coefficients: &[f64]) -> (Vec<f64>, Vec<f64>) {
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

pub(super) fn sign_variations(coefficients: &[f64]) -> usize {
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

pub(super) fn stationary_coefficients(curve: &Curve, query: &[f64]) -> Vec<f64> {
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
pub(super) struct RootBox {
    pub(super) lo: f64,
    pub(super) hi: f64,
    pub(super) coefficients: Vec<f64>,
    pub(super) variation: usize,
}

pub(super) fn isolate_stationary(coefficients: Vec<f64>, floor: f64) -> (Vec<RootBox>, bool) {
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
        // Sign variation ignores zero coefficients. A root exactly on this
        // subdivision boundary would otherwise disappear from both children.
        if left.last() == Some(&0.) {
            roots.push(RootBox {
                lo: middle,
                hi: middle,
                variation: node.variation,
                coefficients: vec![0.],
            });
        }
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

pub(super) fn distance(point: &[f64], query: &[f64]) -> f64 {
    point
        .iter()
        .zip(query)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f64>()
        .sqrt()
}

pub(super) fn point_box_distance(min: &[f64], max: &[f64], point: &[f64]) -> f64 {
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

pub(super) fn tensor_product(first: &[Vec<f64>], second: &[Vec<f64>]) -> Vec<Vec<f64>> {
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

pub(super) fn surface_normal_bounds(surface: &Surface, iu: usize, iv: usize) -> Vec<[f64; 2]> {
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

pub(super) fn fully_bezier_surface(surface: &Surface) -> Result<Surface> {
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

pub(super) fn certify_surface_cell(surface: &Surface, iu: usize, iv: usize) -> Value {
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
    // Aligned corner normals do not prove planarity or exclude an interior fold.
    // Constant coordinates prove an axis-aligned plane; separation additionally
    // proves that its parameterization does not lose rank inside the cell.
    let axis_plane = (0..3).any(|axis| {
        let value=surface.control_points[iu-surface.degree_u][iv-surface.degree_v][axis];
        (iu-surface.degree_u..=iu).all(|u|(iv-surface.degree_v..=iv).all(|v|surface.control_points[u][v][axis]==value))
    });
    if aligned && separated && axis_plane {
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
