//! Fastener standards: preserves the original arithmetic and proof budgets.
use super::*;

/// Looks up standard ISO metric fastener dimensions for M2, M2.5, M3, M4, M5, M6, M8, M10, M12, M16, M20.
pub fn iso_metric_fastener(nominal_mm: f64) -> Result<MetricFastenerSpec> {
    let table: &[(f64, MetricFastenerSpec)] = &[
        (
            2.0,
            MetricFastenerSpec {
                nominal_diameter_mm: 2.0,
                coarse_pitch_mm: 0.4,
                hex_width_across_flats_mm: 4.0,
                hex_head_height_mm: 1.4,
                nut_height_mm: 1.6,
                socket_head_diameter_mm: 3.8,
                socket_head_height_mm: 2.0,
                socket_key_across_flats_mm: 1.5,
                clearance_hole_close_mm: 2.2,
                clearance_hole_medium_mm: 2.4,
            },
        ),
        (
            2.5,
            MetricFastenerSpec {
                nominal_diameter_mm: 2.5,
                coarse_pitch_mm: 0.45,
                hex_width_across_flats_mm: 5.0,
                hex_head_height_mm: 1.7,
                nut_height_mm: 2.0,
                socket_head_diameter_mm: 4.5,
                socket_head_height_mm: 2.5,
                socket_key_across_flats_mm: 2.0,
                clearance_hole_close_mm: 2.7,
                clearance_hole_medium_mm: 2.9,
            },
        ),
        (
            3.0,
            MetricFastenerSpec {
                nominal_diameter_mm: 3.0,
                coarse_pitch_mm: 0.5,
                hex_width_across_flats_mm: 5.5,
                hex_head_height_mm: 2.0,
                nut_height_mm: 2.4,
                socket_head_diameter_mm: 5.5,
                socket_head_height_mm: 3.0,
                socket_key_across_flats_mm: 2.5,
                clearance_hole_close_mm: 3.2,
                clearance_hole_medium_mm: 3.4,
            },
        ),
        (
            4.0,
            MetricFastenerSpec {
                nominal_diameter_mm: 4.0,
                coarse_pitch_mm: 0.7,
                hex_width_across_flats_mm: 7.0,
                hex_head_height_mm: 2.8,
                nut_height_mm: 3.2,
                socket_head_diameter_mm: 7.0,
                socket_head_height_mm: 4.0,
                socket_key_across_flats_mm: 3.0,
                clearance_hole_close_mm: 4.3,
                clearance_hole_medium_mm: 4.5,
            },
        ),
        (
            5.0,
            MetricFastenerSpec {
                nominal_diameter_mm: 5.0,
                coarse_pitch_mm: 0.8,
                hex_width_across_flats_mm: 8.0,
                hex_head_height_mm: 3.5,
                nut_height_mm: 4.7,
                socket_head_diameter_mm: 8.5,
                socket_head_height_mm: 5.0,
                socket_key_across_flats_mm: 4.0,
                clearance_hole_close_mm: 5.3,
                clearance_hole_medium_mm: 5.5,
            },
        ),
        (
            6.0,
            MetricFastenerSpec {
                nominal_diameter_mm: 6.0,
                coarse_pitch_mm: 1.0,
                hex_width_across_flats_mm: 10.0,
                hex_head_height_mm: 4.0,
                nut_height_mm: 5.2,
                socket_head_diameter_mm: 10.0,
                socket_head_height_mm: 6.0,
                socket_key_across_flats_mm: 5.0,
                clearance_hole_close_mm: 6.4,
                clearance_hole_medium_mm: 6.6,
            },
        ),
        (
            8.0,
            MetricFastenerSpec {
                nominal_diameter_mm: 8.0,
                coarse_pitch_mm: 1.25,
                hex_width_across_flats_mm: 13.0,
                hex_head_height_mm: 5.3,
                nut_height_mm: 6.8,
                socket_head_diameter_mm: 13.0,
                socket_head_height_mm: 8.0,
                socket_key_across_flats_mm: 6.0,
                clearance_hole_close_mm: 8.4,
                clearance_hole_medium_mm: 9.0,
            },
        ),
        (
            10.0,
            MetricFastenerSpec {
                nominal_diameter_mm: 10.0,
                coarse_pitch_mm: 1.5,
                hex_width_across_flats_mm: 16.0,
                hex_head_height_mm: 6.4,
                nut_height_mm: 8.4,
                socket_head_diameter_mm: 16.0,
                socket_head_height_mm: 10.0,
                socket_key_across_flats_mm: 8.0,
                clearance_hole_close_mm: 10.5,
                clearance_hole_medium_mm: 11.0,
            },
        ),
        (
            12.0,
            MetricFastenerSpec {
                nominal_diameter_mm: 12.0,
                coarse_pitch_mm: 1.75,
                hex_width_across_flats_mm: 18.0,
                hex_head_height_mm: 7.5,
                nut_height_mm: 10.8,
                socket_head_diameter_mm: 18.0,
                socket_head_height_mm: 12.0,
                socket_key_across_flats_mm: 10.0,
                clearance_hole_close_mm: 13.0,
                clearance_hole_medium_mm: 13.5,
            },
        ),
        (
            16.0,
            MetricFastenerSpec {
                nominal_diameter_mm: 16.0,
                coarse_pitch_mm: 2.0,
                hex_width_across_flats_mm: 24.0,
                hex_head_height_mm: 10.0,
                nut_height_mm: 14.8,
                socket_head_diameter_mm: 24.0,
                socket_head_height_mm: 16.0,
                socket_key_across_flats_mm: 14.0,
                clearance_hole_close_mm: 17.0,
                clearance_hole_medium_mm: 17.5,
            },
        ),
        (
            20.0,
            MetricFastenerSpec {
                nominal_diameter_mm: 20.0,
                coarse_pitch_mm: 2.5,
                hex_width_across_flats_mm: 30.0,
                hex_head_height_mm: 12.5,
                nut_height_mm: 18.0,
                socket_head_diameter_mm: 30.0,
                socket_head_height_mm: 20.0,
                socket_key_across_flats_mm: 17.0,
                clearance_hole_close_mm: 21.0,
                clearance_hole_medium_mm: 22.0,
            },
        ),
    ];
    for &(size, spec) in table {
        if (size - nominal_mm).abs() < 1e-6 {
            return Ok(spec);
        }
    }
    Err(err(
        "fastener",
        "Unsupported ISO metric nominal diameter (supported: M2, M2.5, M3, M4, M5, M6, M8, M10, M12, M16, M20).",
    ))
}

