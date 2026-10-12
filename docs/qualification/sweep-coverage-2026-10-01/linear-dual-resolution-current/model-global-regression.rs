#[test]
fn original_periodic_fixed_normal_affine_hollow_requires_complete_global_cover() {
    let model:brep_core::Model=value_codec::from_str(include_str!("fixtures/periodic-fixed-normal-affine-hollow-model.json")).unwrap();
    assert_eq!(model.faces.len(),256);assert_eq!(model.shells.len(),2);
    let result=dispatch(json!({"op":"brep_sweep_volume_charts_audit","model":model,"capFaces":[],
        "toleranceUv":1e-8,"maxExactWork":1000000,"maxTrimPairs":10000,"maxTrimCells":100000,"maxTrimDomainCells":1000000,
        "maxSpans":1024,"maxLinearCells":100000,"maxPairs":20000,"maxCells":200000,"maxDomainCells":1000000,
        "cellsPerPair":1000,"domainCellsPerPair":10000,"nestingPairs":1000,"nestingCells":100000,"nestingDomainCells":1000000,
        "orientationCells":100000,"orientationDomainCells":1000000,"orientationSpans":100,
        "capBudgets":{"maxWalls":1024,"maxExactWork":1000000,"maxChartCells":1000,"maxTrimPairs":100000,"maxTrimCells":100000,"maxTrimDomainCells":1000000}})).unwrap();
    if let Ok(path)=std::env::var("SWEEP_DUAL_GLOBAL_DUMP") {
        std::fs::write(path,value_codec::to_string(&result).unwrap()).unwrap();
    }
    eprintln!("original affine hollow: charts={}, unresolved={}, pairs={}, cells={}, domainCells={}",
        result["retainedWallCharts"]["allChartsCertified"],result["retainedWallCharts"]["unresolvedFaces"],
        result["allPairsClassified"],result["contactCells"],result["contactDomainCells"]);
    assert_eq!(result["retainedWallCharts"]["allChartsCertified"],true,"{}",result["retainedWallCharts"]["unresolvedFaces"]);
    assert_eq!(result["allFacesInjective"],true);assert_eq!(result["allPairsClassified"],true);
    assert_eq!(result["solidGeometryCertified"],true);
}
