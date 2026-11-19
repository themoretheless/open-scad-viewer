use super::*;
use crate::editing::exact_periodic_surface::*;

pub(super) const OPS: &[&str] = &[
    "measure_input_contour_length",
    "measure_input_segment_length",
    "measure_input_segment_range_length",
    "locate_input_segment_at_length",
    "measure_input_contour_prefix_lengths",
    "locate_input_contour_at_lengths",
    "divide_input_contour_by_length",
    "exact_input_contour_projected_area",
    "exact_input_contour_polynomial_projected_area",
    "bound_input_contour_projected_area",
    "bound_input_patch_max_projected_area",
    "bound_input_patch_max_projection_direction",
];

fn length(p: PeriodicTrimContourLength) -> Value {
    json!({"minimumCells":p.minimum_cells,"length":p.length.map(|r|json!({"boundsMm":r.bounds,"valueMm":r.value,"errorUpperMm":r.error_upper,
        "withinTolerance":r.within_tolerance,"cells":r.cells,"stopReason":format!("{:?}",r.stop_reason)}))})
}
fn station(s: PeriodicTrimLengthLocation) -> Value {
    json!({"parameterBounds":s.parameter_bounds,"cells":s.cells,"status":format!("{:?}",s.status)})
}
fn stations(s: PeriodicTrimLengthLocations) -> Value {
    json!({"stations":s.stations.into_iter().map(station).collect::<Vec<_>>(),"tableCells":s.table_cells,"cells":s.cells})
}
fn maximum(p: PeriodicTrimMaximumProjection) -> Value {
    json!({"maximumBoundsMm2":p.maximum_bounds,"cells":p.cells,"minimumCells":p.minimum_cells,"withinTolerance":p.within_tolerance})
}
pub(super) fn dispatch(v: &Value, op: &str) -> Result<Value> {
    let source = field(v, "surface")?;
    let paths: Vec<Vec<[f64; 2]>> = field(v, "contours")?;
    let index: usize = field(v, "contour")?;
    check(index < paths.len(), "Contour index exceeds input count")?;
    let contours: Vec<_> = paths.iter().map(Vec::as_slice).collect();
    let max_work = field(v, "max_work")?;
    let edges = build_exact_periodic_surface_trim_input_edges(&source, &contours, max_work)?;
    let used = edges.geometry.work_used;
    if edges.geometry.patches.is_none() {
        return Ok(
            json!({"buildRefused":true,"workUsed":used,"refusal":edges.geometry.refusal.map(|r|format!("{r:?}"))}),
        );
    }
    let cells = || field(v, "max_cells");
    let tol = || field(v, "tolerance_mm");
    let area_tol = || field(v, "tolerance_mm2");
    let seg = || field(v, "segment");
    let width = || field(v, "parameter_tolerance");
    let mut report = match op {
        "measure_input_contour_length" => {
            length(edges.measure_input_contour_length(index, tol()?, cells()?)?)
        }
        "measure_input_segment_length" => {
            length(edges.measure_input_segment_length(index, seg()?, tol()?, cells()?)?)
        }
        "measure_input_segment_range_length" => length(edges.measure_input_segment_range_length(
            index,
            seg()?,
            field(v, "range")?,
            tol()?,
            cells()?,
        )?),
        "locate_input_segment_at_length" => station(edges.locate_input_segment_at_length(
            index,
            seg()?,
            field(v, "distance_mm")?,
            tol()?,
            width()?,
            cells()?,
        )?),
        "measure_input_contour_prefix_lengths" => {
            let r = edges.measure_input_contour_prefix_lengths(index, tol()?, cells()?)?;
            json!({"vertexPrefixBoundsMm":r.vertex_prefix_bounds,"minimumCells":r.minimum_cells,"cells":r.cells})
        }
        "locate_input_contour_at_lengths" => {
            let distances: Vec<f64> = field(v, "distances_mm")?;
            stations(edges.locate_input_contour_at_lengths(
                index,
                &distances,
                tol()?,
                width()?,
                cells()?,
            )?)
        }
        "divide_input_contour_by_length" => stations(edges.divide_input_contour_by_length(
            index,
            field(v, "divisions")?,
            tol()?,
            width()?,
            cells()?,
        )?),
        "exact_input_contour_projected_area" | "exact_input_contour_polynomial_projected_area" => {
            let remaining = max_work.saturating_sub(used);
            if remaining == 0 {
                return Ok(json!({"refusal":"WorkLimit","workUsed":used,"exactAreaVector":null}));
            }
            let r = if op == "exact_input_contour_projected_area" {
                edges.exact_input_contour_projected_area(index, remaining)?
            } else {
                edges.exact_input_contour_polynomial_projected_area(index, remaining)?
            };
            return Ok(
                json!({"exactAreaVector":r.projected_area.map(|a|a.map(|x|json!({"numerator":x.numer().to_string(),"denominator":x.denom().to_string()}))),
                "workUsed":used+r.work_used,"refusal":r.refusal.map(|e|format!("{e:?}"))}),
            );
        }
        "bound_input_contour_projected_area" => {
            let r = edges.bound_input_contour_projected_area(index, area_tol()?, cells()?)?;
            json!({"areaVectorBoundsMm2":r.bounds,"withinTolerance":r.within_tolerance,"cells":r.cells,"minimumCells":r.minimum_cells})
        }
        "bound_input_patch_max_projected_area" => {
            maximum(edges.bound_input_patch_max_projected_area(index, area_tol()?, cells()?)?)
        }
        "bound_input_patch_max_projection_direction" => {
            let p =
                edges.bound_input_patch_max_projection_direction(index, area_tol()?, cells()?)?;
            let mut r = maximum(p.maximum);
            r["normalBounds"] = json!(p.normal_bounds);
            r
        }
        _ => unreachable!(),
    };
    report["workUsed"] = json!(used);
    Ok(report)
}
