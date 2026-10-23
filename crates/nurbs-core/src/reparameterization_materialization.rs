//! Algebraic span composition with bounded, representable knot preimages.
//! The final gate compares against the original source and entire authored map.
use super::{
    MapPiece, bound_reparameterization_preimages, budget_controls, certify_reparameterization,
    certify_reparameterized_curve_retention, compose_bezier_with_piece, map_pieces,
};
use crate::{Result, check, curve::Curve};
use value_codec::{Value, json};

fn join(parts: Vec<Curve>, tolerance: f64) -> Result<Curve> {
    let degree = parts
        .iter()
        .map(|c| c.degree)
        .max()
        .ok_or_else(|| crate::input("No composed spans"))?;
    let mut parts = parts
        .into_iter()
        .map(|c| c.elevate(degree))
        .collect::<Result<Vec<_>>>()?
        .into_iter();
    let mut result = parts.next().unwrap();
    for mut next in parts {
        let left = result.control_points.last().unwrap();
        let right = &next.control_points[0];
        let distance = left
            .iter()
            .zip(right)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt();
        check(
            distance <= tolerance,
            "Composed knot join exceeds the retention tolerance",
        )?;
        let factor = result.weights.last().unwrap() / next.weights[0];
        for weight in &mut next.weights {
            *weight *= factor;
        }
        // One owner for the common C0 point. This change is not called exact:
        // the final whole-composition gate includes its effect on the span.
        next.control_points[0] = left.clone();
        next.validate()?;
        result.knots.pop();
        result
            .knots
            .extend(next.knots[degree + 1..].iter().copied());
        result
            .control_points
            .extend(next.control_points.into_iter().skip(1));
        result.weights.extend(next.weights.into_iter().skip(1));
        budget_controls(result.control_points.len())?;
        result.validate()?;
    }
    Ok(result)
}

fn piece(curve: &Curve, piece: &MapPiece, preimages: &mut Vec<Value>) -> Result<Vec<Curve>> {
    if piece.values.len() == 2 && piece.weights[0] == piece.weights[1] {
        let mut result = curve.trim(piece.range[0], piece.range[1])?;
        for knot in &mut result.knots {
            *knot = if *knot == piece.range[0] {
                piece.domain[0]
            } else if *knot == piece.range[1] {
                piece.domain[1]
            } else {
                piece.domain[0]
                    + (*knot - piece.range[0]) / (piece.range[1] - piece.range[0])
                        * (piece.domain[1] - piece.domain[0])
            };
        }
        result.validate()?;
        return Ok(vec![result]);
    }
    if curve.degree.saturating_mul(piece.values.len() - 1) > 25 {
        return Err(crate::resource("Composed degree exceeds 25"));
    }
    // A common map weight scale has no geometric meaning. Normalize before
    // denominator powers, and include any relative-weight rounding in the
    // final audit against the original, unnormalized authored mapping.
    let scale = piece.weights.iter().copied().fold(0., f64::max);
    let mut normalized = piece.clone();
    for weight in &mut normalized.weights {
        *weight /= scale;
    }
    let restricted = curve.trim(piece.range[0], piece.range[1])?;
    let spans = restricted.decompose()?;
    if spans.len() == 1 {
        return Ok(vec![compose_bezier_with_piece(
            spans[0].definition(),
            &normalized,
        )?]);
    }
    let knots: Vec<_> = spans
        .iter()
        .skip(1)
        .map(|s| s.definition().domain()[0])
        .collect();
    let mapping = json!({"pieces":[{"domain":piece.domain,"range":piece.range,
        "controlValues":piece.values,"weights":piece.weights}]});
    let report = bound_reparameterization_preimages(&mapping, &knots, 1e-12, 100000)?;
    let mut breaks = vec![piece.domain[0]];
    for root in report["preimages"].as_array().unwrap() {
        let bracket: [f64; 2] = value_codec::from_value(root["parameterInterval"].clone())
            .map_err(|e| crate::input(e.to_string()))?;
        breaks.push(bracket[0] + (bracket[1] - bracket[0]) * 0.5);
    }
    breaks.push(piece.domain[1]);
    check(
        breaks.windows(2).all(|p| p[0] < p[1]),
        "Knot inverse brackets lack distinct representable split points",
    )?;
    preimages.push(report);
    let mut result = Vec::new();
    for (span, domain) in spans.iter().zip(breaks.windows(2)) {
        let subpiece = restrict_piece(
            &normalized,
            [domain[0], domain[1]],
            span.definition().domain(),
        )?;
        result.push(compose_bezier_with_piece(span.definition(), &subpiece)?);
    }
    // Joining and any source/map trim rounding are audited at the root.
    Ok(result)
}

