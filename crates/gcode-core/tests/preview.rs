use gcode_core::{
    deposited_volume_mm3, emit, lex_program, parse, parse_fdm, parse_fdm_with, DIALECT,
    MachineProfile, PlannedLayer, PlannedPath,
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

#[test]
fn emit_uses_preview_dialect_and_mm_min() {
    let gcode = emit(&[square_layer()], &machine()).unwrap();
    assert!(gcode.starts_with(&format!("; {DIALECT}")));
    assert!(gcode.contains("G90"));
    assert!(gcode.contains("M82"));
    assert!(gcode.contains("F3000.000"));
    assert!(gcode.contains("F7200.000"));
}

#[test]
fn parse_rejects_other_dialects() {
    let error = parse("G21\nG1 X1 Y1 E1\n").unwrap_err();
    assert_eq!(error.code, "GCODE_DIALECT");
}

#[test]
fn preview_round_trips_filament_length_to_volume() {
    let layers = [square_layer()];
    let machine = machine();
    let gcode = emit(&layers, &machine).unwrap();
    let preview = parse(&gcode).unwrap();
    assert_eq!(preview.layers, 1);
    assert!(preview.extrusion_mm > 0.0);
    let volume = deposited_volume_mm3(&layers, &machine);
    let from_e = preview.extrusion_mm * machine.filament_area_mm2();
    assert!((from_e - volume).abs() / volume < 1e-6);
}

#[test]
fn invalid_profile_is_rejected() {
    let machine = MachineProfile {
        filament_diameter_mm: 0.0,
        ..machine()
    };
    assert_eq!(
        emit(&[square_layer()], &machine).unwrap_err().code,
        "GCODE_INVALID_SETTINGS"
    );
}

#[test]
fn emit_writes_temperatures_and_fan() {
    let machine = MachineProfile {
        hotend_c: 210.0,
        bed_c: 60.0,
        chamber_c: 40.0,
        fan_pwm: 128.0,
        ..machine()
    };
    let gcode = emit(&[square_layer()], &machine).unwrap();
    assert!(gcode.contains("M104 S210"));
    assert!(gcode.contains("M109 S210"));
    assert!(gcode.contains("M140 S60"));
    assert!(gcode.contains("M190 S60"));
    assert!(gcode.contains("M141 S40"));
    assert!(gcode.contains("M106 S128"));
    let preview = parse(&gcode).unwrap();
    assert!(preview
        .thermals
        .iter()
        .any(|t| t.hotend_c == 210.0 && t.wait));
    assert!(preview.thermals.iter().any(|t| t.bed_c == 60.0 && t.wait));
    assert!(preview.thermals.iter().any(|t| t.chamber_c == 40.0));
    assert!(preview.thermals.iter().any(|t| t.fan_pwm == 128.0));
    assert_eq!(preview.thermal.hotend_c, 0.0);
    assert_eq!(preview.thermal.bed_c, 0.0);
    assert_eq!(preview.thermal.fan_pwm, 0.0);
    assert_eq!(preview.thermal.chamber_c, 40.0);
}

#[test]
fn retract_is_not_deposited_volume() {
    let machine = MachineProfile {
        retract_mm: 1.0,
        z_hop_mm: 0.4,
        ..machine()
    };
    let layers = [
        square_layer(),
        PlannedLayer {
            z_mm: 0.4,
            paths: square_layer().paths,
        },
    ];
    let gcode = emit(&layers, &machine).unwrap();
    assert!(gcode.contains("G1 E"));
    let preview = parse(&gcode).unwrap();
    let volume = deposited_volume_mm3(&layers, &machine);
    let from_e = preview.extrusion_mm * machine.filament_area_mm2();
    assert!((from_e - volume).abs() / volume < 1e-6);
}

#[test]
fn parse_fdm_reads_marlin_modes_and_skips_host_commands() {
    let gcode = "\
G21
G90
M83
M104 S200
M140 S50
M109 S200
M73 P1
G1 X10 Y0 E1.5 F1800
G1 X10 Y10 E1.5
M106 S255
T0
";
    let preview = parse_fdm(gcode).unwrap();
    assert!((preview.extrusion_mm - 3.0).abs() < 1e-9);
    assert_eq!(preview.thermal.hotend_c, 200.0);
    assert_eq!(preview.thermal.bed_c, 50.0);
    assert_eq!(preview.thermal.fan_pwm, 255.0);
    assert_eq!(preview.thermal.tool, 0);
    assert!(preview.other_commands >= 1);
}

#[test]
fn canned_cycle_is_rejected() {
    let error = parse_fdm("G81 X10 Y10 Z-1 R1\n").unwrap_err();
    assert_eq!(error.code, "GCODE_UNSUPPORTED");
}

#[test]
fn arc_without_linearize_option_is_rejected() {
    let error = parse_fdm("G21\nG90\nG0 X10 Y0\nG2 X0 Y10 I-10 J0\n").unwrap_err();
    assert_eq!(error.code, "GCODE_UNSUPPORTED_ARC");
}

#[test]
fn arc_ij_is_linearized_when_enabled() {
    let machine = MachineProfile {
        linearize_arcs_mm: Some(0.05),
        ..machine()
    };
    let gcode = "G21\nG90\nG0 X10 Y0\nG2 X0 Y10 I-10 J0\n";
    let preview = parse_fdm_with(gcode, &machine).unwrap();
    let last = preview.moves.last().unwrap();
    assert!((last.x - 0.0).abs() < 1e-6);
    assert!((last.y - 10.0).abs() < 1e-6);
    assert!(preview.moves.len() > 2);
}

#[test]
fn travel_without_e_is_not_extrusion() {
    let preview = parse_fdm("G0 X10 Y0\nG1 X20 Y0\n").unwrap();
    assert_eq!(preview.extrusion_mm, 0.0);
    assert!(preview.moves.iter().all(|m| !m.extruded));
}

#[test]
fn input_and_block_budgets_are_enforced() {
    assert_eq!(
        lex_program("G1 X1\n", 3, 10).unwrap_err().code,
        "GCODE_INPUT_LIMIT"
    );
    assert_eq!(
        lex_program("G1 X1\nG1 X2\n", 64, 1).unwrap_err().code,
        "GCODE_BLOCK_LIMIT"
    );
}

#[test]
fn temperature_does_not_change_deposited_volume() {
    let motion = parse_fdm("G90\nM82\nG1 X10 E1\n").unwrap();
    let heated = parse_fdm("M104 S210\nM140 S60\nM109 S210\nG90\nM82\nG1 X10 E1\n").unwrap();
    assert!((motion.extrusion_mm - heated.extrusion_mm).abs() < 1e-12);
    assert_eq!(heated.thermal.hotend_c, 210.0);
}
