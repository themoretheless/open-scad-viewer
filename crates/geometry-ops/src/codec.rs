//! Small helpers over `value_codec` maps so every manual `Deserialize` in this
//! crate reads fields the same way.
use value_codec::{Deserialize, Value, error};

pub(crate) type Object = value_codec::Map<String, Value>;

/// Clones the object out of `value`, failing on any other shape.
pub(crate) fn object(value: Value) -> value_codec::Result<Object> {
    value
        .as_object()
        .cloned()
        .ok_or_else(|| error("Expected object"))
}

/// Removes and decodes a mandatory field.
pub(crate) fn required<'de, T: Deserialize<'de>>(
    object: &mut Object,
    name: &str,
) -> value_codec::Result<T> {
    object
        .remove(name)
        .ok_or_else(|| error(format!("Missing field {name}")))
        .and_then(T::from_value)
}

/// Removes and decodes a field, using `default` when it is absent or `null`.
pub(crate) fn optional<'de, T: Deserialize<'de>>(
    object: &mut Object,
    name: &str,
    default: T,
) -> value_codec::Result<T> {
    match object.remove(name) {
        None | Some(Value::Null) => Ok(default),
        Some(v) => T::from_value(v),
    }
}

pub(crate) fn encode_number(number: f64) -> Value {
    if number.is_finite() {
        value_codec::json!(number)
    } else {
        value_codec::json!(if number.is_nan() {
            "NaN"
        } else if number > 0. {
            "Infinity"
        } else {
            "-Infinity"
        })
    }
}
pub(crate) fn decode_number(value: &Value) -> value_codec::Result<f64> {
    value
        .as_f64()
        .or_else(|| match value.as_str()? {
            "NaN" => Some(f64::NAN),
            "Infinity" => Some(f64::INFINITY),
            "-Infinity" => Some(f64::NEG_INFINITY),
            _ => None,
        })
        .ok_or_else(|| value_codec::error("Invalid fragment number"))
}
