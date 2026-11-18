use nurbs_core::ss_intersection as ss;
use std::sync::Arc;
#[allow(dead_code)]
#[path="support/ss_original_ideal.rs"]
mod fixture;
fn charts() -> [Arc<ss::OriginalSsRootCurve>;2] {
    let mut a=fixture::plane(0.);
    let mut b=fixture::plane(0.);
    for row in &mut b.control_points {for p in row {let v=p[1];p[1]=0.5;p[2]=v-0.5;}}
    a.knots_u=vec![-2.,-2.,6.,6.];a.knots_v=vec![3.,3.,5.,5.];
    b.knots_u=vec![-5.,-5.,3.,3.];b.knots_v=vec![10.,10.,14.,14.];
    let source=ss::FiberSources::new(&a,&b).unwrap();
    let ideal=ss::qualify_original_ss_ideal(&source,[1;4],Default::default()).unwrap();
    let basis=ss::qualify_ss_groebner_basis(&ideal,Default::default()).unwrap();
    let system=ss::qualify_ss_deflated_system(&basis,Default::default()).unwrap();
    let local=ss::qualify_deflated_ss_fiber(&system,[[0.30,0.45],[0.4,0.6],[0.34,0.38],[0.4,0.6]],ss::FiberAxis::SecondU,Default::default()).unwrap().into_fiber().unwrap();
    let native=source.certify([[0.4,1.4],[3.9,4.1],[-2.3,-1.7],[11.9,12.1]],ss::FiberAxis::SecondU,Default::default()).unwrap().into_fiber().unwrap();
    [Arc::new(ss::OriginalSsRootCurve::from_deflated(local)),Arc::new(ss::OriginalSsRootCurve::from_native(native))]
}
fn point(c:Arc<ss::OriginalSsRootCurve>,p:&[f64],d:[f64;2])->Arc<ss::OriginalSsRootPoint> {
    let clock=Arc::new(ss::QualifiedSsAlgebraicClock::from_binary(p,d,Default::default()).unwrap());
    Arc::new(ss::OriginalSsRootPoint::from_algebraic(ss::OriginalSsAlgebraicPoint::new(c,clock).unwrap(),Default::default()).unwrap())
}
#[test]
fn local_and_original_clocks_join_only_after_frame_and_branch_membership() {
    let [local,native]=charts();
    let a=point(local.clone(),&[-1.,0.,8.],[0.34375,0.375]);
    let b=point(native.clone(),&[17.,10.,1.],[-2.3,-1.7]);
    assert!(!a.same_selector(&b));
    assert!(!a.same_source_point(&b,Default::default()).unwrap());
    assert!(a.same_source_point_in_charts(&b,Default::default(),Default::default()).unwrap());
    assert!(b.same_source_point_in_charts(&a,Default::default(),Default::default()).unwrap());
    let port=ss::bind_original_ss_port(a.clone(),local.clone(),[0;4],Default::default()).unwrap();
    let joined=port.rebind_equivalent(b.clone(),Default::default()).unwrap();
    assert!(Arc::ptr_eq(joined.point(),&b));
    joined.reverify().unwrap();
    let other=point(native.clone(),&[17.001,10.,1.],[-2.3,-1.7]);
    assert!(!a.same_source_point_in_charts(&other,Default::default(),Default::default()).unwrap());
    assert!(port.rebind_equivalent(other,Default::default()).is_err());
    assert!(a.same_source_point_in_charts(&b,ss::SsAlgebraicClockLimits {max_operations:1,..Default::default()},Default::default()).is_err());
    assert!(a.same_source_point_in_charts(&b,Default::default(),ss::OriginalSsCurveLimits {world_rectangles:0,..Default::default()}).is_err());
    let rational_local=point(local.clone(),&[-0.375,1.],[0.375;2]);
    let rational_original=point(native.clone(),&[2.,1.],[-2.;2]);
    let binary_local=ss::OriginalSsRootPoint::new(local,0.375).unwrap();
    let binary_original=ss::OriginalSsRootPoint::new(native,-2.).unwrap();
    assert!(binary_local.same_source_point_in_charts(&binary_original,Default::default(),Default::default()).unwrap());
    assert!(rational_local.same_source_point_in_charts(&binary_original,Default::default(),Default::default()).unwrap());
    assert!(binary_local.same_source_point_in_charts(&rational_original,Default::default(),Default::default()).unwrap());
}
#[path="support/ss_root_circle.rs"]
mod circle;
#[test]
fn chart_reconciliation_preserves_a_real_closed_curve_and_rejects_other_branch() {
    let contour=circle::wire();
    let first=&contour.edges()[0];
    let independent=point(first.curve().clone(),&[-5.,16.],[0.3125;2]);
    let end=ss::bind_original_ss_port(independent,first.curve().clone(),[0;4],Default::default()).unwrap();
    let mut edges=vec![ss::OriginalSsRootEdge::new(first.start().clone(),end).unwrap()];
    edges.extend_from_slice(&contour.edges()[1..]);
    assert!(ss::OriginalSsRootWire::new(edges.clone()).is_err());
    let wire=ss::OriginalSsRootWire::reconcile_selectors(edges,Default::default()).unwrap();
    wire.reverify().unwrap();
    let winding=ss::qualify_original_ss_winding(&wire,0,[0.5,0.5],Default::default()).unwrap();
    assert_eq!(winding.winding(),1);
    let upper=point(contour.edges()[0].curve().clone(),&[-1.,0.,8.],[0.34375,0.375]);
    let lower=point(contour.edges()[4].curve().clone(),&[-1.,0.,8.],[0.34375,0.375]);
    assert!(upper.same_source_point_in_charts(&lower,Default::default(),Default::default()).is_err());
}
#[test]
fn different_free_axes_require_original_equations_and_selected_root_membership() {
    let contour=circle::wire();
    let a=point(contour.edges()[1].curve().clone(),&[-257.,0.,4096.],[0.25,0.251]);
    let coefficients=[2576353.,-15605760.,32382976.,-33554432.,16777216.];
    let b=point(contour.edges()[2].curve().clone(),&coefficients,[0.687,0.689]);
    assert_ne!(a.curve().free_axis(),b.curve().free_axis());
    assert!(a.same_source_point_in_charts(&b,Default::default(),Default::default()).unwrap());
    assert!(b.same_source_point_in_charts(&a,Default::default(),Default::default()).unwrap());
    let port=ss::bind_original_ss_port(a.clone(),a.curve().clone(),[0;4],Default::default()).unwrap();
    let joined=port.rebind_equivalent(b.clone(),Default::default()).unwrap();
    assert!(Arc::ptr_eq(joined.point(),&b));joined.reverify().unwrap();
    let lower=point(contour.edges()[2].curve().clone(),&coefficients,[0.311,0.313]);
    assert!(!a.same_source_point_in_charts(&lower,Default::default(),Default::default()).unwrap());
    let mut wrong=coefficients;wrong[0]+=1.;
    let wrong=point(contour.edges()[2].curve().clone(),&wrong,[0.687,0.689]);
    assert!(!a.same_source_point_in_charts(&wrong,Default::default(),Default::default()).unwrap());
    assert!(a.same_source_point_in_charts(&b,Default::default(),ss::OriginalSsCurveLimits {
        projection:ss::ExactSsAlgebraLimits {max_work:1,..Default::default()},..Default::default()
    }).is_err());
}
#[test]
fn target_clock_is_generated_from_original_equations_and_owned_source_root() {
    let contour=circle::wire();
    let source=point(contour.edges()[1].curve().clone(),&[-257.,0.,4096.],[0.25,0.251]);
    let target=contour.edges()[2].curve().clone();
    let projected=source.reparameterize_to(target.clone(),Default::default(),Default::default()).unwrap();
    projected.reverify().unwrap();
    assert_eq!(projected.curve().free_axis(),ss::FiberAxis::SecondV);
    assert!(projected.binary_parameter().is_none());
    let expected=point(target,&[2576353.,-15605760.,32382976.,-33554432.,16777216.],[0.687,0.689]);
    assert!(projected.same_source_point(&expected,Default::default()).unwrap());
    assert!(source.same_source_point_in_charts(&projected,Default::default(),Default::default()).unwrap());
    let returned=projected.reparameterize_to(source.curve().clone(),Default::default(),Default::default()).unwrap();
    assert!(source.same_source_point(&returned,Default::default()).unwrap());
    assert!(source.reparameterize_to(contour.edges()[4].curve().clone(),Default::default(),Default::default()).is_err());
    assert!(source.reparameterize_to(projected.curve().clone(),ss::SsAlgebraicClockLimits {max_degree:2,..Default::default()},Default::default()).is_err());
    assert!(source.reparameterize_to(projected.curve().clone(),Default::default(),ss::OriginalSsCurveLimits {
        projection:ss::ExactSsAlgebraLimits {max_work:1,..Default::default()},..Default::default()
    }).is_err());
}
