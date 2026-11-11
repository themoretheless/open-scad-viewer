//! Compatibility encoding for native exact edits.
use super::*;
use value_codec::{Serialize, Value, json};
pub fn reparameterize_curve(
    curve: &Curve,
    domain: [f64; 2],
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    Ok(reparameterize_curve_report(curve, domain, tolerance)?.to_value())
}
pub fn certify_exact_edit(
    source: &Curve,
    operation: &str,
    parameter: f64,
    count: usize,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    let operation = match operation {
        "insert" => ExactEditOperation::Insert,
        "elevate" => ExactEditOperation::Elevate,
        _ => return Err(crate::input("Unknown exact certified edit")),
    };
    Ok(certify_exact_edit_report(source, operation, parameter, count, tolerance)?.to_value())
}
impl Serialize for ExactCurveEdit {
    fn to_value(&self) -> Value {
        json!({"curve":self.curve,"certificate":{"version":"nurbs-foundation/1",
 "operation":match self.operation {ExactEditOperation::Insert=>"insert",ExactEditOperation::Elevate=>"elevate"},
 "geometryErrorUpper":0.,"periodPreserved":self.period_preserved,"evidence":super::super::tolerance_evidence(&self.tolerance)}})
    }
}
impl Serialize for AffineReparameterization {
    fn to_value(&self) -> Value {
        json!({"curve":self.curve,"certificate":{"version":"nurbs-foundation/1",
 "mapping":{"oldDomain":self.old_domain,"newDomain":self.new_domain,"scale":self.scale},
 "monotone":true,"geometryErrorUpper":0.,"periodPreserved":self.period_preserved,"evidence":super::super::tolerance_evidence(&self.tolerance)}})
    }
}
