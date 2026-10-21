use nurbs_core::{radial_cam::{self as cam,Law,Motion,Request,Report,Stop}};
fn request(roller:f64)->Request {Request {center:[2.,-3.,5.],base_radius:4.,roller_radius:roller,phase_degrees:0.,tolerance:1e-5,max_spans:1024}}
fn point(r:&Report,t:f64)->[f64;3] {let c=r.curves.iter().find(|c| {let d=c.domain();d[0]<=t&&t<=d[1]}).unwrap();let p=c.evaluate(t).unwrap().point;[p[0],p[1],p[2]]}
fn coverage(r:&Report) {
    assert_eq!(r.cells.first().unwrap().angle[0],0.);assert_eq!(r.cells.last().unwrap().angle[1],std::f64::consts::TAU);
    for pair in r.cells.windows(2) {assert_eq!(pair[0].angle[1],pair[1].angle[0]);}
    for pair in r.curves.windows(2) {assert_eq!(pair[0].control_points.last(),pair[1].control_points.first());}
    assert_eq!(r.curves.first().unwrap().control_points.first(),r.curves.last().unwrap().control_points.last());
}
#[test]
fn dwell_circle_has_base_radius_for_knife_edge_and_roller() {
    for roller in [0.,0.75] {
        let r=cam::profile(request(roller),&[Motion {end_degrees:360.,end_lift:0.,law:Law::Dwell}]).unwrap();coverage(&r);
        assert_eq!(r.stop,Stop::SampleEstimateReached);assert!(!r.sampled_undercut);assert_eq!(r.max_sampled_pressure_angle,0.);
        for i in 0..=256 {let p=point(&r,std::f64::consts::TAU*i as f64/256.);assert!(((p[0]-2.).hypot(p[1]+3.)-4.).abs()<2e-5);assert!((p[2]-5.).abs()<1e-12);}
    }
}
#[test]
fn all_motion_laws_close_and_interpolate_lift_stations() {
    for law in [Law::Harmonic,Law::Cycloidal,Law::Polynomial345,Law::Polynomial4567] {
        let r=cam::profile(request(0.75),&[Motion {end_degrees:180.,end_lift:2.,law},Motion {end_degrees:360.,end_lift:0.,law}]).unwrap();coverage(&r);
        assert_eq!(r.stop,Stop::SampleEstimateReached);assert!(r.max_sampled_pressure_angle>0.&&r.max_sampled_pressure_angle<std::f64::consts::FRAC_PI_2);
        let top=point(&r,std::f64::consts::PI);assert!((top[0]+4.).abs()<1e-9 && (top[1]+3.).abs()<1e-9);
        assert!((point(&r,0.)[0]-6.).abs()<1e-9);
    }
}
#[test]
fn roller_contact_midrise_matches_independent_pitch_normal_geometry() {
    let r=cam::profile(request(0.75),&[Motion {end_degrees:180.,end_lift:2.,law:Law::Harmonic},Motion {end_degrees:360.,end_lift:0.,law:Law::Harmonic}]).unwrap();
    // At theta=pi/2: lift=1, dr/dtheta=1; pitch radius=5.75.
    let p=point(&r,std::f64::consts::FRAC_PI_2);let pitch=[2.,-3.+5.75];
    let d=[pitch[0]-p[0],pitch[1]-p[1]];
    assert!((d[0].hypot(d[1])-0.75).abs()<1e-8);
    let tangent=[-5.75,1.];assert!((d[0]*tangent[0]+d[1]*tangent[1]).abs()<1e-8);
}
#[test]
fn budget_preserves_cycle_and_undercut_is_reported_without_false_quality_claim() {
    let motions=[Motion {end_degrees:5.,end_lift:10.,law:Law::Harmonic},Motion {end_degrees:180.,end_lift:10.,law:Law::Dwell},Motion {end_degrees:185.,end_lift:0.,law:Law::Harmonic},Motion {end_degrees:360.,end_lift:0.,law:Law::Dwell}];
    let mut q=request(2.);q.max_spans=4;let r=cam::profile(q,&motions).unwrap();coverage(&r);assert_eq!(r.stop,Stop::SpanBudget);assert!(r.sampled_undercut);assert_eq!(r.cells.len(),4);
    let mut bad=q;bad.base_radius=0.;assert!(cam::profile(bad,&motions).is_err());
    bad=q;bad.roller_radius=-1.;assert!(cam::profile(bad,&motions).is_err());
    bad=q;bad.max_spans=3;assert!(cam::profile(bad,&motions).is_err());
    assert!(cam::profile(q,&[Motion {end_degrees:360.,end_lift:1.,law:Law::Harmonic}]).is_err());
    assert!(cam::profile(q,&[Motion {end_degrees:360.,end_lift:1.,law:Law::Dwell}]).is_err());
    assert!(cam::profile(q,&[]).is_err());
}

#[test]
fn unresolved_angular_precision_retains_full_cycle_without_zero_spans() {
    let motions=[Motion {end_degrees:180.,end_lift:0.,law:Law::Dwell},Motion {end_degrees:180.+1e-12,end_lift:1.,law:Law::Harmonic},Motion {end_degrees:180.+2e-12,end_lift:0.,law:Law::Harmonic},Motion {end_degrees:360.,end_lift:0.,law:Law::Dwell}];
    let mut q=request(0.);q.tolerance=1e-10;q.max_spans=4096;
    let r=cam::profile(q,&motions).unwrap();coverage(&r);
    assert_eq!(r.stop,Stop::ParameterPrecision);
    assert!(r.cells.iter().all(|c|c.angle[0]<c.angle[1]));
    assert!(r.max_sampled_error>q.tolerance);
}
