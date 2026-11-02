use super::*;
use crate::{SourceArena,ToleranceContext,Limits};
use std::sync::atomic::AtomicBool;
fn values()->Vec<AuthoredScalar> {
    let ku=[0.,0.,0.,0.25,1.,1.,1.];let kv=[0.,0.,0.,0.75,1.,1.,1.];let mut a=Vec::new();
    for i in 0..4 {for j in 0..4 {
        a.extend([(ku[i+1]+ku[i+2])*0.5,(kv[j+1]+kv[j+2])*0.5,ku[i+1]*ku[i+2]+kv[j+1]*kv[j+2],1.]);
    }}
    a.extend([1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,0.,-0.5,0.,0.,-0.5,0.]);a.extend(ku);a.extend(kv);
    a.into_iter().map(|x:f64|AuthoredScalar::Binary64Bits(x.to_bits())).collect()
}
fn arena(a:Vec<AuthoredScalar>)->SourceArena {SourceArena::authored("bspline-quadric-fixture",0,a).unwrap()}
fn refs(a:&SourceArena)->(Vec<Vec<[LeafRef;4]>>,[[LeafRef;4];4],Vec<LeafRef>,Vec<LeafRef>) {
    ((0..4).map(|i|(0..4).map(|j|std::array::from_fn(|axis|a.leaf((i*4+j)*4+axis).unwrap())).collect()).collect(),std::array::from_fn(|i|std::array::from_fn(|j|a.leaf(64+i*4+j).unwrap())),(80..87).map(|i|a.leaf(i).unwrap()).collect(),(87..94).map(|i|a.leaf(i).unwrap()).collect())
}
#[test]
fn nonuniform_knots_keep_one_context_for_all_four_original_spans() {
    let a=arena(values());let (s,q,ku,kv)=refs(&a);let t=ToleranceContext::default_valid();
    let mut ctx=PredicateContext::new(&a,&t,Limits::default(),None);
    let full=binary_bspline_quadric_with_constant_products_identity(&mut ctx,&s,[2,2],[&ku,&kv],q,&[]).unwrap();assert_eq!(full.outcome,BezierIdentity::Equal);
    for max in [0,full.work_used-1] {
        let mut ctx=PredicateContext::new(&a,&t,Limits{max_work:max,..Limits::default()},None);
        let r=binary_bspline_quadric_with_constant_products_identity(&mut ctx,&s,[2,2],[&ku,&kv],q,&[]).unwrap();assert_eq!(r.outcome,BezierIdentity::Indeterminate(Reason::ResourceLimit));assert!(r.work_used<=max);
    }
    let mut ctx=PredicateContext::new(&a,&t,Limits{max_work:full.work_used+17,..Limits::default()},None);ctx.charge(17).unwrap();
    let r=binary_bspline_quadric_with_constant_products_identity(&mut ctx,&s,[2,2],[&ku,&kv],q,&[]).unwrap();assert_eq!(r.outcome,BezierIdentity::Equal);assert_eq!(r.work_used,full.work_used+17);
}
#[test]
fn last_span_mutation_is_detected_and_axis_transpose_is_equivalent() {
    let mut a=values();a[62]=AuthoredScalar::Binary64Bits(2_f64.next_up().to_bits());let a=arena(a);let (s,q,ku,kv)=refs(&a);let t=ToleranceContext::default_valid();let mut ctx=PredicateContext::new(&a,&t,Limits::default(),None);
    assert_eq!(binary_bspline_quadric_with_constant_products_identity(&mut ctx,&s,[2,2],[&ku,&kv],q,&[]).unwrap().outcome,BezierIdentity::Different);
    let a=arena(values());let (s,q,ku,kv)=refs(&a);let transpose=(0..4).map(|j|(0..4).map(|i|s[i][j]).collect()).collect::<Vec<_>>();let mut ctx=PredicateContext::new(&a,&t,Limits::default(),None);
    assert_eq!(binary_bspline_quadric_with_constant_products_identity(&mut ctx,&transpose,[2,2],[&kv,&ku],q,&[]).unwrap().outcome,BezierIdentity::Equal);
}
#[test]
fn cancellation_foreign_sources_rationals_and_bad_bases_refuse() {
    let a=arena(values());let (s,q,ku,kv)=refs(&a);let t=ToleranceContext::default_valid();let cancel=AtomicBool::new(true);let mut ctx=PredicateContext::new(&a,&t,Limits::default(),Some(&cancel));
    assert_eq!(binary_bspline_quadric_with_constant_products_identity(&mut ctx,&s,[2,2],[&ku,&kv],q,&[]).unwrap().outcome,BezierIdentity::Indeterminate(Reason::Cancelled));
    let other=arena(values());let mut ctx=PredicateContext::new(&other,&t,Limits::default(),None);assert!(binary_bspline_quadric_with_constant_products_identity(&mut ctx,&s,[2,2],[&ku,&kv],q,&[]).is_err());
    for rational in [false,true] {
        let mut raw=values();if rational {raw[0]=AuthoredScalar::RationalConstant{numerator:0,denominator:1};}else{raw[80]=AuthoredScalar::Binary64Bits((-1_f64).to_bits());}
        let a=arena(raw);let (s,q,ku,kv)=refs(&a);let mut ctx=PredicateContext::new(&a,&t,Limits::default(),None);assert!(binary_bspline_quadric_with_constant_products_identity(&mut ctx,&s,[2,2],[&ku,&kv],q,&[]).is_err());
    }
    let mut ctx=PredicateContext::new(&a,&t,Limits::default(),None);assert!(binary_bspline_quadric_with_constant_products_identity(&mut ctx,&s,[9,2],[&ku,&kv],q,&[]).is_err());
}
