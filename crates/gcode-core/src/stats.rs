//! Filament mass, volume, and material cost estimators for G-code previews and jobs.

/// Computes extruded filament volume in cubic millimeters ($\text{mm}^3$) from filament length
/// (`length_mm`) and filament diameter (`diameter_mm`).
pub fn filament_volume_mm3(length_mm: f64, diameter_mm: f64) -> f64 {
    if !length_mm.is_finite()
        || !diameter_mm.is_finite()
        || length_mm <= 0.0
        || diameter_mm <= 0.0
    {
        return 0.0;
    }
    let radius = 0.5 * diameter_mm;
    std::f64::consts::PI * radius * radius * length_mm
}

/// Computes filament mass in grams ($\text{g}$) from extruded filament length (`length_mm`),
/// filament diameter (`diameter_mm`), and material density (`density_g_cm3`, e.g. `1.24` for PLA,
/// `1.27` for PETG, `1.04` for ABS).
pub fn filament_mass_grams(length_mm: f64, diameter_mm: f64, density_g_cm3: f64) -> f64 {
    if !density_g_cm3.is_finite() || density_g_cm3 <= 0.0 {
        return 0.0;
    }
    let vol_cm3 = filament_volume_mm3(length_mm, diameter_mm) * 1e-3;
    vol_cm3 * density_g_cm3
}

/// Computes estimated filament material cost from length (`length_mm`), diameter (`diameter_mm`),
/// density (`density_g_cm3`), and spool price per kilogram (`cost_per_kg`).
pub fn filament_cost_estimate(
    length_mm: f64,
    diameter_mm: f64,
    density_g_cm3: f64,
    cost_per_kg: f64,
) -> f64 {
    if !cost_per_kg.is_finite() || cost_per_kg <= 0.0 {
        return 0.0;
    }
    filament_mass_grams(length_mm, diameter_mm, density_g_cm3) * 1e-3 * cost_per_kg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filament_mass_and_cost_calculations_are_accurate() {
        // 1000 mm (1 m) of 1.75 mm PLA (density 1.24 g/cm^3):
        // Area = pi * 0.875^2 = 2.40528 mm^2 -> Volume = 2405.28 mm^3 = 2.40528 cm^3 -> Mass ≈ 2.98255 g
        let mass_g = filament_mass_grams(1000.0, 1.75, 1.24);
        assert!((mass_g - 2.98255).abs() < 1e-4);

        let cost = filament_cost_estimate(1000.0, 1.75, 1.24, 20.0);
        assert!((cost - 0.05965).abs() < 1e-4);

        assert_eq!(filament_mass_grams(-10.0, 1.75, 1.24), 0.0);
        assert_eq!(filament_mass_grams(1000.0, 0.0, 1.24), 0.0);
    }
}
