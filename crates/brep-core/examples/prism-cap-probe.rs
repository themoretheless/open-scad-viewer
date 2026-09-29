use brep_core::{prism,analysis,operations};
use nurbs_core::curve::Curve;
use value_codec::json;
fn circle(r:f64)->Curve {
 let w=std::f64::consts::FRAC_1_SQRT_2;
 Curve{degree:2,knots:vec![0.,0.,0.,0.25,0.25,0.5,0.5,0.75,0.75,1.,1.,1.],control_points:vec![[r,0.],[r,r],[0.,r],[-r,r],[-r,0.],[-r,-r],[0.,-r],[r,-r],[r,0.]].into_iter().map(|p|p.to_vec()).collect(),weights:vec![1.,w,1.,w,1.,w,1.,w,1.],periodic:false}
}
fn main(){
 let original=prism::extrude(&[vec![circle(20.)],vec![circle(5.).reverse().unwrap()]],0.,6.).unwrap();
 let profile=prism::recognize(&original).unwrap().expect("Rational annular prism must be recognized");
 assert_eq!((profile.z_min,profile.z_max,profile.loops.len()),(0.,6.,2));
 let side=original.faces.iter().position(|f|{let z=f.surface.control_points[0][0][2];f.surface.control_points.iter().flatten().any(|p|p[2]!=z)}).unwrap();
 assert!(operations::push_planar_face(&original,side,1.).is_err());
 let mut cases=Vec::new();
 for (z,distance,shift,height) in [(6.,1.,0.,7.),(6.,-1.,0.,5.),(0.,1.,-1.,7.)] {
  let cap=original.faces.iter().position(|f|f.surface.control_points.iter().flatten().all(|p|p[2]==z)).unwrap();
  let result=operations::push_planar_face(&original,cap,distance).unwrap();
  assert!(operations::push_planar_face(&original,cap,-6.).is_err());
  let low=result.vertices.iter().map(|v|v.point[2]).fold(f64::INFINITY,f64::min);
  let high=result.vertices.iter().map(|v|v.point[2]).fold(f64::NEG_INFINITY,f64::max);
  assert!((low-shift).abs()<1e-12 && (high-low-height).abs()<1e-12);
  let mass=analysis::mass_properties(&result,1e-7,200_000).unwrap().signed_volume_mm3;
  assert!((mass-375.*std::f64::consts::PI*height).abs()<1e-5);
  for (a,b) in original.faces.iter().zip(&result.faces){assert_eq!(a.surface.weights,b.surface.weights);assert_eq!(a.surface.knots_u,b.surface.knots_u);assert_eq!(a.surface.knots_v,b.surface.knots_v);}
  let before=value_codec::to_value(&original).unwrap();let after=value_codec::to_value(&result).unwrap();
  assert!(!before["topologyIds"].is_null());
  assert_eq!(before["topologyIds"],after["topologyIds"]);
  cases.push(json!({"height":height,"shift":shift,"volume":mass}));
 }
 println!("{}",json!({"recognized":true,"cases":cases,"scope":"native Push/Pull rational prism end caps"}));
}
