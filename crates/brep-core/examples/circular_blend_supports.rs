use brep_core::circular_blend::plane_cylinder_transition;
use value_codec::json;
fn main() {
    let mut cases = Vec::new();
    for (r0, r1) in [(0., 1.25), (1.25, 0.), (0.5, 1.25)] {
        for sweep in [0.31, -0.7, 1.5] {
            let span = plane_cylinder_transition(20., 6., r0, r1, 5.9, sweep).unwrap();
            let sheet = span.to_open_sheet(1e-7).unwrap();
            sheet.validate().unwrap();
            cases.push(json!({"outerRadius":20.,"height":6.,"r0":r0,"r1":r1,"start":5.9,"sweep":sweep,"surface":span.surface,"sheet":sheet}));
        }
    }
    println!(
        "{}",
        json!({"schema":"cad-circular-supports/1","cases":cases})
    );
}
