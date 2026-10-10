//! Wire encoding for native geometry results.
use super::*;

impl value_codec::Serialize for SphereTorusComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Circle {
                curve,
                center,
                radius,
                normal,
                sphere_uv,
                torus_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"circle","curve":curve,"center":center,"radius":radius,
                "normal":normal,"sphereUv":sphere_uv,"torusUv":torus_uv,
                "maxSampleResidual":max_sample_residual}),
        }
    }
}
