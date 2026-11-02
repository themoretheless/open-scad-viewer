//! Exact-rational semantic carrier backed by fresh native frozen-recipe proof.
use crate::{field, input, Result, Value};
use value_codec::json;
pub(crate) const REPRESENTATION: &str = "rational-brep";
pub(crate) fn construct(node: &Value, source: brep_core::Model) -> Result<Value> {
    let ty = &node["valueType"];
    if ty["space"].as_str() != Some("d3")
        || ty["geometryKind"].as_str() != Some("solid")
        || ty["representation"].as_str() != Some(REPRESENTATION)
        || ty["evidence"]["tag"].as_str() != Some("representation-preserving")
    {
        return Err(input(
            "Full Round exact node requires its native-admitted exact solid carrier",
        ));
    }
    let faces: [usize; 3] = field(node, "faces")?;
    let mut request = json!({"op":"cad_full_round_exact","source":source,"faces":faces});
    for key in ["limits", "maxBytes"] {
        if let Some(v) = node.get(key) {
            request[key] = v.clone();
        }
    }
    let artifact = crate::cad_full_round_exact::construct(request)?;
    Ok(
        json!({"kind":"solid","representation":REPRESENTATION,"model":artifact["geometry"].clone(),"artifact":artifact}),
    )
}
pub(crate) fn validate(value: &Value) -> Result<()> {
    if value["kind"].as_str() != Some("solid")
        || value["representation"].as_str() != Some(REPRESENTATION)
    {
        return Err(input(
            "Expected native admitted exact-rational solid result",
        ));
    }
    let artifact = value
        .get("artifact")
        .ok_or_else(|| input("Missing exact-rational frozen artifact"))?;
    if value.get("model") != artifact.get("geometry") {
        return Err(input(
            "Exact semantic model differs from its frozen artifact",
        ));
    }
    brep_core::feature_ops::full_round_curved::admitted::transport::restore(artifact, 4 << 20)?;
    Ok(())
}
