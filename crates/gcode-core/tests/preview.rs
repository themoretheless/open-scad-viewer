use gcode_core::{
    COORDINATE_RESOLUTION_MM, DIALECT, MAX_COORDINATE_MM, MAX_LAYERS, MAX_LINE_BYTES, MAX_MOVES,
    MAX_OUTPUT_BYTES, MachineProfile, PlannedLayer, PlannedPath, deposited_volume_mm3, emit,
    emit_3mf, parse, parse_3mf, path_length_mm,
};

fn machine() -> MachineProfile {
    MachineProfile::default()
}

fn square_layer() -> PlannedLayer {
    PlannedLayer {
        z_mm: 0.2,
        paths: vec![PlannedPath {
            points: vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]],
            closed: true,
        }],
    }
}

fn document(body: &str) -> String {
    format!("; {DIALECT}\n;FILAMENT_DIAMETER_MM:1.75\nG21\nG90\nM82\nM200 D0\nG92 E0\n{body}")
}

fn motion(body: &str) -> String {
    document(&format!(";LAYER:0\nG1 Z0.2 F600\nG0 X10 Y10\n{body}"))
}

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-7 * expected.abs().max(1.0),
        "{actual} != {expected}"
    );
}

#[test]
fn emit_uses_explicit_preview_dialect_and_no_printer_startup_or_shutdown() {
    let gcode = emit(&[square_layer()], &machine()).unwrap();
    assert!(gcode.starts_with(&format!("; {DIALECT}\n")));
    assert!(gcode.contains("G21\nG90\nM82\nM200 D0\nG92 E0\n"));
    assert!(gcode.contains("F3000.000"));
    assert!(gcode.contains("F7200.000"));
    assert!(gcode.contains(";FILAMENT_DIAMETER_MM:1.75"));
    assert!(!gcode.contains("G28"));
    assert!(!gcode.contains("M104"));
    assert!(!gcode.contains("M140"));
}

#[test]
fn parse_requires_exact_dialect_marker() {
    for source in [
        "G21\nG1 X1 Y1 E1\n".to_owned(),
        document("").replacen(DIALECT, &format!("{DIALECT}garbage"), 1),
    ] {
        assert_eq!(parse(&source).unwrap_err().code, "GCODE_DIALECT");
    }
}

#[test]
fn legacy_preview_requires_regeneration_instead_of_guessing_missing_metadata() {
    let error =
        parse("; open-scad-viewer/print-preview 1\nG21\nG90\nM82\nG92 E0\nM104 S0\nM140 S0\nG28\n")
            .unwrap_err();
    assert_eq!(error.code, "GCODE_DIALECT");
    assert!(error.message.contains("regenerate"));
}

#[test]
fn preview_round_trips_filament_volume_and_nominal_time() {
    let layers = [square_layer()];
    let machine = machine();
    let preview = parse(&emit(&layers, &machine).unwrap()).unwrap();
    assert_eq!(preview.layers, 1);
    close(preview.print_distance_mm, 40.0);
    close(preview.travel_distance_mm, 0.0);
    close(preview.estimated_time_s, 40.0 / 50.0);
    close(
        preview.deposited_volume_mm3,
        deposited_volume_mm3(&layers, &machine),
    );
    let packaged = emit_3mf(&layers, &machine).unwrap();
    let from_3mf = parse_3mf(&packaged).unwrap();
    assert_eq!(from_3mf.layers, preview.layers);
    close(from_3mf.deposited_volume_mm3, preview.deposited_volume_mm3);
    close(
        preview.extrusion_mm * machine.filament_area_mm2(),
        preview.deposited_volume_mm3,
    );
    let bounds = preview.bounds.unwrap();
    assert_eq!(bounds.min, [0.0, 0.0, 0.2]);
    assert_eq!(bounds.max, [10.0, 10.0, 0.2]);
    assert_eq!(preview.moves.len(), 5); // initial Z does not invent an XY origin
    assert!(
        preview
            .moves
            .iter()
            .skip(1)
            .all(|m| m.extruded && m.layer_index == 0 && m.feedrate_mm_s == 50.0)
    );
    assert_eq!(preview.moves[0].e, 0.0);
    assert_eq!(preview.moves.last().unwrap().e, preview.extrusion_mm);
}

