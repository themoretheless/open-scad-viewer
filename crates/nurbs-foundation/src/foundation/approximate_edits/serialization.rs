//! Compatibility reports for native approximate edits.
use super::*;
use value_codec::{Serialize, Value, json};
pub fn remove_curve_knot(
    source: &Curve,
    knot: f64,
    max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    Ok(remove_curve_knot_report(source, knot, max_error, tolerance)?.to_value())
}
pub fn rebuild_curve(
    source: &Curve,
    degree: usize,
    control_count: usize,
    max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    Ok(rebuild_curve_report(source, degree, control_count, max_error, tolerance)?.to_value())
}
pub fn reduce_curve_degree(
    source: &Curve,
    degree: usize,
    max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    Ok(reduce_curve_degree_report(source, degree, max_error, tolerance)?.to_value())
}
pub fn reduce_surface_axis(
    source: &Surface,
    axis: Axis,
    operation: &str,
    parameter: f64,
    degree: usize,
    max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    let operation = match operation {
        "remove" => SurfaceSimplification::Remove,
        "reduce" => SurfaceSimplification::Reduce,
        _ => return Err(crate::input("Unknown surface simplification operation")),
    };
    Ok(reduce_surface_axis_report(
        source, axis, operation, parameter, degree, max_error, tolerance,
    )?
    .to_value())
}
pub fn rebuild_surface(
    source: &Surface,
    axis: Axis,
    degree: usize,
    control_count: usize,
    max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    Ok(
        rebuild_surface_report(source, axis, degree, control_count, max_error, tolerance)?
            .to_value(),
    )
}
impl Serialize for CurveEdit {
    fn to_value(&self) -> Value {
        json!({"curve":self.curve,"certificate":self.certificate})
    }
}
impl Serialize for SurfaceEdit {
    fn to_value(&self) -> Value {
        json!({"surface":self.surface,"certificate":self.certificate})
    }
}
impl Serialize for EditCertificate {
    fn to_value(&self) -> Value {
        let (version, operation, method) = match self.operation {
            EditOperation::KnotRemoval => (
                "nurbs-foundation/3",
                "knot-removal",
                "inverse-insertion-with-adaptive-Lipschitz-envelope",
            ),
            EditOperation::CurveRebuild => (
                "nurbs-foundation/3",
                "curve-rebuild",
                "homogeneous-greville-refit-with-Lipschitz-envelope",
            ),
            EditOperation::DegreeReduction => (
                "nurbs-foundation/3",
                "degree-reduction",
                "inverse-elevation-with-adaptive-Lipschitz-envelope",
            ),
            EditOperation::SurfaceKnotRemoval => (
                "nurbs-foundation/2",
                "surface-knot-removal",
                "axiswise-refit-with-outward-global-hull",
            ),
            EditOperation::SurfaceDegreeReduction => (
                "nurbs-foundation/2",
                "surface-degree-reduction",
                "axiswise-refit-with-outward-global-hull",
            ),
            EditOperation::SurfaceRebuild => (
                "nurbs-foundation/3",
                "surface-rebuild",
                "outward-homogeneous-Bernstein-difference",
            ),
        };
        let mut result = json!({"version":version,"operation":operation,"accepted":self.accepted,"rolledBack":!self.accepted,"exactZeroRecognized":self.exact_zero_recognized,"hausdorffErrorUpper":self.error_upper,"budget":self.budget,"method":method,"evidence":super::super::tolerance_evidence(&self.tolerance)});
        if let Some(rebuild) = &self.rebuild {
            result["wrappedStorage"] = json!(rebuild.wrapped_storage);
            result["seam"] = json!(rebuild.seam);
        }
        result
    }
}
