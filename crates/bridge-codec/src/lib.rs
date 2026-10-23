//! JSON request/response helpers shared by the geometry bridge domain crates.

pub use math_core::{Error, Result};
pub use value_codec::{Deserialize, Serialize, Value, json};

/// Typed invalid-input error used by every bridge operation.
pub fn input(message: impl Into<String>) -> Error {
    Error::new("GEOMETRY_INVALID_INPUT", message)
}

pub fn error_json(error: &Error) -> Value {
    json!({"code": error.code, "message": error.message})
}

/// Wire envelope: `{"ok":true,"value":…}` or `{"ok":false,"error":…}`.
pub fn response(result: Result<Value>) -> String {
    match result {
        Ok(value) => json!({"ok":true,"value":value}),
        Err(error) => json!({"ok":false,"error":error_json(&error)}),
    }
    .to_string()
}

pub fn field<T: for<'a> Deserialize<'a>>(v: &Value, k: &str) -> Result<T> {
    value_codec::from_value(v[k].clone()).map_err(|e| input(format!("Invalid {k}: {e}")))
}

/// Consume a single-use field from an owned request; preserve `field`'s missing-value errors.
pub fn take_field<T: for<'a> Deserialize<'a>>(v: &mut Value, k: &str) -> Result<T> {
    let value = v
        .as_object_mut()
        .and_then(|object| object.remove(k))
        .unwrap_or(Value::Null);
    value_codec::from_value(value).map_err(|e| input(format!("Invalid {k}: {e}")))
}

pub fn encode(v: impl Serialize) -> Result<Value> {
    value_codec::to_value(v).map_err(|e| input(e.to_string()))
}

pub fn require_exact_fields(value: &Value, expected: &[&str], label: &str) -> Result<()> {
    let object = value
        .as_object()
        .ok_or_else(|| input(format!("{label} must be an object")))?;
    if object.len() != expected.len() || !expected.iter().all(|field| object.contains_key(*field)) {
        return Err(input(format!(
            "{label} contains missing or unauthorized fields"
        )));
    }
    Ok(())
}

/// The request's `op` string, or `""`.
pub fn op(v: &Value) -> &str {
    v["op"].as_str().unwrap_or("")
}

/// Outcome of offering a request to one domain router. An unhandled request
/// is handed back by value, so routing never copies the payload.
pub enum Routed {
    Handled(Result<Value>),
    Unhandled(Value),
}

/// A domain router: handles the operations it owns, returns the rest.
pub type Router = fn(Value) -> Routed;
