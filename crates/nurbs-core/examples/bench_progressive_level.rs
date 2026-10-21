//! Time an unchanged captured progressive request and its arc-length divisions.
#[cfg(feature = "transport")]
fn main() {
    use std::{fs, time::Instant};
    use value_codec::{Value, json};
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(
        args.len(),
        2,
        "Supply captured request JSON and output JSON"
    );
    let request: Value = value_codec::from_str(&fs::read_to_string(&args[0]).unwrap()).unwrap();
    assert_eq!(request["op"], json!("surface_progressive_sweep_level"));
    let path: nurbs_core::curve::Curve = value_codec::from_value(request["path"].clone()).unwrap();
    let sections = request["preview_sections"].as_u64().unwrap() as usize;
    let tolerance = request["length_tolerance"].as_f64().unwrap();
    let max_cells = request["length_max_cells"].as_u64().unwrap() as usize;
    let mut divisions = vec![];
    for count in [sections, 4 * (sections - 1) + 1] {
        let start = Instant::now();
        let result =
            nurbs_core::curve_measure::divide_by_length(&path, count - 1, tolerance, max_cells)
                .unwrap();
        divisions.push(
            json!({"count":count,"milliseconds":start.elapsed().as_secs_f64()*1000.,
            "cells":result.cells,"withinTolerance":result.within_tolerance}),
        );
    }
    let mut levels = vec![];
    for _ in 0..2 {
        let start = Instant::now();
        let result = nurbs_core::dispatch(request.clone()).unwrap();
        levels.push(json!({"milliseconds":start.elapsed().as_secs_f64()*1000.,"result":result}));
    }
    assert_eq!(levels[0]["result"], levels[1]["result"]);
    let report = json!({"scope":"Native diagnostic only; unchanged captured request and full response equality",
        "divisions":divisions,"levels":levels});
    fs::write(&args[1], value_codec::to_string(&report).unwrap()).unwrap();
    println!("{}", value_codec::to_string(&json!({"divisions":report["divisions"],
        "levelMilliseconds":[report["levels"][0]["milliseconds"],report["levels"][1]["milliseconds"]]})).unwrap());
}

#[cfg(not(feature = "transport"))]
fn main() {
    panic!("Enable the transport feature");
}
