//! Wire encoding for native geometry results.
use super::*;

impl value_codec::Serialize for CylinderCylinderComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Line {
                curve,
                start,
                end,
                direction,
                contact,
                first_uv,
                second_uv,
                max_sample_residual,
            } => {
                let contact = match contact {
                    Contact::Transverse => "transverse",
                    Contact::Boundary => "boundary",
                };
                value_codec::json!({"kind":"line","curve":curve,"start":start,"end":end,
                    "direction":direction,"contact":contact,"firstUv":first_uv,"secondUv":second_uv,
                    "maxSampleResidual":max_sample_residual})
            }
        }
    }
}