/// Looks up standard 3D-printing heat-set insert pocket & boss dimensions for M2, M2.5, M3, M4, M5, M6, M8.
pub fn heat_set_insert_spec(nominal_mm: f64) -> Result<HeatSetInsertSpec> {
    let table: &[(f64, HeatSetInsertSpec)] = &[
        (
            2.0,
            HeatSetInsertSpec {
                nominal_thread_mm: 2.0,
                insert_outer_diameter_mm: 3.5,
                recommended_hole_diameter_mm: 3.2,
                insert_length_mm: 3.0,
                min_pocket_depth_mm: 4.0,
                min_boss_outer_diameter_mm: 6.4,
            },
        ),
        (
            2.5,
            HeatSetInsertSpec {
                nominal_thread_mm: 2.5,
                insert_outer_diameter_mm: 3.8,
                recommended_hole_diameter_mm: 3.5,
                insert_length_mm: 4.0,
                min_pocket_depth_mm: 5.0,
                min_boss_outer_diameter_mm: 7.0,
            },
        ),
        (
            3.0,
            HeatSetInsertSpec {
                nominal_thread_mm: 3.0,
                insert_outer_diameter_mm: 4.5,
                recommended_hole_diameter_mm: 4.0,
                insert_length_mm: 5.7,
                min_pocket_depth_mm: 6.7,
                min_boss_outer_diameter_mm: 8.0,
            },
        ),
        (
            4.0,
            HeatSetInsertSpec {
                nominal_thread_mm: 4.0,
                insert_outer_diameter_mm: 6.0,
                recommended_hole_diameter_mm: 5.6,
                insert_length_mm: 8.1,
                min_pocket_depth_mm: 9.1,
                min_boss_outer_diameter_mm: 10.6,
            },
        ),
        (
            5.0,
            HeatSetInsertSpec {
                nominal_thread_mm: 5.0,
                insert_outer_diameter_mm: 7.0,
                recommended_hole_diameter_mm: 6.4,
                insert_length_mm: 9.5,
                min_pocket_depth_mm: 10.5,
                min_boss_outer_diameter_mm: 12.4,
            },
        ),
        (
            6.0,
            HeatSetInsertSpec {
                nominal_thread_mm: 6.0,
                insert_outer_diameter_mm: 8.7,
                recommended_hole_diameter_mm: 8.0,
                insert_length_mm: 12.7,
                min_pocket_depth_mm: 14.0,
                min_boss_outer_diameter_mm: 15.0,
            },
        ),
        (
            8.0,
            HeatSetInsertSpec {
                nominal_thread_mm: 8.0,
                insert_outer_diameter_mm: 10.5,
                recommended_hole_diameter_mm: 9.8,
                insert_length_mm: 12.7,
                min_pocket_depth_mm: 14.5,
                min_boss_outer_diameter_mm: 18.0,
            },
        ),
    ];
    for &(size, spec) in table {
        if (size - nominal_mm).abs() < 1e-6 {
            return Ok(spec);
        }
    }
    Err(err(
        "heat_set_insert",
        "Unsupported heat-set insert size (supported: M2, M2.5, M3, M4, M5, M6, M8).",
    ))
}

