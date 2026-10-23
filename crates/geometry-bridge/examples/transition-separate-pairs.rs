//! Targeted bounded search, retaining unresolved boxes instead of accepting seeds.
use value_codec::{Value, json};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let fixture: Value =
        value_codec::from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
    let model: brep_core::Model = value_codec::from_value(fixture["sourceModel"].clone()).unwrap();
    model.validate().unwrap();
    let before = format!("{model:?}");
    let domains = model
        .faces
        .iter()
        .map(|f| {
            let loops = std::iter::once(f.outer)
                .chain(f.holes.iter().copied())
                .map(|l| {
                    model.loops[l]
                        .coedges
                        .iter()
                        .map(|c| c.pcurve.clone())
                        .collect()
                })
                .collect::<Vec<Vec<_>>>();
            nurbs_core::trim_domain::TrimDomain::new(&loops, 1e-8).unwrap()
        })
        .collect::<Vec<_>>();
    let mut cases = Vec::new();
    for [a, b] in [[6, 15], [6, 23], [9, 18], [9, 26], [15, 23], [18, 26]] {
        for budget in [256, 1024, 4096] {
            let start = std::time::Instant::now();
            let r = nurbs_core::surface_contact_search::search_trimmed(
                &model.faces[a].surface,
                &model.faces[b].surface,
                [&domains[a], &domains[b]],
                budget,
                1_000_000,
            )
            .unwrap();
            println!(
                "{a}/{b} budget={budget} absence={} cells={} domain={} pending={}",
                r.absence_proven,
                r.cells,
                r.domain_cells,
                r.unresolved.len()
            );
            cases.push(json!({"faces":[a,b],"budget":budget,"absenceProven":r.absence_proven,
                "contact":r.contact.as_ref().map(|w|format!("{w:?}")),"cells":r.cells,
                "domainCells":r.domain_cells,"unresolved":r.unresolved,"elapsedMs":start.elapsed().as_secs_f64()*1000.}));
            if r.absence_proven || r.contact.is_some() {
                break;
            }
        }
    }
    assert_eq!(format!("{model:?}"), before);
    std::fs::write(
        &args[2],
        value_codec::to_string(&json!({"sourceModel":model,"cases":cases})).unwrap(),
    )
    .unwrap();
}
