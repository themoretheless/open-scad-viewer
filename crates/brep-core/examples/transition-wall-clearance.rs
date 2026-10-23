//! Selected opposing-face clearances; not a whole-body thickness certificate.
use value_codec::json;
fn main() {
    let path = std::env::args().nth(1).expect("output JSON path");
    let model =
        brep_core::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
    let before = format!("{model:?}");
    let mut cases = Vec::new();
    for (name, faces, expected) in [
        ("entry-inner", [0, 3], 13.75),
        ("constant-inner", [5, 8], 13.75),
        ("exit-inner", [10, 13], 13.75),
        ("entry-bottom", [0, 4], 4.75),
        ("constant-bottom", [5, 9], 4.75),
        ("exit-bottom", [10, 14], 4.75),
    ] {
        let started = std::time::Instant::now();
        let r = brep_core::face_domain::distance_between_faces(
            &model, faces[0], &model, faces[1], 1e-5, 1e-8, 4096, 100000,
        )
        .unwrap();
        cases.push(json!({"name":name,"faces":faces,"expectedMm":expected,"result":r.to_value(),"elapsedMs":started.elapsed().as_secs_f64()*1000.}));
    }
    assert_eq!(format!("{model:?}"), before);
    std::fs::write(
        path,
        value_codec::to_string(
            &json!({"scope":"selected-opposing-trimmed-faces","sourceModel":model,"cases":cases}),
        )
        .unwrap(),
    )
    .unwrap();
}
