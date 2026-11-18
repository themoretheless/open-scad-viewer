use nurbs_core::{ss_intersection as ss,surface::Surface};
use num_rational::BigRational as Q;
use std::sync::Arc;
#[path="support/ss_root_circle.rs"] mod circle;
fn sheet(x:f64,y:f64)->Surface {
    Surface {degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:vec![vec![vec![x,y,-1.],vec![x,y,1.]],vec![vec![x,y+1.,-1.],vec![x,y+1.,1.]]],
        weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false}
}
fn source(curve:&Arc<ss::OriginalSsRootCurve>,surface:&Surface)->Arc<ss::QualifiedSsThirdSurfaceCandidates> {
    Arc::new(curve.third_surface_candidates(surface,[1,1],Default::default()).unwrap())
}
fn point(curve:Arc<ss::OriginalSsRootCurve>,proof:&ss::QualifiedSsThirdSurfaceCandidates)->Arc<ss::OriginalSsRootPoint> {
    let clock=Arc::new(proof.qualify_candidate_clock([Q::from_float(0.688).unwrap(),Q::from_float(0.6894).unwrap()],Default::default()).unwrap());
    Arc::new(ss::OriginalSsRootPoint::from_algebraic(ss::OriginalSsAlgebraicPoint::new(curve,clock).unwrap(),Default::default()).unwrap())
}
#[test]
fn original_forced_uv_excludes_far_patch_and_other_ss_branch() {
    let wire=circle::wire();let curve=wire.edges()[2].curve().clone();
    let far=source(&curve,&sheet(257./1024.,2.));let p=point(curve.clone(),&far);
    let proof=ss::QualifiedSsThirdSurfaceExclusion::qualify(far.clone(),p.clone(),Default::default()).unwrap();
    assert_eq!(proof.kind(),ss::SsThirdSurfaceExclusionKind::Coordinate(4));
    assert!(proof.bounds()[1]<Q::from_integer(0.into()));
    assert!(Arc::ptr_eq(proof.source(),&far) && Arc::ptr_eq(proof.point(),&p));
    assert!(proof.work()>0 && proof.clock_work()>0);proof.reverify().unwrap();
    let near=source(&curve,&sheet(257./1024.,0.));let other=point(wire.edges()[6].curve().clone(),&near);
    let proof=ss::QualifiedSsThirdSurfaceExclusion::qualify(near,other,Default::default()).unwrap();
    assert_eq!(proof.kind(),ss::SsThirdSurfaceExclusionKind::Coordinate(0));
    assert!(proof.bounds()[1]<proof.allowed()[0]);proof.reverify().unwrap();
}
#[test]
fn nonzero_elimination_constraint_and_constant_disjointness_are_independent_proofs() {
    let wire=circle::wire();let curve=wire.edges()[2].curve().clone();
    let near=source(&curve,&sheet(257./1024.,0.));
    let not_a_root=Arc::new(ss::OriginalSsRootPoint::new(curve.clone(),0.688).unwrap());
    let proof=ss::QualifiedSsThirdSurfaceExclusion::qualify(near,not_a_root,Default::default()).unwrap();
    assert_eq!(proof.kind(),ss::SsThirdSurfaceExclusionKind::EliminationConstraint);proof.reverify().unwrap();
    let mut third=sheet(0.25,0.);for row in &mut third.control_points {for p in row {let y=p[1];let z=p[2];*p=vec![y,z,1.];}}
    let disjoint=source(&curve,&third);assert!(disjoint.proves_disjoint_on_spans());
    let p=Arc::new(ss::OriginalSsRootPoint::new(curve,0.6875).unwrap());
    let proof=ss::QualifiedSsThirdSurfaceExclusion::qualify(disjoint,p,Default::default()).unwrap();
    assert_eq!(proof.kind(),ss::SsThirdSurfaceExclusionKind::EliminationConstraint);proof.reverify().unwrap();
}
#[test]
fn closed_patch_boundary_is_retained_but_tiny_exact_gap_is_excluded() {
    let wire=circle::wire();let curve=wire.edges()[2].curve().clone();
    let p=Arc::new(ss::OriginalSsRootPoint::new(curve.clone(),0.6875).unwrap());
    let boundary=source(&curve,&sheet(0.25,0.6875));
    ss::QualifiedSsThirdSurfacePoint::qualify(boundary.clone(),p.clone(),Default::default()).unwrap().reverify().unwrap();
    assert!(ss::QualifiedSsThirdSurfaceExclusion::qualify(boundary,p.clone(),Default::default()).is_err());
    let epsilon=2_f64.powi(-40);let outside=source(&curve,&sheet(0.25,0.6875+epsilon));
    let proof=ss::QualifiedSsThirdSurfaceExclusion::qualify(outside,p,Default::default()).unwrap();
    assert_eq!(proof.kind(),ss::SsThirdSurfaceExclusionKind::Coordinate(4));
    let exact=Q::from_float(-epsilon).unwrap();assert_eq!(proof.bounds(),&[exact.clone(),exact]);
    proof.reverify().unwrap();
}
#[test]
fn membership_or_resource_failure_is_never_silently_an_exclusion() {
    let wire=circle::wire();let curve=wire.edges()[2].curve().clone();
    let near=source(&curve,&sheet(257./1024.,0.));let p=point(curve.clone(),&near);
    ss::QualifiedSsThirdSurfacePoint::qualify(near.clone(),p.clone(),Default::default()).unwrap();
    assert!(ss::QualifiedSsThirdSurfaceExclusion::qualify(near,p,Default::default()).is_err());
    let far=source(&curve,&sheet(257./1024.,2.));let p=point(curve,&far);
    for limits in [ss::SsThirdSurfaceMembershipLimits {algebra:ss::ExactSsAlgebraLimits {max_work:1,..Default::default()},..Default::default()},
        ss::SsThirdSurfaceMembershipLimits {clock:ss::SsAlgebraicClockLimits {max_operations:1,..Default::default()},..Default::default()}] {
        assert!(ss::QualifiedSsThirdSurfaceExclusion::qualify(far.clone(),p.clone(),limits).is_err());
    }
}
