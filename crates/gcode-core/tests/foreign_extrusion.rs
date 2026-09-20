use gcode_core::{GcodePreview, parse_foreign, parse_foreign_with};

fn preview(body: &str) -> GcodePreview {
    parse_foreign(&format!("G21\nG90\nM83\nG1 X0 Y0 Z0.2 F600\n{body}")).unwrap()
}

fn area(diameter: f64) -> f64 {
    std::f64::consts::PI * (diameter / 2.0).powi(2)
}

fn near(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= expected.abs().max(1.0) * 1e-10,
        "{actual} != {expected}"
    );
}

#[test]
fn volumetric_moves_and_flow_are_converted_to_filament_length() {
    let p = preview("M200 D1.75\nM221 S50\nG1 X10 E2\nG3 I-10 E4\n");
    near(p.deposited_volume_mm3, 3.0);
    near(p.extrusion_mm, 3.0 / area(1.75));
    near(p.moves.last().unwrap().e, p.extrusion_mm);
    near(
        p.print_distance_mm,
        10.0 + 63.0 * 20.0 * (std::f64::consts::PI / 63.0).sin(),
    );
}

#[test]
fn absolute_e_does_not_absorb_flow_or_g92_into_its_position() {
    let p =
        preview("M82\nG92 E100\nM221 S50\nG1 X1 E102\nM221 S200\nG1 X2 E103\nG92 E0\nG1 X3 E1\n");
    near(p.extrusion_mm, 5.0);
    near(p.deposited_volume_mm3, 5.0 * area(1.75));
    assert_eq!(
        p.moves.iter().map(|m| m.e).collect::<Vec<_>>(),
        [0.0, 1.0, 3.0, 5.0]
    );
}

#[test]
fn mode_queries_and_toggles_preserve_diameter_and_prior_totals() {
    let p = preview(
        "M200 D2\nG1 X1 E4\nM200\nM221 S50\nM221\nG1 X2 E4\nM200 D0\nG92 E0\nG1 X3 E2\nM200 S1\nG92 E0\nG1 X4 E4\nM200 S0 D4\nG92 E0\nG1 X5 E2\n",
    );
    near(p.extrusion_mm, 8.0 / area(2.0) + 2.0);
    near(p.deposited_volume_mm3, 8.0 + area(2.0) + area(4.0));
    let disabled = preview("M200 D0 S1\nG1 X1 E1\n");
    near(disabled.extrusion_mm, 1.0);
    let unchanged_e = preview("M82\nM200 D2\nG1 X1 E2\nM200 D0\nG1 X2 E3\nM200 S1\nG1 X3 E4\n");
    near(unchanged_e.extrusion_mm, 3.0 / area(2.0) + 1.0);
    near(unchanged_e.deposited_volume_mm3, 3.0 + area(2.0));
}

#[test]
fn cubic_inches_scale_e_and_g92_but_diameter_remains_linear() {
    let p = preview("G20\nM200 D0.1\nM82\nG92 E0.001\nG1 X1 E0.002\nG21\nG1 X30 E49.161192\n");
    near(p.deposited_volume_mm3, 0.002 * 25.4_f64.powi(3));
    near(p.extrusion_mm, p.deposited_volume_mm3 / area(2.54));
    near(p.moves.last().unwrap().x, 30.0);
}

#[test]
fn tool_targeting_preserves_global_e_and_uses_per_tool_diameter_and_flow() {
    let p = preview(
        "M82\nM200 T0 D2\nM200 T1 D4\nM221 T1 S50\nG1 X1 E4\nT1\nG1 X2 E8\nT0\nG1 X3 E12\n",
    );
    near(p.deposited_volume_mm3, 10.0);
    near(p.extrusion_mm, 8.0 / area(2.0) + 2.0 / area(4.0));
    let active = preview("T1\nM200 D4\nM221 S50\nG1 X1 E4\nT0\nG1 X2 E4\n");
    near(active.deposited_volume_mm3, 6.0);
    near(active.extrusion_mm, 2.0 / area(4.0) + 4.0 / area(1.75));
}

#[test]
fn zero_flow_retraction_and_stationary_prime_keep_positive_advance_contract() {
    let p = preview("M82\nM221 S0\nG1 X1 E5\nM221 S100\nG1 X2 E6\nG1 E4\nG1 E6\nG1 X3\n");
    near(p.extrusion_mm, 3.0);
    near(p.deposited_volume_mm3, 3.0 * area(1.75));
    assert!(!p.moves[1].extruded);
    assert!(p.moves[2].extruded);
    near(p.print_distance_mm, 1.0);
    near(p.travel_distance_mm, 2.0);
}

#[test]
fn explicit_diameter_overrides_header_but_later_m200_updates_its_tool() {
    let text =
        "; filament_diameter = 1.75\nG90\nM83\nG1 X0 Y0 Z0.2\nG1 X1 E1\nM200 S0 D2\nG1 X2 E1\n";
    let p = parse_foreign_with(text, Some(2.85)).unwrap();
    near(p.extrusion_mm, 2.0);
    near(p.deposited_volume_mm3, area(2.85) + area(2.0));
}

