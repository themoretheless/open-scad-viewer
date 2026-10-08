//! Original full-chart exclusion of aligned same-face chords.
//! Distinct face coverage and material volume remain separate requirements.
fn save_completed_cases(output: &str, cases: &[value_codec::Value]) {
    let report = value_codec::json!({
        "schema":"cad-annular-self-wall/2",
        "unresolvedPairsRepresentEndpointExchange":true,
        "scope":"Full original chart intrinsic self-chord exclusion; absence of a certificate remains unresolved, not a thin witness or global wall certificate",
        "maxSineSquared":1e-4,
        "completeAudit":cases.len()==4,
        "completedCases":cases.len(),
        "expectedCases":4,
        "cases":cases
    });
    let temporary = format!("{output}.tmp");
    std::fs::write(&temporary,value_codec::to_string_pretty(&report).unwrap()).unwrap();
    std::fs::rename(&temporary,output).unwrap();
}
fn main() {
    let output = std::env::args().nth(1).expect("output JSON path");
    let adaptive_enabled = std::env::args().nth(2).as_deref() == Some("adaptive");
    let minimum_mm = 4.74999;
    let rational_enabled = std::env::args().nth(5).as_deref() == Some("rational");
    let factored_enabled =
        rational_enabled || std::env::args().nth(5).as_deref() == Some("factored");
    let adaptive_cells: usize = std::env::args()
        .nth(3)
        .map(|s| s.parse().unwrap())
        .unwrap_or(256);
    assert!((1..=100000).contains(&adaptive_cells));
    let adaptive_spans = (adaptive_cells * 8).min(100000);
    let selected: Option<Vec<usize>> = std::env::args()
        .nth(4)
        .map(|s| s.split(',').map(|x| x.parse().unwrap()).collect());
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
                if selected.as_ref().is_some_and(|ids| !ids.contains(&face)) {
                    continue;
                }
                let face_started = std::time::Instant::now();
                let r = nurbs_core::surface_self_chord::qualify(&f.surface, 1e-4, 4096).unwrap();
                let adaptive = if adaptive_enabled && r.certificate.is_none() {
                    let prepared = if factored_enabled && (face == 0 || face == 10) {
                        Some(
                            nurbs_core::factored_surface_normals::prepare(
                                &f.surface,
                                face == 10,
                                100_000_000,
                            )
                            .unwrap(),
                        )
                    } else {
                        None
                    };
                    let differential = if rational_enabled && (face == 0 || face == 10) {
                        Some(
                            nurbs_core::rational_pole_differential::prepare(
                                &f.surface,
                                face == 10,
                                100_000_000,
                            )
                            .unwrap(),
                        )
                    } else {
                        None
                    };
                    let limits = nurbs_core::surface_self_chord_coverage::Limits {
                        cells: adaptive_cells,
                        spans: adaptive_spans,
                    };
                    let a = if let (Some(normals), Some(differential)) = (
                        prepared.as_ref().and_then(|p| p.certificate.as_ref()),
                        differential.as_ref().and_then(|p| p.certificate.as_ref()),
                    ) {
                        nurbs_core::surface_self_chord_coverage::qualify_with_rational_pole(
                            &f.surface,
                            minimum_mm,
                            1e-4,
                            limits,
                            normals,
                            differential,
                        )
                    } else if let Some(proof) =
                        prepared.as_ref().and_then(|p| p.certificate.as_ref())
                    {
                        nurbs_core::surface_self_chord_coverage::qualify_with_factored_normals(
                            &f.surface, minimum_mm, 1e-4, limits, proof,
                        )
                    } else {
                        nurbs_core::surface_self_chord_coverage::qualify(
                            &f.surface, minimum_mm, 1e-4, limits,
                        )
                    }
                    .unwrap();
                    eprintln!(
                        "radii={radii:?} direction={direction} face={face} qualified={} cells={} spans={} pending={} direction_exclusions={} coordinate_exclusions={} coordinate_attempts={} coordinate_refusals={} local_exclusions={} local_attempts={} local_refusals={} factored_angular={} reason={}",
                        a.certificate.is_some(), a.cells, a.spans, a.pending,
                        a.direction_exclusions, a.coordinate_path_exclusions, a.coordinate_path_attempts, a.coordinate_path_refusals, a.local_coordinate_path_exclusions, a.local_coordinate_path_attempts, a.local_coordinate_path_refusals, a.factored_angular_exclusions, a.reason,
                    );
                    Some(
                        value_codec::json!({"qualified":a.certificate.is_some(),"minimumMm":minimum_mm,"cells":a.cells,"spans":a.spans,"pending":a.pending,"uncertain":a.uncertain,"unresolvedPairs":a.unresolved_pairs,"intrinsicExclusions":a.intrinsic_exclusions,"polarExclusions":a.polar_exclusions,"coordinatePathExclusions":a.coordinate_path_exclusions,"coordinatePathAttempts":a.coordinate_path_attempts,"coordinatePathRefusals":a.coordinate_path_refusals,"localCoordinatePathExclusions":a.local_coordinate_path_exclusions,"localMarginExclusions":a.local_margin_exclusions,"localCoordinatePathAttempts":a.local_coordinate_path_attempts,"localCoordinatePathRefusals":a.local_coordinate_path_refusals,"polarFactorRefusals":a.polar_factor_refusals,"angularExclusions":a.angular_exclusions,"factoredAngularExclusions":a.factored_angular_exclusions,"distanceExclusions":a.distance_exclusions,"directionExclusions":a.direction_exclusions,"meanMatrixExclusions":a.mean_matrix_exclusions,"completeRectangleCacheHits":a.complete_rectangle_cache_hits,"rationalDifferentialWork":differential.as_ref().map(|p|p.work),"rationalDifferentialQualified":differential.as_ref().map(|p|p.certificate.is_some()),"rationalDifferentialReason":differential.as_ref().map(|p|p.reason),"factoredNormalWork":prepared.as_ref().map(|p|p.work),"factoredNormalQualified":prepared.as_ref().map(|p|p.certificate.is_some()),"reason":a.reason}),
                    )
                } else {
                    None
                };
                faces.push(value_codec::json!({"face":face,"elapsedMs":face_started.elapsed().as_secs_f64()*1000.,"qualified":r.certificate.is_some(),"complete":r.complete,"cosineSquaredUpper":r.cosine_squared_upper,"spans":r.spans,"reason":r.reason,"adaptive":adaptive}));
            }
            assert_eq!(value_codec::to_string(&model).unwrap(), before);
            cases.push(value_codec::json!({"radii":radii,"direction":direction,"sourceFacesTotal":model.faces.len(),"selectedFaces":selected,"adaptiveCellsPerFace":adaptive_cells,"adaptiveSpansPerFace":adaptive_spans,"faces":faces}));
            save_completed_cases(&output, &cases);
        }
    }

}
