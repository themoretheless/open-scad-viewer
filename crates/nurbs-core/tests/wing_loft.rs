use nurbs_core::{primitives,wing_loft::{self,WingSection,TipClosure}};
fn section(span:f64)->WingSection {WingSection {profile:primitives::ellipse_arc([1.,0.,0.],[1.,0.,0.],[0.,0.15,0.],0.,360.).unwrap(),span,twist:0.1*span,dihedral:0.17,sweep:0.12,taper:1.-0.05*span}}
#[test]
fn rational_wing_interpolates_placed_airfoils_and_preserves_source() {
    let sections=[section(0.),section(2.),section(5.)];let before=sections.iter().map(|s|s.profile.clone()).collect::<Vec<_>>();
    let surface=wing_loft::loft_wing(&sections).unwrap();
    for s in &sections {for i in 0..=32 {
        let u=i as f64/32.;let q=s.profile.evaluate(4.*u).unwrap().point;
        let (sin,cos)=s.twist.sin_cos();let expected=[s.span*s.sweep.tan()+s.taper*(q[0]*cos-q[1]*sin),s.span.abs()*s.dihedral.tan()+s.taper*(q[0]*sin+q[1]*cos),s.span];
        let actual=surface.evaluate(u,s.span/5.).unwrap().point;
        for k in 0..3 {assert!((actual[k]-expected[k]).abs()<1e-10,"{actual:?} != {expected:?}");}
    }}
    assert!(sections.iter().zip(before).all(|(s,b)|s.profile==b));
}
#[test]
fn large_finite_reference_distances_do_not_overflow_squares() {
    let mut a=section(0.);let mut b=section(1.);a.twist=0.;b.twist=0.;b.span=1e200;b.dihedral=0.;b.sweep=0.;
    let surface=wing_loft::loft_wing(&[a,b]).unwrap();
    let p=surface.evaluate(0.2,0.5).unwrap().point;
    assert!((p[2]/1e200-0.5).abs()<1e-12);
}
#[test]
fn tip_policy_uses_actual_extra_section_budget_and_invalid_spans_fail() {
    let sections=(0..11).map(|i|section(i as f64)).collect::<Vec<_>>();
    assert!(wing_loft::loft_wing_with_tip(&sections,&TipClosure::None).is_ok());
    assert!(wing_loft::loft_wing_with_tip(&sections[..10],&TipClosure::Collapse).is_ok());
    assert!(wing_loft::loft_wing_with_tip(&sections[..9],&TipClosure::Dome {height:0.5}).is_ok());
    assert!(wing_loft::loft_wing_with_tip(&sections,&TipClosure::Collapse).is_err());
    assert!(wing_loft::loft_wing_with_tip(&sections[..10],&TipClosure::Dome {height:0.5}).is_err());
    assert!(wing_loft::loft_wing_with_tip(&sections[..2],&TipClosure::Dome {height:f64::NAN}).is_err());
    assert!(wing_loft::loft_wing(&[section(1.),section(0.)]).is_err());
    assert!(wing_loft::loft_wing(&[section(0.),section(0.)]).is_err());
    let mut bad=section(1.);bad.taper=0.;assert!(wing_loft::loft_wing(&[section(0.),bad]).is_err());
}
