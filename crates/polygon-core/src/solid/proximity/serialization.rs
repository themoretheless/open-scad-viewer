//! Optional wire-format implementations, separate from native algorithms.
use super::*;

impl value_codec::Serialize for Deviation {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "sampledMaxMm".into(),
            value_codec::Serialize::to_value(&self.sampled_max_mm),
        );
        object.insert(
            "sampledRmsMm".into(),
            value_codec::Serialize::to_value(&self.sampled_rms_mm),
        );
        object.insert(
            "sampleCount".into(),
            value_codec::Serialize::to_value(&self.sample_count),
        );
        object.insert(
            "errorBoundCertified".into(),
            value_codec::Serialize::to_value(&self.error_bound_certified),
        );
        value_codec::Value::Object(object)
    }
}
