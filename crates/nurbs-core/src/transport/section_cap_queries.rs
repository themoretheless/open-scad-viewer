use super::*;
use crate::{curve::Curve, editing::exact_surface_derivative as plane};
pub(super) fn build(v: &Value, op: &str) -> Result<Value> {
    let rings: Vec<Vec<Curve>> = field(v, "rings")?;
    let refs = rings.iter().map(Vec::as_slice).collect::<Vec<_>>();
    let normal = field(v, "plane_normal")?;
    let offset = field(v, "plane_offset_mm")?;
    let tol = field(v, "boundary_tolerance_mm")?;
    let work = field(v, "max_work")?;
    let (cap, used, mut report) = match op {
        "cap_section_wire" => {
            crate::check(rings.len() == 1, "A single cap requires one outer ring")?;
            let r = plane::cap_surface_plane_section(&rings[0], normal, offset, tol, work)?;
            let c = r
                .outcome
                .map_err(|e| input(format!("Cap refused: {e:?}")))?;
            (
                plane::PlanarSectionCapWithHoles {
                    face: c.face,
                    reversed: vec![c.reversed],
                    boundary_gap_upper: c.boundary_gap_upper,
                },
                r.work_used,
                json!({}),
            )
        }
        "cap_section_holes" => {
            let r = plane::cap_surface_plane_section_with_holes(&refs, normal, offset, tol, work)?;
            (
                r.outcome
                    .map_err(|e| input(format!("Cap rings refused: {e:?}")))?,
                r.work_used,
                json!({}),
            )
        }
        _ => {
            let r = plane::arrange_surface_plane_section_caps(&refs, normal, offset, tol, work)?;
            let a = r
                .outcome
                .map_err(|e| input(format!("Cap arrangement refused: {e:?}")))?;
            let index: usize = field(v, "region")?;
            let info = json!({"parents":a.parents,"regions":a.regions.iter().map(|r|&r.rings).collect::<Vec<_>>(),"selectedRegion":index});
            let c = a
                .regions
                .into_iter()
                .nth(index)
                .ok_or_else(|| input("Unknown cap region"))?
                .cap;
            (c, r.work_used, info)
        }
    };
    crate::check(used < work, "Cap support work limit reached")?;
    let r = plane::lift_planar_cap_support_bounded(
        &cap.face.projection,
        field(v, "support_tolerance_mm")?,
        work - used,
    )?;
    let support = r
        .outcome
        .map_err(|e| input(format!("Cap support refused: {e:?}")))?;
    report["surface"] = json!(support.surface);
    report["supportErrorUpper"] = json!(support.error_upper.to_string());
    report["boundaryGapUpper"] = json!(cap.boundary_gap_upper.to_string());
    report["trimLoops"] = json!(cap.face.loops);
    report["reversed"] = json!(cap.reversed);
    report["workUsed"] = json!(used + r.work_used);
    Ok(report)
}
