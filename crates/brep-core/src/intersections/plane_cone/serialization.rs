//! Wire encoding for native geometry results.
use super::*;

impl value_codec::Serialize for PlaneConeComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Circle {
                curve,
                center,
                radius,
                normal,
                full,
                plane_uv,
                cone_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"circle","curve":curve,"center":center,"radius":radius,
                "normal":normal,"full":full,"planeUv":plane_uv,"coneUv":cone_uv,
                "maxSampleResidual":max_sample_residual}),
            Self::Line {
                curve,
                start,
                end,
                direction,
                contact,
                plane_uv,
                cone_uv,
                max_sample_residual,
            } => {
                let contact = match contact {
                    Contact::Transverse => "transverse",
                    Contact::Boundary => "boundary",
                };
                value_codec::json!({"kind":"line","curve":curve,"start":start,"end":end,
                    "direction":direction,"contact":contact,"planeUv":plane_uv,
                    "coneUv":cone_uv,"maxSampleResidual":max_sample_residual})
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
                cone_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"ellipse","curve":curve,"center":center,
                "semiMajor":semi_major,"semiMinor":semi_minor,"major":major,"minor":minor,
                "normal":normal,"full":full,"planeUv":plane_uv,"coneUv":cone_uv,
                "maxSampleResidual":max_sample_residual}),
            Self::Parabola {
                curve,
                vertex,
                direction,
                focal_length,
                normal,
                plane_uv,
                cone_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"parabola","curve":curve,"vertex":vertex,
                "direction":direction,"focalLength":focal_length,"normal":normal,
                "planeUv":plane_uv,"coneUv":cone_uv,"maxSampleResidual":max_sample_residual}),
            Self::Hyperbola {
                curve,
                center,
                semi_transverse,
                semi_conjugate,
                transverse,
                conjugate,
                normal,
                plane_uv,
                cone_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"hyperbola","curve":curve,"center":center,
                "semiTransverse":semi_transverse,"semiConjugate":semi_conjugate,
                "transverse":transverse,"conjugate":conjugate,"normal":normal,
                "planeUv":plane_uv,"coneUv":cone_uv,"maxSampleResidual":max_sample_residual}),
        }
    }
}
