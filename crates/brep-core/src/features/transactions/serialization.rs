//! Wire encoding for native geometry results.
use super::*;

impl value_codec::Serialize for AuthorizedHealResult {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({
            "model":self.model,
            "certificate":{
                "capability":self.capability,
                "status":self.status,
                "context":{
                    "version":self.context.version,
                    "canonical":self.context.canonical
                },
                "displacementLedger":self.displacement.iter().map(|entry| value_codec::json!({
                    "operation":entry.operation,
                    "actualMm":entry.actual_mm,
                    "cumulativeMm":entry.cumulative_mm
                })).collect::<Vec<_>>(),
                "cumulativeDisplacementMm":self.cumulative_displacement_mm,
                "sew":{
                    "matched":self.sew.matched,
                    "complete":self.sew.complete,
                    "displacementBudgetOk":self.sew.displacement_budget_ok
                },
                "audit":{
                    "ok":self.audit.ok,
                    "bodyCount":self.audit.body_count,
                    "shellCount":self.audit.shell_count,
                    "selfIntersectionPairsCandidate": self.audit.self_intersection_pairs_candidate,
                "selfIntersectionComplete": self.audit.self_intersection_complete,
                "selfIntersectionPairsChecked":self.audit.self_intersection_pairs_checked
                },
                "changeSet":self.change_set,
                "namingComplete":self.naming_complete
            }
        })
    }
}

/// Maximum encoded snapshot size; decoding checks this before JSON allocation.
pub const MAX_MODEL_SNAPSHOT_BYTES: usize = 8 * 1024 * 1024;
impl ModelSnapshot {
    /// Versioned full-model snapshot. Store identities, checkouts and history
    /// are intentionally session-local and are not serialized here.
    pub fn to_json(&self) -> Result<String> {
        use value_codec::Serialize;
        let value = value_codec::json!({"format":"brep-model-snapshot","schemaVersion":1,"model":self.model().to_value()});
        let encoded = value_codec::to_string(&value)
            .map_err(|e| nurbs_core::Error::new("BREP_SNAPSHOT_INVALID", e.to_string()))?;
        if encoded.len() > MAX_MODEL_SNAPSHOT_BYTES {
            return Err(nurbs_core::Error::new(
                "BREP_SNAPSHOT_RESOURCE_LIMIT",
                "Snapshot exceeds 8 MiB",
            ));
        }
        Ok(encoded)
    }
    /// Decode a strict snapshot envelope and re-run kernel validation. This is
    /// not the legacy import route: identity tables must be explicitly present.
    pub fn from_json(encoded: &str) -> Result<Self> {
        use value_codec::Deserialize;
        let invalid = |message: &str| nurbs_core::Error::new("BREP_SNAPSHOT_INVALID", message);
        if encoded.len() > MAX_MODEL_SNAPSHOT_BYTES {
            return Err(nurbs_core::Error::new(
                "BREP_SNAPSHOT_RESOURCE_LIMIT",
                "Snapshot exceeds 8 MiB",
            ));
        }
        let value: value_codec::Value =
            value_codec::from_str_strict(encoded).map_err(|e| invalid(&e.to_string()))?;
        let object = value
            .as_object()
            .ok_or_else(|| invalid("Expected snapshot object"))?;
        if object.len() != 3
            || object.get("format") != Some(&value_codec::json!("brep-model-snapshot"))
            || object.get("schemaVersion") != Some(&value_codec::json!(1))
        {
            return Err(invalid("Unknown snapshot format, version or fields"));
        }
        let model = object
            .get("model")
            .ok_or_else(|| invalid("Missing model"))?;
        if !model
            .as_object()
            .is_some_and(|m| m.contains_key("topologyIds"))
        {
            return Err(invalid("Snapshot requires topology identity tables"));
        }
        let model = Model::from_value(model.clone()).map_err(|e| invalid(&e.to_string()))?;
        Self::new(model)
    }
}

