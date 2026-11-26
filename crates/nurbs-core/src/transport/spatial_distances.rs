use super::*;
use crate::analysis::curve_measure::{
    closest_spatial_contour_pair, inspect_spatial_contour_clearances,
    measure_spatial_contour_sets_distance, prove_spatial_contour_clearance_along, ClearanceDecision,
};
use crate::curve::Curve;

pub(super) const OPS: &[&str] = &[
    "spatial_contour_sets_distance", "closest_spatial_contours",
    "inspect_spatial_clearances", "prove_spatial_clearance_along",
];

pub(super) fn apply(v: &Value, op: &str) -> Result<Value> {
    let curves: Vec<Curve> = field(v, "curves")?;
    check((2..=256).contains(&curves.len()), "Expected two to 256 closed contours")?;
    let refs: Vec<_> = curves.iter().collect();
    let mut report = if op == "prove_spatial_clearance_along" {
        check(curves.len() == 2, "Separating plane requires exactly two contours")?;
        let proved = prove_spatial_contour_clearance_along(
            refs[0], refs[1], field(v, "normal")?, field(v, "minimum_mm")?,
        )?;
        json!({"proved":proved})
    } else {
        let tolerance = field(v, "tolerance_mm")?;
        let cells = field(v, "max_cells")?;
        check(cells <= 1_000_000, "Spatial distance cell budget exceeded")?;
        if op == "inspect_spatial_clearances" {
            let checks = inspect_spatial_contour_clearances(
                &refs, field(v, "minimum_mm")?, tolerance, cells,
            )?;
            let rows: Vec<_> = checks.into_iter().map(|row| {
                let decision = match row.decision {
                    ClearanceDecision::MeetsMinimum => "meets_minimum",
                    ClearanceDecision::BelowMinimum => "below_minimum",
                    ClearanceDecision::Unresolved => "unresolved",
                };
                json!({"contours":row.contours,"bounds_mm":row.bounds,
                       "parameters":row.witness_parameters,"cells":row.cells,"decision":decision})
            }).collect();
            json!({"checks":rows})
        } else {
            let distance = if op == "closest_spatial_contours" {
                closest_spatial_contour_pair(&refs, tolerance, cells)?
            } else {
                let split: usize = field(v, "first_count")?;
                check(split > 0 && split < refs.len(), "Both spatial contour sets must be nonempty")?;
                measure_spatial_contour_sets_distance(&refs[..split], &refs[split..], tolerance, cells)?
            };
            json!({"bounds_mm":distance.bounds,"contours":distance.witness_contours,
                   "parameters":distance.witness_parameters,"cells":distance.work_cells,
                   "converged":distance.converged})
        }
    };
    report["curve"] = json!(curves[0]);
    Ok(report)
}
