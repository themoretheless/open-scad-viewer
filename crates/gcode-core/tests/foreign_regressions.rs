use gcode_core::{MAX_MOVES, parse_foreign};

#[test]
fn compact_extruder_words_are_not_scientific_notation() {
    let plain = parse_foreign("G90\nM83\nG1 X0 Y0 Z0.2\nG1 X10 Y1 E2 F600\n").unwrap();
    for body in [
        "G1X10Y1E2F600",
        "g01x10y1e2f600",
        "N42 G01 X10 Y1 E2 F600*0",
        "n42g01x10y1e2f600*255",
        "G1 X10 (ignored E99) Y1 E2 F600 ; ignored X99*0",
        "G1 X10 (ignored ; E99) Y1 E2 F600",
    ] {
        let actual = parse_foreign(&format!("G90\nM83\nG1 X0 Y0 Z0.2\n{body}\n"))
            .unwrap_or_else(|error| panic!("{body}: {error}"));
        assert_eq!(actual, plain, "{body}");
    }
}

#[test]
fn full_circle_without_endpoint_axes_is_not_dropped() {
    let preview = parse_foreign("G90\nM83\nG1 X10 Y0 Z0.2 F600\nG3 I-10 J0 E1\n").unwrap();
    assert!(preview.moves.len() > 50);
    assert!((preview.print_distance_mm - std::f64::consts::TAU * 10.0).abs() < 0.1);
    let last = preview.moves.last().unwrap();
    assert_eq!([last.x, last.y, last.z], [10.0, 0.0, 0.2]);
    assert_eq!(preview.extrusion_mm, 1.0);
}

#[test]
fn expanded_arc_points_share_the_output_move_budget() {
    let mut program = String::from("G90\nM83\nG1 X20 Y0 Z0.2 F600\n");
    for _ in 0..1_562 {
        program.push_str("G2 X20 Y0 I-20 E0.001\n");
    }
    for _ in 0..31 {
        program.push_str("G1 X20\n");
    }
    assert_eq!(parse_foreign(&program).unwrap().moves.len(), MAX_MOVES);
    for extra in ["G1 X20\n", "G2 X20 Y0 I-20 E0.001\n"] {
        let error = match parse_foreign(&format!("{program}{extra}")) {
            Err(error) => error,
            Ok(preview) => panic!("Accepted {} output moves", preview.moves.len()),
        };
        assert_eq!(error.code, "GCODE_MOVE_LIMIT");
        assert!(error.message.starts_with("Line 1597:"), "{error}");
    }
}

#[test]
fn word_layouts_preserve_modal_preview_and_layer_identity() {
    for seed in 0..32 {
        let mut spaced = String::from("G90\nM83\nG1 X0 Y0 Z0.2 F600\n");
        let mut compact = spaced.clone();
        let mut comments = spaced.clone();
        for i in 0..64 {
            if i % 16 == 0 {
                for program in [&mut spaced, &mut compact, &mut comments] {
                    program.push_str(&format!(";LAYER:{}\n", i / 16));
                }
            }
            let x = (i * 13 + seed) % 31;
            let y = (i * 7 + seed * 3) % 23;
            let z = 0.2 * (1 + i / 16) as f64;
            spaced.push_str(&format!("G1 X{x} Y{y} Z{z} E0.125 F600\n"));
            compact.push_str(&format!("n{i}g01x{x}y{y}z{z}e0.125f600*0\n"));
            comments.push_str(&format!("G01 X{x} (a;(b)*0) Y{y} Z{z} E0.125 F600; end\n"));
        }
        let expected = parse_foreign(&spaced).unwrap();
        assert_eq!(parse_foreign(&compact).unwrap(), expected);
        assert_eq!(parse_foreign(&comments).unwrap(), expected);
    }
}

#[test]
fn tolerant_reader_keeps_numeric_and_input_limits() {
    for (line, code) in [
        ("G1 Xabc", "GCODE_INVALID_NUMBER"),
        ("G1 X", "GCODE_SYNTAX"),
        ("G1 X1.2.3", "GCODE_INVALID_NUMBER"),
        ("G1 X1000001", "GCODE_INVALID_COORDINATE"),
        ("G1 F0", "GCODE_INVALID_FEEDRATE"),
        ("G2 I0 J0", "GCODE_INVALID_COORDINATE"),
        ("G3 R10", "GCODE_INVALID_COORDINATE"),
    ] {
        let error = parse_foreign(&format!("G1 X0 Y0 Z0\n{line}\n")).unwrap_err();
        assert_eq!(error.code, code, "{line}");
        assert!(error.message.starts_with("Line 2:"));
    }
    let long_line = format!(";{}", "x".repeat(gcode_core::MAX_LINE_BYTES));
    assert_eq!(
        parse_foreign(&long_line).unwrap_err().code,
        "GCODE_LINE_LIMIT"
    );
    let long_file = "\n".repeat(gcode_core::MAX_OUTPUT_BYTES + 1);
    assert_eq!(
        parse_foreign(&long_file).unwrap_err().code,
        "GCODE_OUTPUT_LIMIT"
    );
}
