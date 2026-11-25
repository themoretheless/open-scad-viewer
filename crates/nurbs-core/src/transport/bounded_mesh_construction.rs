use super::*;

pub(super) fn build(v: &Value, op: &str) -> Result<Value> {
    let tolerance = field(v, "tolerance_mm")?;
    let budget = field(v, "max_cells")?;
    if op == "bounded_curve_chord" {
        let report = crate::curve_tessellation::tessellate(&field(v,"curve")?, tolerance, budget)?;
        check(report.within_tolerance, "Chord tessellation did not meet the requested tolerance")?;
        let segment = report.segments.get(field::<usize>(v, "piece")?).ok_or_else(|| input("Unknown chord index"))?;
        let [a,b] = segment.domain;
        let curve = crate::curve::Curve { degree:1, knots:vec![a,a,b,b], control_points:segment.points.to_vec(),
            weights:vec![1.,1.], periodic:false };
        curve.validate()?;
        return Ok(json!({"curve":curve,"domain":segment.domain,"pieceErrorUpperMm":segment.error_upper,
            "pieces":report.segments.len(),"errorUpperMm":report.error_upper,"cells":report.cells}));
    }
    let report=crate::surface_tessellation::tessellate(&field(v,"surface")?, tolerance, budget)?;
    check(report.within_tolerance,&format!("Surface tessellation exceeds tolerance: {:?}",report.reason))?;
    let positions:Vec<_>=report.points.into_iter().flatten().collect();
    let uv:Vec<_>=report.parameters.into_iter().flatten().collect();
    let indices:Vec<_>=report.triangles.iter().flat_map(|t|t.vertices).collect();
    Ok(json!({"positions":positions,"uv":uv,"indices":indices,"triangles":report.triangles.len(),
        "cells":report.cells,"errorUpperMm":report.error_upper,"stop":format!("{:?}",report.reason)}))
}
