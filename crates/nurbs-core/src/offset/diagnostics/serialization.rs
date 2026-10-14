//! Host encoding for native represented-chain diagnostics.
use super::*;
use value_codec::{Value, json};
impl Diagnostics {
    pub fn to_value(&self) -> Value {
        json!({"scope":"represented-offset-chain","method":"outward-line-pair-interval-exact/2",
            "crossings":self.crossings,"contacts":self.contacts,"uncertain":self.uncertain,
            "degenerate":self.degenerate,"complete":self.complete,"checks":self.checks,
            "enumerationComplete":self.enumeration_complete(),
            "predicatesComplete":self.predicates_complete(),
            "totalPairs":self.total_pairs,"simple":self.is_simple(),
            "originalOffsetTopologyCertified":false})
    }
}
impl value_codec::Serialize for Diagnostics {
    fn to_value(&self) -> Value {
        Diagnostics::to_value(self)
    }
}
