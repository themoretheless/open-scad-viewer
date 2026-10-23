//! Wire encoding for native geometry results.
use super::*;

impl value_codec::Serialize for ConeConeComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Circle {
                curve,
                center,
                radius,
                normal,
                first_uv,
                second_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"circle","curve":curve,"center":center,"radius":radius,
                "normal":normal,"firstUv":first_uv,"secondUv":second_uv,
                "maxSampleResidual":max_sample_residual}),
        }
    }
}
