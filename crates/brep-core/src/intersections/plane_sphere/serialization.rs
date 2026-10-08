//! Wire encoding for native geometry results.
use super::*;

impl value_codec::Serialize for PlanePatchCurve {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({"patch":self.patch,"arcs":self.arcs})
    }
}

impl value_codec::Serialize for PlaneSphereComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Circle {
                curve,
                center,
                radius,
                normal,
                full,
                plane_uv,
                sphere_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"circle","curve":curve,"center":center,"radius":radius,
                "normal":normal,"full":full,"planeUv":plane_uv,"sphereUv":sphere_uv,
                "maxSampleResidual":max_sample_residual}),
        }
    }
}
