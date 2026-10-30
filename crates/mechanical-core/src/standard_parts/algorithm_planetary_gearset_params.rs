//! standard parts algorithm: planetary_gearset_params.
use super::*;
/// Computes epicyclic planetary gearset parameters and verifies concentricity ($Z_{\text{ring}} = Z_{\text{sun}} + 2 Z_{\text{planet}}$),
/// equal-spacing assembly condition ($(Z_{\text{sun}} + Z_{\text{ring}}) \bmod N_{\text{planets}} = 0$),
/// and adjacent planet tip clearance.
pub fn planetary_gearset_params(
    sun_teeth: usize,
    planet_teeth: usize,
    num_planets: usize,
    module_mm: f64,
) -> std::result::Result<PlanetaryGearsetSpec, String> {
    if sun_teeth < 6 || planet_teeth < 6 {
        return Err("Sun and planet tooth counts must be at least 6.".into());
    }
    if !(2..=8).contains(&num_planets) {
        return Err("Number of planets must be between 2 and 8.".into());
    }
    if !module_mm.is_finite() || !(0.1..=100.0).contains(&module_mm) {
        return Err("Module must be finite and between 0.1 and 100 mm.".into());
    }
    let ring_teeth = sun_teeth + 2 * planet_teeth;
    if !(sun_teeth + ring_teeth).is_multiple_of(num_planets) {
        return Err(format!(
            "Assembly condition failed: (Z_sun + Z_ring) = {} is not divisible by num_planets = {}.",
            sun_teeth + ring_teeth,
            num_planets
        ));
    }
    let zs = sun_teeth as f64;
    let zp = planet_teeth as f64;
    let zr = ring_teeth as f64;
    let center_distance_mm = 0.5 * module_mm * (zs + zp);
    let planet_tip_diameter_mm = module_mm * (zp + 2.0);
    let center_to_center = 2.0 * center_distance_mm * (PI / num_planets as f64).sin();
    let tip_clearance = center_to_center - planet_tip_diameter_mm;
    if tip_clearance <= 0.0 {
        return Err(format!(
            "Adjacent planet tips collide (clearance = {tip_clearance:.3} mm)."
        ));
    }
    Ok(PlanetaryGearsetSpec {
        sun_teeth,
        planet_teeth,
        ring_teeth,
        num_planets,
        module_mm,
        center_distance_mm,
        sun_pitch_diameter_mm: module_mm * zs,
        planet_pitch_diameter_mm: module_mm * zp,
        ring_pitch_diameter_mm: module_mm * zr,
        sun_to_carrier_ratio: 1.0 + zr / zs,
        adjacent_planet_tip_clearance_mm: tip_clearance,
    })
}
