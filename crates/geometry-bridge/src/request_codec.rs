use super::*;

/// Preserve the existing application wire contract for extracted mesh libraries.
pub(super) fn legacy_mesh_error(error: Error) -> Error {
    match error.code {
        "MESH_INVALID_INPUT"
        | "MESH_QUERY_INVALID_INPUT"
        | "MESH_SECTION_INVALID_INPUT"
        | "MESH_IO_INVALID_INPUT" => Error::new(
            "POLYGON_INVALID_INPUT",
            error
                .message
                .replace("the mesh resource budget", "the polygon resource budget"),
        ),
        _ => error,
    }
}
pub(super) fn error_json(error: &Error) -> Value {
    json!({"code": error.code, "message": error.message})
}
/// Parse the shared lumped/consistent mass-model selector.
pub(crate) fn mass_model(v: &Value) -> Result<mechanics_core::MassModel> {
    match v.as_str().unwrap_or("") {
        "lumped" => Ok(mechanics_core::MassModel::Lumped),
        "consistent" => Ok(mechanics_core::MassModel::Consistent),
        _ => Err(input("massModel must be lumped or consistent")),
    }
}
/// Serialize a singularity report; shared by the truss and frame diagnose ops.
pub(crate) fn diagnosis_json(d: &mechanics_core::diagnostics::SingularityDiagnosis) -> Value {
    json!({
        "stable": d.is_stable(),
        "minNormalizedPivot": d.min_normalized_pivot,
        "issues": d.issues.iter().map(|i| json!({
            "node": i.node,
            "dof": i.dof,
            "dofName": i.dof_name,
            "issue": match i.issue {
                mechanics_core::diagnostics::DofIssue::Unrestrained => "unrestrained",
                mechanics_core::diagnostics::DofIssue::Mechanism => "mechanism",
            },
        })).collect::<Vec<_>>(),
    })
}
pub(crate) fn mesh_from_triangles(t: geometry_ops::Triangles) -> Mesh {
    t.into()
}
pub(crate) fn triangles_from_mesh(m: &Mesh) -> geometry_ops::Triangles {
    geometry_ops::Triangles {
        positions: m.positions.clone(),
        indices: m.indices.clone(),
    }
}
pub(super) fn field<T: for<'a> Deserialize<'a>>(v: &Value, k: &str) -> Result<T> {
    value_codec::from_value(v[k].clone()).map_err(|e| input(format!("Invalid {k}: {e}")))
}
pub(super) fn optional_field<T: for<'a> Deserialize<'a>>(v: &Value, k: &str) -> Result<Option<T>> {
    match v.get(k) {
        Some(value) if !value.is_null() => field(v, k).map(Some),
        _ => Ok(None),
    }
}
/// Consume a single-use field from an owned request; preserve `field`'s missing-value errors.
pub(super) fn take_field<T: for<'a> Deserialize<'a>>(v: &mut Value, k: &str) -> Result<T> {
    let value = v
        .as_object_mut()
        .and_then(|object| object.remove(k))
        .unwrap_or(Value::Null);
    value_codec::from_value(value).map_err(|e| input(format!("Invalid {k}: {e}")))
}
pub(super) fn encode(v: impl Serialize) -> Result<Value> {
    value_codec::to_value(v).map_err(|e| input(e.to_string()))
}

pub fn execute(input_text: &str) -> String {
    if input_text.len() > 32 * 1024 * 1024 {
        return response(Err(input("Geometry request exceeds 32 MiB")));
    }
    response(
        value_codec::from_str(input_text)
            .map_err(|e| input(e.to_string()))
            .and_then(dispatch),
    )
}
