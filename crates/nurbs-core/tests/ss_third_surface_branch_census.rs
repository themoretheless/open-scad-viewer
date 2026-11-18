use nurbs_core::{ss_intersection as ss,surface::Surface};
use std::sync::Arc;
#[path="support/ss_root_circle.rs"] mod circle;
fn sheet(y:f64)->Surface {
    Surface {degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:vec![vec![vec![0.25,y,-1.],vec![0.25,y,1.]],vec![vec![0.25,y+1.,-1.],vec![0.25,y+1.,1.]]],
        weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false}
}
fn source(curve:&Arc<ss::OriginalSsRootCurve>,surface:&Surface)->Arc<ss::QualifiedSsThirdSurfaceCandidates> {
    Arc::new(curve.third_surface_candidates(surface,[1,1],Default::default()).unwrap())
}
#[test]
fn full_owned_graph_census_keeps_every_real_member_and_replays() {
    let wire=circle::wire();let curve=wire.edges()[2].curve().clone();let source=source(&curve,&sheet(0.));
    let proof=ss::QualifiedSsThirdSurfaceBranchCensus::qualify(source.clone(),curve.clone(),Default::default()).unwrap();
    assert_eq!(proof.root_census().roots().len(),2);assert_eq!(proof.entries().len(),2);
    assert!(proof.entries().iter().all(|e|e.is_inside()));
    assert!(proof.queries()>0 && proof.reserved_work()<=ss::SsThirdSurfaceBranchLimits::default().max_reserved_work);
    for entry in proof.entries() {
        assert!(Arc::ptr_eq(entry.point().curve(),&curve));
        let ss::SsThirdSurfaceBranchEntry::Inside(member)=entry else {panic!("Expected membership")};
        assert!(Arc::ptr_eq(member.source(),&source));member.reverify().unwrap();
    }
    proof.reverify().unwrap();
}
#[test]
fn mixed_and_fully_excluded_roots_remain_in_complete_ordered_coverage() {
    let wire=circle::wire();let curve=wire.edges()[2].curve().clone();
    let mixed=ss::QualifiedSsThirdSurfaceBranchCensus::qualify(source(&curve,&sheet(0.5)),curve.clone(),Default::default()).unwrap();
    assert_eq!(mixed.entries().iter().map(|e|e.is_inside()).collect::<Vec<_>>(),vec![false,true]);
    mixed.reverify().unwrap();
    for (curve,third) in [(curve,sheet(2.)),(wire.edges()[6].curve().clone(),sheet(0.))] {
        let proof=ss::QualifiedSsThirdSurfaceBranchCensus::qualify(source(&curve,&third),curve,Default::default()).unwrap();
        assert_eq!(proof.entries().len(),2);assert!(proof.entries().iter().all(|e|!e.is_inside()));proof.reverify().unwrap();
    }
}
#[test]
fn empty_proof_still_checks_original_graph_axis_and_complete_span_authority() {
    let wire=circle::wire();let curve=wire.edges()[2].curve().clone();
    let mut third=sheet(0.);for row in &mut third.control_points {for p in row {let y=p[1];let z=p[2];*p=vec![y,z,1.];}}
    let source=source(&curve,&third);
    let proof=ss::QualifiedSsThirdSurfaceBranchCensus::qualify(source.clone(),curve,Default::default()).unwrap();
    assert!(proof.entries().is_empty());assert_eq!(proof.queries(),0);proof.reverify().unwrap();
    assert!(ss::QualifiedSsThirdSurfaceBranchCensus::qualify(source,wire.edges()[0].curve().clone(),Default::default()).is_err());
}
#[test]
fn unresolved_lifts_and_global_or_local_budgets_never_return_partial_coverage() {
    let wire=circle::wire();let curve=wire.edges()[2].curve().clone();
    let mut third=sheet(0.);third.weights=vec![vec![1.,2.],vec![3.,4.]];
    assert!(ss::QualifiedSsThirdSurfaceBranchCensus::qualify(source(&curve,&third),curve.clone(),Default::default()).is_err());
    let source=source(&curve,&sheet(0.));
    for limits in [ss::SsThirdSurfaceBranchLimits {max_queries:1,..Default::default()},
        ss::SsThirdSurfaceBranchLimits {max_reserved_work:1,..Default::default()},
        ss::SsThirdSurfaceBranchLimits {roots:ss::SsAlgebraicRootCensusLimits {max_roots:1,..Default::default()},..Default::default()},
        ss::SsThirdSurfaceBranchLimits {point:ss::SsThirdSurfaceMembershipLimits {algebra:ss::ExactSsAlgebraLimits {max_work:1,..Default::default()},..Default::default()},..Default::default()}] {
        assert!(ss::QualifiedSsThirdSurfaceBranchCensus::qualify(source.clone(),curve.clone(),limits).is_err());
    }
}
