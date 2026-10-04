use super::{Approximation, Level, LevelReport, MultiApproximation, MultiLevel};
use value_codec::{Serialize, Value, json};

impl Serialize for LevelReport {
    fn to_value(&self) -> Value {
        json!({
            "accepted":self.accepted,
            "sections":self.sections,
            "stations":self.stations,
            "sampledControlDeviation":self.sampled_control_deviation,
            "budget":self.budget,
            "closedPath":self.closed_path,
            "lengthResidualUpper":self.length_residual_upper,
            "continuousBound":false,
            "roundingCertified":false,
            "seamContinuity":if self.closed_path {"C0"} else {"open"},
            "method":"progressive-fourfold-section-refinement"
        })
    }
}

impl Serialize for Approximation {
    fn to_value(&self) -> Value {
        json!({"patches":self.patches,"report":self.levels.last(),"levels":self.levels})
    }
}

impl Serialize for MultiApproximation {
    fn to_value(&self) -> Value {
        json!({"patches":self.patches,"profilePatchRanges":self.profile_patch_ranges,
            "report":self.levels.last(),"levels":self.levels})
    }
}

impl Serialize for Level {
    fn to_value(&self) -> Value {
        json!({"patches":self.patches,"report":self.report,"preview":true})
    }
}
impl Serialize for MultiLevel {
    fn to_value(&self) -> Value {
        json!({"patches":self.patches,"profilePatchRanges":self.profile_patch_ranges,
            "report":self.report,"preview":true})
    }
}
