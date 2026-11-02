//! Original source iso / independent target window; no topology admission.
use crate::{Result, Value, field, require_exact_fields};
use nurbs_core::{
    intersection::certified_curve_surface::all_surface_iso_in_window, surface::Surface,
};
use value_codec::json;
pub(crate) fn run(v: Value) -> Result<Value> {
    require_exact_fields(
        &v,
        &[
            "op",
            "source",
            "axis",
            "parameter",
            "target",
            "window",
            "maxCells",
        ],
        "source iso / target window",
    )?;
    let source: Surface = field(&v, "source")?;
    let target: Surface = field(&v, "target")?;
    let axis: usize = field(&v, "axis")?;
    let parameter: f64 = field(&v, "parameter")?;
    let window: [[f64; 2]; 3] = field(&v, "window")?;
    let max_cells: usize = field(&v, "maxCells")?;
    let r = all_surface_iso_in_window(&source, axis, parameter, &target, window, max_cells)?;
    let roots: Vec<_> = r
        .roots
        .iter()
        .map(|root| {
            json!({
             "method":"original-curve-surface-krawczyk","spans":root.spans,
             "existenceDomain":root.existence_domain,"parameterBox":root.parameter_box,
             "positionBounds":root.position_bounds,"crossingSign":root.crossing_sign
            })
        })
        .collect();
    let presence = !roots.is_empty();
    let absence = r.complete && !presence;
    Ok(json!({"schema":"nurbs-surface-iso-target-window/1",
     "status":if presence{"present"}else if absence{"absent-in-window"}else{"unresolved"},
     "axis":axis,"parameter":parameter,"sourceIsoParameterCoordinates":"normalized-original-transverse-domain",
     "parameterCoordinates":["source-natural-t","target-natural-u","target-natural-v"],
     "window":window,"maxCells":max_cells,"cells":r.cells,"roots":roots,
     "presenceProven":presence,"windowAbsenceProven":absence,"windowCoverageComplete":r.complete,
     "unresolved":r.unresolved,"allSurfaceBranchesEnumerated":false,
     "trimAdmission":false,"bodyAdmission":false
    }))
}
