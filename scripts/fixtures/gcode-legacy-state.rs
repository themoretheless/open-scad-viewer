use gcode_core::parse_fdm;

fn main() {
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
    println!("{{\"controlExtrusionMm\":{},\"retractExtrusionMm\":{},\"recoveredExtrusionMm\":{},\"reprapTemperatureExtrusionMm\":{},\"coolingTargetC\":{},\"targetedToolField\":{},\"switchedTool\":{},\"switchedHotendC\":{}}}",
        control.extrusion_mm, retract.extrusion_mm, recovered.extrusion_mm,
        reprap.extrusion_mm, cooling.thermal.hotend_c, targeted.thermal.tool,
        switched.thermal.tool, switched.thermal.hotend_c);
}
