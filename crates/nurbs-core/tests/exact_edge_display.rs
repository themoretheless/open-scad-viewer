use num_traits::Signed;
use nurbs_core::original_graph_transition::{Budget,sample_polynomial_edge};
use num_rational::BigRational as Q;
#[test]
fn nondyadic_edge_samples_have_exact_endpoint_ownership_and_bounded_storage_error(){
 let q=|a:i64,b:i64|Q::new(a.into(),b.into());
 let p=[[q(1,3),q(0,1),q(0,1)],[q(2,3),q(0,1),q(0,1)]];
 let d=sample_polynomial_edge(&p,3,&mut Budget::new(10000).unwrap()).unwrap();
 assert_eq!(d.points.len(),4);
 let bound=Q::from_float(d.sample_storage_error_upper_mm).unwrap();
 for (i,point) in d.points.iter().enumerate(){
  let expected=q(1,3)+q(i as i64,9);
  let difference=expected-Q::from_float(point[0]).unwrap();
  assert!(difference>=-bound.clone()&&difference<=bound);
  assert_eq!([point[1],point[2]],[0.,0.]);
 }
 assert!(d.sample_storage_error_upper_mm>0.);
 assert!(sample_polynomial_edge(&p,3,&mut Budget::new(0).unwrap()).is_err());
 assert!(sample_polynomial_edge(&p,0,&mut Budget::new(1000).unwrap()).is_err());
}
#[test]
fn quadratic_samples_preserve_curvature_shape_instead_of_interpolating_endpoints(){
 let q=|a:i64,b:i64|Q::new(a.into(),b.into());
 let p=[[q(0,1),q(0,1),q(0,1)],[q(1,2),q(0,1),q(0,1)],[q(1,1),q(1,1),q(0,1)]];
 let d=sample_polynomial_edge(&p,3,&mut Budget::new(10000).unwrap()).unwrap();
 let bound=Q::from_float(d.sample_storage_error_upper_mm).unwrap();
 for (i,point) in d.points.iter().enumerate(){let t=q(i as i64,3);let delta=(Q::from_float(point[0]).unwrap()-&t).abs()+(Q::from_float(point[1]).unwrap()-&t*&t).abs();assert!(delta<=bound);}
 assert!(d.points[1][1]<d.points[1][0]);
}