pub const MAX_MODEL_HISTORY_BYTES: usize = 32 * 1024 * 1024;
/// Encode current state and both history stacks. Pending transactions and store
/// identity are not portable; loading always creates a fresh revision-zero store.
pub fn history_to_json(store: &ModelStore) -> Result<String> {
    let invalid = |m: &str| nurbs_core::Error::new("BREP_HISTORY_INVALID", m);
    let mut bytes = 0usize;
    let mut encode = |snapshot: &ModelSnapshot| -> Result<value_codec::Value> {
        let encoded = snapshot.to_json()?;
        bytes = bytes.saturating_add(encoded.len());
        if bytes > MAX_MODEL_HISTORY_BYTES {
            return Err(nurbs_core::Error::new(
                "BREP_HISTORY_RESOURCE_LIMIT",
                "History exceeds 32 MiB",
            ));
        }
        value_codec::from_str(&encoded).map_err(|e| invalid(&e.to_string()))
    };
    let current = encode(store.snapshot())?;
    let (undo, redo) = store.history_snapshots();
    let undo = undo.iter().map(&mut encode).collect::<Result<Vec<_>>>()?;
    let redo = redo.iter().map(&mut encode).collect::<Result<Vec<_>>>()?;
    let envelope = value_codec::json!({"format":"brep-model-history","schemaVersion":1,"current":current,"undo":undo,"redo":redo});
    let encoded = value_codec::to_string(&envelope).map_err(|e| invalid(&e.to_string()))?;
    if encoded.len() > MAX_MODEL_HISTORY_BYTES {
        return Err(nurbs_core::Error::new(
            "BREP_HISTORY_RESOURCE_LIMIT",
            "History exceeds 32 MiB",
        ));
    }
    Ok(encoded)
}
pub fn history_from_json(encoded: &str) -> Result<ModelStore> {
    let invalid = |m: &str| nurbs_core::Error::new("BREP_HISTORY_INVALID", m);
    if encoded.len() > MAX_MODEL_HISTORY_BYTES {
        return Err(nurbs_core::Error::new(
            "BREP_HISTORY_RESOURCE_LIMIT",
            "History exceeds 32 MiB",
        ));
    }
    let value: value_codec::Value =
        value_codec::from_str_strict(encoded).map_err(|e| invalid(&e.to_string()))?;
    let object = value
        .as_object()
        .ok_or_else(|| invalid("Expected history object"))?;
    if object.len() != 5
        || object.get("format") != Some(&value_codec::json!("brep-model-history"))
        || object.get("schemaVersion") != Some(&value_codec::json!(1))
    {
        return Err(invalid("Unknown history format, version or fields"));
    }
    let undo = object
        .get("undo")
        .and_then(value_codec::Value::as_array)
        .ok_or_else(|| invalid("Missing undo stack"))?;
    let redo = object
        .get("redo")
        .and_then(value_codec::Value::as_array)
        .ok_or_else(|| invalid("Missing redo stack"))?;
    if undo.len().saturating_add(redo.len()) > brep_topology::MAX_TOPOLOGY_HISTORY {
        return Err(nurbs_core::Error::new(
            "BREP_HISTORY_RESOURCE_LIMIT",
            "History exceeds 32 snapshots",
        ));
    }
    let decode = |value: &value_codec::Value| -> Result<ModelSnapshot> {
        ModelSnapshot::from_json(
            &value_codec::to_string(value).map_err(|e| invalid(&e.to_string()))?,
        )
    };
    let current = decode(
        object
            .get("current")
            .ok_or_else(|| invalid("Missing current snapshot"))?,
    )?;
    let undo = undo.iter().map(decode).collect::<Result<Vec<_>>>()?;
    let redo = redo.iter().map(decode).collect::<Result<Vec<_>>>()?;
    ModelStore::from_history(current, undo, redo)
}
