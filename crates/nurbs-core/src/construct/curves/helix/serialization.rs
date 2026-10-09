//! Helix wire encoding.
use super::*;
use value_codec::{Serialize, Value, json};
impl Serialize for Approximation {
    fn to_value(&self) -> Value {
        json!({"curve":self.curve,"report":{"spans":self.spans,"budget":self.budget,
            "realArithmeticErrorEstimate":self.real_arithmetic_error_estimate,
            "continuousBound":false,"roundingCertified":false,
            "method":"uniform-angle-cubic-Hermite-fourth-derivative-estimate"}})
    }
}
