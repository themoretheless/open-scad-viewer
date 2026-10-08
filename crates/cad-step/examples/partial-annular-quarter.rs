use brep_core::circular_blend::partial_annular_quarter;
use cad_step::export_step_v9;

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
        let diagnostic = brep_core::self_intersection::inspect(
            &model,
            1e-8,
            4096,
            brep_core::face_contacts::Limits {
                pairs: 1000,
                cells: 20000,
                domain_cells: 200000,
                cells_per_pair: 64,
                domain_cells_per_pair: 1000,
            },
        )
        .unwrap();
        let report = value_codec::json!({
            "absenceProven": diagnostic.absence_proven,
            "allFacesInjective": diagnostic.faces.all_faces_injective,
            "allPairsClassified": diagnostic.pairs.all_pairs_classified,
            "totalPairs": diagnostic.pairs.total_pairs,
            "visitedPairs": diagnostic.pairs.pairs.len(),
            "nextPair": diagnostic.pairs.next_pair,
            "faces": diagnostic.faces.faces.iter().map(|face| value_codec::json!({
                "face": face.face,
                "proven": face.result.as_ref().is_some_and(|r| r.proven),
                "reason": face.result.as_ref().map(|r| r.reason),
            })).collect::<Vec<_>>(),
            "pairs": diagnostic.pairs.pairs.iter().map(|pair| value_codec::json!({
                "faces": pair.faces, "reason": pair.reason,
            })).collect::<Vec<_>>(),
        });
        std::fs::write(
            format!("{directory}/{name}-diagnostic.json"),
            value_codec::to_string_pretty(&report).unwrap(),
        )
        .unwrap();
        println!("{name}: absence proven = {}", diagnostic.absence_proven);
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
