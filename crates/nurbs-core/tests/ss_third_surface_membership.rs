use nurbs_core::{ss_intersection as ss,surface::Surface};
use num_rational::BigRational as Q;
use std::sync::Arc;
#[path="support/ss_root_circle.rs"] mod circle;
fn sheet(x:f64,y:f64)->Surface {
    Surface {degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:vec![vec![vec![x,y,-1.],vec![x,y,1.]],vec![vec![x,y+1.,-1.],vec![x,y+1.,1.]]],
        weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false}
}
fn q(n:i64,d:i64)->Q {Q::new(n.into(),d.into())}
fn candidate(curve:&Arc<ss::OriginalSsRootCurve>,third:&Surface)->Arc<ss::QualifiedSsThirdSurfaceCandidates> {
    Arc::new(curve.third_surface_candidates(third,[1,1],Default::default()).unwrap())
}
fn point(curve:Arc<ss::OriginalSsRootCurve>,candidate:&ss::QualifiedSsThirdSurfaceCandidates)->Arc<ss::OriginalSsRootPoint> {
    let clock=Arc::new(candidate.qualify_candidate_clock([Q::from_float(0.688).unwrap(),Q::from_float(0.6894).unwrap()],Default::default()).unwrap());
    Arc::new(ss::OriginalSsRootPoint::from_algebraic(ss::OriginalSsAlgebraicPoint::new(curve,clock).unwrap(),Default::default()).unwrap())
}
#[test]
fn irrational_original_point_has_proved_real_finite_third_uv_and_replays() {
    let wire=circle::wire();let curve=wire.edges()[2].curve().clone();
    let third=sheet(257./1024.,0.);let source=candidate(&curve,&third);let selected=point(curve,&source);
    let proof=ss::QualifiedSsThirdSurfacePoint::qualify(source.clone(),selected.clone(),Default::default()).unwrap();
    assert!(Arc::ptr_eq(proof.point(),&selected));assert!(Arc::ptr_eq(proof.source(),&source));
    assert_eq!(proof.local_bounds()[0],[q(257,1024),q(257,1024)]);
    assert_eq!(proof.local_bounds()[3],proof.local_bounds()[4]);
    assert_eq!(proof.local_bounds()[5],[q(1,2),q(1,2)]);
    assert!(proof.work()>0 && proof.clock_work()>0);proof.reverify().unwrap();
}
#[test]
fn rational_surface_weights_and_nonunit_third_knots_preserve_exact_native_uv() {
    let wire=circle::wire();let curve=wire.edges()[2].curve().clone();
    let mut third=sheet(0.25,0.);third.weights=vec![vec![1.,2.];2];
    third.knots_u=vec![-4.,-4.,6.,6.];third.knots_v=vec![10.,10.,14.,14.];
    let source=candidate(&curve,&third);
    let selected=Arc::new(ss::OriginalSsRootPoint::new(curve,0.6875).unwrap());
    let proof=ss::QualifiedSsThirdSurfacePoint::qualify(source,selected,Default::default()).unwrap();
    assert_eq!(proof.local_bounds()[5],[q(1,3),q(1,3)]);
    assert_eq!(proof.third_uv_bounds(),&[[q(23,8),q(23,8)],[q(34,3),q(34,3)]]);
    proof.reverify().unwrap();
}
#[test]
fn real_elimination_candidates_outside_third_patch_or_wrong_ss_branch_are_refused() {
    let wire=circle::wire();let curve=wire.edges()[2].curve().clone();
    let far=candidate(&curve,&sheet(257./1024.,2.));let p=point(curve.clone(),&far);
    assert!(ss::QualifiedSsThirdSurfacePoint::qualify(far,p,Default::default()).is_err());
    let near=candidate(&curve,&sheet(257./1024.,0.));
    let not_a_root=Arc::new(ss::OriginalSsRootPoint::new(curve.clone(),0.688).unwrap());
    assert!(ss::QualifiedSsThirdSurfacePoint::qualify(near.clone(),not_a_root,Default::default()).is_err());
    let wrong=point(wire.edges()[6].curve().clone(),&near);
    assert!(ss::QualifiedSsThirdSurfacePoint::qualify(near,wrong,Default::default()).is_err());
}
#[test]
fn unavailable_lifts_and_exhausted_policies_never_authorize_membership() {
    let wire=circle::wire();let curve=wire.edges()[2].curve().clone();
    let source=candidate(&curve,&sheet(257./1024.,0.));let selected=point(curve.clone(),&source);
    assert!(ss::QualifiedSsThirdSurfacePoint::qualify(source.clone(),selected.clone(),ss::SsThirdSurfaceMembershipLimits {
        algebra:ss::ExactSsAlgebraLimits {max_work:1,..Default::default()},..Default::default()
    }).is_err());
    assert!(ss::QualifiedSsThirdSurfacePoint::qualify(source,selected,ss::SsThirdSurfaceMembershipLimits {
        clock:ss::SsAlgebraicClockLimits {max_operations:1,..Default::default()},..Default::default()
    }).is_err());
    let mut nonlinear=sheet(257./1024.,0.);nonlinear.weights=vec![vec![1.,2.],vec![3.,4.]];
    let source=candidate(&curve,&nonlinear);let selected=point(curve,&source);
    assert!(ss::QualifiedSsThirdSurfacePoint::qualify(source,selected,Default::default()).is_err());
}
