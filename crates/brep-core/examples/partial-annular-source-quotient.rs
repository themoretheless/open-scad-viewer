use brep_core::{circular_blend::partial_annular_quarter, transform::affine};
fn main() {
    let path = std::env::args().nth(1).expect("report path");
    let mut cases = Vec::new();
    let (sa, ca) = 0.37_f64.sin_cos();
    let (sb, cb) = (-0.61_f64).sin_cos();
    let matrices = [
        [
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
        [
            [ca * cb, -sa, ca * sb, 17.],
            [sa * cb, ca, sa * sb, -9.],
            [-sb, 0., cb, 23.],
            [0., 0., 0., 1.],
        ],
        [
            [0., 0., 1., 7.],
            [1., 0., 0., -3.],
            [0., 1., 0., 11.],
            [0., 0., 0., 1.],
        ],
    ];
    for scale in [0.5, 1., 2.] {
        for radius in [0.25, 1.25, 2.5] {
            for direction in [-1., 1.] {
                let base = partial_annular_quarter(
                    20. * scale,
                    5. * scale,
                    6. * scale,
                    radius * scale,
                    direction,
                    1e-7,
                )
                .unwrap();
                for (placement, matrix) in matrices.iter().enumerate() {
                    let model = affine(&base, *matrix).unwrap();
                    let before = format!("{model:?}");
                    for (face, end) in [(0, 0), (10, 1)] {
                        let r = nurbs_core::surface_quotient_injectivity::certify_source_frame(
                            &model.faces[face].surface,
                            end,
                            16,
                            256,
                        )
                        .unwrap();
                        assert!(
                            r.proven,
                            "scale {scale}, radius {radius}, direction {direction}, placement {placement}, face {face}: {r:?}"
                        );
                        cases.push(value_codec::json!({"scale":scale,"radius":radius*scale,"direction":direction,"placement":placement,"face":face,"collapsedEnd":end,
                    "proven":r.proven,"cells":r.cells,"reason":r.reason,"sourceFrame":r.source_frame,"weightedBounds":r.weighted_bounds,"dominanceMarginLower":r.dominance_margin_lower,"bandMarginsLower":r.band_margins_lower}));
                    }
                    assert_eq!(format!("{model:?}"), before);
                }
            }
        }
    }
    assert_eq!(cases.len(), 108);
    std::fs::write(path,value_codec::to_string_pretty(&value_codec::json!({"schema":"partial-annular-source-quotient/1","scope":"Within each transition face using a frame derived from original controls; no G1 or distinct-face certificate","matrices":matrices,"cases":cases})).unwrap()).unwrap();
    println!("Proven source-frame quotient charts: 108/108");
}
