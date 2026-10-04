//! Host decoding and compatibility reports for rational parameter mapping.
use super::*;
use value_codec::{Serialize, Value, json};
pub fn decode_mapping(mapping: &Value) -> Result<ParameterMapping> {
    decode_mapping_with_support(mapping, false)
}
fn decode_mapping_with_support(mapping: &Value, needs_support: bool) -> Result<ParameterMapping> {
    if let Some(factors) = mapping.get("composition").and_then(Value::as_array) {
        check(
            !factors.is_empty() && factors.len() <= 16,
            "Mapping composition needs 1..16 factors",
        )?;
        let children_need_support = factors.len() > 1;
        return Ok(ParameterMapping::Composition {
            factors: factors
                .iter()
                .map(|factor| decode_mapping_with_support(factor, children_need_support))
                .collect::<Result<Vec<_>>>()?,
            piece_support: if needs_support {
                Some(decode_pieces(mapping)?)
            } else {
                None
            },
        });
    }
    Ok(ParameterMapping::Pieces(decode_pieces(mapping)?))
}
fn decode_pieces(mapping: &Value) -> Result<Vec<MapPiece>> {
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
            let decode_error = |error: value_codec::Error| crate::input(error.to_string());
            Ok(MapPiece {
                domain: value_codec::from_value(piece["domain"].clone()).map_err(decode_error)?,
                range: value_codec::from_value(piece["range"].clone()).map_err(decode_error)?,
                values: value_codec::from_value(piece["controlValues"].clone())
                    .map_err(decode_error)?,
                weights: value_codec::from_value(piece["weights"].clone()).map_err(decode_error)?,
            })
        })
        .collect()
}
fn original_mapping(mut encoded: Value, source: &Value) -> Value {
    encoded["mapping"] = source.clone();
    if let Some(factors) = source.get("composition").and_then(Value::as_array) {
        if let Some(certificates) = encoded["factors"].as_array_mut() {
            for (certificate, factor) in certificates.iter_mut().zip(factors) {
                *certificate = original_mapping(certificate.clone(), factor);
            }
        }
    }
    encoded
}
pub fn certify_reparameterization(
    mapping: &Value,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    Ok(original_mapping(
        certify_reparameterization_report(&decode_mapping(mapping)?, tolerance)?.to_value(),
        mapping,
    ))
}
pub fn evaluate_reparameterized_curve(
    curve: &Curve,
    mapping: &Value,
    u: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    let result =
        evaluate_reparameterized_curve_report(curve, &decode_mapping(mapping)?, u, tolerance)?;
    let mut encoded = result.to_value();
    encoded["certificate"] = original_mapping(encoded["certificate"].clone(), mapping);
    Ok(encoded)
}
pub fn materialize_reparameterized_curve(
    curve: &Curve,
    mapping: &Value,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    let result =
        materialize_reparameterized_curve_report(curve, &decode_mapping(mapping)?, tolerance)?;
    let mut encoded = result.to_value();
    encoded["certificate"]["mapCertificate"] =
        original_mapping(encoded["certificate"]["mapCertificate"].clone(), mapping);
    Ok(encoded)
}
impl Serialize for MapPiece {
    fn to_value(&self) -> Value {
        json!({"domain":self.domain,"range":self.range,"controlValues":self.values,"weights":self.weights})
    }
}
impl Serialize for ParameterMapping {
    fn to_value(&self) -> Value {
        match self {
            Self::Pieces(pieces) => json!({"pieces":pieces}),
            Self::Composition {
                factors,
                piece_support,
            } => {
                let mut result = json!({"composition":factors});
                if let Some(pieces) = piece_support {
                    result["pieces"] = json!(pieces);
                }
                result
            }
        }
    }
}
impl Serialize for MapPieceCertificate {
    fn to_value(&self) -> Value {
        json!({"domain":self.domain,"range":self.range,"derivativeNumeratorBounds":self.derivative_numerator_bounds,"denominatorBounds":self.denominator_bounds,"inverseInterval":self.domain,"method":"rational-Bernstein-positive-derivative"})
    }
}
impl Serialize for ParameterMapCertificate {
    fn to_value(&self) -> Value {
        let mut result = json!({"version":"nurbs-foundation/3","mapping":self.mapping,"composition":"exact-semantic-evaluation","evidence":super::super::tolerance_evidence(&self.tolerance)});
        match &self.proof {
            ParameterMapProof::Piecewise(pieces) => {
                result["classification"] = json!("certified_strictly_monotone");
                result["pieces"] = json!(pieces);
                result["inverse"] = json!("interval-bisection-certified");
            }
            ParameterMapProof::Composition(factors) => {
                result["classification"] = json!("certified_strictly_monotone_composition");
                result["factors"] = json!(factors);
                result["inverse"] = json!("reverse-factor-interval-chain");
            }
        }
        result
    }
}
impl Serialize for ReparameterizedEvaluation {
    fn to_value(&self) -> Value {
        json!({"version":"nurbs-foundation/3","parameter":self.parameter,"sourceParameter":self.source_parameter,"evaluation":self.evaluation,"certificate":self.certificate})
    }
}
impl Serialize for MappingTree {
    fn to_value(&self) -> Value {
        match self {
            Self::Composition { depth, children } => {
                json!({"kind":"nested_composition","depth":depth,"children":children})
            }
            Self::Piecewise { piece_count } => json!({"kind":"piecewise","pieceCount":piece_count}),
        }
    }
}
impl Serialize for MaterializedCurve {
    fn to_value(&self) -> Value {
        json!({"curve":self.curve,"certificate":{
  "version":"nurbs-foundation/5","operation":"materialize-nonlinear-reparameterization","exact":true,
  "fittedToExactPromotion":false,"degreeGrowth":self.degree_growth,"periodicCoverMaterialized":self.periodic_cover,
  "compositionTree":self.tree,"controlCount":self.curve.control_points.len(),"resource":{"maxControls":256,"maxDegree":25,"maxNesting":8},
  "method":"homogeneous-Bernstein-clear-denominator-composition","mapCertificate":self.map_certificate,
  "evidence":super::super::tolerance_evidence(&self.tolerance)}})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn identity() -> Value {
        json!({"pieces":[{"domain":[0.,1.],"range":[0.,1.],"controlValues":[0.,1.],"weights":[1.,1.]}]})
    }
    #[test]
    fn legacy_composite_support_and_original_metadata_are_preserved() {
        let leaf = identity();
        let hybrid = json!({"composition":[leaf.clone()],"pieces":leaf["pieces"].clone(),"label":"composite-support"});
        let mapping = json!({"composition":[hybrid.clone(),leaf],"label":"original-document"});
        let proof = certify_reparameterization(&mapping, None).unwrap();
        assert_eq!(proof["mapping"], mapping);
        assert_eq!(proof["factors"][0]["mapping"], hybrid);
        assert_eq!(
            proof["classification"],
            json!("certified_strictly_monotone_composition")
        );
    }
    #[test]
    fn unused_piece_payload_does_not_override_composition() {
        let mapping = json!({"composition":[identity()],"pieces":"unused-payload"});
        let proof = certify_reparameterization(&mapping, None).unwrap();
        assert_eq!(proof["mapping"], mapping);
    }
}