/// Generates a 2D hex nut profile (outer hexagon + inner thread minor-diameter circle) for an ISO metric size.
pub fn hex_nut_profile(nominal_mm: f64) -> Result<Vec<Vec<Point>>> {
    let spec = iso_metric_fastener(nominal_mm)?;
    let r_circum = spec.hex_width_across_flats_mm / 3.0_f64.sqrt();
    let hex: Vec<Point> = (0..6)
        .map(|i| {
            let a = TAU * (i as f64) / 6.0;
            [r_circum * a.cos(), r_circum * a.sin()]
        })
        .collect();
    let r_hole = 0.5 * (spec.nominal_diameter_mm - 1.0825 * spec.coarse_pitch_mm);
    let hole: Vec<Point> = (0..32)
        .rev()
        .map(|i| {
            let a = TAU * (i as f64) / 32.0;
            [r_hole * a.cos(), r_hole * a.sin()]
        })
        .collect();
    Ok(vec![hex, hole])
}

/// Looks up DIN 6885 parallel key and keyway dimensions for a shaft diameter in [6, 130] mm.
pub fn shaft_keyway_din6885(shaft_diameter_mm: f64) -> Result<KeywaySpec> {
    if !shaft_diameter_mm.is_finite() || !(6.0..=130.0).contains(&shaft_diameter_mm) {
        return Err(err(
            "keyway",
            "Shaft diameter for DIN 6885 keyway must be between 6 and 130 mm.",
        ));
    }
    let d = shaft_diameter_mm;
    let (b, h, t1, t2) = if d <= 8.0 {
        (2.0, 2.0, 1.2, 1.0)
    } else if d <= 10.0 {
        (3.0, 3.0, 1.8, 1.4)
    } else if d <= 12.0 {
        (4.0, 4.0, 2.5, 1.8)
    } else if d <= 17.0 {
        (5.0, 5.0, 3.0, 2.3)
    } else if d <= 22.0 {
        (6.0, 6.0, 3.5, 2.8)
    } else if d <= 30.0 {
        (8.0, 7.0, 4.0, 3.3)
    } else if d <= 38.0 {
        (10.0, 8.0, 5.0, 3.3)
    } else if d <= 44.0 {
        (12.0, 8.0, 5.0, 3.3)
    } else if d <= 50.0 {
        (14.0, 9.0, 5.5, 3.8)
    } else if d <= 58.0 {
        (16.0, 10.0, 6.0, 4.3)
    } else if d <= 65.0 {
        (18.0, 11.0, 7.0, 4.4)
    } else if d <= 75.0 {
        (20.0, 12.0, 7.5, 4.9)
    } else if d <= 85.0 {
        (22.0, 14.0, 9.0, 5.4)
    } else if d <= 95.0 {
        (25.0, 14.0, 9.0, 5.4)
    } else if d <= 110.0 {
        (28.0, 16.0, 10.0, 6.4)
    } else {
        (32.0, 18.0, 11.0, 7.4)
    };
    Ok(KeywaySpec {
        key_width_b_mm: b,
        key_height_h_mm: h,
        shaft_groove_depth_t1_mm: t1,
        hub_groove_depth_t2_mm: t2,
    })
}

/// Computes DIN 471 (external) or DIN 472 (internal) retaining ring groove dimensions.
pub fn retaining_ring_groove(
    nominal_diameter_mm: f64,
    internal_bore: bool,
) -> Result<RetainingRingGrooveSpec> {
    if !nominal_diameter_mm.is_finite() || !(4.0..=100.0).contains(&nominal_diameter_mm) {
        return Err(err(
            "retaining_ring",
            "Nominal diameter must be between 4 and 100 mm.",
        ));
    }
    let d = nominal_diameter_mm;
    let (s, m, radial_depth) = if d < 10.0 {
        (0.8, 0.9, 0.3)
    } else if d < 18.0 {
        (1.0, 1.1, 0.45)
    } else if d < 25.0 {
        (1.2, 1.3, 0.6)
    } else if d < 35.0 {
        (1.5, 1.6, 0.75)
    } else if d < 50.0 {
        (1.75, 1.85, 1.0)
    } else {
        (2.0, 2.15, 1.25)
    };
    let d2 = if internal_bore {
        d + 2.0 * radial_depth
    } else {
        d - 2.0 * radial_depth
    };
    Ok(RetainingRingGrooveSpec {
        nominal_diameter_mm: d,
        groove_diameter_d2_mm: d2,
        groove_width_m_mm: m,
        ring_thickness_s_mm: s,
    })
}

