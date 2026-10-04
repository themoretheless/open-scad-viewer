//! Existing host response shape for the native sweep diagnostic.
use super::{CheckedSweep, SweepReport};
use value_codec::{Serialize, Value, json};

impl Serialize for SweepReport {
    fn to_value(&self) -> Value {
        json!({
            "accepted": self.accepted,
            "sampledControlDeviation": self.sampled_control_deviation,
            "budget": self.budget,
            "stations": self.stations,
            "sections": self.sections,
            "continuousBound": false,
            "closedPath": self.closed_path,
            "seamContinuity": if self.closed_path { "C0" } else { "open" },
            "method": "double-reflection-fourfold-section-refinement"
        })
    }
}

impl Serialize for CheckedSweep {
    fn to_value(&self) -> Value {
        json!({"surface": self.surface, "report": self.report})
    }
}
