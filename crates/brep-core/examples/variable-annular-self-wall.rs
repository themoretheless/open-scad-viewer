//! Original full-chart exclusion of aligned same-face chords.
//! Distinct face coverage and material volume remain separate requirements.
fn main() {
    let output = std::env::args().nth(1).expect("output JSON path");
    let adaptive_enabled = std::env::args().nth(2).as_deref() == Some("adaptive");
    let minimum_mm = 4.74999;
    let mut cases = Vec::new();
    for radii in [[0.5, 1.25], [1.25, 0.5]] {
        for direction in [1., -1.] {
            let model = brep_core::circular_blend::partial_annular_variable_arc(
                20.,
                5.,
                6.,
                radii,
                [0.3, 0.7, 0.4].map(|x| x * direction),
                1e-7,
            )
            .unwrap();
            let before = value_codec::to_string(&model).unwrap();
            let mut faces = Vec::new();
            for (face, f) in model.faces.iter().enumerate() {
                let r = nurbs_core::surface_self_chord::qualify(&f.surface, 1e-4, 4096).unwrap();
                let adaptive = if adaptive_enabled && r.certificate.is_none() {
                    let a = nurbs_core::surface_self_chord_coverage::qualify(
                        &f.surface,
                        minimum_mm,
                        1e-4,
                        nurbs_core::surface_self_chord_coverage::Limits {
                            cells: 256,
                            spans: 2048,
                        },
                    )
                    .unwrap();
                    Some(
                        value_codec::json!({"qualified":a.certificate.is_some(),"minimumMm":minimum_mm,"cells":a.cells,"spans":a.spans,"pending":a.pending,"uncertain":a.uncertain,"intrinsicExclusions":a.intrinsic_exclusions,"angularExclusions":a.angular_exclusions,"distanceExclusions":a.distance_exclusions,"reason":a.reason}),
                    )
                } else {
                    None
                };
                faces.push(value_codec::json!({"face":face,"qualified":r.certificate.is_some(),"complete":r.complete,"cosineSquaredUpper":r.cosine_squared_upper,"spans":r.spans,"reason":r.reason,"adaptive":adaptive}));
            }
            assert_eq!(value_codec::to_string(&model).unwrap(), before);
            cases.push(value_codec::json!({"radii":radii,"direction":direction,"faces":faces}));
        }
    }
    std::fs::write(output,value_codec::to_string_pretty(&value_codec::json!({"schema":"cad-annular-self-wall/1","scope":"Full original chart intrinsic self-chord exclusion; absence of a certificate remains unresolved, not a thin witness or global wall certificate","maxSineSquared":1e-4,"cases":cases})).unwrap()).unwrap();
}
