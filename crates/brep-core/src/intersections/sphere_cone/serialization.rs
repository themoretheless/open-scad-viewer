//! Wire encoding for native geometry results.
use super::*;

impl value_codec::Serialize for SphereConeComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Circle {
                curve,
                center,
                radius,
                normal,
                sphere_uv,
                cone_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"circle","curve":curve,"center":center,"radius":radius,
                "normal":normal,"sphereUv":sphere_uv,"coneUv":cone_uv,
                "maxSampleResidual":max_sample_residual}),
        }
    }
}
