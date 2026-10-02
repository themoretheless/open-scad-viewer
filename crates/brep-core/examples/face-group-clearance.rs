//! Complete selected-face unions; not a global material-thickness certificate.
use value_codec::json;
fn main() {
    let path = std::env::args().nth(1).expect("output JSON path");
    let model =
        brep_core::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
    let before = format!("{model:?}");
    let mut groups = Vec::new();
    for (name, a, b, expected) in [
        (
            "blend-to-entire-inner-wall",
            vec![0, 5, 10],
            vec![3, 8, 13, 17, 21, 25],
            13.75,
        ),
        (
            "blend-to-entire-bottom",
            vec![0, 5, 10],
            vec![4, 9, 14, 18, 22, 26],
            4.75,
        ),
        (
            "retained-outer-to-entire-inner-wall",
            vec![2, 7, 12, 16, 20, 24],
            vec![3, 8, 13, 17, 21, 25],
            15.,
        ),
    ] {
        let started = std::time::Instant::now();
        let r = brep_core::shell_distance::distance_between_face_sets(
            &model, &a, &model, &b, 1e-5, 1e-8, 10000, 1000000,
        )
        .unwrap();
        let passed = r.converged
            && r.lower_bound_mm <= expected
            && r.upper_bound_mm.is_some_and(|hi| hi >= expected);
        groups.push(json!({"name":name,"originalFaceSets":[a,b],"expectedMm":expected,"passed":passed,"pairs":r.pairs,"evaluatedPairs":r.evaluated_pairs,
            "distanceIntervalMm":[r.lower_bound_mm,r.upper_bound_mm],"faces":r.faces,"converged":r.converged,"reason":r.reason,
            "cells":r.cells,"domainCells":r.domain_cells,"witness":r.witness.map(|w|w.to_value()),"elapsedMs":started.elapsed().as_secs_f64()*1000.}));
    }
    assert_eq!(format!("{model:?}"), before);
    let passed = groups.iter().all(|g| g["passed"] == json!(true));
    std::fs::write(path,value_codec::to_string_pretty(&json!({"scope":"selected-trimmed-face-unions","sourceModel":model,"passed":passed,"groups":groups})).unwrap()).unwrap();
    if !passed {
        std::process::exit(1);
    }
}
