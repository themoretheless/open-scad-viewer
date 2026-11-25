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

pub(super) fn compress(v: &Value, op: &str) -> Result<Value> {
    use crate::editing::exact_surface_knot_removal::{
        TensorCompression, compress_tensor_surface, compress_tensor_surface_batch,
    };
    fn summary(patch: &TensorCompression) -> Value {
        json!({"removedKnots":patch.removed_knots,"reducedDegrees":patch.reduced_degrees,
            "completed":patch.completed,"controlNet":[patch.surface.control_points.len(),
            patch.surface.control_points[0].len()],"workUsed":patch.work_used})
    }
    let budget = field(v,"max_work")?;
    if op == "compress_tensor_surface" {
        let r = compress_tensor_surface(&field(v,"source")?,budget)?;
        return Ok(json!({"surface":r.surface,"patches":[summary(&r)],"workUsed":r.work_used}));
    }
    let sources: Vec<crate::surface::Surface> = field(v,"surfaces")?;
    let index: usize = field(v,"patch_index")?;
    if sources.len()>32 || index>=sources.len() { return Err(input("Invalid compression batch or patch index")); }
    let r=compress_tensor_surface_batch(&sources,budget)?;
    let patches=r.patches.ok_or_else(||input("Exact batch compression exhausted its shared work budget"))?;
    Ok(json!({"surface":patches[index].surface,"patches":patches.iter().map(summary).collect::<Vec<_>>(),"selectedPatch":index,"workUsed":r.work_used}))
}

pub(super) fn grid(v: &Value, op: &str) -> Result<Value> {
    let sources: Vec<crate::surface::Surface> = field(v, "surfaces")?;
    if sources.is_empty() || sources.len() > 32 {
        return Err(input("Patch layout requires 1 to 32 source surfaces"));
    }
    let budget = field(v, "max_work")?;
    match op {
        "merge_patch_strip" => {
            let axis = match field::<String>(v, "direction")?.as_str() {
                "u" => Axis::U, "v" => Axis::V, _ => return Err(input("Unknown strip axis")),
            };
            let r = exact_surface_merge::merge_surface_strip(&sources, axis, budget)?;
            let surface = r.surface.ok_or_else(|| input(format!("Exact strip refused: {:?}", r.refusal)))?;
            Ok(json!({"surface":surface,"removedKnots":r.removed_knots,"workUsed":r.work_used}))
        }
        "merge_patch_grid" => {
            let r = exact_surface_merge::merge_surface_grid(&sources, field(v,"columns")?, budget)?;
            let surface = r.surface.ok_or_else(|| input(format!("Exact grid refused: {:?}", r.refusal)))?;
            Ok(json!({"surface":surface,"alignmentInsertions":r.inserted_knots_for_alignment,"workUsed":r.work_used}))
        }
        _ => {
            let r = exact_surface_merge::infer_surface_grid_layout(&sources,budget)?;
            let layout = r.layout.ok_or_else(|| input(format!("Patch layout refused: {:?}",r.refusal)))?;
            Ok(json!({"order":layout.order,"uBreaks":layout.u_breaks,"vBreaks":layout.v_breaks,"workUsed":r.work_used}))
        }
    }
}
