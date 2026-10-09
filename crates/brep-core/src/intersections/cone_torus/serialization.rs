//! Wire encoding for native geometry results.
use super::*;

impl value_codec::Serialize for ConeTorusComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Circle {
                curve,
                center,
                radius,
                normal,
                cone_uv,
                torus_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"circle","curve":curve,"center":center,"radius":radius,
                "normal":normal,"coneUv":cone_uv,"torusUv":torus_uv,
                "maxSampleResidual":max_sample_residual}),
        }
    }
}
