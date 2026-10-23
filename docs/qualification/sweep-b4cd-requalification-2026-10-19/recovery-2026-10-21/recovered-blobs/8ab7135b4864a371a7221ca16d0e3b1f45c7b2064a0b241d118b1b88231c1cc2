//! Read-only inspection of actual supplied retained geometry; no source-bound claim.
fn main()->Result<(),Box<dyn std::error::Error>> {
    let path=std::env::args().nth(1).ok_or("Supply captured model JSON")?;
    let input:value_codec::Value=value_codec::from_str(&std::fs::read_to_string(path)?)?;
    let model:brep_core::Model=value_codec::from_value(input["model"].clone())?;
    let agreement=brep_core::boundary_agreement::verify_exact(&model,1000000)?;
    println!("Actual coedges: {}/{} equal, joins={}, work={}",agreement.uses.iter().filter(|u|u.decision.as_ref().is_some_and(|d|d.outcome==cad_predicates::BezierIdentity::Equal)).count(),agreement.uses.len(),agreement.all_joins_exact,agreement.work);
    for (faces,station) in [([0,1],false),([0,4],true)] {
        let opposite=brep_core::shared_boundary::inspect_opposite_pair(&model,faces)?;
        let mut a=model.faces[faces[0]].surface.clone();let mut b=model.faces[faces[1]].surface.clone();
        if station {a=nurbs_core::surface_edit::transpose(&a)?;b=nurbs_core::surface_edit::transpose(&b)?;}
        let proof=nurbs_core::analysis::surface_injectivity::certify_shared_natural_boundary(&a,&b,[true,false],1000)?;
        println!("Pair {faces:?}, actual opposite={}, complete shared proof={}, reason={}, spans={}",opposite.is_some(),proof.proven,proof.reason,proof.spans);
    }
    if std::env::args().any(|a|a=="--limited-global") {
        let result=geometry_bridge::dispatch(value_codec::json!({"op":"brep_sweep_volume_audit","model":model,"capFaces":[],
            "toleranceUv":1e-8,"maxExactWork":1000000,"maxTrimPairs":10000,"maxTrimCells":100000,"maxTrimDomainCells":1000000,
            "maxSpans":1024,"maxLinearCells":100000,"maxPairs":32,"maxCells":10000,"maxDomainCells":10000,
            "cellsPerPair":1000,"domainCellsPerPair":10000,"nestingPairs":1000,"nestingCells":100000,"nestingDomainCells":1000000,
            "orientationCells":100000,"orientationDomainCells":1000000,"orientationSpans":100,
            "capBudgets":{"maxWalls":1024,"maxExactWork":1000000,"maxChartCells":1000,"maxTrimPairs":100000,"maxTrimCells":100000,"maxTrimDomainCells":1000000}
        }))?;
        let boundary=&result["boundary"];
        println!("Limited global: exact={}, faces={}, next={}, cells={}",boundary["exactBoundaryCertified"],result["allFacesInjective"],result["nextPair"],result["contactCells"]);
        for pair in boundary["pairs"].as_array().unwrap() {println!("Pair: {} {}",pair["faces"],pair["reason"]);}
    }
    Ok(())
}
