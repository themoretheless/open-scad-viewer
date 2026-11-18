//! Compatibility encoding for native interpolation and approximation.
use super::*;
use value_codec::{Serialize, Value, json};
pub fn interpolate_polyline(
    points: Vec<Vec<f64>>,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    Ok(interpolate_polyline_report(points, tolerance)?.to_value())
}
pub fn approximate_curve(curve: &Curve, tolerance: Option<ToleranceContext>) -> Result<Value> {
    Ok(approximate_curve_report(curve, tolerance)?.to_value())
}
impl Serialize for PolylineInterpolation {
    fn to_value(&self) -> Value {
        json!({"curve":self.curve,"certificate":{"version":"nurbs-foundation/1","method":"piecewise-linear-interpolation","dataSiteErrorUpper":0.,"evidence":super::super::tolerance_evidence(&self.tolerance)}})
    }
}
impl Serialize for CurveApproximation {
    fn to_value(&self) -> Value {
        json!({"curve":self.curve,"certificate":{"version":"nurbs-foundation/1","method":"per-span-convex-hull-diameter","hausdorffErrorUpper":self.hausdorff_error_upper,"withinEntityTolerance":self.within_entity_tolerance,"evidence":super::super::tolerance_evidence(&self.tolerance)}})
    }
}
