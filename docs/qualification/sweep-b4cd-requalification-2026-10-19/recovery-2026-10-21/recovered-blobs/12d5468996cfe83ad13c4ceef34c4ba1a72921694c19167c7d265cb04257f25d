//! Wire encoding for native geometry results.
use super::*;

impl value_codec::Serialize for PlaneCylinderComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Circle {
                curve,
                center,
                radius,
                normal,
                full,
                plane_uv,
                cylinder_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"circle","curve":curve,"center":center,"radius":radius,
                "normal":normal,"full":full,"planeUv":plane_uv,"cylinderUv":cylinder_uv,
                "maxSampleResidual":max_sample_residual}),
            Self::Line {
                curve,
                start,
                end,
                direction,
                contact,
                plane_uv,
                cylinder_uv,
                max_sample_residual,
            } => {
                let contact = match contact {
                    Contact::Transverse => "transverse",
                    Contact::Boundary => "boundary",
                };
                value_codec::json!({"kind":"line","curve":curve,"start":start,"end":end,
                    "direction":direction,"contact":contact,"planeUv":plane_uv,
                    "cylinderUv":cylinder_uv,"maxSampleResidual":max_sample_residual})
            }
            Self::Ellipse {
                curve,
                center,
                semi_major,
                semi_minor,
                major,
                minor,
                normal,
                full,
                plane_uv,
                cylinder_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"ellipse","curve":curve,"center":center,
                "semiMajor":semi_major,"semiMinor":semi_minor,"major":major,"minor":minor,
                "normal":normal,"full":full,"planeUv":plane_uv,"cylinderUv":cylinder_uv,
                "maxSampleResidual":max_sample_residual}),
        }
    }
}
