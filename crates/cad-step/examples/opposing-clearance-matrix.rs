use value_codec::json;
fn main() {
    let directory = std::env::args().nth(1).expect("output directory");
    std::fs::create_dir_all(&directory).unwrap();
    let mut specimens = Vec::new();
    for scale in [0.5, 1., 2.] {
        for base_radius in [0.25, 1.25, 2.5] {
            for direction in [-1., 1.] {
                let parameters = [20. * scale, 5. * scale, 6. * scale, base_radius * scale];
                let [outer, inner, height, radius] = parameters;
                let model = brep_core::circular_blend::partial_annular_quarter(
                    outer, inner, height, radius, direction, 1e-7,
                )
                .unwrap();
                let before = format!("{model:?}");
                let mut cases = Vec::new();
                let mut anchors = std::collections::BTreeMap::new();
                for (name, faces, expected) in [
                    ("entry-inner", [0, 3], outer - inner - radius),
                    ("constant-inner", [5, 8], outer - inner - radius),
                    ("exit-inner", [10, 13], outer - inner - radius),
                    ("entry-bottom", [0, 4], height - radius),
                    ("constant-bottom", [5, 9], height - radius),
                    ("exit-bottom", [10, 14], height - radius),
                ] {
                    let r = brep_core::face_domain::distance_between_faces(
                        &model, faces[0], &model, faces[1], 1e-5, 1e-8, 4096, 100000,
                    )
                    .unwrap();
                    let passed = r.converged
                        && r.lower_bound_mm <= expected
                        && r.upper_bound_mm.is_some_and(|hi| hi >= expected);
                    cases.push(json!({"name":name,"faces":faces,"expectedMm":expected,"passed":passed,"result":r.to_value()}));
                    for face in faces {
                        let s = &model.faces[face].surface;
                        let (u, v) = if [4, 9, 14].contains(&face) {
                            let angle = direction
                                * std::f64::consts::PI
                                * match face {
                                    4 => 1. / 16.,
                                    9 => 1. / 4.,
                                    14 => 7. / 16.,
                                    _ => unreachable!(),
                                };
                            let radial = (outer + inner) / 2.;
                            (radial * angle.cos(), radial * angle.sin())
                        } else {
                            (
                                (s.knots_u[s.degree_u] + s.knots_u[s.control_points.len()]) / 2.,
                                (s.knots_v[s.degree_v] + s.knots_v[s.control_points[0].len()]) / 2.,
                            )
                        };
                        anchors.insert(face, s.evaluate(u, v).unwrap().point);
                    }
                }
                assert_eq!(format!("{model:?}"), before);
                let name = format!("case-{}", specimens.len());
                let (step, _, _) = cad_step::export_step_v9(&model).unwrap();
                std::fs::write(format!("{directory}/{name}.step"), step).unwrap();
                specimens.push(json!({"name":name,"parameters":parameters,"direction":direction,"anchors":anchors.into_iter().map(|(face,point)|json!({"face":face,"point":point})).collect::<Vec<_>>(),"cases":cases}));
            }
        }
    }
    let passed = specimens.iter().all(|s| {
        s["cases"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["passed"] == json!(true))
    });
    std::fs::write(format!("{directory}/manifest.json"),value_codec::to_string_pretty(&json!({"schema":"cad-opposing-clearance-matrix/1","passed":passed,"specimens":specimens})).unwrap()).unwrap();
    println!("{} specimens, passed={passed}", specimens.len());
    if !passed {
        std::process::exit(1);
    }
}
