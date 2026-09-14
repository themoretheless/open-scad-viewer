use gcode_core::{emit, parse, parse_fdm, MachineProfile, PlannedLayer, PlannedPath};

fn main() {
    let layer = PlannedLayer {
        z_mm: 0.2,
        paths: vec![PlannedPath {
            points: vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]],
            closed: true,
        }],
    };
    let machine = MachineProfile {
        hotend_c: 200.0,
        bed_c: 60.0,
        ..MachineProfile::default()
    };
    let gcode = emit(&[layer], &machine).expect("emit");
    let preview = parse(&gcode).expect("preview dialect");
    let foreign = parse_fdm(&gcode).expect("fdm parse");
    println!(
        "moves={} e_mm={:.4} hotend={} bed={}",
        preview.moves.len(),
        preview.extrusion_mm,
        foreign.thermal.hotend_c,
        foreign.thermal.bed_c
    );
}
