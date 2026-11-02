use value_codec::json;
fn main(){
    let dir=std::path::PathBuf::from(std::env::args().nth(1).unwrap());
    let model:brep_core::Model=value_codec::from_str(&std::fs::read_to_string(dir.join("body.json")).unwrap()).unwrap();
    let report=brep_core::face_domain::audit_trim_regions(&model,1e-8,10000,100000,1000000).unwrap();
    let result=json!({"allValid":report.all_valid,"pairs":report.pairs,"cells":report.cells,"domainCells":report.domain_cells,
        "faces":report.faces.iter().enumerate().map(|(face,r)|r.as_ref().map(|r|json!({"face":face,"valid":r.valid,"reason":r.reason,"problemLoops":r.problem_loops,"loopSimple":r.loops.iter().map(|l|l.proven_simple).collect::<Vec<_>>(),"loopOutcomes":r.loops.iter().map(|l|format!("{:?}",l.outcome())).collect::<Vec<_>>(),"curveInjectivity":r.loops.iter().map(|l|l.injective.clone()).collect::<Vec<_>>()}))).collect::<Vec<_>>()});
    std::fs::write(dir.join("trim-diagnostic.json"),value_codec::to_string(&result).unwrap()).unwrap();
    println!("{}",value_codec::to_string(&result).unwrap());
}
