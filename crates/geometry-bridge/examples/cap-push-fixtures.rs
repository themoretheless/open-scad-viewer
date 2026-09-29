use value_codec::json;
use nurbs_core::curve::Curve;
fn circle(r:f64)->Curve {
 let w=std::f64::consts::FRAC_1_SQRT_2;
 Curve{degree:2,knots:vec![0.,0.,0.,0.25,0.25,0.5,0.5,0.75,0.75,1.,1.,1.],control_points:vec![[r,0.],[r,r],[0.,r],[-r,r],[-r,0.],[-r,-r],[0.,-r],[r,-r],[r,0.]].into_iter().map(|p|p.to_vec()).collect(),weights:vec![1.,w,1.,w,1.,w,1.,w,1.],periodic:false}
}
fn main(){
 let bracket=brep_core::operations::extrude_polygon(&[[0.,0.],[40.,0.],[40.,5.],[5.,5.],[5.,30.],[0.,30.]],0.,20.).unwrap();
 let enclosure=brep_core::operations::boolean(&brep_core::cuboid([0.;3],[40.,30.,20.]).unwrap(),&brep_core::cuboid([2.,2.,2.],[38.,28.,22.]).unwrap(),"difference").unwrap();
 let flange=brep_core::prism::extrude(&[vec![circle(20.)],vec![circle(5.).reverse().unwrap()]],0.,6.).unwrap();
 let mut cases=Vec::new();
 for (name,model,volume) in [("bracket",bracket,6825.),("enclosure",enclosure,7416.),("flange",flange,2625.*std::f64::consts::PI)]{
  let display=geometry_bridge::dispatch(json!({"op":"brep_nurbs_tessellate","model":model,"segments":2})).unwrap();
  let mesh=json!({"positions":display["positions"],"indices":display["indices"]});
  let body=json!({"id":name,"name":name,"brep":model,"mesh":mesh});
  let topology=geometry_bridge::dispatch(json!({"op":"cad_mesh_topology","mesh":mesh})).unwrap();
  let top=topology["faces"].as_array().unwrap().iter().enumerate().filter(|(_,f)|f["normal"][2].as_f64().unwrap()>0.99).max_by(|(_,a),(_,b)|a["center"][2].as_f64().unwrap().total_cmp(&b["center"][2].as_f64().unwrap())).unwrap().0;
  let request=json!({"op":"cad_planar_edit","body":body,"action":"push","faces":[top],"amount":1.});
  let response=geometry_bridge::execute(&request.to_string());
  let decoded=value_codec::from_str::<value_codec::Value>(&response).unwrap();
  if decoded["ok"].as_bool()==Some(true) {
   let result:brep_core::Model=value_codec::from_value(decoded["value"]["brep"].clone()).unwrap();
   let actual=brep_core::analysis::mass_properties(&result,1e-7,200_000).unwrap().signed_volume_mm3;
   assert!((actual-volume).abs()<1e-5);
  }
  cases.push(json!({"name":name,"request":request,"response":value_codec::from_str::<value_codec::Value>(&response).unwrap(),"expectedVolume":volume}));
 }
 println!("{}",json!({"cases":cases}));
}
