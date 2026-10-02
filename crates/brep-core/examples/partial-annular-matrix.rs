use brep_core::{circular_blend::partial_annular_arc, export_step_v9};

fn main() {
    let directory = std::env::args().nth(1).expect("output directory");
    std::fs::create_dir_all(&directory).unwrap();
    let mut cases = Vec::new();
    let angles = std::env::args().nth(2).as_deref() == Some("angles");
    let scales = if angles { vec![1.] } else { vec![0.5, 1., 2.] };
    let radii = if angles {
        vec![1.25]
    } else {
        vec![0.25, 1.25, 2.5]
    };
    let sweeps = if angles {
        vec![0.31, std::f64::consts::PI / 3., std::f64::consts::PI]
    } else {
        vec![std::f64::consts::FRAC_PI_2]
    };
    for scale in scales {
        for &base_radius in &radii {
            for &angle in &sweeps {
                for direction in [-1., 1.] {
                    let parameters = [20. * scale, 5. * scale, 6. * scale, base_radius * scale];
                    let [outer, inner, height, radius] = parameters;
                    let model =
                        partial_annular_arc(outer, inner, height, radius, direction * angle, 1e-7)
                            .unwrap();
                    let name = format!("case-{}", cases.len());
                    let (step, _, _) = export_step_v9(&model).unwrap();
                    std::fs::write(format!("{directory}/{name}.step"), step).unwrap();
                    cases.push(value_codec::json!({"name": name, "parameters": parameters, "direction": direction, "sweep": direction*angle}));
                }
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
