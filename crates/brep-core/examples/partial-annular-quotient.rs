use brep_core::circular_blend::partial_annular_quarter;
fn main() {
    let path = std::env::args().nth(1).expect("report path");
    let mut cases = Vec::new();
    for scale in [0.5, 1., 2.] {
        for radius in [0.25, 1.25, 2.5] {
            for direction in [-1., 1.] {
                let model = partial_annular_quarter(
                    20. * scale,
                    5. * scale,
                    6. * scale,
                    radius * scale,
                    direction,
                    1e-7,
                )
                .unwrap();
                let before = format!("{model:?}");
                for (face, end, projection) in [
                    (0, 0, [[0., direction, 0.], [1., 0., -1.]]),
                    (10, 1, [[1., 0., 0.], [0., direction, -1.]]),
                ] {
                    let proof = nurbs_core::surface_quotient_injectivity::certify(
                        &model.faces[face].surface,
                        end,
                        projection,
                        16,
                        256,
                    )
                    .unwrap();
                    cases.push(value_codec::json!({"scale":scale,"radius":radius*scale,"direction":direction,"face":face,"collapsedEnd":end,"projection":projection,
                "proven":proof.proven,"reason":proof.reason,"cells":proof.cells,"weightedBounds":proof.weighted_bounds,"dominanceMarginLower":proof.dominance_margin_lower,"bandMarginsLower":proof.band_margins_lower}));
                }
                assert_eq!(format!("{model:?}"), before);
            }
        }
    }
    let proven = cases
        .iter()
        .filter(|c| c["proven"].as_bool() == Some(true))
        .count();
    assert_eq!(
        proven,
        cases.len(),
        "Every declared quotient fixture must pass"
    );
    std::fs::write(path,value_codec::to_string_pretty(&value_codec::json!({"schema":"partial-annular-quotient/1","scope":"Within each collapsed transition face; no G1 or distinct-face certificate","provenCases":proven,"cases":cases})).unwrap()).unwrap();
    println!("Proven quotient charts: {proven}/36");
}
