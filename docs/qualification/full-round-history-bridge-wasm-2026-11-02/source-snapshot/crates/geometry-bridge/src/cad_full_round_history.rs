//! Stateless transport of native proof-owned feature history. No host history
//! algorithm or caller-supplied credit/certificate is authoritative.
use crate::{Result, Value, field, input};
use brep_core::feature_ops::full_round_curved::admitted::history::{Budget, codec};
use brep_core::profile_certificate::original_restriction::wire::material::Limits;
use value_codec::json;
fn allowed(v: &Value, keys: &[&str]) -> Result<()> {
    let map = v
        .as_object()
        .ok_or_else(|| input("Expected exact history request object"))?;
    if map.keys().any(|key| !keys.contains(&key.as_str())) {
        return Err(input("Unknown exact history request field"));
    }
    Ok(())
}
fn optional<T: for<'a> value_codec::Deserialize<'a>>(v: &Value, key: &str) -> Result<Option<T>> {
    v.get(key).map(|_| field(v, key)).transpose()
}
fn budget(v: &Value) -> Result<Budget> {
    let Some(b) = v.get("budget") else {
        return Ok(Budget::default());
    };
    allowed(b, &["maxRevisions", "maxBytes", "material"])?;
    let d = Budget::default();
    let material = if let Some(m) = b.get("material") {
        crate::cad_full_round_exact::limits(&json!({"limits":m}))?
    } else {
        Limits::default()
    };
    Ok(Budget {
        max_revisions: optional(b, "maxRevisions")?.unwrap_or(d.max_revisions),
        max_bytes: optional(b, "maxBytes")?.unwrap_or(d.max_bytes),
        material,
    })
}
pub(crate) fn dispatch(v: Value) -> Result<Value> {
    let action: String = field(&v, "action")?;
    let extra: &[&str] = match action.as_str() {
        "create" => &["artifact"],
        "restore" | "undo" | "redo" | "branch" => &["history"],
        "rebase-source" => &["history", "source"],
        "reselect" => &["history", "faceIds"],
        _ => return Err(input("Unknown exact history action")),
    };
    let mut keys = vec!["op", "action", "budget"];
    keys.extend(extra);
    allowed(&v, &keys)?;
    let budget = budget(&v)?;
    // Creation uses the SAME bounded replay owner, so requested proof limits
    // are enforced before reconstructing an untrusted artifact.
    let created;
    let document = if action == "create" {
        let artifact = v
            .get("artifact")
            .ok_or_else(|| input("Missing exact history root artifact"))?;
        created = json!({"format":"exact-full-round-history/1","cursor":0,"revisions":[{"edit":"root","artifact":artifact}]});
        &created
    } else {
        v.get("history")
            .ok_or_else(|| input("Missing exact history document"))?
    };
    let mut history = codec::restore(document, budget)?;
    match action.as_str() {
        "rebase-source" => {
            history.rebase_source(&field(&v, "source")?)?;
        }
        "reselect" => {
            history.reselect(field(&v, "faceIds")?)?;
        }
        "undo" => {
            history.undo()?;
        }
        "redo" => {
            history.redo()?;
        }
        "branch" => {
            history = history.branch()?;
        }
        _ => {}
    }
    let work = history.work();
    let reply = json!({"document":codec::export(&history)?,"cursor":history.cursor(),"revisionCount":history.revisions().len(),
        "selectedSourceFaceIds":history.selected_source_ids(),
        "work":{"pairProjections":work.pair_projections,"atoms":work.atoms,"rayCells":work.ray_cells}});
    if value_codec::encode_binary(&reply)
        .map_err(|e| input(e.to_string()))?
        .len()
        > budget.max_bytes
    {
        return Err(input("Exact history reply exceeds byte allowance"));
    }
    Ok(reply)
}
