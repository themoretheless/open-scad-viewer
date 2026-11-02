use nurbs_core::{curve::Curve,editing::exact_degree_reduction::{reduce,Refusal}};
fn cubic()->Curve {Curve{degree:3,knots:vec![2.,2.,2.,2.,5.,5.,5.,5.],control_points:vec![vec![0.,0.,2.],vec![2.,4.,4.],vec![4.,4.,6.],vec![6.,0.,8.]],weights:vec![1.;4],periodic:false}}
#[test]
fn cubic_quadratic_law_is_reduced_without_rounding_alpha_or_changing_clock(){
 let source=cubic();let r=reduce(&source,1_000_000).unwrap();assert!(r.reduced,"{:?}",r.refusal);assert_eq!(r.curve.degree,2);assert_eq!(r.curve.control_points,vec![vec![0.,0.,2.],vec![3.,6.,5.],vec![6.,0.,8.]]);assert_eq!(r.curve.domain(),[2.,5.]);
 for i in 0..=32{let t=i as f64/32.;let p=r.curve.evaluate(2.+3.*t).unwrap().point;let expected=[6.*t,12.*t*(1.-t),2.+6.*t];for axis in 0..3{assert!((p[axis]-expected[axis]).abs()<1e-12);}}
 assert_eq!(source,cubic());
}
#[test]
fn rational_quadratic_clock_survives_cubic_reduction(){
 let source=Curve{degree:3,knots:vec![0.,0.,0.,0.,1.,1.,1.,1.],control_points:vec![vec![0.,0.],vec![8.,0.],vec![7.,0.],vec![3.,0.]],weights:vec![1.,3.,4.,4.],periodic:false};let r=reduce(&source,1_000_000).unwrap();assert!(r.reduced,"{:?}",r.refusal);assert_eq!(r.curve.weights,vec![1.,4.,4.]);assert_eq!(r.curve.control_points,vec![vec![0.,0.],vec![9.,0.],vec![3.,0.]]);
 for i in 0..=32{let t=i as f64/32.;let x=r.curve.evaluate(t).unwrap().point[0];assert!((x-(72.*t-60.*t*t)/(1.+6.*t-3.*t*t)).abs()<1e-12);}
}
#[test]
fn unequal_original_intervals_and_c0_join_are_retained(){
 let mut source=cubic();source.knots=[vec![2.;4],vec![5.;3],vec![9.;4]].concat();source.control_points.extend([vec![8.,-4.,10.],vec![10.,-4.,12.],vec![12.,0.,14.]]);source.weights=vec![1.;7];let r=reduce(&source,1_000_000).unwrap();assert!(r.reduced,"{:?}",r.refusal);assert_eq!(r.curve.knots,[vec![2.;3],vec![5.;2],vec![9.;3]].concat());assert_eq!(r.curve.control_points.len(),5);
 for i in 0..=32{let t=i as f64/32.;let p=r.curve.evaluate(5.+4.*t).unwrap().point;assert!((p[0]-6.-6.*t).abs()<1e-12);assert!((p[1]+12.*t*(1.-t)).abs()<1e-12);assert!((p[2]-8.-6.*t).abs()<1e-12);}
}
#[test]
fn rounded_re_elevation_and_nonuniform_rational_clock_cannot_admit_a_fake_zero(){
 for source in [Curve{degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![1.,0.],vec![1.,0.],vec![1_f64.next_up(),0.]],weights:vec![1.;3],periodic:false},Curve{degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![0.,0.],vec![1.,0.],vec![2.,0.]],weights:vec![1.,2.,1.],periodic:false}] {let r=reduce(&source,1_000_000).unwrap();assert!(!r.reduced);assert_eq!(r.curve,source);assert_eq!(r.refusal,Some(Refusal::DifferentDefinition));}
}
#[test]
fn one_ledger_covers_the_proposal_and_complete_original_identity(){
 let source=cubic();let full=reduce(&source,1_000_000).unwrap();assert!(full.reduced);
 for budget in [1,full.work_used-1]{let r=reduce(&source,budget).unwrap();assert!(!r.reduced);assert_eq!(r.curve,source);assert!(r.work_used<=budget);assert_eq!(r.refusal,Some(Refusal::ResourceLimit));}
 assert_eq!(reduce(&source,full.work_used).unwrap().curve,full.curve);
 for budget in [0,1_000_001]{assert!(reduce(&source,budget).is_err());}
}
#[test]
fn unsupported_basis_and_true_degree_are_not_silently_approximated(){
 let line=Curve{degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![vec![0.,0.],vec![1.,0.]],weights:vec![1.;2],periodic:false};assert!(reduce(&line,1_000_000).is_err());
 let smooth=cubic().insert(3.,1).unwrap();assert!(reduce(&smooth,1_000_000).is_err());
 let mut genuine=cubic();genuine.control_points[1][1]+=0.125;let r=reduce(&genuine,1_000_000).unwrap();assert!(!r.reduced);assert_eq!(r.curve,genuine);
}
#[test]
fn periodic_and_unclamped_valid_sources_are_explicitly_outside_admission(){
 let periodic=Curve{degree:2,knots:vec![-2.,-1.,0.,1.,2.,3.,4.,5.],control_points:vec![vec![0.,0.],vec![1.,0.],vec![0.,1.],vec![0.,0.],vec![1.,0.]],weights:vec![1.;5],periodic:true};periodic.validate().unwrap();assert!(reduce(&periodic,1_000_000).is_err());
 let unclamped=Curve{degree:3,knots:vec![-3.,-2.,-1.,0.,0.5,1.,2.,3.,4.],control_points:(0..5).map(|i|vec![i as f64,0.]).collect(),weights:vec![1.;5],periodic:false};unclamped.validate().unwrap();assert!(reduce(&unclamped,1_000_000).is_err());
}
#[test]
fn degree_twenty_five_never_raises_proof_or_proposal_budget(){
 let source=Curve{degree:25,knots:[vec![0.;26],vec![1.;26]].concat(),control_points:(0..26).map(|i|vec![i as f64,0.]).collect(),weights:vec![1.;26],periodic:false};
 let r=reduce(&source,1_000_000).unwrap();assert!(r.work_used<=1_000_000);
 if r.reduced {assert_eq!(r.curve.degree,24);}else{assert_eq!(r.curve,source);assert!(r.refusal.is_some());}
}
#[test]
fn constant_curve_can_reduce_even_when_its_denominator_degree_does_not(){
 let source=Curve{degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![5.,7.];3],weights:vec![1.,2.,1.],periodic:false};let r=reduce(&source,1_000_000).unwrap();assert!(r.reduced,"{:?}",r.refusal);assert_eq!(r.curve.degree,1);assert_eq!(r.curve.control_points,vec![vec![5.,7.];2]);assert_eq!(r.curve.weights,vec![1.;2]);
}
