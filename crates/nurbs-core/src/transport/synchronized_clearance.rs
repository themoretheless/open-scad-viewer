use super::*;
use crate::analysis::curve_measure::{
    measure_synchronized_curve_clearance, measure_affine_synchronized_curve_clearance,
    partition_synchronized_curve_clearance, partition_affine_synchronized_curve_clearance,
};

pub(super) fn dispatch(v: &Value, op: &str) -> Result<Value> {
    let [a,b]: [crate::curve::Curve; 2] = field(v, "curves")?;
    let curves = [&a, &b];
    let radii = field(v, "radii_mm")?;
    let budget = field(v, "max_cells")?;
    if op == "synchronized_contact_intervals" {
        let report = partition_synchronized_curve_clearance(curves, field(v,"domain")?, radii, field(v,"parameter_tolerance")?, budget)?;
        let cells=report.cells.into_iter().map(|c| json!({
            "domain":c.parameter_interval,"spans":c.spans,"gap":c.gap_bounds,"status":format!("{:?}",c.status)
        })).collect::<Vec<_>>();
        return Ok(json!({"cells":cells,"workUsed":report.work_cells}));
    }
    if op == "affine_contact_intervals" {
        let report = partition_affine_synchronized_curve_clearance(curves, field(v,"windows")?, radii, field(v,"progress_tolerance")?, budget)?;
        let cells=report.cells.into_iter().map(|c| json!({
            "progress":c.progress_interval,"gap":c.gap_bounds,"status":format!("{:?}",c.status)
        })).collect::<Vec<_>>();
        return Ok(json!({"cells":cells,"workUsed":report.work_cells}));
    }
    let tol = field(v, "gap_tolerance_mm")?;
    let report = if op == "minimum_synchronized_gap" {
        measure_synchronized_curve_clearance(curves, field(v,"domain")?, radii, tol, budget)?
    } else {
        measure_affine_synchronized_curve_clearance(curves, field(v,"windows")?, radii, tol, budget)?
    };
    let mut result=json!({"gap":report.gap_bounds,"workUsed":report.work_cells,"stopReason":format!("{:?}",report.stop_reason)});
    let key = if op == "minimum_synchronized_gap" {"witnessParameter"} else {"witnessProgress"};
    result[key] = json!(report.witness_parameter);
    Ok(result)
}
