use geometry_bridge::dispatch;
use value_codec::{json,Value};
fn save(root:&std::path::Path,name:&str,value:&Value){std::fs::write(root.join(name),value_codec::to_string(value).unwrap()).unwrap();}
fn main(){
 let args:Vec<_>=std::env::args().collect();let original:brep_core::Model=value_codec::from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
 let root=std::path::Path::new(&args[2]);std::fs::create_dir_all(root).unwrap();
 let model=brep_core::transform::affine(&original,[[1.,1.,1.,8.],[1.,2.,1.,-4.],[1.,1.,2.,2.],[0.,0.,0.,1.]]).unwrap();let before=model.clone();
 let chart=nurbs_core::surface_injectivity::certify_chart(&model.faces[0].surface,1000).unwrap();
 save(root,"chart.json",&json!({"proven":chart.proven,"work":chart.spans,"reason":chart.reason,"linearProjection":chart.linear_projection,"contractionUpper":chart.contraction_upper}));
 assert!(chart.proven,"{chart:?}");
 let (text,exported,_)=cad_step::export_step_v9(&model).unwrap();assert!(exported.complete);
 let (restored,imported,_)=cad_step::import_step_v9(&text).unwrap();assert!(imported.complete);
 let mut common:Value=value_codec::from_str(include_str!("/private/tmp/osv-quintic-global-20261103/crates/geometry-bridge/tests/fixtures/general-frame-cap-certificate-v1.json")).unwrap();common=common["common"].clone();common["certified"]=json!(true);common["jointContinuousGlobalG2Certified"]=json!(true);
 for (name,a,b) in [("original.json",&model,&model),("imported.json",&restored,&restored),("transport.json",&model,&restored)] {
    let mut request=common.clone();request["reference"]=value_codec::to_value(a).unwrap();request["retained"]=value_codec::to_value(b).unwrap();
    let report=dispatch(request.clone()).unwrap();save(root,name,&report);
    assert_eq!(report["certified"],true,"{name}: {report:?}");assert_eq!(report["errorUpper"].as_f64(),Some(0.));assert_eq!(report["smoothnessCertified"],false);assert_eq!(report["sourceMotionLawCertified"],false);
    for role in ["referenceVolume","retainedVolume"] {assert_eq!(report[role]["proven"],true);assert_eq!(report[role]["allPairsClassified"],true);assert_eq!(report[role]["allFacesInjective"],true);}
    request["limits"]["linearCellsPerBoundary"]=json!(1);
    for role in ["referenceVolume","retainedVolume"] {request["limits"][role]["maxSpans"]=json!(1);}
    let refused=dispatch(request).unwrap();assert_eq!(refused["certified"],false);assert_eq!(refused["continuousBound"],true);save(root,&format!("short-{name}"),&refused);
 }
 let audit=dispatch(json!({"op":"brep_complete_boundary_smoothness_audit","model":value_codec::to_value(&restored).unwrap(),"order":2,"maxExactWork":2000000,"maxAgreementWork":1000000,"maxRegularityCells":100000,"capFaces":[2,3],"seams":[],"fullBoundaryG2Certified":true})).unwrap();
 assert_eq!(audit["fullBoundaryG1Certified"],false);assert_eq!(audit["fullBoundaryG2Certified"],false);save(root,"smoothness.json",&audit);
 assert_eq!(model,before);std::fs::write(root.join("quintic-body.step"),text).unwrap();save(root,"body.json",&value_codec::to_value(&model).unwrap());
 println!("original/imported/transported joint certificates: 3 passed; incomplete global budgets: 3 refused; sharp whole-boundary G1/G2: false; input unchanged");
}
