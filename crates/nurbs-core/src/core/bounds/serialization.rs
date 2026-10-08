//! Wire encoding, separate from native calculations.
use super::*;

impl value_codec::Serialize for Bounds {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({"min": self.min, "max": self.max})
    }
}
