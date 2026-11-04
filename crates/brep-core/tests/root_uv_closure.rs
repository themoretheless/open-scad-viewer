use brep_core::root_model::{RootBrep,QualifiedRootUvClosure,qualify_root_uv_joint};
use std::sync::Arc;
#[path="../../nurbs-core/tests/support/ss_root_circle.rs"] mod fixture;
#[test]
fn both_face_projections_have_exact_uv_closed_incidence() {
    let wire=fixture::wire();
    let source=Arc::new(RootBrep::paired_contours(&wire,&[],1e-7).unwrap());
    let proof=QualifiedRootUvClosure::qualify(source.clone(),16).unwrap();
    assert_eq!(proof.joints(),16);assert!(Arc::ptr_eq(proof.source(),&source));proof.reverify().unwrap();
    assert!(QualifiedRootUvClosure::qualify(source.clone(),15).is_err());
    assert!(QualifiedRootUvClosure::qualify(source,0).is_err());
    let a=wire.edges()[0].end();let b=wire.edges()[1].start();
    qualify_root_uv_joint(a,0,b,0).unwrap();
    assert!(qualify_root_uv_joint(a,0,b,1).is_err());
    assert!(qualify_root_uv_joint(a,2,b,0).is_err());
    assert!(qualify_root_uv_joint(a,0,wire.edges()[2].start(),0).is_err());
}
#[test]
fn reversed_coedge_traversal_checks_its_actual_endpoints() {
    let original=RootBrep::paired_contours(&fixture::wire(),&[],1e-7).unwrap();
    let mut t=original.topology().clone();
    for l in &mut t.loops {
        l.coedges.reverse();
        for c in &mut l.coedges {c.reversed=!c.reversed;c.pcurve.reversed=c.reversed;}
    }
    let model=Arc::new(RootBrep::from_topology(t).unwrap());
    let proof=QualifiedRootUvClosure::qualify(model,16).unwrap();
    assert_eq!(proof.joints(),16);proof.reverify().unwrap();
}

use nurbs_core::{ss_intersection as ss,surface::Surface};
use num_rational::BigRational as Q;
fn sheet(x:f64,y:f64)->Surface {
    Surface {degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:vec![vec![vec![x,y,-1.],vec![x,y,1.]],vec![vec![x,y+1.,-1.],vec![x,y+1.,1.]]],
        weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false}
}
fn candidate(curve:&Arc<ss::OriginalSsRootCurve>,third:&Surface)->Arc<ss::QualifiedSsThirdSurfaceCandidates> {
    Arc::new(curve.third_surface_candidates(third,[1,1],Default::default()).unwrap())
}
fn point(curve:Arc<ss::OriginalSsRootCurve>,candidate:&ss::QualifiedSsThirdSurfaceCandidates)->Arc<ss::OriginalSsRootPoint> {
    let clock=Arc::new(candidate.qualify_candidate_clock([Q::from_float(0.688).unwrap(),Q::from_float(0.6894).unwrap()],Default::default()).unwrap());
    Arc::new(ss::OriginalSsRootPoint::from_algebraic(ss::OriginalSsAlgebraicPoint::new(curve,clock).unwrap(),Default::default()).unwrap())
}
fn target_pair(w:&ss::QualifiedSsThirdSurfacePoint, roles:[usize;2])->Arc<ss::OriginalSsRootCurve> {
    let surfaces=[w.source().original().source().first(),w.source().original().source().second(),w.source().third()];
    let src=ss::FiberSources::new(surfaces[roles[0]],surfaces[roles[1]]).unwrap();
    let spans=roles.map(|r| if r==1 {[2,2]} else {[1,1]});
    let ideal=ss::qualify_original_ss_ideal(&src,[spans[0][0],spans[0][1],spans[1][0],spans[1][1]],Default::default()).unwrap();
    let gb=ss::qualify_ss_groebner_basis(&ideal,Default::default()).unwrap();
    let sys=ss::qualify_ss_deflated_system(&gb,Default::default()).unwrap();
    let mut domain=roles.map(|r| if r==2 {[[0.67,0.70],[0.45,0.55]]} else {[[0.24,0.27],[0.67,0.70]]});
    domain[1][if roles[1]==2 {0} else {1}]=[0.66,0.71];
    let axis=if roles[0]==2 {ss::FiberAxis::FirstU} else {ss::FiberAxis::FirstV};
    let f=ss::qualify_deflated_ss_fiber(&sys,[domain[0][0],domain[0][1],domain[1][0],domain[1][1]],axis,Default::default()).unwrap();
    let report=format!("{:?}",f.report());
    Arc::new(ss::OriginalSsRootCurve::from_deflated(f.into_fiber().unwrap_or_else(||panic!("{report}"))))
}

#[test]
fn mixed_pair_uv_joint_uses_the_common_surface_role() {
    let wire=fixture::wire();let curve=wire.edges()[2].curve().clone();
    let source=candidate(&curve,&sheet(257./1024.,0.));let selected=point(curve,&source);
    let witness=Arc::new(ss::QualifiedSsThirdSurfacePoint::qualify(source,selected,Default::default()).unwrap());
    let ac=ss::bind_original_ss_third_port(witness.clone(),target_pair(&witness,[0,2]),[0,2],Default::default()).unwrap();
    let cb=ss::bind_original_ss_third_port(witness.clone(),target_pair(&witness,[2,1]),[2,1],Default::default()).unwrap();
    qualify_root_uv_joint(&ac,1,&cb,0).unwrap();
    assert!(qualify_root_uv_joint(&ac,0,&cb,1).is_err());
    ac.reverify().unwrap();cb.reverify().unwrap();
}
