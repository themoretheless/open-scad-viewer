use brep_core::{circular_blend::partial_annular_quarter, export_step_v9};

fn main() {
    let directory = std::env::args().nth(1).expect("output directory");
    std::fs::create_dir_all(&directory).unwrap();
    let mut cases = Vec::new();
    for scale in [0.5, 1., 2.] {
        for base_radius in [0.25, 1.25, 2.5] {
            for direction in [-1., 1.] {
                let parameters = [20. * scale, 5. * scale, 6. * scale, base_radius * scale];
                let [outer, inner, height, radius] = parameters;
                let model =
                    partial_annular_quarter(outer, inner, height, radius, direction, 1e-7).unwrap();
                let name = format!("case-{}", cases.len());
                let (step, _, _) = export_step_v9(&model).unwrap();
                std::fs::write(format!("{directory}/{name}.step"), step).unwrap();
                cases.push(value_codec::json!({"name": name, "parameters": parameters, "direction": direction}));
            }
        }
    }
    std::fs::write(
        format!("{directory}/manifest.json"),
        value_codec::to_string_pretty(
            &value_codec::json!({"schema": "cad-partial-quarter-matrix/1", "cases": cases}),
        )
        .unwrap(),
    )
    .unwrap();
    println!("Exported {} independent specimens", cases.len());
}
