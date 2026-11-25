use super::*;
use crate::{chord_winding::FillRule, curve::Curve, curve_offset, planar_toolpath};

pub(super) fn select(v: &Value, op: &str) -> Result<Value> {
    let source: Vec<Curve> = field(v,"curves")?;
    let piece: usize = field(v,"piece")?;
    if op == "clean_offset_boundary" {
        let rule = if field(v,"even_odd")? { FillRule::EvenOdd } else { FillRule::NonZero };
        let limits = curve_offset::CleanupLimits {
            pairs:field(v,"max_pairs")?, edges:field(v,"max_edges")?, witness_checks:field(v,"max_checks")?,
        };
        let r=curve_offset::cleanup_orthogonal_miter_offset(&source,field(v,"distance_mm")?,rule,limits)?;
        let component:usize=field(v,"component")?;
        let curve=r.curves.as_ref().map(|loops|loops.get(component).and_then(|c|c.get(piece))
            .ok_or_else(||input("Unknown offset component or edge"))).transpose()?;
        return Ok(json!({"curve":curve,"stop":format!("{:?}",r.stop),"components":r.curves.as_ref().map(Vec::len),
            "offsetVertices":r.offset_vertices,"pairs":r.pairs,"errorUpperMm":r.source_to_arrangement_error_upper}));
    }
    let spec=planar_toolpath::Spec { tool_radius:field(v,"tool_radius_mm")?,stock_allowance:field(v,"stock_mm")?,
        stepover:field(v,"stepover_mm")?,passes:field(v,"passes")?,tolerance:field(v,"tolerance_mm")? };
    let r=planar_toolpath::generate(&source,spec,planar_toolpath::Limits::default())?;
    let index:usize=field(v,"pass_index")?;
    let selected=r.paths.as_ref().map(|paths| paths.get(index).ok_or_else(||input("Unknown toolpath pass"))).transpose()?;
    let curve=selected.map(|pass|pass.curves.get(piece).ok_or_else(||input("Unknown edge of toolpath pass"))).transpose()?;
    Ok(json!({"curve":curve,"stop":format!("{:?}",r.stop),"qualifiedPasses":r.qualified_passes,
        "minimumClearanceMm":selected.map(|p|p.minimum_clearance_bounds),"outsideProven":selected.map(|p|p.outside_proven),
        "nominalErrorUpperMm":selected.map(|p|p.nominal_offset_error_upper),"pairs":r.clearance_pairs}))
}
