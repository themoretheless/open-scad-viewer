//! Full nominal parameter correspondence for saved gear arcs and radial lines.
use super::*;
use nurbs_core::{interval_eval::Interval as I, involute::GearParameterBounds};
fn interval(p: [f64; 2]) -> Result<I> {
    I::new(p[0], p[1])
}
fn pair(x: I) -> [f64; 2] {
    [x.lo, x.hi]
}
pub(super) fn certify(
    spec: &GearSpec,
    wires: &[Vec<Curve>],
    bounds: &GearParameterBounds,
    radial_lines: bool,
) -> Result<f64> {
    let tooth_wire = usize::from(spec.internal);
    let count = if radial_lines { 6 } else { 4 };
    let base = interval(bounds.base_radius)?;
    let low = interval(if spec.internal {
        bounds.tip_radius
    } else {
        bounds.root_radius
    })?;
    let high = if spec.internal {
        bounds.root_radius
    } else {
        bounds.tip_radius
    };
    if (radial_lines && base.lo <= low.hi) || (!radial_lines && low.lo < base.hi) {
        return Err(invalid("Nominal root/base branch is not separated"));
    }
    let step = interval(bounds.tooth_step)?;
    let low_half = interval(bounds.half_low_angle)?;
    let high_half = interval(bounds.half_high_angle)?;
    if high_half.lo <= 0.
        || low_half.hi >= step.div(I::point(2.))?.lo
        || low_half.lo <= high_half.hi
    {
        return Err(invalid("Nominal gear angular ordering is not separated"));
    }
    let mut maximum = 0_f64;
    for (wi, wire) in wires.iter().enumerate() {
        for (ci, c) in wire.iter().enumerate() {
            if c.degree == 5 {
                continue;
            }
            let error = if wi == tooth_wire {
                let original = if spec.internal {
                    wire.len() - 1 - ci
                } else {
                    ci
                };
                let tooth = original / count;
                let item = original % count;
                let center = step.mul(I::point(
                    tooth as f64 + if spec.internal { 0.5 } else { 0. },
                ))?;
                if c.degree == 1 {
                    let (from, to, sign) = match item {
                        0 if radial_lines => (pair(low), pair(base), -1.),
                        4 if radial_lines => (pair(base), pair(low), 1.),
                        _ => return Err(invalid("Unknown radial transition identity")),
                    };
                    let angle = center.add(low_half.mul(I::point(sign))?)?;
                    let (from, to) = if spec.internal {
                        (to, from)
                    } else {
                        (from, to)
                    };
                    nurbs_core::primitives::certify_radial_segment_radians(
                        c,
                        [0.; 3],
                        from,
                        to,
                        pair(angle),
                    )?
                    .error_upper
                } else {
                    let (radius, from, to) = if item == if radial_lines { 2 } else { 1 } {
                        (high, center.sub(high_half)?, center.add(high_half)?)
                    } else if item == count - 1 {
                        (
                            pair(low),
                            center.add(low_half)?,
                            center.add(step)?.sub(low_half)?,
                        )
                    } else {
                        return Err(invalid("Unknown tooth arc identity"));
                    };
                    let (from, to) = if spec.internal {
                        (to, from)
                    } else {
                        (from, to)
                    };
                    nurbs_core::primitives::certify_circle_arc_radians(
                        c,
                        [0.; 3],
                        radius,
                        pair(from),
                        pair(to),
                    )?
                    .error_upper
                }
            } else {
                let radius = if spec.internal {
                    interval(bounds.root_radius)?.add(I::point(spec.rim_width))?
                } else {
                    I::point(spec.bore).div(I::point(2.))?
                };
                let pi_step = step
                    .mul(I::point(spec.teeth as f64))?
                    .div(I::point(wire.len() as f64))?;
                let index = if spec.internal {
                    ci
                } else {
                    wire.len() - 1 - ci
                };
                let from = pi_step.mul(I::point(index as f64))?;
                let to = pi_step.mul(I::point((index + 1) as f64))?;
                let (from, to) = if spec.internal {
                    (from, to)
                } else {
                    (to, from)
                };
                nurbs_core::primitives::certify_circle_arc_radians(
                    c,
                    [0.; 3],
                    pair(radius),
                    pair(from),
                    pair(to),
                )?
                .error_upper
            };
            maximum = maximum.max(error);
        }
    }
    Ok(maximum)
}