#[test]
fn malformed_extrusion_settings_fail_with_their_line_number() {
    for command in [
        "M200 D-1",
        "M200 D1000001",
        "M200 S2",
        "M200 S0.5",
        "M200 T4294967296 D2",
        "M200 T-1 D2",
        "M221 T1.5 S50",
        "M221 S-1",
        "T-1",
        "T1.5",
        "T4294967296",
    ] {
        let error =
            parse_foreign(&format!("G1 X0 Y0 Z0\n{command}\nG1 X1 E1\n")).expect_err(command);
        assert_eq!(error.code, "GCODE_INVALID_SETTINGS", "{command}: {error}");
        assert!(error.message.starts_with("Line 2:"), "{command}: {error}");
    }
}

#[test]
fn absolute_and_relative_programs_agree_across_units_diameters_and_flows() {
    for diameter in [1.75, 2.85, 4.0] {
        for flow in [0.0, 25.0, 100.0, 250.0] {
            for volumetric in [false, true] {
                for inches in [false, true] {
                    let mut results = Vec::new();
                    for relative in [false, true] {
                        let mut body = format!(
                            "M200 D{diameter} S{}\nM221 S{flow}\n{}\n{}\n",
                            u8::from(volumetric),
                            if inches { "G20" } else { "G21" },
                            if relative { "M83" } else { "M82" }
                        );
                        let mut e = 0.0;
                        for i in 0..20 {
                            if i % 7 == 0 {
                                body.push_str("G92 E0\n");
                                e = 0.0;
                            }
                            let delta = if i % 3 == 0 { -0.25 } else { 0.5 };
                            e += delta;
                            body.push_str(&format!(
                                "G1 X{} E{}\n",
                                i + 1,
                                if relative { delta } else { e }
                            ));
                        }
                        let p = preview(&body);
                        let unit: f64 = if inches { 25.4 } else { 1.0 };
                        let advance =
                            6.5 * if volumetric { unit.powi(3) } else { unit } * flow / 100.0;
                        near(
                            p.extrusion_mm,
                            if volumetric {
                                advance / area(diameter)
                            } else {
                                advance
                            },
                        );
                        near(
                            p.deposited_volume_mm3,
                            if volumetric {
                                advance
                            } else {
                                advance * area(diameter)
                            },
                        );
                        results.push(p);
                    }
                    near(results[0].extrusion_mm, results[1].extrusion_mm);
                    near(
                        results[0].deposited_volume_mm3,
                        results[1].deposited_volume_mm3,
                    );
                    assert_eq!(results[0].print_distance_mm, results[1].print_distance_mm);
                }
            }
        }
    }
}

#[test]
fn numeric_extremes_are_rejected_instead_of_nonfinite_or_zero_material() {
    let huge = format!("{:.0}", f64::MAX);
    let tiny = format!("{:.324}", f64::from_bits(1));
    for body in [
        format!("M221 S{huge}\nG1 X1 E100\n"),
        format!("M221 S0\nG92 E{huge}\nG1 X1 E{huge}\n"),
        format!("M221 S{tiny}\n"),
        format!("M221 S{:.210}\nG1 X1 E{:.210}\n", 1e-200, 1e-200),
        format!("G20\nM200 D0.1\nG92 E{huge}\n"),
    ] {
        let error = parse_foreign(&format!("G90\nM83\nG1 X0 Y0 Z0\n{body}")).expect_err(&body);
        assert_eq!(error.code, "GCODE_NUMERIC", "{body}: {error}");
        assert!(error.message.starts_with("Line "));
    }
    let error = parse_foreign_with("G1 X0 Y0 Z0\n", Some(1e-200)).unwrap_err();
    assert_eq!(error.code, "GCODE_INVALID_SETTINGS");
}

#[test]
fn sparse_tool_identifiers_are_bounded_and_foreign_selector_syntax_is_not_misapplied() {
    let p = preview("M200 T4294967295 D2\nM221 T4294967295 S50\nT4294967295\nG1 X1 E4\n");
    near(p.deposited_volume_mm3, 2.0);
    near(p.extrusion_mm, 2.0 / area(2.0));
    let error = parse_foreign("G1 X0 Y0 Z0\nM221 D1 S50\nG1 X1 E1\n").unwrap_err();
    assert_eq!(error.code, "GCODE_UNSUPPORTED_COMMAND");
    let p = preview("TURN_OFF_HEATERS\nT1000_RESET\nSET_PRESSURE_ADVANCE ADVANCE=0.04\nG1 X1 E1\n");
    near(p.extrusion_mm, 1.0);
    // High numeric identifiers occur in slicer macros too; they must not cause
    // a huge index-based allocation. Their firmware macro behavior is not modeled.
    let p = preview("T1000\nG1 X1 E1\nT255\nG1 X2\nT0\nG1 X3 E1\n");
    near(p.extrusion_mm, 2.0);
    for selector in ["T", "M200 T", "M221 T"] {
        let mut text = String::from("G1 X0 Y0 Z0\n");
        for i in 1..256 {
            text.push_str(&format!("{selector}{}\n", i * 1_000));
        }
        assert!(parse_foreign(&text).is_ok());
        text.push_str(&format!("{selector}256000\n"));
        assert_eq!(parse_foreign(&text).unwrap_err().code, "GCODE_TOOL_LIMIT");
    }
}