/// Computes static axial/radial O-ring groove dimensions for standard ISO 3601 cross-sections.
pub fn o_ring_groove_spec(cross_section_d2_mm: f64, dynamic_seal: bool) -> Result<ORingGrooveSpec> {
    if !cross_section_d2_mm.is_finite() || !(1.0..=10.0).contains(&cross_section_d2_mm) {
        return Err(err(
            "o_ring",
            "O-ring cross-section diameter d2 must be between 1.0 and 10.0 mm.",
        ));
    }
    let squeeze = if dynamic_seal { 0.14 } else { 0.22 };
    let depth = cross_section_d2_mm * (1.0 - squeeze);
    let width = cross_section_d2_mm * if dynamic_seal { 1.38 } else { 1.32 };
    Ok(ORingGrooveSpec {
        cross_section_d2_mm,
        groove_depth_h_mm: (depth * 100.0).round() / 100.0,
        groove_width_b_mm: (width * 100.0).round() / 100.0,
        compression_ratio: squeeze,
        corner_radius_mm: (cross_section_d2_mm * 0.15).clamp(0.2, 1.0),
    })
}

/// Computes ISO 286 H7 bore and g6/k6/p6 shaft tolerance bands for diameters in [3, 120] mm.
pub fn iso286_shaft_housing_fit(
    nominal_diameter_mm: f64,
    fit: BearingFitClass,
) -> Result<BearingFitReport> {
    if !nominal_diameter_mm.is_finite() || !(3.0..=120.0).contains(&nominal_diameter_mm) {
        return Err(err(
            "iso286_fit",
            "Nominal diameter for ISO 286 fit must be between 3 and 120 mm.",
        ));
    }
    let d = nominal_diameter_mm;
    // Standard tolerance unit i (um) = 0.45 * D^(1/3) + 0.001 * D
    let i_um = 0.45 * d.cbrt() + 0.001 * d;
    let it7_um = (16.0 * i_um).round();
    let it6_um = (10.0 * i_um).round();
    let (shaft_lower_um, shaft_upper_um) = match fit {
        BearingFitClass::ClearanceH7g6 => {
            let es = -(2.5 * d.powf(0.34)).round();
            (es - it6_um, es)
        }
        BearingFitClass::TransitionH7k6 => {
            let ei = (0.6 * d.cbrt()).round();
            (ei, ei + it6_um)
        }
        BearingFitClass::InterferenceH7p6 => {
            let ei = (it7_um + 2.0).round();
            (ei, ei + it6_um)
        }
    };
    let hole_min_mm = d;
    let hole_max_mm = d + it7_um * 1e-3;
    let shaft_min_mm = d + shaft_lower_um * 1e-3;
    let shaft_max_mm = d + shaft_upper_um * 1e-3;
    Ok(BearingFitReport {
        nominal_diameter_mm: d,
        hole_min_mm,
        hole_max_mm,
        shaft_min_mm,
        shaft_max_mm,
        min_clearance_um: -shaft_upper_um,
        max_clearance_um: it7_um - shaft_lower_um,
    })
}

/// Computes ISO 3601 axial/radial O-ring gland dimensions from cord diameter (`cord_diameter_mm`),
/// O-ring inner diameter (`inner_diameter_mm`), and `static_seal` flag (`true` for static seal, `false` for dynamic).
pub fn oring_gland_spec(
    cord_diameter_mm: f64,
    inner_diameter_mm: f64,
    static_seal: bool,
) -> Result<OringGlandSpec> {
    if !cord_diameter_mm.is_finite() || !(1.0..=10.0).contains(&cord_diameter_mm) {
        return Err(err(
            "o_ring",
            "O-ring cord_diameter_mm must be between 1.0 and 10.0 mm.",
        ));
    }
    if !inner_diameter_mm.is_finite() || !(2.0..=1000.0).contains(&inner_diameter_mm) {
        return Err(err(
            "o_ring",
            "O-ring inner_diameter_mm must be between 2.0 and 1000.0 mm.",
        ));
    }
    let groove = o_ring_groove_spec(cord_diameter_mm, !static_seal)?;
    let gland_outer = inner_diameter_mm + 2.0 * groove.groove_depth_h_mm;
    let oring_area = PI * (0.5 * cord_diameter_mm).powi(2);
    let gland_area = groove.groove_depth_h_mm * groove.groove_width_b_mm;
    Ok(OringGlandSpec {
        cord_diameter_mm,
        inner_diameter_mm,
        gland_inner_diameter_mm: inner_diameter_mm,
        gland_outer_diameter_mm: gland_outer,
        groove_depth_mm: groove.groove_depth_h_mm,
        groove_width_mm: groove.groove_width_b_mm,
        compression_ratio: groove.compression_ratio,
        gland_fill_ratio: oring_area / gland_area.max(1e-9),
        corner_radius_mm: groove.corner_radius_mm,
    })
}