fn restrict_piece(piece: &MapPiece, domain: [f64; 2], range: [f64; 2]) -> Result<MapPiece> {
    // Scalar map controls are parameters, not geometric coordinates. Keep
    // their homogeneous subdivision independent of Curve's coordinate cap.
    let split = |net: &[[f64; 2]], parameter: f64| {
        let mut work = net.to_vec();
        let mut left = vec![work[0]];
        let mut right = vec![*work.last().unwrap()];
        for size in (1..net.len()).rev() {
            for i in 0..size {
                for axis in 0..2 {
                    work[i][axis] =
                        (1. - parameter) * work[i][axis] + parameter * work[i + 1][axis];
                }
            }
            left.push(work[0]);
            right.push(work[size - 1]);
        }
        right.reverse();
        (left, right)
    };
    let extent = piece.domain[1] - piece.domain[0];
    let low = (domain[0] - piece.domain[0]) / extent;
    let high = (domain[1] - piece.domain[0]) / extent;
    check(
        low.is_finite() && high.is_finite() && low >= 0. && low < high && high <= 1.,
        "Map subdivision parameters are not distinct in binary64",
    )?;
    let net: Vec<_> = piece
        .values
        .iter()
        .zip(&piece.weights)
        .map(|(&x, &w)| [x * w, w])
        .collect();
    let prefix = if high == 1. { net } else { split(&net, high).0 };
    let restricted = if low == 0. {
        prefix
    } else {
        split(&prefix, low / high).1
    };
    let weights: Vec<_> = restricted.iter().map(|p| p[1]).collect();
    let values: Vec<_> = restricted.iter().map(|p| p[0] / p[1]).collect();
    crate::numeric(
        values.iter().all(|v| v.is_finite()) && weights.iter().all(|&w| w.is_finite() && w > 0.),
        "Map subdivision left its finite positive representation window",
    )?;
    Ok(MapPiece {
        domain,
        range,
        values,
        weights,
    })
}

fn tree(
    source: &Curve,
    mapping: &Value,
    tolerance: f64,
    depth: usize,
    preimages: &mut Vec<Value>,
) -> Result<Curve> {
    check(depth <= 8, "Composition nesting exceeds 8")?;
    if let Some(parts) = mapping.get("composition").and_then(Value::as_array) {
        check(
            !parts.is_empty() && parts.len() <= 8,
            "Composition needs 1..8 factors",
        )?;
        let mut result = source.clone();
        for part in parts.iter().rev() {
            result = tree(&result, part, tolerance, depth + 1, preimages)?;
        }
        return Ok(result);
    }
    let mut parts = Vec::new();
    for map in map_pieces(mapping)? {
        parts.extend(piece(source, &map, preimages)?);
    }
    join(parts, tolerance)
}

