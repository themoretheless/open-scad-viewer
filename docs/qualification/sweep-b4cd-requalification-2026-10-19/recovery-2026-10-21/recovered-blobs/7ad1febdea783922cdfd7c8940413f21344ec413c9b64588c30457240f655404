use nurbs_core::{surface::Surface,continuity::match_surface_jets_checked};
use value_codec::json;
fn main(){
 let reference=Surface{degree_u:3,degree_v:3,knots_u:vec![2.,2.,2.,2.,5.,5.,5.,5.],knots_v:vec![-1.,-1.,-1.,-1.,3.,3.,3.,3.],
  control_points:(0..4).map(|i|(0..4).map(|j|vec![i as f64,j as f64,0.15*(i*i)as f64+0.1*(i*j)as f64+0.2*(j*j)as f64]).collect()).collect(),
  weights:(0..4).map(|i|(0..4).map(|j|1.+0.03*i as f64+0.02*j as f64+0.01*(i*j)as f64).collect()).collect(),periodic_u:false,periodic_v:false};
 let mut edited=reference.clone();for row in &mut edited.control_points{for p in row{p[0]+=20.;p[2]-=3.;}}
 let mut cases=Vec::new();
 for r in ["uMin","uMax","vMin","vMax"]{for e in ["uMin","uMax","vMin","vMax"]{for order in [1,2]{for scale in [0.4,1.,1.7]{for reverse in [false,true]{
  let result=match_surface_jets_checked(&reference,&edited,r,e,order,scale,reverse,1e-6).unwrap();
  cases.push(json!({"reference":reference,"result":result,"referenceBoundary":r,"editedBoundary":e,"order":order,"scale":scale,"reverse":reverse}));
 }}}}}
 println!("{}",json!(cases));
}
