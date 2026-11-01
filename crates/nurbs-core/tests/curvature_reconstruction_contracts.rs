use nurbs_core::curve_reconstruction::reconstruct_from_curvature;
fn endpoint(k: &[(f64,f64)]) -> ([f64;2], f64) {
    let r=reconstruct_from_curvature(k,[0.;3],0.,1e-7).unwrap();
    let p=r.curve.evaluate(r.curve.domain()[1]).unwrap().point;
    ([p[0],p[1]],r.quadrature_error_estimate)
}
#[test]
fn nonuniform_straight_stations_have_no_spurious_quadrature_error() {
    let (p,e)=endpoint(&[(0.,0.),(0.1,0.),(0.7,0.),(1.,0.)]);
    assert!((p[0]-1.).abs()<1e-12 && p[1].abs()<1e-12);
    assert!(e<1e-14,"straight-line quadrature estimate {e}");
}
#[test]
fn linear_curvature_matches_an_independent_dense_fresnel_integral() {
    let k:Vec<_>=(0..=32).map(|i| {let s=i as f64/32.;(s,s)}).collect();
    let (p,_)=endpoint(&k);
    // Independent midpoint quadrature at 65536 panels; error is O(h^2).
    let mut reference=[0.;2];
    for i in 0..65536 {
        let s=(i as f64+0.5)/65536.;let theta=0.5*s*s;
        reference[0]+=theta.cos()/65536.;reference[1]+=theta.sin()/65536.;
    }
    assert!((p[0]-reference[0]).hypot(p[1]-reference[1])<2e-8,"{p:?} vs {reference:?}");
}
#[test]
fn nonuniform_linear_curvature_reports_the_integrated_station_angles() {
    let k=[(5.,-0.2),(5.1,-0.1),(5.35,0.15),(5.7,0.5),(6.,0.8)];
    let r=reconstruct_from_curvature(&k,[2.,-1.,19.],0.3,1e-5).unwrap();
    assert_eq!(r.sites.len(),k.len());assert_eq!(r.tangent_angles.len(),k.len());
    assert_eq!(r.sites[0],[2.,-1.,0.]);
    for (i,&(s,_)) in k.iter().enumerate() {
        let t=s-5.;let expected=0.3-0.2*t+0.5*t*t;
        assert!((r.tangent_angles[i]-expected).abs()<2e-15);
    }
    assert!(r.fitted_site_max_deviation.is_finite());
    assert_eq!(r.fit_converged,r.fitted_site_max_deviation<=1e-5);
}
#[test]
fn three_stations_are_supported_with_a_quadratic_fit() {
    let r=reconstruct_from_curvature(&[(0.,0.),(0.2,0.),(1.,0.)],[0.;3],0.,1e-8).unwrap();
    assert_eq!(r.curve.degree,2);assert!(r.fit_converged);
    assert!(r.fitted_site_max_deviation<1e-8);
    assert_eq!(r.undefined_curvature_sites,0);
    for p in r.sites {assert!(p[1].abs()<1e-14 && p[2]==0.);}
}
#[test]
fn constant_signed_curvature_follows_an_independent_circle_equation() {
    for sign in [-1.,1.] {
        let k:Vec<_>=(0..=40).map(|i|(i as f64/40.,sign*0.5)).collect();
        let r=reconstruct_from_curvature(&k,[0.;3],0.,1e-6).unwrap();
        for (i,p) in r.sites.iter().enumerate() {
            let s=i as f64/40.;let a=0.5*s;
            let expected=[2.*a.sin(),sign*2.*(1.-a.cos())];
            assert!((p[0]-expected[0]).hypot(p[1]-expected[1])<1e-10);
        }
    }
}
#[test]
fn malformed_and_oversized_samples_and_nonfinite_options_are_refused() {
    let valid=[(0.,0.),(0.5,0.),(1.,0.)];
    for tolerance in [0.,-1.,f64::NAN,f64::INFINITY] {
        assert!(reconstruct_from_curvature(&valid,[0.;3],0.,tolerance).is_err());
    }
    for k in [vec![],vec![(0.,0.),(1.,0.)],vec![(0.,0.),(0.,0.),(1.,0.)],vec![(0.,0.),(1.,f64::NAN),(2.,0.)],vec![(0.,0.),(1.,1e13),(2.,0.)],vec![(0.,0.);4097]] {
        assert!(reconstruct_from_curvature(&k,[0.;3],0.,1e-3).is_err());
    }
    assert!(reconstruct_from_curvature(&valid,[0.,f64::NAN,0.],0.,1e-3).is_err());
    assert!(reconstruct_from_curvature(&valid,[0.;3],f64::INFINITY,1e-3).is_err());
}
#[test]
fn rapid_constant_curvature_uses_more_panels_and_matches_a_circle() {
    let k=[(0.,4.),(0.2,4.),(0.6,4.),(1.,4.)];
    let r=reconstruct_from_curvature(&k,[0.;3],0.,1e-6).unwrap();
    assert!(r.integration_panels>k.len()-1);
    assert!(r.integration_panels<=nurbs_core::curve_reconstruction::MAX_INTEGRATION_PANELS);
    let end=r.sites.last().unwrap();
    assert!((end[0]-4_f64.sin()/4.).hypot(end[1]-(1.-4_f64.cos())/4.)<3e-6);
}
#[test]
fn unresolved_high_phase_and_unrepresentable_rotation_are_refused() {
    assert!(reconstruct_from_curvature(&[(0.,1e12),(0.5,1e12),(1.,1e12)],[0.;3],0.,1e-3).is_err());
    assert!(reconstruct_from_curvature(&[(0.,1.),(0.5,1.),(1.,1.)],[0.;3],1e300,1e-3).is_err());
}
