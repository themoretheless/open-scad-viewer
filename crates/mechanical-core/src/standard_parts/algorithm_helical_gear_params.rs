//! standard parts algorithm: helical_gear_params.
use super::*;
/// Computes helical gear geometry, virtual spur tooth count, and twist angle for extrusion.
pub fn helical_gear_params(o: &HelicalGearOptions) -> Result<HelicalGearReport> {
    if !(3..=500).contains(&o.teeth) {
        return Err(err("helical_gear", "Teeth must be between 3 and 500."));
    }
    if !o.normal_module_mm.is_finite() || !(0.1..=100.0).contains(&o.normal_module_mm) {
        return Err(err(
            "helical_gear",
            "normal_module_mm must be between 0.1 and 100 mm.",
        ));
    }
    if !o.normal_pressure_angle_deg.is_finite()
        || !(12.0..=35.0).contains(&o.normal_pressure_angle_deg)
    {
        return Err(err(
            "helical_gear",
            "normal_pressure_angle_deg must be between 12 and 35 degrees.",
        ));
    }
    if !o.helix_angle_deg.is_finite()
        || o.helix_angle_deg.abs() < 1.0
        || o.helix_angle_deg.abs() > 60.0
    {
        return Err(err(
            "helical_gear",
            "helix_angle_deg magnitude must be between 1 and 60 degrees.",
        ));
    }
    if !o.face_width_mm.is_finite() || !(0.5..=2000.0).contains(&o.face_width_mm) {
        return Err(err(
            "helical_gear",
            "face_width_mm must be between 0.5 and 2000 mm.",
        ));
    }
    let beta = o.helix_angle_deg.to_radians();
    let cos_b = beta.cos();
    let sin_b = beta.sin().abs();
    let alpha_n = o.normal_pressure_angle_deg.to_radians();
    let mt = o.normal_module_mm / cos_b;
    let alpha_t = (alpha_n.tan() / cos_b).atan();
    let d = mt * o.teeth as f64;
    let da = d + 2.0 * o.normal_module_mm;
    let df = d - 2.5 * o.normal_module_mm;
    let px = (PI * o.normal_module_mm) / sin_b;
    let lead = PI * d / beta.tan();
    let overlap = o.face_width_mm / px;
    let zv = (o.teeth as f64) / (cos_b * cos_b * cos_b);
    let twist_deg = 360.0 * o.face_width_mm / lead;
    Ok(HelicalGearReport {
        transverse_module_mm: mt,
        transverse_pressure_angle_deg: alpha_t.to_degrees(),
        pitch_diameter_mm: d,
        tip_diameter_mm: da,
        root_diameter_mm: df,
        axial_pitch_mm: px,
        lead_mm: lead.abs(),
        helix_overlap_ratio: overlap,
        virtual_spur_teeth: zv,
        twist_angle_deg: twist_deg,
    })
}
