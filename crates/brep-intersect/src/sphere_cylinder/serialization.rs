//! Wire encoding for native geometry results.
use super::*;

impl value_codec::Serialize for CylinderPatchCurve {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({"patch":self.patch,"arcs":self.arcs})
    }
}

impl value_codec::Serialize for SphereCylinderComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Circle {
                curve,
                center,
                radius,
                normal,
                sphere_uv,
                cylinder_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"circle","curve":curve,"center":center,"radius":radius,
                "normal":normal,"sphereUv":sphere_uv,"cylinderUv":cylinder_uv,
                "maxSampleResidual":max_sample_residual}),
        }
    }
}
