use nurbs_core::{curve::Curve,editing::{exact_curve_merge::{merge,Refusal},exact_knot_removal::simplify}};
fn quadratic()->Curve{Curve{degree:2,knots:vec![2.,2.,2.,5.,5.,5.],control_points:vec![vec![0.,0.],vec![3.,6.],vec![6.,0.]],weights:vec![1.;3],periodic:false}}
#[test]
fn split_merge_and_exact_compaction_recover_original_nonunit_profile(){
 let source=quadratic();let [a,b]=source.split(3.5).unwrap();let saved=(a.clone(),b.clone());let report=merge(&a,&b).unwrap();assert!(report.refusal.is_none());let curve=report.curve.unwrap();assert_eq!((a,b),saved);
 assert_eq!(curve.knots,vec![2.,2.,2.,3.5,3.5,5.,5.,5.]);
 for i in 0..=32{let t=i as f64/32.;let p=curve.evaluate(2.+3.*t).unwrap().point;assert!((p[0]-6.*t).abs()<1e-13);assert!((p[1]-12.*t*(1.-t)).abs()<1e-13);}
 let compact=simplify(&curve,1_000_000).unwrap();assert!(compact.completed_pass);assert_eq!(compact.removed,2);assert_eq!(compact.curve,source);
}
fn rational()->(Curve,Curve){
 let make=|domain:[f64;2],x:[f64;2],w:[f64;2]|Curve{degree:1,knots:vec![domain[0],domain[0],domain[1],domain[1]],control_points:x.iter().map(|&x|vec![x,2.+2.*x,4.+2.*x]).collect(),weights:w.to_vec(),periodic:false};
 (make([-2.,2.],[0.,1.],[1.,2.]),make([2.,8.],[1.,3.],[2.,4.]))
}
#[test]
fn rational_three_dimensional_curves_preserve_each_absolute_clock(){
 let (a,b)=rational();let c=merge(&a,&b).unwrap().curve.unwrap();assert_eq!(c.knots,vec![-2.,-2.,2.,8.,8.]);assert_eq!(c.weights,vec![1.,2.,4.]);assert_eq!(c.control_points,vec![vec![0.,2.,4.],vec![1.,4.,6.],vec![3.,8.,10.]]);
 for i in 0..=32{let t=i as f64/32.;for (u,x) in [(-2.+4.*t,2.*t/(1.+t)),(2.+6.*t,(1.+5.*t)/(1.+t))]{let p=c.evaluate(u).unwrap().point;let law=[x,2.+2.*x,4.+2.*x];for k in 0..3{assert!((p[k]-law[k]).abs()<1e-13);}}}
}
#[test]
fn genuine_corner_remains_and_is_not_silently_smoothed(){
 let a=Curve{degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![vec![0.,0.],vec![1.,0.]],weights:vec![1.;2],periodic:false};let b=Curve{knots:vec![1.,1.,2.,2.],control_points:vec![vec![1.,0.],vec![1.,1.]],..a.clone()};
 let c=merge(&a,&b).unwrap().curve.unwrap();assert_eq!(c.evaluate(0.5).unwrap().point,vec![0.5,0.]);assert_eq!(c.evaluate(1.5).unwrap().point,vec![1.,0.5]);let compact=simplify(&c,1_000_000).unwrap();assert_eq!(compact.removed,0);assert_eq!(compact.curve,c);
}
#[test]
fn gauge_changes_and_one_ulp_mismatch_are_refused_without_editing_sources(){
 let (a,b)=rational();let mut gauge=b.clone();for w in &mut gauge.weights{*w*=2.;}let mut ulp=b.clone();ulp.control_points[0][0]=1_f64.next_up();
 for right in [gauge,ulp]{let saved=right.clone();let r=merge(&a,&right).unwrap();assert!(r.curve.is_none());assert_eq!(r.refusal,Some(Refusal::SeamMismatch));assert_eq!(right,saved);}
}
#[test]
fn invalid_and_incompatible_storage_are_distinguished(){
 let (a,b)=rational();let mut gap=b.clone();gap.knots=vec![3.,3.,8.,8.];assert_eq!(merge(&a,&gap).unwrap().refusal,Some(Refusal::NonAdjacentDomain));
 let mut dim=b.clone();for p in &mut dim.control_points{p.pop();}assert_eq!(merge(&a,&dim).unwrap().refusal,Some(Refusal::DimensionMismatch));
 let mut unclamped=b.clone();unclamped.knots=vec![0.,2.,8.,9.];unclamped.validate().unwrap();assert_eq!(merge(&a,&unclamped).unwrap().refusal,Some(Refusal::NonClamped));
 let elevated=b.elevate(2).unwrap();assert_eq!(merge(&a,&elevated).unwrap().refusal,Some(Refusal::DegreeMismatch));
 let periodic=Curve{degree:2,knots:vec![-2.,-1.,0.,1.,2.,3.,4.,5.],control_points:vec![vec![0.,0.],vec![1.,0.],vec![0.,1.],vec![0.,0.],vec![1.,0.]],weights:vec![1.;5],periodic:true};assert_eq!(merge(&periodic,&periodic).unwrap().refusal,Some(Refusal::Periodic));
 let mut invalid=b;invalid.weights[0]=f64::NAN;assert!(merge(&a,&invalid).is_err());
}
#[test]
fn structural_merging_keeps_the_original_control_cap(){
 let make=|start:usize|Curve{degree:1,knots:[vec![start as f64;2],(start+1..start+129).map(|u|u as f64).collect(),vec![(start+129) as f64;2]].concat(),control_points:(start..start+130).map(|x|vec![x as f64,0.]).collect(),weights:vec![1.;130],periodic:false};let a=make(0);let b=make(129);a.validate().unwrap();b.validate().unwrap();let r=merge(&a,&b).unwrap();assert!(r.curve.is_none());assert_eq!(r.refusal,Some(Refusal::ControlLimit));
}
