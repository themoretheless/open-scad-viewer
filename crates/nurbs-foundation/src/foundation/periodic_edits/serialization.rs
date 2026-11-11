//! Compatibility adapters for native periodic edits.
use super::*;
use value_codec::{Serialize, Value, json};
fn parse_operation(name: &str) -> Result<PeriodicEditOperation> {
    match name {
        "remove" => Ok(PeriodicEditOperation::Remove),
        "insert" => Ok(PeriodicEditOperation::Insert),
        "elevate" => Ok(PeriodicEditOperation::Elevate),
        "reduce" => Ok(PeriodicEditOperation::Reduce),
        _ => Err(crate::input("Unknown periodic edit")),
    }
}
fn operation_name(op: PeriodicEditOperation) -> &'static str {
    match op {
        PeriodicEditOperation::Remove => "remove",
        PeriodicEditOperation::Insert => "insert",
        PeriodicEditOperation::Elevate => "elevate",
        PeriodicEditOperation::Reduce => "reduce",
    }
}
pub fn edit_periodic_curve(
    source: &Curve,
    operation: &str,
    parameter: f64,
    degree: usize,
    max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    Ok(edit_periodic_curve_report(
        source,
        parse_operation(operation)?,
        parameter,
        degree,
        max_error,
        tolerance,
    )?
    .to_value())
}
pub fn edit_periodic_surface(
    source: &Surface,
    axis: Axis,
    operation: &str,
    parameter: f64,
    degree: usize,
    max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    Ok(edit_periodic_surface_report(
        source,
        axis,
        parse_operation(operation)?,
        parameter,
        degree,
        max_error,
        tolerance,
    )?
    .to_value())
}
pub fn split_periodic_curve(
    source: &Curve,
    parameter: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    Ok(split_periodic_curve_report(source, parameter, tolerance)?.to_value())
}
impl Serialize for SeamContinuity {
    fn to_value(&self) -> Value {
        json!({"available":self.residual_upper.is_some(),"residualUpper":self.residual_upper,"certified":self.certified})
    }
}
impl Serialize for SeamCertificate {
    fn to_value(&self) -> Value {
        json!({"domain":self.domain,"period":self.period,"c0":self.c0,"c1":self.c1,"c2":self.c2})
    }
}
impl Serialize for PeriodicCurveEdit {
    fn to_value(&self) -> Value {
        let e = &self.evidence;
        json!({"curve":self.curve,"certificate":{"version":"nurbs-foundation/3","operation":operation_name(e.operation),"accepted":e.accepted,"rolledBack":!e.accepted,"hausdorffErrorUpper":e.error_upper,"budget":e.budget,"wrappedStorage":true,"seam":self.seam,"method":"cyclic-collocation-with-global-outward-hull","evidence":super::super::tolerance_evidence(&e.tolerance)}})
    }
}
impl Serialize for PeriodicSurfaceEdit {
    fn to_value(&self) -> Value {
        let e = &self.evidence;
        json!({"surface":self.surface,"certificate":{"version":"nurbs-foundation/3","operation":format!("periodic-surface-{}",operation_name(e.operation)),"axis":match self.axis {Axis::U=>"u",Axis::V=>"v"},"accepted":e.accepted,"rolledBack":!e.accepted,"hausdorffErrorUpper":e.error_upper,"budget":e.budget,"wrappedStorage":true,"representativeSeam":self.seam,"conditioning":{"sampledTransverseLines":self.sampled_transverse_lines},"evidence":super::super::tolerance_evidence(&e.tolerance)}})
    }
}
impl Serialize for PeriodicCurveSplit {
    fn to_value(&self) -> Value {
        json!({"curves":self.curves,"certificate":{"version":"nurbs-foundation/3","operation":"periodic-seam-split","sourceSeam":self.seam,"coverage":self.coverage,"orientationPreserved":true,"evidence":super::super::tolerance_evidence(&self.tolerance)}})
    }
}
