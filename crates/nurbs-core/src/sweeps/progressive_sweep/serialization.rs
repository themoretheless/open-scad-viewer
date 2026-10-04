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
            "continuousBound":self.continuous_bound,
            "roundingCertified":self.continuous_bound,
            "continuousErrorUpper":self.continuous_error_upper,
            "knownProfileErrorUpper":self.known_profile_error_upper,
            "errorCertificateCells":self.error_certificate_cells,
            "decompositionProducts":self.decomposition_products,
            "errorCertificateReason":self.error_certificate_reason,
            "continuousErrorScope":"retained-patches-relative-to-original-profile-transport",
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
