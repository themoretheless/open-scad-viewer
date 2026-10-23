use nurbs_core::{curve::Curve,continuity::curve_match::checked};
use value_codec::json;
fn main(){
 let mut cases=Vec::new();
 for dimension in [2,3]{for scale in [1e-6,1.,1e6]{
  let mut a=Curve{degree:3,knots:vec![2.,2.,2.,2.,5.,5.,5.,5.],control_points:vec![vec![0.,1.,2.],vec![1.,2.,3.],vec![3.,1.,4.],vec![4.,2.,5.]],weights:vec![1.,0.7,1.3,0.9],periodic:false};
  for p in &mut a.control_points{p.truncate(dimension);}
  let mut b=a.clone();b.knots=vec![-2.,-2.,-2.,-2.,7.,7.,7.,7.];b.weights=vec![0.8,1.2,1.,0.6];for p in &mut b.control_points{for x in p{*x=*x*scale+20.;}}
  for r in ["start","end"]{for e in ["start","end"]{
   let result=checked(&a,&b,r,e,1e-6).unwrap();assert_eq!(result["report"]["accepted"],json!(true));
   cases.push(json!({"reference":a,"editedBefore":b,"result":result,"referenceEnd":r,"editedEnd":e}));
  }}
 }}println!("{}",json!(cases));
}
