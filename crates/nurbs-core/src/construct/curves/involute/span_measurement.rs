//! Base-tangent (common-normal) inspection of nominal external spur teeth.
use super::{GearParameterBounds, derive_gear_parameter_bounds};
use crate::{Result, check, distance_bounds::Interval as I};

fn interval([lo, hi]: [f64; 2]) -> Result<I> {
    I::new(lo, hi)
}

fn span(g: &GearParameterBounds, count: usize) -> Result<Option<I>> {
    // The two parallel tangent contacts are symmetric about the span midpoint.
    let roll = interval(g.half_base_angle)?
        .add(interval(g.tooth_step)?.mul(I::point(0.5 * (count - 1) as f64))?)?;
    if roll.lo < g.roll_start[1] || roll.hi > g.roll_end[0] {
        return Ok(None);
    }
    Ok(Some(interval(g.base_radius)?.mul(I::point(2.))?.mul(roll)?))
}

/// Width in mm across k teeth with symmetric contacts on the active involutes.
/// Assumes ideal planar jaws and the nominal sharp, unshifted external profile.
pub fn external_spur_span(
    module: f64,
    teeth: usize,
    pressure_deg: f64,
    clearance: f64,
    thinning: f64,
    count: usize,
) -> Result<[f64; 2]> {
    let g = derive_gear_parameter_bounds(module, teeth, pressure_deg, clearance, thinning, false)?;
    check((1..=teeth).contains(&count), "Invalid span tooth count")?;
    let value =
        span(&g, count)?.ok_or_else(|| crate::input("Span contact outside proven involute"))?;
    Ok([value.lo, value.hi])
}

/// Enumerate tooth counts whose symmetric contacts lie provably inside the active flanks.
pub fn measurable_spur_spans(
    module: f64,
    teeth: usize,
    pressure_deg: f64,
    clearance: f64,
    thinning: f64,
) -> Result<Vec<usize>> {
    let g = derive_gear_parameter_bounds(module, teeth, pressure_deg, clearance, thinning, false)?;
    let mut counts = Vec::new();
    for k in 1..=teeth {
        if span(&g, k)?.is_some() {
            counts.push(k);
        }
    }
    Ok(counts)
}

/// Recover pitch-circle tooth thinning in mm from a measured span interval.
/// Requires the entire inferred interval to be in the supported thinning range
/// and both extreme configurations to admit involute contacts.
pub fn tooth_thinning_from_span(
    module: f64,
    teeth: usize,
    pressure_deg: f64,
    clearance: f64,
    count: usize,
    measured_mm: [f64; 2],
) -> Result<[f64; 2]> {
    let g = derive_gear_parameter_bounds(module, teeth, pressure_deg, clearance, 0., false)?;
    check((1..=teeth).contains(&count), "Invalid span tooth count")?;
    let width = interval(measured_mm)?;
    check(width.lo > 0., "Expected positive measured width")?;
    let base = interval(g.base_radius)?;
    let nominal_roll = interval(g.half_base_angle)?
        .add(interval(g.tooth_step)?.mul(I::point(0.5 * (count - 1) as f64))?)?;
    let roll = width.div(base.mul(I::point(2.))?)?;
    check(
        roll.lo >= g.roll_start[1] && roll.hi <= g.roll_end[0],
        "Measured contacts outside active flank",
    )?;
    let loss = nominal_roll
        .sub(roll)?
        .mul(interval(g.pitch_radius)?)?
        .mul(I::point(2.))?;
    check(
        loss.lo >= 0. && loss.hi <= module / 2.,
        "Measured thinning outside supported range",
    )?;
    Ok([loss.lo, loss.hi])
}
