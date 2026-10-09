use value_codec::{Value, json};
fn main() {
    let output = std::env::args().nth(1).expect("output path");
    let cube = brep_core::cuboid([0.; 3], [10., 20., 30.]).unwrap();
    let enclosure = brep_core::operations::boolean(
        &brep_core::cuboid([0.; 3], [40., 30., 20.]).unwrap(),
        &brep_core::cuboid([1.4, 1.4, 2.], [38.6, 28.6, 22.]).unwrap(),
        "difference",
    )
    .unwrap();
    let cylinder = brep_core::cylinder(5., 6.).unwrap();
    let mut cases = Vec::new();
    for (name, model, origin, direction, pairs, controls, converged) in [
        (
            "box-minimum",
            &cube,
            [-2., 10., 15.],
            [14., 0., 0.],
            10000,
            10000,
            true,
        ),
        (
            "box-long-witness",
            &cube,
            [5., -2., 15.],
            [0., 24., 0.],
            10000,
            10000,
            false,
        ),
        (
            "enclosure-minimum",
            &enclosure,
            [-2., 15., 10.],
            [3.5, 0., 0.],
            10000,
            10000,
            true,
        ),
        (
            "curved-self-unresolved",
            &cylinder,
            [-7., -7., 3.],
            [14., 14., 0.],
            10000,
            10000,
            false,
        ),
        (
            "face-pair-limit",
            &cube,
            [-2., 10., 15.],
            [14., 0., 0.],
            1,
            10000,
            false,
        ),
        (
            "plane-control-limit",
            &cube,
            [-2., 10., 15.],
            [14., 0., 0.],
            10000,
            1,
            false,
        ),
        (
            "oblique-witness",
            &cube,
            [-2., 2., 15.],
            [14., 8., 0.],
            10000,
            10000,
            false,
        ),
    ] {
        let request: Value = json!({"op":"cad_whole_wall","model":model,"origin":origin,"direction":direction,
            "toleranceUv":1e-7,"toleranceMm":1e-5,"maxDistanceCells":10000,"maxDistanceDomainCells":1000000,
            "normalAudit":{"maxSineSquared":1e-6,"maxSpans":100},
            "coverageLimits":{"maxFacePairs":pairs,"maxPlaneControls":controls,"maxNormalSpans":10000},
            "limits":{"pointCells":100000,"pointDomainCells":1000000,"segmentCells":100000,"segmentDomainCells":1000000,
            "validity":{"exactWork":1000000,"trimPairs":10000,"trimCells":100000,"trimDomainCells":1000000,"spans":4096,
                "facePairs":10000,"faceCells":150000,"faceDomainCells":1500000,"faceCellsPerPair":1024,"faceDomainCellsPerPair":100000,
                "nestingPairs":100,"nestingCells":100000,"nestingDomainCells":1000000,"orientationCells":100000,"orientationDomainCells":1000000,"orientationSpans":100}}});
        let result = geometry_bridge::dispatch(request.clone()).unwrap();
        assert_eq!(result["converged"], json!(converged), "{name}");
        cases.push(json!({"name":name,"request":request,"result":result}));
    }
    std::fs::write(
        output,
        value_codec::to_string(&json!({"cases":cases})).unwrap(),
    )
    .unwrap();
}
