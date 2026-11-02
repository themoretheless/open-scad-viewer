use nurbs_core::construct::curves::airfoil::{CstProfile,cst_points,cst_fit};
fn profile()->CstProfile {CstProfile{n1:0.5,n2:1.,upper:vec![0.2,0.3,0.1],lower:vec![-0.15,-0.2,-0.1]}}
#[test]
fn inverse_fit_recovers_dimensionless_coefficients_at_nonunit_chord() {
    let p=profile();let points=cst_points(&p,2.5,64).unwrap();
    let r=cst_fit(&points,2,0.5,1.,0.).unwrap();
    for (a,b) in r.profile.upper.iter().zip(&p.upper).chain(r.profile.lower.iter().zip(&p.lower)) {assert!((a-b).abs()<1e-7,"recovered coefficient {a} vs {b}");}
}
#[test]
fn translated_chord_line_does_not_lose_the_lower_surface() {
    let p=profile();let mut points=cst_points(&p,1.,64).unwrap();
    for q in &mut points {q[0]+=10.;q[1]+=3.;}
    let r=cst_fit(&points,2,0.5,1.,0.).unwrap();
    for (a,b) in r.profile.upper.iter().zip(&p.upper).chain(r.profile.lower.iter().zip(&p.lower)) {assert!((a-b).abs()<1e-7);}
}
#[test]
fn sampled_profile_agrees_with_an_independent_quadratic_shape() {
    let p=profile();let samples=cst_points(&p,2.5,16).unwrap();
    for q in &samples[..=16] {
        let x=q[0]/2.5;
        let y=2.5*x.sqrt()*(1.-x)*(0.2*(1.-x).powi(2)+0.6*x*(1.-x)+0.1*x*x);
        assert!((q[1]-y).abs()<2e-15);
    }
    assert_eq!(samples.first(),samples.last());
}
#[test]
fn authored_vertical_3d_frame_round_trips_a_profile_and_places_its_nurbs() {
    use nurbs_core::construct::curves::airfoil::{CstFrame,cst_fit_in_frame};
    let frame=CstFrame::new([7.,-2.,3.],[0.,2.5,0.],[0.,0.,2.]).unwrap();
    let p=profile();let local=cst_points(&p,1.,64).unwrap();
    let points:Vec<_>=local.iter().map(|p|frame.point(p[0],p[1]).unwrap()).collect();
    assert!(cst_fit(&points,2,0.5,1.,0.).is_err());
    let r=cst_fit_in_frame(&points,frame,2,0.5,1.,0.).unwrap();
    for (a,b) in r.profile.upper.iter().zip(&p.upper).chain(r.profile.lower.iter().zip(&p.lower)) {assert!((a-b).abs()<1e-7);}
    let reconstructed=r.points(64).unwrap();
    for (a,b) in points.iter().zip(&reconstructed) {assert!(a.iter().zip(b).all(|(a,b)|(a-b).abs()<1e-9));}
    let c=r.curve(64,3,24).unwrap();
    for i in 0..=64 {assert!((c.evaluate(i as f64/64.).unwrap().point[0]-7.).abs()<1e-11);}
    assert!(r.max_out_of_plane<1e-12 && r.max_residual<1e-9);
}
#[test]
fn labeled_sides_support_two_positive_cambered_surfaces() {
    use nurbs_core::construct::curves::airfoil::{CstFrame,cst_fit_sides};
    let mut p=profile();p.lower=vec![0.05,0.1,0.03];
    let points=cst_points(&p,1.,64).unwrap();let frame=CstFrame::new([0.;3],[1.,0.,0.],[0.,1.,0.]).unwrap();
    let r=cst_fit_sides(&points[..65],&points[65..],frame,2,0.5,1.,0.).unwrap();
    for (a,b) in r.profile.upper.iter().zip(&p.upper).chain(r.profile.lower.iter().zip(&p.lower)) {assert!((a-b).abs()<1e-7);}
}
#[test]
fn out_of_plane_data_is_measured_in_3d_not_silently_fitted_exactly() {
    use nurbs_core::construct::curves::airfoil::{CstFrame,cst_fit_in_frame};
    let frame=CstFrame::new([0.;3],[1.,0.,0.],[0.,1.,0.]).unwrap();
    let mut points=cst_points(&profile(),1.,64).unwrap();points[20][2]=0.01;
    let r=cst_fit_in_frame(&points,frame,2,0.5,1.,0.).unwrap();
    assert!(r.max_out_of_plane>=0.01);
    assert!(r.hausdorff_residual>=0.00999);
}
#[test]
fn regularization_remains_dimensionless_under_world_scaling() {
    let a=cst_fit(&cst_points(&profile(),1.,64).unwrap(),2,0.5,1.,0.01).unwrap();
    let b=cst_fit(&cst_points(&profile(),4.,64).unwrap(),2,0.5,1.,0.01).unwrap();
    for (x,y) in a.profile.upper.iter().chain(&a.profile.lower).zip(b.profile.upper.iter().chain(&b.profile.lower)) {assert!((x-y).abs()<1e-12);}
    assert!((b.max_residual-4.*a.max_residual).abs()<1e-12);
}
#[test]
fn checked_nurbs_builder_and_unsupported_options_preserve_existing_limits() {
    use nurbs_core::construct::curves::airfoil::{CstFrame,cst_airfoil,cst_fit_in_frame,cst_shape};
    let c=cst_airfoil(&profile(),2.5,64,3,24).unwrap();c.validate().unwrap();
    assert_eq!(c.control_points.first(),c.control_points.last());
    assert!(cst_shape(0.5,0.5,1.,&[]).is_nan());
    assert!(CstFrame::new([0.;3],[0.;3],[0.,1.,0.]).is_err());
    assert!(CstFrame::new([0.;3],[1.,0.,0.],[1.,1.,0.]).is_err());
    let points=cst_points(&profile(),1.,64).unwrap();
    for order in [0,12,usize::MAX] {assert!(cst_fit(&points,order,0.5,1.,0.).is_err());}
    for lambda in [-1.,2.,f64::NAN,f64::INFINITY] {assert!(cst_fit(&points,2,0.5,1.,lambda).is_err());}
    assert!(cst_fit(&vec![[0.;3];4097],2,0.5,1.,0.).is_err());
    let frame=CstFrame::new([0.;3],[0.1,0.,0.],[0.,1.,0.]).unwrap();
    assert!(cst_fit_in_frame(&points,frame,2,0.5,1.,0.).is_err());
    assert!(cst_airfoil(&profile(),1.,2048,3,24).is_err());
}
#[test]
fn fitted_nurbs_tracks_an_independent_parabolic_profile_equation() {
    use nurbs_core::construct::curves::airfoil::cst_airfoil;
    let p=CstProfile{n1:1.,n2:1.,upper:vec![0.2,0.2],lower:vec![-0.2,-0.2]};
    let c=cst_airfoil(&p,1.,128,3,32).unwrap();
    for i in 0..=256 {
        let q=c.evaluate(i as f64/256.).unwrap().point;
        assert!((q[1].abs()-0.2*q[0]*(1.-q[0])).abs()<1e-3,"{q:?}");
    }
}
#[test]
fn frame_normalization_handles_extreme_finite_direction_scales() {
    use nurbs_core::construct::curves::airfoil::CstFrame;
    let frame=CstFrame::new([0.;3],[1.,0.,0.],[0.,f64::MAX,f64::MAX]).unwrap();
    let p=frame.point(0.5,0.2).unwrap();let expected=0.2/std::f64::consts::SQRT_2;
    assert!((p[0]-0.5).abs()<1e-15 && (p[1]-expected).abs()<1e-15 && (p[2]-expected).abs()<1e-15);
    let tiny=CstFrame::new([0.;3],[1e-300,0.,0.],[0.,1e-300,0.]).unwrap();
    assert_eq!(tiny.point(1.,1.).unwrap(),[1e-300,1e-300,0.]);
}
