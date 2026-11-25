use super::*;
use crate::editing::{exact_curve_merge, exact_surface_merge};
use crate::surface::Axis;

pub(super) fn merge(v: &Value, op: &str) -> Result<Value> {
    if op == "merge_curve_exact" {
        let left: crate::curve::Curve = field(v, "left")?;
        let right = field(v, "right")?;
        left.validate()?;
        let seam = left.domain()[1];
        let report = exact_curve_merge::merge(&left, &right)?;
        let curve = report.curve.ok_or_else(|| input(format!("Exact curve merge refused: {:?}", report.refusal)))?;
        return Ok(json!({"curve":curve,"seam":seam,"domain":curve.domain()}));
    }
    let left: crate::surface::Surface = field(v,"left")?;
    let right = field(v,"right")?;
    let direction: String = field(v,"direction")?;
    let axis = match direction.as_str() {
        "u" => Axis::U, "v" => Axis::V, _ => return Err(input("Unknown surface merge direction"))
    };
    let report = exact_surface_merge::merge(&left,&right,axis)?;
    let surface = report.surface.ok_or_else(|| input(format!("Exact surface merge refused: {:?}", report.refusal)))?;
    let seam = match axis { Axis::U => left.knots_u[left.control_points.len()], Axis::V => left.knots_v[left.control_points[0].len()] };
    Ok(json!({"surface":surface,"seam":seam,"direction":direction}))
}
