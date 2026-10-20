//! Original stored-arc radial bounds, using construction-owned arc identities.
use super::*;
use nurbs_core::{interval_eval::Interval as I, involute::GearParameterBounds};
pub(super) fn certify(
    spec: &GearSpec,
    wires: &[Vec<Curve>],
    ideal: &GearParameterBounds,
    radial_lines: bool,
) -> Result<f64> {
    let tooth_wire = usize::from(spec.internal);
    let count = if radial_lines { 6 } else { 4 };
    let high_index = if radial_lines { 2 } else { 1 };
    let high = if spec.internal {
        ideal.root_radius
    } else {
        ideal.tip_radius
    };
    let low = if spec.internal {
        ideal.tip_radius
    } else {
        ideal.root_radius
    };
    let mut maximum = 0_f64;
    let mut arcs = 0;
    for (wi, wire) in wires.iter().enumerate() {
        for (ci, c) in wire.iter().enumerate() {
            if c.degree != 2 {
                continue;
            }
            arcs += 1;
            let radius = if wi == tooth_wire {
                let original = if spec.internal {
                    wire.len() - 1 - ci
                } else {
                    ci
                };
                match original % count {
                    n if n == high_index => high,
                    n if n == count - 1 => low,
                    _ => {
                        return Err(invalid(
                            "Arc identity is absent from the gear construction map",
                        ));
                    }
                }
            } else if spec.internal {
                let outside = I::new(ideal.root_radius[0], ideal.root_radius[1])?
                    .add(I::point(spec.rim_width))?;
                [outside.lo, outside.hi]
            } else {
                let bore = I::point(spec.bore).div(I::point(2.))?;
                [bore.lo, bore.hi]
            };
            let proof = nurbs_core::radial_bounds::curve_radius_error(c, [0.; 3], radius)?
                .ok_or_else(|| invalid("Original gear arc radial bound remains unresolved"))?;
            maximum = maximum.max(proof.error_upper);
        }
    }
    if arcs == 0 {
        return Err(invalid("Gear radial certificate requires original arcs"));
    }
    Ok(maximum)
}
