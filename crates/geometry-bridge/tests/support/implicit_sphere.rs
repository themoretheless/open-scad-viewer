use value_codec::{Value, json};
pub fn request(radius: f64, center: [f64; 3]) -> Value {
    json!({"op":"brep_implicit_sphere_source_construct","source":{"center":center,"radius":radius},"toleranceUV":1e-8,
        "limits":{"implicitExactWork":16000000,"implicitCells":100000,"regularityCells":100000,"linearCells":0,
            "volume":{"boundary":{"exactWork":1000000,"trimPairs":10000,"trimCells":100000,"trimDomainCells":1000000,"spans":1000,
                "contacts":{"pairs":10000,"cells":100000,"domainCells":1000000,"cellsPerPair":1000,"domainCellsPerPair":10000}},
                "nestingPairs":100,"nestingCells":100000,"nestingDomainCells":1000000,"orientationCells":100000,"orientationDomainCells":1000000,"orientationSpans":100}}})
}
pub fn full(r: &Value, stage: &str) {
    for key in [
        "certified",
        "solidGeometryCertified",
        "sourceBoundaryExact",
        "continuousBound",
        "topologyPreserved",
        "fullBoundaryG1Certified",
        "fullBoundaryG2Certified",
        "idealSourceBoundaryG1Certified",
        "idealSourceBoundaryG2Certified",
        "quotientVertexLinksComplete",
    ] {
        assert_eq!(
            r[key], true,
            "{stage}/{key}: reason={} regularity={} volume={}",
            r["reason"], r["regularity"], r["volume"]
        );
    }
    assert_eq!(r["boundaryErrorUpper"], 0.);
    assert_eq!(r["seamContinuity"], "G2");
    assert_eq!(r["regularity"]["allChartsRegularModuloPoles"], true);
    assert_eq!(r["volume"]["absenceOfSelfIntersections"], true);
    assert_eq!(r["volume"]["proven"], true);
    for face in r["implicitFaces"].as_array().unwrap() {
        assert_eq!(face["exactMembership"], true);
        assert_eq!(face["gradientNonzero"], true);
        assert!(face["gradientLower"].as_f64().unwrap() > 0.);
    }
}
