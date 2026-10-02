use brep_core::{circular_blend::partial_annular_quarter, export_step_v9};

fn main() {
    let directory = std::env::args().nth(1).expect("output directory");
    std::fs::create_dir_all(&directory).unwrap();
    for direction in [-1., 1.] {
        let model = partial_annular_quarter(20., 5., 6., 1.25, direction, 1e-7).unwrap();
        let name = if direction < 0. {
            "negative"
        } else {
            "positive"
        };
        std::fs::write(
            format!("{directory}/{name}.json"),
            value_codec::to_string(&model).unwrap(),
        )
        .unwrap();
        match export_step_v9(&model) {
            Ok((text, _, _)) => {
                std::fs::write(format!("{directory}/{name}.step"), text).unwrap();
                println!("{name}: STEP exported");
            }
            Err(error) => {
                std::fs::write(
                    format!("{directory}/{name}-refusal.txt"),
                    format!("{error:?}\n"),
                )
                .unwrap();
                println!("{name}: STEP refused: {error:?}");
            }
        }
    }
}
