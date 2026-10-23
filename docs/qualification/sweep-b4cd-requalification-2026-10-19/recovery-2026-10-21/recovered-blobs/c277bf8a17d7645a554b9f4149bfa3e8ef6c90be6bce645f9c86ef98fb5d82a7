//! B-rep-specific admission of the SemanticProgram v1.2 execution plan.
//! Full envelope, provenance and capability admission is separate.
use super::{Result, Value, field};

fn invalid(message: &str) -> super::Error {
    super::Error::new("BREP_SEMANTIC_CONTRACT", message)
}

pub fn validate(plan: &Value, node_count: usize) -> Result<()> {
    let object = plan
        .as_object()
        .ok_or_else(|| invalid("Expected semantic execution plan"))?;
    let keys = ["version", "evaluationOrder", "discardedEffects", "terminal"];
    if object.len() != keys.len() || keys.iter().any(|key| !object.contains_key(*key)) {
        return Err(invalid(
            "Semantic execution plan fields do not match the contract",
        ));
    }
    if plan["version"].as_str() != Some("semantic-execution-v2") {
        return Err(invalid("Unknown semantic execution contract"));
    }
    if node_count > 25_000 {
        return Err(invalid("Semantic execution node limit exceeded"));
    }
    let order: Vec<usize> = field(plan, "evaluationOrder")?;
    if order.len() != node_count || order.iter().enumerate().any(|(index, node)| index != *node) {
        return Err(invalid(
            "Semantic execution order must contain every node in authored order",
        ));
    }
    if !plan["discardedEffects"]
        .as_array()
        .is_some_and(Vec::is_empty)
    {
        return Err(invalid("Discarded legacy effects are forbidden for brep-1"));
    }
    if !plan["terminal"].is_null() {
        return Err(invalid("Legacy terminal traces are forbidden for brep-1"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use value_codec::json;
    fn plan() -> Value {
        json!({"version":"semantic-execution-v2","evaluationOrder":[0,1],"discardedEffects":[],"terminal":null})
    }
    #[test]
    fn admits_exact_authored_order_and_empty_program() {
        validate(&plan(), 2).unwrap();
        let mut empty = plan();
        empty["evaluationOrder"] = json!([]);
        validate(&empty, 0).unwrap();
    }
    #[test]
    fn refuses_plan_changes_that_skip_duplicate_reorder_or_add_legacy_work() {
        for (key, value) in [
            ("evaluationOrder", json!([0])),
            ("evaluationOrder", json!([0, 0])),
            ("evaluationOrder", json!([1, 0])),
            ("evaluationOrder", json!([0, 2])),
            ("evaluationOrder", json!([0, 1.5])),
            ("version", json!("semantic-execution-v1")),
            (
                "discardedEffects",
                json!([{"tag":"legacy-difference-cutters"}]),
            ),
            ("terminal", json!({"tag":"legacy-language-error"})),
            ("extra", json!(true)),
        ] {
            let mut malformed = plan();
            malformed[key] = value;
            assert!(validate(&malformed, 2).is_err(), "accepted {key}");
        }
        assert!(validate(&Value::Null, 0).is_err());
        assert!(validate(&plan(), 25_001).is_err());
    }
}
