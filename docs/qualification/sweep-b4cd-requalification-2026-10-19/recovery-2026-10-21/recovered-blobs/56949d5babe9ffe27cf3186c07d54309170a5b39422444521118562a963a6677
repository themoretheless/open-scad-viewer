//! Host adapters for native surface jet matching and its certificates.
use super::*;
use value_codec::{Serialize, Value, json};

pub fn match_surface_jets(
    reference: &Surface,
    edited: &Surface,
    reference_boundary: &str,
    edited_boundary: &str,
    order: usize,
    scale: f64,
) -> Result<Value> {
    Ok(match_surface_jets_report(
        reference,
        edited,
        reference_boundary,
        edited_boundary,
        order,
        scale,
    )?
    .to_value())
}
pub fn match_surface_jets_oriented(
    reference: &Surface,
    edited: &Surface,
    reference_boundary: &str,
    edited_boundary: &str,
    order: usize,
    scale: f64,
    reverse: bool,
) -> Result<Value> {
    Ok(match_surface_jets_oriented_report(
        reference,
        edited,
        reference_boundary,
        edited_boundary,
        order,
        scale,
        reverse,
    )?
    .to_value())
}
pub fn match_surface_jets_checked(
    reference: &Surface,
    edited: &Surface,
    reference_boundary: &str,
    edited_boundary: &str,
    order: usize,
    scale: f64,
    reverse: bool,
    max_error: f64,
) -> Result<Value> {
    Ok(match_surface_jets_checked_report(
        reference,
        edited,
        reference_boundary,
        edited_boundary,
        order,
        scale,
        reverse,
        max_error,
    )?
    .to_value())
}
impl Serialize for SeamRegularity {
    fn to_value(&self) -> Value {
        match self {
            Self::NonC1Basis {
                unresolved_intervals,
            } => {
                json!({"certified":false,"reason":"seam-basis-is-not-C1","unresolvedIntervals":unresolved_intervals})
            }
            Self::Bernstein {
                domain,
                spans,
                inspected_cells,
                accepted_cells,
                unresolved_count,
                unresolved_intervals,
                resource_limit_reached,
            } => json!({
    "certified":self.certified(),"method":"outward-Bernstein-normal-component-separation",
    "domain":domain,"spans":spans,"inspectedCells":inspected_cells,"acceptedCells":accepted_cells,
    "unresolvedCount":unresolved_count,"unresolvedIntervals":unresolved_intervals,
    "resourceLimitReached":resource_limit_reached,"maxDepth":12,"maxCells":4096}),
        }
    }
}
impl Serialize for SurfaceJetErrorBounds {
    fn to_value(&self) -> Value {
        json!({"wholeSeam":true,"normalizedParameters":true,"method":"outward-homogeneous-jet-difference-hull",
   "positionUpper":self.position_upper,"firstDerivativeUpper":self.first_derivative_upper,
   "secondDerivativeUpper":self.second_derivative_upper,"mixedDerivativeUpper":self.mixed_derivative_upper})
    }
}
impl Serialize for SurfaceJetMatch {
    fn to_value(&self) -> Value {
        json!({"surface":self.surface,"report":self.report})
    }
}
impl Serialize for SurfaceJetReport {
    fn to_value(&self) -> Value {
        let mut report = json!({"continuityOrder":self.continuity_order,"normalScale":self.normal_scale,
   "method":"homogeneous-normalized-boundary-jets","regularityCertified":self.regularity_certified(),
   "referenceRegularity":self.reference_regularity,"editedRegularity":self.edited_regularity,
   "errorBounds":self.error_bounds,"seamBasis":"exact-affine-knot-correspondence","modifiedLayers":self.continuity_order+1});
        if self.reversed {
            report["reversed"] = json!(true);
        }
        if let Some(decision) = &self.decision {
            report["accepted"] = json!(decision.accepted);
            report["maxError"] = json!(decision.max_error);
            report["errorUpper"] = json!(decision.error_upper);
            report["tangentialSmoothnessCertified"] =
                json!(decision.tangential_smoothness_certified);
            report["reason"] = json!(match decision.reason {
                SurfaceJetDecisionReason::UnprovenRegularity => "unproven-regularity",
                SurfaceJetDecisionReason::UnprovenTangentialSmoothness =>
                    "unproven-tangential-smoothness",
                SurfaceJetDecisionReason::DeviationExceedsBudget => "deviation-exceeds-budget",
                SurfaceJetDecisionReason::Accepted => "accepted",
            });
        }
        report
    }
}
