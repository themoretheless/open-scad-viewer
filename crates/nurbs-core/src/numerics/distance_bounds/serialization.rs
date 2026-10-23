//! Host labels for native distance termination reasons.
use super::DistanceStopReason;
impl value_codec::Serialize for DistanceStopReason {
    fn to_value(&self) -> value_codec::Value {
        value_codec::Serialize::to_value(match self {
            Self::Tolerance => "tolerance",
            Self::Separated => "separated",
            Self::WorkLimit => "work-limit",
            Self::PrecisionLimit => "precision-limit",
            Self::EmptyDomain => "empty-domain",
            Self::DomainWorkLimit => "domain-work-limit",
        })
    }
}
