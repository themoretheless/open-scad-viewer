//! Host encoding of native G1 endpoint matches.
use super::*;
use value_codec::{Serialize, Value, json};
pub fn checked(
    reference: &Curve,
    edited: &Curve,
    reference_end: &str,
    edited_end: &str,
    max_angle_degrees: f64,
) -> Result<Value> {
    Ok(checked_report(
        reference,
        edited,
        reference_end,
        edited_end,
        max_angle_degrees,
    )?
    .to_value())
}
impl Serialize for CurveMatch {
    fn to_value(&self) -> Value {
        json!({"curve":self.curve,"report":self.report})
    }
}
impl Serialize for CurveMatchReport {
    fn to_value(&self) -> Value {
        json!({"accepted":self.accepted,"positionErrorUpper":0.,"sineAngleUpper":self.sine_angle_upper,
        "angleDegreesUpper":self.angle_degrees_upper,"maxAngleDegrees":self.max_angle_degrees,
        "regularityCertified":self.regularity_certified,"orientationCertified":self.orientation_certified,
        "method":"outward-endpoint-handle-wedge","reason":match self.reason {
         CurveMatchReason::UnprovenEndpointRegularity=>"unproven-endpoint-regularity",
         CurveMatchReason::UnprovenTangentOrientation=>"unproven-tangent-orientation",
         CurveMatchReason::TangentErrorExceedsBudget=>"tangent-error-exceeds-budget",
         CurveMatchReason::Accepted=>"accepted",
        }})
    }
}