#[test]
fn initial_unknown_position_is_excluded_from_bounds_and_distance() {
    let preview = parse(&motion("G1 X13 Y14 E1\n")).unwrap();
    assert_eq!(preview.moves.len(), 2);
    assert_eq!(preview.moves[0].x, 10.0);
    assert_eq!(preview.bounds.unwrap().min, [10.0, 10.0, 0.2]);
    close(preview.print_distance_mm, 5.0);
    close(preview.travel_distance_mm, 0.0);
    close(preview.estimated_time_s, 0.5);
}

#[test]
fn parser_tracks_modal_feedrate_e_deltas_and_real_travel() {
    let preview = parse(&motion("G1 X13 Y14 E1 F300\nG1 F120\nG1 X16 Y18 E1 ; E is unchanged\nG0 X19 Y22 ; E99 is only a comment\n")).unwrap();
    assert_eq!(preview.moves.len(), 4);
    assert!(preview.moves[1].extruded);
    assert!(!preview.moves[2].extruded);
    assert!(!preview.moves[3].extruded);
    close(preview.extrusion_mm, 1.0);
    close(preview.print_distance_mm, 5.0);
    close(preview.travel_distance_mm, 10.0);
    close(preview.estimated_time_s, 1.0 + 2.5 + 2.5);
    close(preview.moves[3].feedrate_mm_s, 2.0);
}

#[test]
fn layers_support_model_space_coordinates_and_count_vertical_travel() {
    let mut first = square_layer();
    first.z_mm = -1.0;
    let mut second = square_layer();
    second.z_mm = -0.5;
    let preview = parse(&emit(&[first, second], &machine()).unwrap()).unwrap();
    assert_eq!(preview.layers, 2);
    close(preview.travel_distance_mm, 0.5);
    close(preview.print_distance_mm, 80.0);
    close(preview.estimated_time_s, 1.6 + 0.5 / 120.0);
    assert_eq!(preview.moves.last().unwrap().layer_index, 1);
    assert_eq!(preview.bounds.unwrap().min[2], -1.0);
}

#[test]
fn empty_plan_round_trips_without_invented_bounds() {
    let preview = parse(&emit(&[], &machine()).unwrap()).unwrap();
    assert_eq!(preview.layers, 0);
    assert!(preview.moves.is_empty());
    assert!(preview.bounds.is_none());
    assert_eq!(preview.estimated_time_s, 0.0);
}

#[test]
fn closed_paths_include_closing_segment_without_duplicating_input() {
    let mut path = square_layer().paths.remove(0);
    assert_eq!(path_length_mm(&path), 40.0);
    path.closed = false;
    assert_eq!(path_length_mm(&path), 30.0);
    path.points.clear();
    assert_eq!(path_length_mm(&path), 0.0);
}

#[test]
fn coordinates_are_quantized_before_extrusion_is_calculated() {
    let layer = PlannedLayer {
        z_mm: 0.200004,
        paths: vec![PlannedPath {
            points: vec![[1.000004, 2.0], [2.000006, 2.0]],
            closed: false,
        }],
    };
    let preview = parse(&emit(&[layer], &machine()).unwrap()).unwrap();
    close(preview.print_distance_mm, 1.00001);
    close(
        preview.deposited_volume_mm3,
        1.00001 * machine().bead_area_mm2(),
    );
    assert_eq!(preview.moves[0].z, 0.2);
}

