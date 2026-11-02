use nurbs_core::{curve::Curve,surface::Surface,editing::rational_gauge::{self as gauge,Refusal}};
fn rational()->Curve{Curve{degree:2,knots:vec![-2.,-2.,-2.,6.,6.,6.],control_points:vec![vec![0.,0.],vec![1.,1.],vec![2.,0.]],weights:vec![1.,2.,4.],periodic:false}}
#[test]
fn binary_gauge_keeps_complete_rational_clock_and_input_data(){
 let source=rational();
 for exponent in [-30,0,30]{let r=gauge::curve(&source,exponent).unwrap();assert_eq!(r.exponent,exponent);assert!(r.refusal.is_none());let out=r.geometry.unwrap();assert_eq!(out.control_points,source.control_points);assert_eq!(out.knots,source.knots);assert_eq!(out.degree,source.degree);
  for (a,b) in source.weights.iter().zip(&out.weights){assert_eq!(a.to_bits()&((1u64<<52)-1),b.to_bits()&((1u64<<52)-1));}
  for i in 0..=32{let t=i as f64/32.;let p=out.evaluate(-2.+8.*t).unwrap().point;assert!((p[0]-4.*t/(1.+t)).abs()<1e-13);assert!((p[1]-4.*t*(1.-t)/(1.+t).powi(2)).abs()<1e-13);}
 }
 assert_eq!(source,rational());
}
#[test]
fn automatic_normalization_uses_exact_common_power_not_rounded_max_division(){
 let mut source=rational();source.weights=vec![0.1,0.3,0.9];let out=gauge::normalize_curve(&source).unwrap();assert_eq!(out.exponent,1);assert_eq!(out.geometry.unwrap().weights,vec![0.2,0.6,1.8]);
 let mut large=rational();large.weights=vec![1e10,2e10,4e10];let out=gauge::normalize_curve(&large).unwrap().geometry.unwrap();let max=out.weights.iter().copied().fold(0.,f64::max);assert!(max>=1.&&max<2.);
 let out=gauge::normalize_curve(&rational()).unwrap();assert_eq!(out.exponent,-2);assert_eq!(out.geometry.unwrap().weights,vec![0.25,0.5,1.]);
}
fn surface()->Surface{Surface{degree_u:2,degree_v:1,knots_u:vec![0.,0.,0.,1.,1.,1.],knots_v:vec![0.,0.,1.,1.],control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]];3],weights:vec![vec![1.,1.],vec![2.,1.],vec![1.,1.]],periodic_u:false,periodic_v:false}}
#[test]
fn tensor_uses_one_global_gauge_not_independent_row_normalizations(){
 let source=surface();let out=gauge::normalize_surface(&source).unwrap();assert_eq!(out.exponent,-1);let out=out.geometry.unwrap();assert_eq!(out.weights,vec![vec![0.5,0.5],vec![1.,0.5],vec![0.5,0.5]]);assert_eq!(out.control_points,source.control_points);assert!((out.evaluate(0.5,0.5).unwrap().point[1]-0.4).abs()<1e-14);
 for i in 0..=16{for j in 0..=16{let u=i as f64/16.;let v=j as f64/16.;let p=out.evaluate(u,v).unwrap().point;let w=1.+2.*u*(1.-u);let y=v/(w*(1.-v)+v);assert!((p[1]-y).abs()<1e-13);}}
 assert_eq!(source,surface());
}
#[test]
fn periodic_aliases_and_absolute_domains_remain_literal(){
 let source=Curve{degree:2,knots:vec![-2.,-1.,0.,1.,2.,3.,4.,5.],control_points:vec![vec![0.,0.],vec![1.,0.],vec![0.,1.],vec![0.,0.],vec![1.,0.]],weights:vec![2.,4.,2.,2.,4.],periodic:true};source.validate().unwrap();let out=gauge::normalize_curve(&source).unwrap().geometry.unwrap();assert!(out.periodic);assert_eq!(out.knots,source.knots);assert_eq!(out.control_points,source.control_points);assert_eq!(out.weights,vec![0.5,1.,0.5,0.5,1.]);out.validate().unwrap();
 let mut s=surface();s.degree_v=1;s.knots_v=vec![-1.,0.,1.,2.,3.];s.periodic_v=true;s.control_points=vec![vec![vec![0.,0.,0.],vec![0.,1.,0.],vec![0.,0.,0.]];3];s.weights=vec![vec![2.;3];3];let out=gauge::surface(&s,1).unwrap().geometry.unwrap();assert!(out.periodic_v);assert_eq!(out.knots_v,s.knots_v);assert_eq!(out.weights,vec![vec![4.;3];3]);
}
#[test]
fn range_failure_is_atomic_including_late_weights_and_existing_input_limits(){
 let source=rational();let r=gauge::curve(&source,38).unwrap();assert!(r.geometry.is_none());assert_eq!(r.refusal,Some(Refusal::WeightRange{index:2}));assert_eq!(source,rational());
 for value in [1e-12,1e12]{let mut s=source.clone();s.weights.fill(value);let exponent=if value==1e-12{-1}else{1};let r=gauge::curve(&s,exponent).unwrap();assert!(r.geometry.is_none());assert_eq!(r.refusal,Some(Refusal::WeightRange{index:0}));}
 for exponent in [-80,80,i16::MIN,i16::MAX]{assert!(gauge::curve(&source,exponent).is_err());assert!(gauge::surface(&surface(),exponent).is_err());}
 let mut invalid=source;invalid.weights[0]=f64::NAN;assert!(gauge::normalize_curve(&invalid).is_err());
 let mut invalid=surface();invalid.weights[0][0]=0.;assert!(gauge::normalize_surface(&invalid).is_err());
}
#[test]
fn all_existing_surface_controls_can_change_gauge_without_raising_limits(){
 let n=256;let knots=[vec![0.;2],(1..n-1).map(|i|i as f64).collect(),vec![(n-1) as f64;2]].concat();let source=Surface{degree_u:1,degree_v:1,knots_u:knots.clone(),knots_v:knots,control_points:(0..n).map(|i|(0..n).map(|j|vec![i as f64,j as f64,0.]).collect()).collect(),weights:vec![vec![2.;n];n],periodic_u:false,periodic_v:false};let r=gauge::normalize_surface(&source).unwrap();assert_eq!(r.exponent,-1);let out=r.geometry.unwrap();assert!(out.weights.iter().flatten().all(|&w|w==1.));assert_eq!(out.control_points,source.control_points);assert_eq!(out.knots_u,source.knots_u);
}
#[test]
fn exact_gauge_alignment_enables_profile_merge_without_changing_either_clock(){
 use nurbs_core::editing::exact_curve_merge;
 let left=Curve{degree:1,knots:vec![-2.,-2.,2.,2.],control_points:vec![vec![0.,0.],vec![1.,0.]],weights:vec![1.,2.],periodic:false};
 let right=Curve{degree:1,knots:vec![2.,2.,8.,8.],control_points:vec![vec![1.,0.],vec![3.,0.]],weights:vec![4.,8.],periodic:false};
 assert!(exact_curve_merge::merge(&left,&right).unwrap().curve.is_none());
 let aligned=gauge::curve(&right,-1).unwrap().geometry.unwrap();let merged=exact_curve_merge::merge(&left,&aligned).unwrap().curve.unwrap();assert_eq!(right.weights,vec![4.,8.]);assert_eq!(merged.weights,vec![1.,2.,4.]);
 for i in 0..=32{let t=i as f64/32.;for (u,x) in [(-2.+4.*t,2.*t/(1.+t)),(2.+6.*t,(1.+5.*t)/(1.+t))]{assert!((merged.evaluate(u).unwrap().point[0]-x).abs()<1e-13);}}
}
