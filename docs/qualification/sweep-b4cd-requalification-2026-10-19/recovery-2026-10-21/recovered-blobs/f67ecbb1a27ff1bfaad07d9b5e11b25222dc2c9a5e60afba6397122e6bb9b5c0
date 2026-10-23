use nurbs_core::{surface::{Surface,Axis},continuity::preparation::{prepare,certify,checked_with_conversion}};
use value_codec::json;
fn patch(degree:usize,knots:Vec<f64>)->Surface{
 let n=knots.len()-degree-1;
 Surface{degree_u:3,degree_v:degree,knots_u:vec![2.,2.,2.,2.,5.,5.,5.,5.],knots_v:knots,
 control_points:(0..4).map(|i|(0..n).map(|j|vec![i as f64,j as f64,0.1*(i*j)as f64]).collect()).collect(),
 weights:(0..4).map(|i|(0..n).map(|j|1.+0.02*i as f64+0.03*j as f64).collect()).collect(),periodic_u:false,periodic_v:false}
}
fn main(){
 let a=patch(2,vec![3.,3.,3.,5.,7.,7.,7.]);let b=patch(3,vec![-2.,-2.,-2.,-2.,1.,2.,2.,2.,2.]);let mut cases=Vec::new();
 for reverse in [false,true]{for refined in [false,true]{let mut candidate=prepare(&a,&b,"uMax","uMin",reverse).unwrap();if refined{candidate.reference=candidate.reference.edit_axis(Axis::U,|c|c.insert(3.,2)).unwrap();candidate.edited=candidate.edited.edit_axis(Axis::U,|c|c.insert(4.,1)).unwrap();}let report=certify(&a,&b,"uMax","uMin",&candidate,1e-8).unwrap();
 cases.push(json!({"source":a,"target":candidate.reference,"reverse":false,"bound":report["referenceErrorUpper"]}));
 cases.push(json!({"source":b,"target":candidate.edited,"reverse":reverse,"bound":report["editedErrorUpper"]}));}}
 let points:Vec<Vec<Vec<f64>>>=(0..6).map(|i|(0..6).map(|j|{let u=(i%4)as f64*std::f64::consts::FRAC_PI_2;let v=(j%4)as f64*std::f64::consts::FRAC_PI_2;vec![(2.+0.5*v.cos())*u.cos(),(2.+0.5*v.cos())*u.sin(),0.5*v.sin()]}).collect()).collect();
 let source=Surface{degree_u:2,degree_v:2,knots_u:(0..9).map(|i|i as f64).collect(),knots_v:(0..9).map(|i|i as f64).collect(),weights:(0..6).map(|i|(0..6).map(|j|[1.,0.8,1.2,1.][i%4]*[1.,0.9,1.1,1.][j%4]).collect()).collect(),control_points:points,periodic_u:true,periodic_v:true};
 for axis in [Axis::U,Axis::V]{let result=nurbs_core::foundation::rebuild_surface(&source,axis,3,15,0.2,None).unwrap();assert_eq!(result["certificate"]["accepted"],json!(true));cases.push(json!({"source":source,"target":result["surface"],"reverse":false,"bound":result["certificate"]["hausdorffErrorUpper"]}));}
 let periodic=|degree,unique:usize|Surface{degree_u:3,degree_v:degree,knots_u:vec![0.,0.,0.,0.,1.,1.,1.,1.],knots_v:(0..unique+2*degree+1).map(|i|i as f64).collect(),control_points:(0..4).map(|i|(0..unique+degree).map(|j|{let t=(j%unique)as f64*std::f64::consts::TAU/unique as f64;vec![i as f64,t.cos(),t.sin()]}).collect()).collect(),weights:(0..4).map(|i|(0..unique+degree).map(|j|1.+0.02*i as f64+0.01*(j%unique)as f64).collect()).collect(),periodic_u:false,periodic_v:true};
 let a=periodic(2,4);let b=periodic(3,8);
 for reverse in [false,true]{let candidate=prepare(&a,&b,"uMax","uMin",reverse).unwrap();let report=certify(&a,&b,"uMax","uMin",&candidate,1e-8).unwrap();assert_eq!(report["accepted"],json!(true));cases.push(json!({"source":a,"target":candidate.reference,"reverse":false,"bound":report["referenceErrorUpper"]}));cases.push(json!({"source":b,"target":candidate.edited,"reverse":reverse,"bound":report["editedErrorUpper"]}));}
 let open=patch(3,vec![0.,0.,0.,0.,1.,1.,1.,1.]);
 for reverse in [false,true]{for(first,second)in[(&a,&open),(&open,&a)]{
  let result=checked_with_conversion(first,second,"uMax","uMin",reverse,1e-8,true).unwrap();assert_eq!(result["report"]["accepted"],json!(true));
  cases.push(json!({"source":first,"target":result["reference"],"reverse":false,"bound":result["report"]["referenceErrorUpper"]}));
  cases.push(json!({"source":second,"target":result["edited"],"reverse":reverse,"bound":result["report"]["editedErrorUpper"]}));
 }}
 println!("{}",json!(cases));
}