#[test]
fn invalid_profiles_include_derived_overflow_underflow_and_rounding() {
    for value in [
        0.0,
        -1.0,
        f64::NAN,
        f64::INFINITY,
        f64::MAX,
        f64::MIN_POSITIVE,
        COORDINATE_RESOLUTION_MM / 2.0,
    ] {
        for field in 0..3 {
            let mut machine = machine();
            match field {
                0 => machine.filament_diameter_mm = value,
                1 => machine.line_width_mm = value,
                _ => machine.layer_height_mm = value,
            }
            assert_eq!(
                emit(&[square_layer()], &machine).unwrap_err().code,
                "GCODE_INVALID_SETTINGS"
            );
        }
    }
    for value in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::MAX, 0.0001 / 60.0] {
        let mut machine = machine();
        machine.print_feedrate_mm_s = value;
        assert_eq!(
            emit(&[square_layer()], &machine).unwrap_err().code,
            "GCODE_INVALID_SETTINGS"
        );
        machine = MachineProfile::default();
        machine.travel_feedrate_mm_s = value;
        assert_eq!(
            emit(&[square_layer()], &machine).unwrap_err().code,
            "GCODE_INVALID_SETTINGS"
        );
    }
}

#[test]
fn emitter_rejects_extrusion_that_would_round_to_zero() {
    let mut machine = machine();
    machine.layer_height_mm = COORDINATE_RESOLUTION_MM;
    machine.line_width_mm = COORDINATE_RESOLUTION_MM;
    assert_eq!(
        emit(&[square_layer()], &machine).unwrap_err().code,
        "GCODE_NUMERIC"
    );
    // Formatting uses ties-to-even. Multiplying E by 1e7 and rounding can
    // incorrectly report an advance when the actual exported word is E0.
    machine.line_width_mm = 0.003942714488218457;
    machine.filament_diameter_mm = 1.002;
    let layer = PlannedLayer {
        z_mm: 0.2,
        paths: vec![PlannedPath {
            points: vec![[0.0, 0.0], [1.0, 0.0]],
            closed: false,
        }],
    };
    assert_eq!(emit(&[layer], &machine).unwrap_err().code, "GCODE_NUMERIC");
}

#[test]
fn emitter_rejects_nonfinite_or_out_of_range_points_and_heights() {
    for value in [
        f64::NAN,
        f64::INFINITY,
        -f64::INFINITY,
        MAX_COORDINATE_MM + 1.0,
        f64::MAX,
    ] {
        let mut layer = square_layer();
        layer.paths[0].points[1][0] = value;
        assert_eq!(
            emit(&[layer], &machine()).unwrap_err().code,
            "GCODE_INVALID_COORDINATE"
        );
        let mut layer = square_layer();
        layer.z_mm = value;
        assert_eq!(
            emit(&[layer], &machine()).unwrap_err().code,
            "GCODE_INVALID_HEIGHT"
        );
    }
}

#[test]
fn emitter_rejects_nonincreasing_or_indistinguishable_layers() {
    for z in [0.1, 0.2, 0.2000001] {
        let first = square_layer();
        let mut second = square_layer();
        second.z_mm = z;
        assert_eq!(
            emit(&[first, second], &machine()).unwrap_err().code,
            "GCODE_INVALID_HEIGHT"
        );
    }
}

#[test]
fn emitter_rejects_invalid_closed_path() {
    let mut layer = square_layer();
    layer.paths[0].points.truncate(2);
    assert_eq!(
        emit(&[layer], &machine()).unwrap_err().code,
        "GCODE_INVALID_PATH"
    );
}

#[test]
fn parser_requires_complete_prologue_and_metadata() {
    for command in [
        "G21\n",
        "G90\n",
        "M82\n",
        "M200 D0\n",
        "G92 E0\n",
        ";FILAMENT_DIAMETER_MM:1.75\n",
    ] {
        assert_eq!(
            parse(&motion("").replacen(command, "", 1))
                .unwrap_err()
                .code,
            "GCODE_PROLOGUE",
            "{command}"
        );
    }
    assert_eq!(
        parse(&format!("; {DIALECT}\n")).unwrap_err().code,
        "GCODE_PROLOGUE"
    );
    assert_eq!(
        parse(&document(";FILAMENT_DIAMETER_MM:2.85\n"))
            .unwrap_err()
            .code,
        "GCODE_METADATA"
    );
}