/// Materialize a monotone map across any source knot spans. All rounding,
/// representable split points and C0 ownership edits are included in a final
/// whole-domain numerical certificate. An unresolved certificate refuses.
pub fn materialize_reparameterized_curve_bounded(
    source: &Curve,
    mapping: &Value,
    tolerance: f64,
    max_cells: usize,
    max_map_evaluations: usize,
) -> Result<Value> {
    source.validate()?;
    check(
        max_cells > 0
            && max_cells <= 1_000_000
            && max_map_evaluations > 0
            && max_map_evaluations <= 1_000_000,
        "Composition verification budgets must be in 1..1000000",
    )?;
    check(
        !source.periodic,
        "Bounded materialization needs an open fundamental cover",
    )?;
    check(
        tolerance.is_finite() && tolerance > 0.,
        "Composition tolerance must be positive",
    )?;
    let map_certificate = certify_reparameterization(mapping, None)?;
    let range: [f64; 2] = value_codec::from_value(map_certificate["range"].clone())
        .map_err(|e| crate::input(e.to_string()))?;
    let [a, b] = source.domain();
    check(
        range[0] >= a && range[1] <= b,
        "Map range is outside the source domain",
    )?;
    let mut preimages = Vec::new();
    let result = tree(source, mapping, tolerance, 0, &mut preimages)?;
    let retention = certify_reparameterized_curve_retention(
        source,
        mapping,
        &result,
        tolerance,
        max_cells,
        max_map_evaluations,
    )?;
    if retention["certificate"]["accepted"].as_bool() != Some(true) {
        return Err(crate::numeric_err(format!(
            "Composed curve retention is {}",
            retention["certificate"]["status"]
                .as_str()
                .unwrap_or("unresolved")
        )));
    }
    Ok(
        json!({"curve":result,"certificate":{"operation":"materialize-bounded-reparameterization",
        "exact":false,"fittedToExactPromotion":false,"method":"homogeneous-Bernstein-composition-with-enclosed-knot-preimages",
        "preimages":preimages,"retention":retention["certificate"],"mapCertificate":map_certificate}}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source() -> Curve {
        Curve {
            degree: 1,
            knots: vec![0., 0., 0.5, 1., 1.],
            control_points: vec![vec![0., 0.], vec![0.5, 1.], vec![1., 0.]],
            weights: vec![1., 0.75, 1.],
            periodic: false,
        }
    }
    fn mapping() -> Value {
        json!({"pieces":[{"domain":[0.,1.],"range":[0.,1.],"controlValues":[0.,0.25,1.],"weights":[1.,1.,1.]}]})
    }
    #[test]
    fn nonlinear_map_crosses_an_irrational_source_knot_preimage() {
        let original = source();
        let mapped =
            materialize_reparameterized_curve_bounded(&original, &mapping(), 1e-5, 50000, 200000)
                .unwrap();
        let result: Curve = value_codec::from_value(mapped["curve"].clone()).unwrap();
        let upper = mapped["certificate"]["retention"]["errorUpper"]
            .as_f64()
            .unwrap();
        assert!(upper <= 1e-5);
        assert_eq!(mapped["certificate"]["exact"], false);
        assert_eq!(
            mapped["certificate"]["preimages"].as_array().unwrap().len(),
            1
        );
        for i in 0..=1000 {
            let u = i as f64 / 1000.;
            let expected = original.evaluate((u + u * u) * 0.5).unwrap().point;
            let actual = result.evaluate(u).unwrap().point;
            let distance = expected
                .iter()
                .zip(actual)
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!(distance <= upper, "{u}: {distance} > {upper}");
        }
    }
    #[test]
    fn nested_rational_maps_cross_source_knots_and_retain_authored_order() {
        let original = source();
        let rational = json!({"pieces":[{"domain":[0.,1.],"range":[0.,1.],"controlValues":[0.,1.],"weights":[1.,2.]}]});
        let mapping = json!({"composition":[mapping(),rational]});
        let mapped =
            materialize_reparameterized_curve_bounded(&original, &mapping, 1e-5, 50000, 200000)
                .unwrap();
        let result: Curve = value_codec::from_value(mapped["curve"].clone()).unwrap();
        let upper = mapped["certificate"]["retention"]["errorUpper"]
            .as_f64()
            .unwrap();
        for i in 0..=1000 {
            let u = i as f64 / 1000.;
            let p = (u + u * u) * 0.5;
            let expected = original.evaluate(2. * p / (1. + p)).unwrap().point;
            let actual = result.evaluate(u).unwrap().point;
            let distance = expected
                .iter()
                .zip(actual)
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!(distance <= upper, "{u}: {distance} > {upper}");
        }
    }
    #[cfg(feature = "transport")]
    #[test]
    fn json_materialization_reports_numerical_retention_and_refuses_depleted_work() {
        let request = json!({"op":"curve_materialize_reparameterization_bounded","curve":source(),"mapping":mapping(),
            "errorBudget":1e-5,"maxCells":50000,"maxMapEvaluations":200000});
        let mapped = crate::dispatch(request.clone()).unwrap();
        assert_eq!(mapped["certificate"]["retention"]["accepted"], true);
        let mut depleted = request;
        depleted["maxCells"] = json!(1);
        assert!(crate::dispatch(depleted).is_err());
    }
    #[test]
    fn scalar_parameters_do_not_inherit_geometric_coordinate_limits() {
        let mut original = source();
        original.knots = vec![-1e9, -1e9, 0., 1e9, 1e9];
        let controls = [-1e9, -5e8, 2e9, 5e8, 1e9];
        let weights = [1., 1., 0.001, 1., 1.];
        let map = json!({"pieces":[{"domain":[0.,1.],"range":[-1e9,1e9],
            "controlValues":controls,"weights":weights}]});
        let mapped =
            materialize_reparameterized_curve_bounded(&original, &map, 1e-5, 50000, 200000)
                .unwrap();
        let result: Curve = value_codec::from_value(mapped["curve"].clone()).unwrap();
        let upper = mapped["certificate"]["retention"]["errorUpper"]
            .as_f64()
            .unwrap();
        for i in 0..=1000 {
            let u = i as f64 / 1000.;
            let coefficients = [1., 4., 6., 4., 1.];
            let basis: Vec<_> = (0..=4)
                .map(|j| {
                    coefficients[j] * u.powi(j as i32) * (1. - u).powi((4 - j) as i32) * weights[j]
                })
                .collect();
            let parameter = controls.iter().zip(&basis).map(|(p, w)| p * w).sum::<f64>()
                / basis.iter().sum::<f64>();
            let expected = original.evaluate(parameter).unwrap().point;
            let actual = result.evaluate(u).unwrap().point;
            let distance = expected
                .iter()
                .zip(actual)
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!(distance <= upper);
        }
    }
    #[test]
    fn affine_partial_range_near_an_endpoint_preserves_the_authored_domain() {
        let original = crate::primitives::line([0., 0., 0.], [1., 0., 0.]).unwrap();
        let epsilon = 2e-15;
        let map = json!({"pieces":[{"domain":[0.,1.],"range":[epsilon,1.],"controlValues":[epsilon,1.],"weights":[1.,1.]}]});
        let mapped =
            materialize_reparameterized_curve_bounded(&original, &map, 1e-8, 100, 1000).unwrap();
        let result: Curve = value_codec::from_value(mapped["curve"].clone()).unwrap();
        assert_eq!(result.domain(), [0., 1.]);
        assert_eq!(result.control_points[0][0], epsilon);
        for u in [0., 0.13, 0.5, 1.] {
            assert!(
                (result.evaluate(u).unwrap().point[0] - (epsilon + (1. - epsilon) * u)).abs()
                    < 1e-14
            );
        }
    }
    #[test]
    fn common_map_weight_scale_does_not_exhaust_composed_weights() {
        let original = source().elevate(2).unwrap();
        let mut scaled = mapping();
        scaled["pieces"][0]["weights"] = json!([1e12, 1e12, 1e12]);
        let mapped =
            materialize_reparameterized_curve_bounded(&original, &scaled, 1e-5, 50000, 200000)
                .unwrap();
        let result: Curve = value_codec::from_value(mapped["curve"].clone()).unwrap();
        let upper = mapped["certificate"]["retention"]["errorUpper"]
            .as_f64()
            .unwrap();
        for i in 0..=1000 {
            let u = i as f64 / 1000.;
            let expected = original.evaluate((u + u * u) * 0.5).unwrap().point;
            let actual = result.evaluate(u).unwrap().point;
            let distance = expected
                .iter()
                .zip(actual)
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!(distance <= upper);
        }
        let line = crate::primitives::line([0., 0., 0.], [1., 0., 0.])
            .unwrap()
            .elevate(2)
            .unwrap();
        assert!(
            materialize_reparameterized_curve_bounded(&line, &scaled, 1e-5, 50000, 200000).is_ok()
        );
    }
    #[test]
    fn incomplete_whole_domain_verification_refuses_materialization() {
        assert!(
            materialize_reparameterized_curve_bounded(&source(), &mapping(), 1e-5, 1, 100).is_err()
        );
    }
}
