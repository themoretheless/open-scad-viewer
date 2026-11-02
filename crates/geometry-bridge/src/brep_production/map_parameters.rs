//! Canonical common-parameter equality for native map production replay.
use crate::{Result, Value, semantic_contract_error as invalid};
pub(super) fn derive(node: &Value, kind: &str) -> Result<String> {
    Ok(match kind {
        "transform" => {
            let mut out = String::new();
            super::super::brep_identity::stable_json(&node["matrix"], &mut out);
            out
        }
        "projection" => match &node["cut"] {
            Value::Bool(value) => value.to_string(),
            Value::Number(number) => super::super::brep_identity::js_number(number),
            _ => return Err(invalid("Semantic projection node is missing its cut")),
        },
        "offset" => match &node["distance"] {
            Value::Number(number) => super::super::brep_identity::js_number(number),
            _ => return Err(invalid("Semantic offset node is missing its distance")),
        },
        "full-round-exact" => {
            let faces = node["faces"]
                .as_array()
                .filter(|faces| faces.len() == 3)
                .ok_or_else(|| invalid("Exact Full Round requires three face indices"))?;
            if faces.iter().any(|face| face.as_u64().is_none()) {
                return Err(invalid(
                    "Exact Full Round face indices must be nonnegative integers",
                ));
            }
            let mut parameters = value_codec::json!({"faces":faces});
            for key in ["limits", "maxBytes"] {
                if let Some(value) = node.get(key) {
                    parameters[key] = value.clone();
                }
            }
            let mut out = String::new();
            super::super::brep_identity::stable_json(&parameters, &mut out);
            out
        }
        _ => return Err(invalid("Unexpected semantic map node kind")),
    })
}
