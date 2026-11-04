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
fn triple_vertex_binds_other_ordered_pairs_without_replacing_its_clock() {
    let wire=circle::wire(); let curve=wire.edges()[2].curve().clone();
    let source=candidate(&curve,&sheet(257./1024.,0.)); let selected=point(curve,&source);
    let witness=Arc::new(ss::QualifiedSsThirdSurfacePoint::qualify(source,selected.clone(),Default::default()).unwrap());
    for roles in [[0,2],[1,2],[2,0],[2,1]] {
        let target=target_pair(&witness,roles);
        assert!(ss::bind_original_ss_port(selected.clone(),target.clone(),[0;4],Default::default()).is_err());
        let port=ss::bind_original_ss_third_port(witness.clone(),target.clone(),roles,Default::default()).unwrap();
        assert!(Arc::ptr_eq(port.point(),&selected)); assert!(Arc::ptr_eq(port.target(),&target));
        assert_eq!(port.kind(),ss::OriginalSsPortKind::OriginalThirdSurface);
        assert_eq!(port.parameter_bounds(),&witness.local_bounds()[3]);
        port.reverify().unwrap();
        let policy=Arc::new(selected.with_query_policy(Default::default()).unwrap());
        assert!(port.rebind_equivalent(policy,Default::default()).is_ok());
        assert!(ss::bind_original_ss_third_port(witness.clone(),target.clone(),[0,1],Default::default()).is_err());
        assert!(ss::bind_original_ss_third_port(witness.clone(),target,roles,ss::OriginalSsCurveLimits {
            projection:ss::ExactSsAlgebraLimits {max_work:1,..Default::default()},..Default::default()
        }).is_err());
    }
}
