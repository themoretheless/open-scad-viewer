//! Fresh native protocol/material/STEP receipt for miter-hollow-body.r data.
//! This is not a WASM, Rush, viewport or UI timeout qualification.
use geometry_bridge::dispatch;
use std::{path::PathBuf, time::Instant};
use value_codec::{Value, json};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = PathBuf::from(std::env::args().nth(1).expect("Output directory required"));
    std::fs::create_dir_all(&directory)?;
    let outer = nurbs_core::primitives::circle([0.; 3], [0., 0., 1.], 0.5)?;
    let hole = nurbs_core::primitives::circle([0.; 3], [0., 0., -1.], 0.2)?;
    let request = json!({"op":"brep_miter_annular_owned_construct","direct":true,
        "loops":[[outer],[hole]],"points":[[0.,0.,0.],[0.,0.,10.],[10.,0.,10.],[10.,10.,15.]],
        "normal":[1.,0.,0.],"closed":false,"miterLimit":2.,"smoothnessMaxWork":2000000});
    let start = Instant::now();
    let body = dispatch(request.clone())?;
    let construct_ms = start.elapsed().as_secs_f64() * 1000.;
    assert!(body["budget"].is_null());
    assert!(body["boundaryCertificate"]["withinBudget"].is_null());
    assert_eq!(body["boundaryCertificate"]["continuousBound"], true);
    assert_eq!(
        body["capFaceGroups"],
        json!([[24, 25, 26, 27], [28, 29, 30, 31]])
    );
    assert_eq!(body["solidGeometryCertified"], false);
    println!(
        "Native source construction complete ({construct_ms:.2} ms); auditing actual material"
    );
    let start = Instant::now();
    // Same finite budgets as DEFAULT_SWEEP_VOLUME_BUDGETS. Empty planar cap
    // selection leaves every actual face and all496 pairs to the native audit.
    let volume = dispatch(
        json!({"op":"brep_sweep_volume_audit","model":body["model"],"capFaces":[],
        "toleranceUv":1e-8,"maxExactWork":1000000,"maxTrimPairs":10000,"maxTrimCells":100000,
        "maxTrimDomainCells":1000000,"maxSpans":1024,"maxLinearCells":100000,
        "maxPairs":20000,"maxCells":200000,"maxDomainCells":1000000,"cellsPerPair":1000,"domainCellsPerPair":10000,
        "nestingPairs":1000,"nestingCells":100000,"nestingDomainCells":1000000,
        "orientationCells":100000,"orientationDomainCells":1000000,"orientationSpans":100,
        "capBudgets":{"maxWalls":1024,"maxExactWork":1000000,"maxChartCells":1000,
            "maxTrimPairs":100000,"maxTrimCells":100000,"maxTrimDomainCells":1000000}}),
    )?;
    let volume_ms = start.elapsed().as_secs_f64() * 1000.;
    assert_eq!(volume["solidGeometryCertified"], true);
    assert_eq!(volume["boundary"]["allFacesInjective"], true);
    assert_eq!(volume["boundary"]["allPairsClassified"], true);
    assert_eq!(volume["boundary"]["totalPairs"], 496);
    assert!(volume["boundary"]["nextPair"].is_null());
    let model: brep_core::Model = value_codec::from_value(body["model"].clone())?;
    let (step, certificate, _) = brep_core::export_step_v5(&model)?;
    assert!(certificate.complete);
    std::fs::write(directory.join("unbudgeted-source-annular.step"), step)?;
    let receipt: Value = json!({"schema":"native-unbudgeted-annular-source/1",
        "scope":"Native source protocol, actual material and STEP; not WASM/Rush/UI performance qualification",
        "request":request,"body":body,"volume":volume,"constructMs":construct_ms,"volumeMs":volume_ms,
        "stepCapability":certificate.capability,"stepComplete":certificate.complete});
    std::fs::write(
        directory.join("native-source.json"),
        value_codec::to_string(&receipt)?,
    )?;
    println!("Actual material all496pairs and complete STEP passed ({volume_ms:.2} ms)");
    Ok(())
}
