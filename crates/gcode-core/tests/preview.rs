use gcode_core::{
    deposited_volume_mm3, emit, parse, MachineProfile, PlannedLayer, PlannedPath, DIALECT,
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
    let mut machine = machine();
    machine.filament_diameter_mm = 0.0;
    assert_eq!(
        emit(&[square_layer()], &machine).unwrap_err().code,
        "GCODE_INVALID_SETTINGS"
    );
}
