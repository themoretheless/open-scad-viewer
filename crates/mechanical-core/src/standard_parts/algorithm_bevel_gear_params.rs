//! standard parts algorithm: bevel_gear_params.
use super::*;
/// Computes straight bevel gear blank and cone geometry.
pub fn bevel_gear_params(o: &BevelGearOptions) -> Result<BevelGearReport> {
    if !(6..=300).contains(&o.pinion_teeth) || !(6..=300).contains(&o.gear_teeth) {
        return Err(err("bevel_gear", "Tooth counts must be between 6 and 300."));
    }
    if !o.module_mm.is_finite() || !(0.2..=100.0).contains(&o.module_mm) {
        return Err(err(
            "bevel_gear",
            "module_mm must be between 0.2 and 100 mm.",
        ));
    }
    if !o.shaft_angle_deg.is_finite() || !(15.0..=165.0).contains(&o.shaft_angle_deg) {
        return Err(err(
            "bevel_gear",
            "shaft_angle_deg must be between 15 and 165 degrees.",
        ));
    }
    if !o.pressure_angle_deg.is_finite() || !(14.5..=30.0).contains(&o.pressure_angle_deg) {
        return Err(err(
            "bevel_gear",
            "pressure_angle_deg must be between 14.5 and 30 degrees.",
        ));
    }
    let z1 = o.pinion_teeth as f64;
    let z2 = o.gear_teeth as f64;
    let sigma = o.shaft_angle_deg.to_radians();
    let delta1 = (sigma.sin() / (z2 / z1 + sigma.cos())).atan();
    let delta1 = if delta1 < 0.0 { delta1 + PI } else { delta1 };
    let delta2 = sigma - delta1;
    let d1 = o.module_mm * z1;
    let d2 = o.module_mm * z2;
    let r_cone = d1 / (2.0 * delta1.sin());
    if !o.face_width_mm.is_finite() || o.face_width_mm <= 0.1 || o.face_width_mm > r_cone * 0.5 {
        return Err(err(
            "bevel_gear",
            "face_width_mm must be positive and at most 0.5 * outer_cone_distance.",
        ));
    }
    let da1 = d1 + 2.0 * o.module_mm * delta1.cos();
    let da2 = d2 + 2.0 * o.module_mm * delta2.cos();
    let zv1 = z1 / delta1.cos().abs().max(1e-6);
    let zv2 = z2 / delta2.cos().abs().max(1e-6);
    Ok(BevelGearReport {
        pinion_pitch_angle_deg: delta1.to_degrees(),
        gear_pitch_angle_deg: delta2.to_degrees(),
        pinion_pitch_diameter_mm: d1,
        gear_pitch_diameter_mm: d2,
        outer_cone_distance_mm: r_cone,
        pinion_tip_diameter_mm: da1,
        gear_tip_diameter_mm: da2,
        pinion_virtual_teeth: zv1,
        gear_virtual_teeth: zv2,
    })
}
