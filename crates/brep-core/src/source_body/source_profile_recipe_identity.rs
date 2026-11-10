//! Exact JSON numeric identity across JavaScript integer transport.
use value_codec::{Number, Value};
pub(super) fn same_recipe(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => {
            if let (Number::Float(a), Number::Float(b)) = (a, b) {
                return a.is_finite() && b.is_finite() && a.to_bits() == b.to_bits();
            }
            if a == b {
                return true;
            }
            let exact_integer = |n: &Number| -> Option<f64> {
                match n {
                    Number::Unsigned(v) if *v <= 9_007_199_254_740_991 => Some(*v as f64),
                    Number::Signed(v) if v.unsigned_abs() <= 9_007_199_254_740_991 => {
                        Some(*v as f64)
                    }
                    _ => None,
                }
            };
            match (a, b) {
                (Number::Float(f), n) | (n, Number::Float(f)) => {
                    exact_integer(n).is_some_and(|v| f.is_finite() && f.to_bits() == v.to_bits())
                }
                (Number::Signed(s), Number::Unsigned(u))
                | (Number::Unsigned(u), Number::Signed(s)) => *s >= 0 && *s as u64 == *u,
                _ => false,
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| same_recipe(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(k, v)| b.get(k).is_some_and(|w| same_recipe(v, w)))
        }
        _ => a == b,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_numbers_preserve_identity_without_rounding() {
        assert!(same_recipe(
            &Value::Number(Number::Float(5.)),
            &Value::Number(Number::Unsigned(5))
        ));
        assert!(!same_recipe(
            &Value::Number(Number::Float(-0.)),
            &Value::Number(Number::Unsigned(0))
        ));
        assert!(!same_recipe(
            &Value::Number(Number::Float(-0.)),
            &Value::Number(Number::Float(0.))
        ));
        assert!(!same_recipe(
            &Value::Number(Number::Float(f64::INFINITY)),
            &Value::Number(Number::Float(f64::INFINITY))
        ));
        assert!(!same_recipe(
            &Value::Number(Number::Float(9_007_199_254_740_992.)),
            &Value::Number(Number::Unsigned(9_007_199_254_740_993))
        ));
        assert!(!same_recipe(
            &Value::Number(Number::Float(5.000000000000001)),
            &Value::Number(Number::Unsigned(5))
        ));
    }
}
