use geometry_bridge::dispatch;
use nurbs_core::{surface::Surface,curve::Curve};
use value_codec::{Value,json};
#[path="/private/tmp/osv-sweep-gram-20261102-r4/crates/geometry-bridge/tests/support/rational_closed_material.rs"]
mod baseline;
fn request()->Value {
    let radius=60.;let height=81.*radius;
    let surface=Surface{degree_u:2,degree_v:1,knots_u:vec![0.75,0.75,0.75,5.25,5.25,5.25],knots_v:vec![-2.5,-2.5,2.5,2.5],
        control_points:(0..3).map(|i|[-2.5,2.5].into_iter().map(|z|{
            let h=if i==1{height}else{0.};vec![radius*(0.75+2.25*i as f64)-8.*h,radius*z,h]
        }).collect()).collect(),weights:vec![vec![1.;2];3],periodic_u:false,periodic_v:false};
    let control=[[1.,0.],[1.,0.5],[0.5,1.],[0.,1.]];
    let mut trims=Vec::new();let mut profiles=Vec::new();
    let c3=[1.,3.,3.,1.];let c6=[1.,6.,15.,20.,15.,6.,1.];
    for branch in 0..4 {
        let xy=control.map(|[mut x,mut y]|{for _ in 0..branch{(x,y)=(-y,x);}[3.+x,y]});
        trims.push(Curve{degree:3,knots:[vec![0.;4],vec![1.;4]].concat(),control_points:xy.iter().map(|p|p.to_vec()).collect(),weights:vec![1.;4],periodic:false});
        let poles=(0..=6).map(|k|{
            let mut r=0.;let mut z=0.;let mut h=0.;
            for i in 0..4 {for j in 0..4 {if i+j!=k{continue;}
                let factor=(radius/c6[k])*c3[i]*c3[j];
                r+=factor*xy[i][0];z+=factor*xy[i][1];
                h+=8.*factor*(xy[i][0]-0.75)*(5.25-xy[j][0]);
            }}vec![r-8.*h,z,h]
        }).collect();
        profiles.push(Curve{degree:6,knots:[vec![0.;7],vec![1.;7]].concat(),control_points:poles,weights:vec![1.;7],periodic:false});
    }
    let xy=[[1.,0.],[1.,1.],[0.,1.],[-1.,1.],[-1.,0.],[-1.,-1.],[0.,-1.],[1.,-1.],[1.,0.]];
    let motion=(0..4).map(|i|{
        let poses=xy[2*i..=2*i+2].iter().map(|&[x,y]|[
            [x/64.,y/64.,0.],[0.,0.,1./64.],[x/8.-y/1048576.,y/8.+x/1048576.,0.],[0.;3]
        ]).collect::<Vec<nurbs_core::affine_volume::Pose>>();json!({"controls":poses,"weights":[1.,0.5,1.]})
    }).collect::<Vec<_>>();
    let mut r=baseline::request(false,false);
    r["source"]["profiles"]=value_codec::to_value(vec![profiles]).unwrap();
    r["source"]["motion"]=json!(motion);r["material"]=json!({"surface":surface,"trims":[trims]});
    r["capProfiles"]=json!([]);r["limits"]["volume"]["maxCells"]=json!(100000);
    r
}
fn full(r:&Value){
    for k in ["certified","solidGeometryCertified","continuousBound","jointContinuousGlobalCertified","jointContinuousGlobalG1Certified","jointContinuousGlobalG2Certified","fullBoundaryG1Certified","fullBoundaryG2Certified"]{
        assert_eq!(r[k],true,"{k}: reason={} binding={} smoothness={}",r["reason"],r["retainedBinding"],r["smoothness"]);
    }
    assert_eq!(r["sourceShear"]["base"],0);assert_eq!(r["sourceShear"]["dependent"],2);assert_eq!(r["sourceShear"]["scale"],8.);
    assert_eq!(r["physicalCapCount"],0);
}
fn main(){
    let out=std::path::PathBuf::from(std::env::args().nth(1).unwrap());std::fs::create_dir_all(&out).unwrap();
    let mut req=request();std::fs::write(out.join("request.json"),value_codec::to_string(&req).unwrap()).unwrap();
    let r=dispatch(req.clone()).unwrap();std::fs::write(out.join("construct.json"),value_codec::to_string(&r).unwrap()).unwrap();full(&r);
    let model:brep_core::Model=value_codec::from_value(r["model"].clone()).unwrap();assert_eq!(model.validate().unwrap().boundary_edge_count,0);
    let (text,exported,_)=cad_step::export_step_v9(&model).unwrap();assert!(exported.complete);std::fs::write(out.join("moving-carrier.step"),&text).unwrap();
    let (restored,imported,_)=cad_step::import_step_v9(&text).unwrap();assert!(imported.complete);
    req["op"]=json!("brep_rational_closed_material_audit");req["model"]=value_codec::to_value(restored).unwrap();req["limits"]["constructionRows"]=json!(0);
    let audit=dispatch(req).unwrap();std::fs::write(out.join("audit.json"),value_codec::to_string(&audit).unwrap()).unwrap();full(&audit);
    let original=request();
    for (name,mut denied) in [("empty-volume-budget",original.clone()),("closure-gap",original.clone()),("interior-fold",original)] {
        denied["jointContinuousGlobalG2Certified"]=json!(true);
        denied["continuousBound"]=json!(true);
        denied["sourceShear"]=json!({"base":2,"dependent":0,"scale":8.});
        match name {
            "empty-volume-budget"=>denied["limits"]["volume"]["maxCells"]=json!(0),
            "closure-gap"=>denied["source"]["motion"][1]["controls"][0][3][0]=json!(0.125),
            _=>{denied["source"]["origin"]=json!([180.,0.,0.]);denied["limits"]["volume"]["maxCells"]=json!(10000);}
        }
        let response=dispatch(denied).unwrap();
        std::fs::write(out.join(format!("refused-{name}.json")),value_codec::to_string(&response).unwrap()).unwrap();
        assert_eq!(response["certified"],false,"{name}: {response:?}");
        assert_eq!(response["jointContinuousGlobalG2Certified"],false);assert!(response["model"].is_null());
    }
    std::fs::write(out.join("result.json"),value_codec::to_string(&json!({"passed":true,"sourceSnapshot":"source-r4.json","constructJointG2":true,"stepExportComplete":true,"stepImportComplete":true,"freshAuditJointG2":true,"externalOpenCascade":false,"limitsIncreased":false,"correctRefusalCases":3})).unwrap()).unwrap();
    println!("new moving nonplanar carrier: construct + STEP + fresh joint G2 passed");
}
