use gcode_core::{emit, parse_fdm, MachineProfile, PlannedLayer, PlannedPath, Units};

fn main() {
    let layers = [PlannedLayer {
        z_mm: 0.2,
        paths: vec![PlannedPath { points: vec![[10.0, 0.0], [20.0, 0.0]], closed: false }],
    }];
    let absolute = emit(&layers, &MachineProfile::default()).unwrap();
    let relative = emit(&layers, &MachineProfile { xyz_absolute: false, ..MachineProfile::default() }).unwrap();
    let inches = emit(&layers, &MachineProfile { units: Units::Inches, ..MachineProfile::default() }).unwrap();
    let template = emit(&layers, &MachineProfile { start_gcode: "G91".into(), ..MachineProfile::default() }).unwrap();
    assert!(relative.contains("G91\n"));
    assert!(inches.contains("G20\n"));
    for output in [&absolute, &relative, &inches, &template] {
        assert!(output.contains("G0 X10.00000 Y0.00000 F7200.000"));
        assert!(output.contains("G1 X20.00000 Y0.00000 E"));
    }
    // The default epilogue homes after printing; inspect the last deposited move.
    let final_x = |text: &str| parse_fdm(text).unwrap().moves.iter().rev().find(|m| m.extruded).unwrap().x;
    assert_eq!(final_x(&absolute), 20.0);
    assert_eq!(final_x(&relative), 30.0);
    assert_eq!(final_x(&inches), 508.0);
    assert_eq!(final_x(&template), 30.0);
    let invalid_layers = [PlannedLayer {
        z_mm: 0.2,
        paths: vec![PlannedPath { points: vec![[0.0, 0.0], [f64::NAN, 1.0]], closed: false }],
    }];
    let invalid_output = emit(&invalid_layers, &MachineProfile::default()).unwrap();
    assert!(invalid_output.contains("NaN"));
    let start = "G90\nM82\nG1 X10 E5\n";
    let control = parse_fdm(&format!("{start}G1 X20 E6\n")).unwrap();
    let retract = parse_fdm(&format!("{start}G10\nG1 X20 E6\n")).unwrap();
    let recovered = parse_fdm(&format!("{start}G10\nG11\nG1 X20 E6\n")).unwrap();
    let reprap = parse_fdm(&format!("{start}G10 P0 S210 R210\nG1 X20 E6\n")).unwrap();
    let cooling = parse_fdm("M104 S210\nM109 R180\n").unwrap();
    let targeted = parse_fdm("T0\nM104 T1 S210\n").unwrap();
    let switched = parse_fdm("T0\nM104 T1 S210\nT0\n").unwrap();
    assert_eq!(control.extrusion_mm, 6.0);
    assert_eq!(retract.extrusion_mm, 7.0);
    assert_eq!(recovered.extrusion_mm, 6.0);
    assert_eq!(reprap.extrusion_mm, 7.0);
    assert_eq!(cooling.thermal.hotend_c, 0.0);
    assert!(cooling.thermal.wait);
    assert_eq!(targeted.thermal.tool, 1);
    assert_eq!(switched.thermal.tool, 0);
    assert_eq!(switched.thermal.hotend_c, 210.0);
    println!("{{\"emitterAbsoluteFinalX\":{},\"emitterRelativeFinalX\":{},\"emitterInchesFinalX\":{},\"emitterTemplateFinalX\":{},\"emitterAcceptsNan\":true,\"controlExtrusionMm\":{},\"retractExtrusionMm\":{},\"recoveredExtrusionMm\":{},\"reprapTemperatureExtrusionMm\":{},\"coolingTargetC\":{},\"targetedToolField\":{},\"switchedTool\":{},\"switchedHotendC\":{}}}",
        final_x(&absolute), final_x(&relative), final_x(&inches), final_x(&template),
        control.extrusion_mm, retract.extrusion_mm, recovered.extrusion_mm,
        reprap.extrusion_mm, cooling.thermal.hotend_c, targeted.thermal.tool,
        switched.thermal.tool, switched.thermal.hotend_c);
}
