//! Host adapters for native seam preparation and its acceptance certificate.
use super::*;
use value_codec::{Serialize, Value, json};
pub fn certify(
    reference: &Surface,
    edited: &Surface,
    reference_boundary: &str,
    edited_boundary: &str,
    candidate: &PreparedSeams,
    budget: f64,
) -> Result<Value> {
    Ok(certify_report(
        reference,
        edited,
        reference_boundary,
        edited_boundary,
        candidate,
        budget,
    )?
    .to_value())
}
pub fn checked(
    reference: &Surface,
    edited: &Surface,
    reference_boundary: &str,
    edited_boundary: &str,
    reverse: bool,
    budget: f64,
) -> Result<Value> {
    Ok(checked_report(
        reference,
        edited,
        reference_boundary,
        edited_boundary,
        reverse,
        budget,
    )?
    .to_value())
}
pub fn checked_with_conversion(
    reference: &Surface,
    edited: &Surface,
    reference_boundary: &str,
    edited_boundary: &str,
    reverse: bool,
    budget: f64,
    open_periodic: bool,
) -> Result<Value> {
    Ok(checked_with_conversion_report(
        reference,
        edited,
        reference_boundary,
        edited_boundary,
        reverse,
        budget,
        open_periodic,
    )?
    .to_value())
}
impl Serialize for PreparationCertificate {
    fn to_value(&self) -> Value {
        let reason = match self.reason {
            PreparationReason::UnprovenParameterNormalization => "unproven-parameter-normalization",
            PreparationReason::DeviationExceedsBudget => "deviation-exceeds-budget",
            PreparationReason::Accepted => "accepted",
        };
        let mut proof = json!({"accepted":self.accepted,"reason":reason,"budget":self.budget});
        if let Some(errors) = self.errors {
            proof["referenceErrorUpper"] = json!(errors[0]);
            proof["editedErrorUpper"] = json!(errors[1]);
            proof["wholeSurface"] = json!(true);
            proof["method"] = json!("outward-homogeneous-Bernstein-difference");
        }
        proof
    }
}
impl Serialize for PreparedBasis {
    fn to_value(&self) -> Value {
        json!({"degree":self.degree,"controlCount":self.control_count,"normalizedSeam":true,
   "editedReversed":self.edited_reversed,"referenceDomain":self.reference_domain,"editedDomain":self.edited_domain,
   "periodicityRemoved":{"reference":self.periodicity_removed[0],"edited":self.periodicity_removed[1]}})
    }
}
impl Serialize for CheckedPreparation {
    fn to_value(&self) -> Value {
        json!({"reference":self.reference,"edited":self.edited,"report":self.report,"basis":self.basis})
    }
}
