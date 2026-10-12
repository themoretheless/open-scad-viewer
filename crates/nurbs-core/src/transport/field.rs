use super::*;

pub(super) fn field<T: for<'a> Deserialize<'a>>(v: &Value, k: &str) -> Result<T> {
    value_codec::from_value(v[k].clone()).map_err(|e| input(format!("Invalid {k}: {e}")))
}
pub(super) fn encode<T: Serialize>(v: T) -> Result<Value> {
    value_codec::to_value(v).map_err(|e| numeric_err(e.to_string()))
}
pub(super) fn optional_field<T: for<'a> Deserialize<'a>>(v: &Value, k: &str) -> Result<Option<T>> {
    match v.get(k) {
        Some(value) if !value.is_null() => value_codec::from_value(value.clone())
            .map(Some)
            .map_err(|e| input(format!("Invalid {k}: {e}"))),
        _ => Ok(None),
    }
}
pub(super) fn control_tangents(v: &Value) -> Result<Option<[Vec<[f64; 3]>; 2]>> {
    let start = optional_field(v, "start_tangents")?;
    let end = optional_field(v, "end_tangents")?;
    let controls = match (start, end) {
        (None, None) => None,
        (Some(a), Some(b)) => Some([a, b]),
        _ => return Err(input("Cartesian loft requires both endpoint tangent fields")),
    };
    check(controls.is_none() || v.get("boundary_tangents").is_none_or(Value::is_null),
        "Specify authored control tangents or normalized boundary fields, not both")?;
    Ok(controls)
}
