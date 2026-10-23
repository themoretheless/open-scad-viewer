//! Spring standards: preserves the original arithmetic and proof budgets.
use super::*;

/// Computes helical spring stiffness, deflection, and Wahl-corrected torsional shear stress.
pub fn helical_spring_analysis(o: &HelicalSpringOptions) -> Result<HelicalSpringReport> {
    if !o.wire_diameter_mm.is_finite()
        || o.wire_diameter_mm <= 0.05
        || !o.mean_coil_diameter_mm.is_finite()
        || o.mean_coil_diameter_mm < 3.0 * o.wire_diameter_mm
    {
        return Err(err(
            "helical_spring",
            "mean_coil_diameter_mm must be at least 3 * wire_diameter_mm.",
        ));
    }
    if !o.active_coils.is_finite()
        || o.active_coils < 1.0
        || !o.shear_modulus_mpa.is_finite()
        || o.shear_modulus_mpa <= 100.0
        || !o.axial_load_n.is_finite()
        || o.axial_load_n < 0.0
    {
        return Err(err(
            "helical_spring",
            "active_coils, shear_modulus_mpa, and axial_load_n must be valid positive numbers.",
        ));
    }
    let d = o.wire_diameter_mm;
    let dm = o.mean_coil_diameter_mm;
    let c = dm / d;
    let wahl = (4.0 * c - 1.0) / (4.0 * c - 4.0) + 0.615 / c;
    let k = (o.shear_modulus_mpa * d.powi(4)) / (8.0 * dm.powi(3) * o.active_coils);
    let deflection = o.axial_load_n / k;
    let tau = wahl * (8.0 * o.axial_load_n * dm) / (PI * d.powi(3));
    Ok(HelicalSpringReport {
        spring_index: c,
        wahl_factor: wahl,
        spring_rate_n_per_mm: k,
        deflection_mm: deflection,
        max_shear_stress_mpa: tau,
        solid_length_mm: (o.active_coils + 2.0) * d,
    })
}

/// Computes Almen–Laszlo / DIN 2092 Belleville disc spring axial load and stiffness.
pub fn disc_spring_load(o: &DiscSpringOptions) -> Result<DiscSpringReport> {
    if !o.outer_diameter_mm.is_finite()
        || !o.inner_diameter_mm.is_finite()
        || o.inner_diameter_mm <= 0.5
        || o.outer_diameter_mm <= o.inner_diameter_mm * 1.15
    {
        return Err(err(
            "disc_spring",
            "outer_diameter_mm must exceed 1.15 * inner_diameter_mm.",
        ));
    }
    if !o.thickness_mm.is_finite()
        || o.thickness_mm <= 0.05
        || !o.free_cone_height_mm.is_finite()
        || o.free_cone_height_mm <= 0.01
        || !o.deflection_mm.is_finite()
        || o.deflection_mm < 0.0
    {
        return Err(err(
            "disc_spring",
            "thickness, cone height, and deflection must be finite and valid.",
        ));
    }
    if !o.young_mpa.is_finite()
        || o.young_mpa <= 100.0
        || !o.poisson.is_finite()
        || !(0.0..0.49).contains(&o.poisson)
    {
        return Err(err(
            "disc_spring",
            "Invalid Young's modulus or Poisson ratio.",
        ));
    }
    let delta = o.outer_diameter_mm / o.inner_diameter_mm;
    let k1 = (1.0 / PI)
        * ((delta - 1.0) / delta).powi(2)
        / ((delta + 1.0) / (delta - 1.0) - 2.0 / delta.ln());
    let coeff = (4.0 * o.young_mpa)
        / (1.0 - o.poisson * o.poisson)
        * (o.thickness_mm.powi(4) / (k1 * o.outer_diameter_mm.powi(2)));
    let s = o.deflection_mm;
    let h0 = o.free_cone_height_mm;
    let t = o.thickness_mm;
    let load_at = |def: f64| {
        let u = def / t;
        let ratio = h0 / t;
        coeff * u * ((ratio - u) * (ratio - 0.5 * u) + 1.0)
    };
    let f = load_at(s);
    let f_flat = load_at(h0);
    let ds = 1e-4 * t;
    let rate = (load_at(s + ds) - load_at((s - ds).max(0.0))) / (2.0 * ds);
    Ok(DiscSpringReport {
        axial_load_n: f,
        flat_load_n: f_flat,
        spring_rate_n_per_mm: rate,
    })
}