#[test]
fn parser_rejects_unsupported_modes_commands_and_extrusion() {
    for command in [
        "G20",
        "G91",
        "M83",
        "M200 D1.75",
        "G28",
        "M104 S0",
        "M140 S0",
        "G2 X12 I1",
        "G92 E0",
        "T1",
        "N1 G1 X12*32",
    ] {
        assert_eq!(
            parse(&motion(command)).unwrap_err().code,
            "GCODE_UNSUPPORTED_COMMAND",
            "{command}"
        );
    }
    for command in ["G1 X12 E-1", "G0 X12 E1", "G1 E1"] {
        assert_eq!(
            parse(&motion(command)).unwrap_err().code,
            "GCODE_UNSUPPORTED_EXTRUSION",
            "{command}"
        );
    }
    assert_eq!(
        parse(&motion("G1 X11 E1\nG1 X12 E0.5")).unwrap_err().code,
        "GCODE_UNSUPPORTED_EXTRUSION"
    );
    assert_eq!(
        parse(&document(";LAYER:0\nG1 X1 Y1 Z0.2 E1 F600"))
            .unwrap_err()
            .code,
        "GCODE_UNSUPPORTED_EXTRUSION"
    );
}

#[test]
fn parser_rejects_malformed_numbers_duplicate_words_and_unicode_without_panicking() {
    for token in ["XNaN", "Xinf", "X1e309", "X1foo", "X1Y2", "FNaN", "ENaN"] {
        let error = parse(&motion(&format!("G1 {token}"))).unwrap_err();
        assert_eq!(error.code, "GCODE_INVALID_NUMBER", "{token}");
        assert!(error.message.starts_with("Line "));
    }
    for words in ["X1 X2", "F600 F120", "Q12", "X", "é1", "💥", "X1 )", ""] {
        assert_eq!(
            parse(&motion(&format!("G1 {words}"))).unwrap_err().code,
            "GCODE_SYNTAX",
            "{words}"
        );
    }
    for value in ["0", "-100", "0.0001", "60000001"] {
        assert_eq!(
            parse(&motion(&format!("G1 X12 F{value}")))
                .unwrap_err()
                .code,
            "GCODE_INVALID_FEEDRATE"
        );
    }
    assert_eq!(
        parse(&motion("G1 X1000001")).unwrap_err().code,
        "GCODE_INVALID_COORDINATE"
    );
    assert_eq!(
        parse(&motion("G1 X12 E1e308")).unwrap_err().code,
        "GCODE_NUMERIC"
    );
}

#[test]
fn parser_rejects_layer_inconsistencies() {
    for body in [
        ";LAYER:1\nG1 Z0.2 F600",
        ";LAYER:bad",
        ";LAYER:0",
        ";LAYER:0\n;LAYER:1\nG1 Z0.2 F600",
        ";LAYER:0\n;Z:0.3\nG1 Z0.2 F600",
        ";LAYER:0\nG1 X1 Y1 F600",
        ";LAYER:0\nG1 Z0.2 F600\n;LAYER:1\nG1 Z0.2",
        ";LAYER:0\nG1 Z0.2 F600\nG1 Z0.3",
        ";Z:0.2",
        "G1 X1 F600",
    ] {
        assert_eq!(
            parse(&document(body)).unwrap_err().code,
            "GCODE_LAYER",
            "{body}"
        );
    }
}

#[test]
fn comments_blank_lines_and_crlf_are_supported() {
    let source =
        motion("\n; Unicode comment ✓\nG1 X13 Y14 E1 ; ignored E99\n").replace('\n', "\r\n");
    let preview = parse(&source).unwrap();
    close(preview.extrusion_mm, 1.0);
    close(preview.print_distance_mm, 5.0);
}

