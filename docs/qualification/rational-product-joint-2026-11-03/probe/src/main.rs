use nurbs_core::curve::Curve;
use geometry_bridge::dispatch;
use value_codec::{json,Value};
fn profile(z:f64)->Vec<Curve> {
    let c=Curve{degree:5,knots:[vec![0.;6],vec![1.;6]].concat(),control_points:(0..6).map(|i|vec![if i%2==0 {-10.}else{10.},2.*(i*i)as f64-9.*i as f64+7.5,z]).collect(),weights:(0..6).map(|i|2f64.powi(5-i)).collect(),periodic:false};
    let line=|a:Vec<f64>,b:Vec<f64>|Curve{degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![a,b],weights:vec![1.;2],periodic:false};
    vec![c.clone(),line(c.control_points[5].clone(),vec![0.,10.,z]),line(vec![0.,10.,z],c.control_points[0].clone())]
}
fn save(root:&std::path::Path,name:&str,value:&Value){std::fs::write(root.join(name),value_codec::to_string(value).unwrap()).unwrap();}
fn short(root:&std::path::Path,name:&str,request:&Value){
    let mut q=request.clone();q["limits"]["referenceVolume"]["maxCells"]=json!(1);
    q["limits"]["retainedVolume"]["maxCells"]=json!(1);
    let r=dispatch(q).unwrap();save(root,name,&r);
    assert_eq!(r["continuousBound"],true);assert_eq!(r["errorUpper"].as_f64(),Some(0.));assert_eq!(r["certified"],false);
}
fn main(){
    let out=std::path::PathBuf::from(std::env::args().nth(1).unwrap());std::fs::create_dir_all(&out).unwrap();
    let sections=(0..3).map(|i|vec![profile(i as f64)]).collect::<Vec<_>>();
    let mut model=brep_core::rational_section_loft(&sections).unwrap();
    for f in model.faces.len()-2..model.faces.len() {
        let s=&mut model.0.faces[f].surface;let a=s.control_points[0][0].clone();let b=s.control_points[1][1].clone();
        s.knots_u=vec![a[0],a[0],b[0],b[0]];s.knots_v=vec![a[1],a[1],b[1],b[1]];
        let wire=model.faces[f].outer;
        for c in &mut model.0.loops[wire].coedges {
            let mut p=model.0.edges[c.edge].curve.clone();for point in &mut p.control_points {point.truncate(2);}
            if c.reversed {p=p.reverse().unwrap();}c.pcurve=p;
        }
    }
    model.validate().unwrap();let before=model.clone();let caps=[model.faces.len()-2,model.faces.len()-1];
    save(&out,"body.json",&value_codec::to_value(&model).unwrap());
    let station=brep_core::sweep_smoothness::inspect_axis(&model,&caps,brep_core::sweep_smoothness::Axis::Station,2000000).unwrap();
    save(&out,"station.json",&json!({"g1":station.g1_certified,"g2":station.g2_certified,"complete":station.extraction_complete,"edges":station.edge_ids.clone(),"work":station.exact_work,"unpaired":station.unpaired_edges.clone()}));
    assert!(station.extraction_complete&&station.g1_certified&&station.g2_certified);
    assert_eq!(station.edge_ids.len(),3);
    let mut request:Value=value_codec::from_str(include_str!("/private/tmp/osv-rational-product-20261103/crates/geometry-bridge/tests/fixtures/general-frame-cap-certificate-v1.json")).unwrap();request=request["common"].clone();
    request["reference"]=value_codec::to_value(&model).unwrap();request["retained"]=value_codec::to_value(&model).unwrap();
    request["certified"]=json!(true);request["jointContinuousGlobalG2Certified"]=json!(true);
    let report=dispatch(request.clone()).unwrap();save(&out,"boundary.json",&report);
    if report["certified"]!=true {
        let limits=brep_core::volume_validity::Limits{boundary:brep_core::boundary_embedding::Limits{exact_work:1000000,trim_pairs:10000,trim_cells:100000,trim_domain_cells:1000000,spans:1000,contacts:brep_core::face_contacts::Limits{pairs:10000,cells:100000,domain_cells:1000000,cells_per_pair:1000,domain_cells_per_pair:10000}},nesting_pairs:100,nesting_cells:100000,nesting_domain_cells:1000000,orientation_cells:100000,orientation_domain_cells:1000000,orientation_spans:100};
        let detail=brep_core::volume_validity::inspect_with_linear(&model,1e-8,limits,10000).unwrap();
        let lines=detail.boundary.intersections.pairs.pairs.iter().map(|p|format!("{:?}: {} source-cells={} hull-cells={}\n",p.faces,p.reason,p.source_chart_cells,p.hull_cells)).collect::<String>();
        std::fs::write(out.join("global-diagnostic.txt"),lines).unwrap();
    }
    assert_eq!(report["certified"],true,"{report:?}");assert_eq!(report["errorUpper"].as_f64(),Some(0.));
    assert_eq!(report["smoothnessCertified"],false);
    short(&out,"short-original.json",&request);
    let (step,certificate,_)=cad_step::export_step_v9(&model).unwrap();assert!(certificate.complete);
    std::fs::write(out.join("body.step"),&step).unwrap();
    let (restored,certificate,_)=cad_step::import_step_v9(&step).unwrap();assert!(certificate.complete);
    let mut imported=request.clone();imported["reference"]=value_codec::to_value(&restored).unwrap();imported["retained"]=value_codec::to_value(&restored).unwrap();
    let report=dispatch(imported.clone()).unwrap();save(&out,"imported.json",&report);assert_eq!(report["certified"],true);
    short(&out,"short-imported.json",&imported);
    request["retained"]=value_codec::to_value(&restored).unwrap();let transported=dispatch(request.clone()).unwrap();save(&out,"transport.json",&transported);
    assert_eq!(transported["certified"],true,"{transported:?}");
    short(&out,"short-transport.json",&request);
    let station=brep_core::sweep_smoothness::inspect_axis(&restored,&caps,brep_core::sweep_smoothness::Axis::Station,2000000).unwrap();
    save(&out,"imported-station.json",&json!({"g1":station.g1_certified,"g2":station.g2_certified,"complete":station.extraction_complete,"edges":station.edge_ids.clone(),"work":station.exact_work}));
    assert!(station.extraction_complete&&station.g1_certified&&station.g2_certified);assert_eq!(station.edge_ids.len(),3);
    assert_eq!(model,before);
    println!("rational clocks: whole boundary error 0, global volume, three actual station G1/G2 seams and STEP transport passed");
}
