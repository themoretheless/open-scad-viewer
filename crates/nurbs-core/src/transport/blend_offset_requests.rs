use super::*;
use crate::surface_blend::{self, Boundary, Options, Order};

fn boundary(value: &Value, key: &str) -> Result<Boundary> {
    match field::<String>(value, key)?.as_str() {
        "u_min" => Ok(Boundary::UStart),
        "u_max" => Ok(Boundary::UEnd),
        "v_min" => Ok(Boundary::VStart),
        "v_max" => Ok(Boundary::VEnd),
        _ => Err(input("Expected a U or V surface boundary")),
    }
}

pub(super) fn blend(v: &Value) -> Result<Value> {
    let surface = field(v, "surface")?;
    let target = field(v, "target")?;
    let options = Options {
        boundaries: [boundary(v, "source_edge")?, boundary(v, "target_edge")?],
        gains: field(v, "gains")?,
        reverse_u: [field(v, "reverse_u")?, field(v, "reverse_target_u")?],
        max_patches: field(v, "max_patches")?,
    };
    let report = match field::<String>(v, "continuity_class")?.as_str() {
        "G1" => surface_blend::g1(&surface, &target, options)?,
        "G2" => surface_blend::g2(&surface, &target, options)?,
        _ => return Err(input("Expected G1 or G2 for a surface blend")),
    };
    let order = match report.real_arithmetic_order {
        Order::C1 => "C1",
        Order::C2 => "C2",
    };
    Ok(json!({
        "patchCount":report.patches.len(),
        "patches":report.patches,
        "realArithmeticOrder":order,
        "roundingCertified":false,
    }))
}

pub(super) fn offset(v: &Value) -> Result<Value> {
    let report = surface_offset::offset(
        &field(v, "surface")?, field(v, "distance")?, field(v, "tolerance_mm")?,
    )?;
    let assurance = match report.assurance() {
        surface_offset::OffsetAssurance::ExactIdentity => "exact_identity",
        surface_offset::OffsetAssurance::SampledDeviation => "sampled_deviation",
    };
    Ok(json!({
        "surface":report.surface,
        "assurance":assurance,
        "maxSampledDeviationMm":report.max_deviation,
        "degenerateRegions":report.degenerate_regions,
        "exceedsFoldFreeBound":report.exceeds_fold_free_bound,
    }))
}
