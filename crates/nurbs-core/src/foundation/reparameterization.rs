use super::*;

#[derive(Clone)]
pub(super) struct MapPiece {
    pub(super) domain: [f64; 2],
    pub(super) range: [f64; 2],
    pub(super) values: Vec<f64>,
    pub(super) weights: Vec<f64>,
}
pub(super) fn map_pieces(mapping: &Value) -> Result<Vec<MapPiece>> {
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
pub(super) fn evaluate_map_piece(piece: &MapPiece, u: f64) -> f64 {
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
pub(super) fn evaluate_mapping(mapping: &Value, u: f64) -> Result<f64> {
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
    certify_mapping(mapping,tolerance,0)
}
pub(super) fn certify_mapping(mapping: &Value, tolerance: Option<ToleranceContext>, depth: usize) -> Result<Value> {
    check(depth <= 8,"Mapping certification nesting exceeds 8")?;
    if let Some(composition) = mapping.get("composition").and_then(Value::as_array) {
        check(
            !composition.is_empty() && composition.len() <= 16,
            "Mapping composition needs 1..16 factors",
        )?;
        let factors = composition
            .iter()
            .map(|factor| certify_mapping(factor, None, depth + 1))
            .collect::<Result<Vec<_>>>()?;
        for pair in factors.windows(2) {
            check(
                pair[0]["range"] == pair[1]["domain"],
                "Composed mapping ranges and domains must match exactly",
            )?;
        }
        let tolerance = context(tolerance);
        return Ok(
            json!({"version":"nurbs-foundation/3","mapping":mapping.clone(),
            "classification":"certified_strictly_monotone_composition","factors":factors,
            "domain":factors.first().unwrap()["domain"].clone(),"range":factors.last().unwrap()["range"].clone(),
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
            let bounds = preimages::prove_increasing(piece)?;
            Ok(
                json!({"domain":piece.domain,"range":piece.range,"derivativeNumeratorBounds":bounds,
            "denominatorBounds":interval(piece.weights.iter().copied()),
            "inverseInterval":piece.domain,"method":"outward-pairwise-Bernstein-subdivision-positive-derivative"}),
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let tolerance = context(tolerance);
    Ok(
        json!({"version":"nurbs-foundation/3","mapping":mapping.clone(),"classification":"certified_strictly_monotone",
        "domain":[pieces.first().unwrap().domain[0],pieces.last().unwrap().domain[1]],
        "range":[pieces.first().unwrap().range[0],pieces.last().unwrap().range[1]],
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
