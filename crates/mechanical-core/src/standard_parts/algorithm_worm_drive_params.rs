//! standard parts algorithm: worm_drive_params.
use super::*;
/// Calculates worm drive geometry, mechanical efficiency, and self-locking condition.
pub fn worm_drive_params(o: &WormDriveOptions) -> Result<WormDriveReport> {
    if !(1..=8).contains(&o.worm_starts) || !(10..=300).contains(&o.wheel_teeth) {
        return Err(err(
            "worm_drive",
            "worm_starts must be 1..=8 and wheel_teeth 10..=300.",
        ));
    }
    if !o.axial_module_mm.is_finite() || !(0.3..=50.0).contains(&o.axial_module_mm) {
        return Err(err(
            "worm_drive",
            "axial_module_mm must be between 0.3 and 50 mm.",
        ));
    }
    if !o.worm_pitch_diameter_mm.is_finite()
        || o.worm_pitch_diameter_mm < 3.0 * o.axial_module_mm
        || o.worm_pitch_diameter_mm > 50.0 * o.axial_module_mm
    {
        return Err(err(
            "worm_drive",
            "worm_pitch_diameter_mm must be between 3*m and 50*m.",
        ));
    }
    if !o.friction_coefficient.is_finite() || !(0.0..=0.5).contains(&o.friction_coefficient) {
        return Err(err(
            "worm_drive",
            "friction_coefficient must be between 0.0 and 0.5.",
        ));
    }
    let z1 = o.worm_starts as f64;
    let z2 = o.wheel_teeth as f64;
    let px = PI * o.axial_module_mm;
    let lead = z1 * px;
    let gamma = (lead / (PI * o.worm_pitch_diameter_mm)).atan();
    let d2 = o.axial_module_mm * z2;
    let center = 0.5 * (o.worm_pitch_diameter_mm + d2);
    let rho = o.friction_coefficient.atan();
    let efficiency = gamma.tan() / (gamma + rho).tan();
    Ok(WormDriveReport {
        gear_ratio: z2 / z1,
        axial_pitch_mm: px,
        lead_mm: lead,
        lead_angle_deg: gamma.to_degrees(),
        wheel_pitch_diameter_mm: d2,
        center_distance_mm: center,
        worm_tip_diameter_mm: o.worm_pitch_diameter_mm + 2.0 * o.axial_module_mm,
        worm_root_diameter_mm: o.worm_pitch_diameter_mm - 2.4 * o.axial_module_mm,
        efficiency: efficiency.clamp(0.0, 1.0),
        self_locking: gamma <= rho,
    })
}
