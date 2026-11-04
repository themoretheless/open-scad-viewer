use nurbs_core::{ss_intersection as ss,surface::Surface};
use num_rational::BigRational as Q;
#[path="support/ss_root_circle.rs"]
mod circle;
fn original()->ss::QualifiedOriginalSsIdeal {
    let wire=circle::wire();
    ss::qualify_original_ss_ideal(wire.edges()[0].curve().source(),[1,1,2,2],Default::default()).unwrap()
}
fn sheet(x:f64,y:f64,rational:bool)->Surface {
    Surface {degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:vec![vec![vec![x,y,-1.],vec![x,y,1.]],vec![vec![x,y+1.,-1.],vec![x,y+1.,1.]]],
        weights:if rational {vec![vec![1.,2.],vec![3.,4.]]} else {vec![vec![1.;2];2]},periodic_u:false,periodic_v:false}
}
fn q(n:i64,d:i64)->Q {Q::new(n.into(),d.into())}
#[test]
fn actual_third_surface_generates_and_replays_a_source_clock_constraint() {
    let ideal=original();
    let third=sheet(0.25,0.,false);
    let proof=ss::qualify_ss_third_surface_candidates(&ideal,&third,[1,1],3,Default::default()).unwrap();
    assert_eq!(proof.axis(),3);assert_eq!(proof.third_spans(),[1,1]);assert_eq!(proof.third(),&third);
    assert!(!proof.proves_disjoint_on_spans());assert!(proof.work()>0);
    assert_eq!(proof.polynomial(),&[q(55,256),q(-1,1),q(1,1)]);
    let clock=proof.qualify_candidate_clock([q(68,100),q(70,100)],Default::default()).unwrap();
    assert!(clock.equals_rational(&q(11,16),Default::default()).unwrap().0);
    clock.reverify().unwrap();proof.reverify().unwrap();
    let all=proof.isolate_candidate_clocks([q(0,1),q(1,1)],Default::default()).unwrap();
    assert_eq!(all.roots().len(),2);
    assert!(all.roots()[0].equals_rational(&q(5,16),Default::default()).unwrap().0);
    assert!(all.roots()[1].equals_rational(&q(11,16),Default::default()).unwrap().0);
    all.reverify().unwrap();
    let x=ss::qualify_ss_third_surface_candidates(&ideal,&third,[1,1],2,Default::default()).unwrap();
    assert_eq!(x.polynomial(),&[q(-1,4),q(1,1)]);
    let wire=circle::wire();
    let on_branch=wire.edges()[0].curve().third_surface_candidates(&third,[1,1],Default::default()).unwrap();
    assert_eq!(on_branch.polynomial(),x.polynomial());
    assert_eq!(on_branch.axis(),2);
}
#[test]
fn nonconstant_rational_weights_preserve_original_triple_equation_constraints() {
    let proof=ss::qualify_ss_third_surface_candidates(&original(),&sheet(0.25,0.,true),[1,1],3,Default::default()).unwrap();
    assert_eq!(proof.polynomial(),&[q(55,256),q(-1,1),q(1,1)]);
    proof.reverify().unwrap();
}
#[test]
fn an_elimination_root_does_not_authorize_bounded_third_face_membership() {
    // The third patch is entirely at y>=2. Complex/unbounded parameter
    // elimination still yields the same candidate polynomial as the near patch.
    let proof=ss::qualify_ss_third_surface_candidates(&original(),&sheet(0.25,2.,false),[1,1],3,Default::default()).unwrap();
    let candidate=proof.qualify_candidate_clock([q(68,100),q(70,100)],Default::default()).unwrap();
    assert!(candidate.equals_rational(&q(11,16),Default::default()).unwrap().0);
    assert!(!proof.proves_disjoint_on_spans());
}
#[test]
fn coincident_surface_invalid_inputs_and_exhaustion_do_not_claim_candidates() {
    let ideal=original();
    assert!(ss::qualify_ss_third_surface_candidates(&ideal,ideal.source().first(),[1,1],3,Default::default()).is_err());
    assert!(ss::qualify_ss_third_surface_candidates(&ideal,&sheet(0.25,0.,false),[1,1],4,Default::default()).is_err());
    assert!(ss::qualify_ss_third_surface_candidates(&ideal,&sheet(0.25,0.,false),[0,1],3,Default::default()).is_err());
    assert!(ss::qualify_ss_third_surface_candidates(&ideal,&sheet(0.25,0.,false),[1,1],3,ss::ExactSsGroebnerLimits {
        algebra:ss::ExactSsAlgebraLimits {max_work:1,..Default::default()},..Default::default()
    }).is_err());
    let mut invalid=sheet(0.25,0.,false);invalid.weights[0][0]=0.;
    assert!(ss::qualify_ss_third_surface_candidates(&ideal,&invalid,[1,1],3,Default::default()).is_err());
}
#[test]
fn original_triple_equations_can_prove_disjointness_without_real_sampling() {
    let mut third=sheet(0.25,0.,true);
    for row in &mut third.control_points {for p in row {let y=p[1];let z=p[2];*p=vec![y,z,1.];}}
    let proof=ss::qualify_ss_third_surface_candidates(&original(),&third,[1,1],2,Default::default()).unwrap();
    assert!(proof.proves_disjoint_on_spans());
    assert_eq!(proof.polynomial(),&[q(1,1)]);
    assert!(proof.isolate_candidate_clocks([q(0,1),q(1,1)],Default::default()).unwrap().roots().is_empty());
    assert!(proof.qualify_candidate_clock([q(0,1),q(1,1)],Default::default()).is_err());
    proof.reverify().unwrap();
}
