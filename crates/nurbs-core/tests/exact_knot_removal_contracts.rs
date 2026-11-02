use nurbs_core::{curve::Curve,editing::exact_knot_removal::{remove,Refusal}};
fn quadratic(weights:Vec<f64>,points:Vec<Vec<f64>>)->Curve {Curve{degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:points,weights,periodic:false}}
fn polynomial()->Curve {quadratic(vec![1.;3],vec![vec![0.,0.],vec![1.,2.],vec![2.,0.]])}
#[test]
fn polynomial_removal_preserves_complete_nominal_law_and_output_controls() {
    let base=polynomial();let source=base.insert(0.5,1).unwrap();let saved=source.clone();let r=remove(&source,0.5,1_000_000).unwrap();assert!(r.removed,"{:?}",r.refusal);assert_eq!(r.curve,base);assert_eq!(source,saved);
    for i in 0..=128 {let t=i as f64/128.;let p=r.curve.evaluate(t).unwrap().point;assert!((p[0]-2.*t).abs()<1e-14);assert!((p[1]-4.*t*(1.-t)).abs()<1e-14);}
}
#[test]
fn rational_weights_are_verified_as_actual_emitted_geometry() {
    let base=quadratic(vec![1.,3.,1.],vec![vec![0.,0.],vec![2.,4.],vec![4.,0.]]);let source=base.insert(0.5,1).unwrap();let r=remove(&source,0.5,1_000_000).unwrap();assert!(r.removed,"{:?}",r.refusal);assert_eq!(r.curve,base);
    for i in 0..=64 {let t=i as f64/64.;let w=1.+4.*t-4.*t*t;let p=r.curve.evaluate(t).unwrap().point;assert!((p[0]-(12.*t-8.*t*t)/w).abs()<1e-13);assert!((p[1]-24.*t*(1.-t)/w).abs()<1e-13);}
}
#[test]
fn rounded_insertion_is_not_silently_relabelled_as_exact_source_history() {
    let base=quadratic(vec![1.,2.,1.],vec![vec![0.,0.],vec![1.,2.],vec![2.,0.]]);let source=base.insert(0.5,1).unwrap();let r=remove(&source,0.5,1_000_000).unwrap();assert!(!r.removed);assert_eq!(r.curve,source);assert_eq!(r.refusal,Some(Refusal::DifferentDefinition));
}
#[test]
fn a_tiny_nonzero_residual_preserves_the_source_instead_of_snapping() {
    let mut source=polynomial().insert(0.5,1).unwrap();source.control_points[1][1]+=1e-10;let r=remove(&source,0.5,1_000_000).unwrap();assert!(!r.removed);assert_eq!(r.curve,source);assert_eq!(r.refusal,Some(Refusal::DifferentDefinition));
}
#[test]
fn one_ledger_covers_proposal_and_exact_comparison() {
    let source=polynomial().insert(0.5,1).unwrap();let full=remove(&source,0.5,1_000_000).unwrap();assert!(full.removed&&full.work_used>1);
    for work in [1,full.work_used-1] {let r=remove(&source,0.5,work).unwrap();assert!(!r.removed);assert!(r.work_used<=work);assert_eq!(r.curve,source);assert_eq!(r.refusal,Some(Refusal::ResourceLimit));}
    assert_eq!(remove(&source,0.5,full.work_used).unwrap().curve,full.curve);
}
#[test]
fn repeated_knots_and_three_dimensional_nonunit_domains_are_retained() {
    let base=Curve{degree:3,knots:vec![10.,10.,10.,10.,14.,14.,14.,14.],control_points:vec![vec![10.,20.,30.],vec![12.,24.,30.],vec![14.,24.,32.],vec![18.,20.,34.]],weights:vec![1.;4],periodic:false};
    let source=base.insert(11.,2).unwrap();let once=remove(&source,11.,1_000_000).unwrap();assert!(once.removed,"{:?}",once.refusal);let twice=remove(&once.curve,11.,1_000_000).unwrap();assert!(twice.removed,"{:?}",twice.refusal);assert_eq!(twice.curve,base);assert_eq!(twice.curve.domain(),[10.,14.]);
}
#[test]
fn collinearity_is_insufficient_when_the_original_parameter_clock_differs() {
    let source=Curve{degree:1,knots:vec![0.,0.,0.5,1.,1.],control_points:vec![vec![0.,0.],vec![0.5,0.],vec![1.,0.]],weights:vec![1.,2.,1.],periodic:false};let r=remove(&source,0.5,1_000_000).unwrap();assert!(!r.removed);assert_eq!(r.curve,source);
    let source=Curve{degree:1,knots:vec![0.,0.,0.25,1.,1.],control_points:vec![vec![0.,0.],vec![0.25,0.],vec![1.,0.]],weights:vec![1.;3],periodic:false};assert!(remove(&source,0.25,1_000_000).unwrap().removed);
}
#[test]
fn absent_endpoint_invalid_budget_and_unsupported_storage_refuse() {
    let source=polynomial().insert(0.5,1).unwrap();for knot in [0.,1.,0.25,f64::NAN] {assert!(remove(&source,knot,1_000_000).is_err());}for work in [0,1_000_001] {assert!(remove(&source,0.5,work).is_err());}
    let periodic=Curve{degree:1,knots:vec![-1.,0.,1.,2.,3.],control_points:vec![vec![0.,0.],vec![1.,0.],vec![0.,0.]],weights:vec![1.;3],periodic:true};periodic.validate().unwrap();assert!(remove(&periodic,1.,1_000_000).is_err());
}
#[test]
fn existing_degree_twenty_five_limit_never_raises_its_work_budget() {
    let base=Curve{degree:25,knots:[vec![0.;26],vec![1.;26]].concat(),control_points:(0..26).map(|i|vec![i as f64,2.*i as f64]).collect(),weights:vec![1.;26],periodic:false};
    let source=base.insert(0.5,1).unwrap();let r=remove(&source,0.5,1_000_000).unwrap();assert!(r.work_used<=1_000_000);
    if r.removed {assert_eq!(r.curve,base);}else{assert_eq!(r.refusal,Some(Refusal::ResourceLimit));assert_eq!(r.curve,source);}
}
#[test]
fn subnormal_scale_perturbation_cannot_become_a_zero_tolerance_edit() {
    let base=quadratic(vec![1.;3],vec![vec![0.,0.],vec![1e-300,2e-300],vec![2e-300,0.]]);let mut source=base.insert(0.5,1).unwrap();source.control_points[1][1]=source.control_points[1][1].next_up();
    let r=remove(&source,0.5,1_000_000).unwrap();assert!(!r.removed,"subnormal residual was lost");assert_eq!(r.curve,source);assert!(r.refusal.is_some());
    let source=Curve{degree:2,knots:vec![-2.,-1.,0.,0.25,1.,2.,3.],control_points:vec![vec![0.,0.],vec![1.,1.],vec![2.,0.],vec![3.,2.]],weights:vec![1.;4],periodic:false};source.validate().unwrap();assert!(remove(&source,0.25,1_000_000).is_err());
}