#[test]
fn input_and_single_line_size_limits_are_enforced() {
    let source = format!("; {DIALECT}\n{}", ";\n".repeat(MAX_OUTPUT_BYTES / 2));
    assert_eq!(parse(&source).unwrap_err().code, "GCODE_OUTPUT_LIMIT");
    assert_eq!(
        parse(&document(&format!(";{}", "a".repeat(MAX_LINE_BYTES))))
            .unwrap_err()
            .code,
        "GCODE_LINE_LIMIT"
    );
}

#[test]
fn excessive_layer_counts_are_rejected_on_both_sides() {
    assert_eq!(
        emit(&vec![square_layer(); MAX_LAYERS + 1], &machine())
            .unwrap_err()
            .code,
        "GCODE_LAYER_LIMIT"
    );
    let body: String = (0..=MAX_LAYERS)
        .map(|i| format!(";LAYER:{i}\nG1 Z{i} F600\n"))
        .collect();
    assert_eq!(
        parse(&document(&body)).unwrap_err().code,
        "GCODE_LAYER_LIMIT"
    );
}

#[test]
fn one_enormous_layer_is_bounded_before_generation() {
    let layer = PlannedLayer {
        z_mm: 0.2,
        paths: vec![PlannedPath {
            points: vec![[0.0, 0.0]; MAX_MOVES],
            closed: false,
        }],
    };
    assert_eq!(
        emit(&[layer], &machine()).unwrap_err().code,
        "GCODE_MOVE_LIMIT"
    );
    let layer = PlannedLayer {
        z_mm: 0.2,
        paths: vec![
            PlannedPath {
                points: vec![],
                closed: false
            };
            MAX_MOVES + 1
        ],
    };
    assert_eq!(
        emit(&[layer], &machine()).unwrap_err().code,
        "GCODE_MOVE_LIMIT"
    );
}

#[test]
fn empty_layer_z_movement_counts_toward_the_emit_budget() {
    let first = PlannedLayer {
        z_mm: 0.1,
        paths: vec![PlannedPath {
            points: (0..MAX_MOVES - 1)
                .map(|i| [(i % 2) as f64 * COORDINATE_RESOLUTION_MM, 0.0])
                .collect(),
            closed: false,
        }],
    };
    let second = PlannedLayer {
        z_mm: 0.2,
        paths: vec![],
    };
    let profile = MachineProfile {
        print_feedrate_mm_s: 0.001 / 60.0,
        travel_feedrate_mm_s: 0.001 / 60.0,
        ..machine()
    };
    assert_eq!(
        emit(&[first, second], &profile).unwrap_err().code,
        "GCODE_MOVE_LIMIT"
    );
}

#[test]
fn move_limit_applies_to_parsed_moves_even_when_position_is_incomplete() {
    assert_eq!(
        parse(&motion(&"G1 X10\n".repeat(MAX_MOVES)))
            .unwrap_err()
            .code,
        "GCODE_MOVE_LIMIT"
    );
    let body = format!(";LAYER:0\n{}", "G1 Z0.2 F600\n".repeat(MAX_MOVES + 1));
    assert_eq!(
        parse(&document(&body)).unwrap_err().code,
        "GCODE_MOVE_LIMIT"
    );
}

#[test]
fn output_is_bounded_within_a_single_layer() {
    let layer = PlannedLayer {
        z_mm: 0.2,
        paths: vec![PlannedPath {
            points: (0..90_000)
                .map(|i| [999_990.0 + (i % 2) as f64, 999_990.0])
                .collect(),
            closed: false,
        }],
    };
    assert_eq!(
        emit(&[layer], &machine()).unwrap_err().code,
        "GCODE_OUTPUT_LIMIT"
    );
}
