use nurbs_core::{surface::Surface,offset::fillet_surfaces::{self as f,FilletOptions,RadiusLaw}};
fn plane(u:[f64;3],v:[f64;3])->Surface {
    let p=|a:f64,b:f64|(0..3).map(|k|[2.,3.,4.][k]+10.*a*u[k]+10.*b*v[k]).collect();
    Surface {degree_u:1,degree_v:1,knots_u:vec![2.,2.,7.,7.],knots_v:vec![-3.,-3.,1.,1.],control_points:vec![vec![p(0.,0.),p(0.,1.)],vec![p(1.,0.),p(1.,1.)]],weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false}
}
fn supports()->(Surface,Surface) {(plane([1.,0.,0.],[0.,1.,0.]),plane([0.,1.,0.],[0.,0.,1.]))}
fn bounds(s:&Surface)->[[f64;2];2] {[[s.knots_u[s.degree_u],s.knots_u[s.control_points.len()]],[s.knots_v[s.degree_v],s.knots_v[s.control_points[0].len()]]]}
#[test]
fn translated_nonunit_domain_constant_fillet_matches_independent_cylinder() {
    let (a,b)=supports();let before=(a.clone(),b.clone());let r=1.25;
    let report=f::rolling_ball_fillet(&a,&b,r,&FilletOptions::default()).unwrap();
    assert!(report.failure.is_none(),"{:?}",report.failure);let s=report.surface.unwrap();let d=bounds(&s);
    for i in 0..=12 {for j in 0..=12 {
        let p=s.evaluate(d[0][0]+(d[0][1]-d[0][0])*i as f64/12.,d[1][0]+(d[1][1]-d[1][0])*j as f64/12.).unwrap().point;
        assert!(((p[0]-2.-r).hypot(p[2]-4.-r)-r).abs()<1e-6);
    }}
    assert_eq!((a,b),before);
}
#[test]
fn linear_and_spline_laws_keep_station_radii_and_plane_contacts() {
    let (a,b)=supports();
    for law in [RadiusLaw::Linear {start:0.5,end:1.5},RadiusLaw::Spline(vec![(0.,0.8),(0.5,1.2),(1.,0.6)])] {
        let report=f::variable_radius_fillet(&a,&b,law.clone(),&FilletOptions::default()).unwrap();
        assert!(report.failure.is_none(),"{:?}",report.failure);
        for p in &report.spine {
            let r=law.evaluate_checked(p.t).unwrap();assert!((p.radius-r).abs()<1e-10);
            assert!((p.center[0]-2.-r).abs()<1e-6 && (p.center[2]-4.-r).abs()<1e-6);
            assert!((p.contact_a[2]-4.).abs()<1e-6 && (p.contact_b[0]-2.).abs()<1e-6);
        }
        let s=report.surface.unwrap();let d=bounds(&s);
        for t in [0.07,0.37,0.93] {
            let v=d[1][0]+(d[1][1]-d[1][0])*t;
            let left=s.evaluate(d[0][0],v).unwrap();let right=s.evaluate(d[0][1],v).unwrap();
            assert!((left.point[2]-4.).abs()<1e-8 && (right.point[0]-2.).abs()<1e-8);
            let n=left.unit_normal().unwrap();assert!(n[0].abs()<1e-6 && n[1].abs()<1e-6);
            let n=right.unit_normal().unwrap();assert!(n[1].abs()<1e-6 && n[2].abs()<1e-6);
        }
    }
}
#[test]
fn invalid_public_laws_do_not_panic_and_builder_failures_are_explicit() {
    for law in [RadiusLaw::Spline(vec![]),RadiusLaw::Spline(vec![(0.,1.)]),RadiusLaw::Spline(vec![(0.,1.),(0.,2.)]),RadiusLaw::Constant(-1.)] {
        assert!(law.evaluate(0.5).is_nan());assert!(law.evaluate_checked(0.5).is_err());
    }
    assert!(RadiusLaw::Constant(1.).evaluate_checked(f64::NAN).is_err());
    // Positive natural-cubic data can overshoot below zero between knots.
    let overshoot=RadiusLaw::Spline(vec![(0.,1.),(1./3.,0.01),(2./3.,0.01),(1.,1.)]);
    assert!(overshoot.evaluate(0.5)<0.);
    assert!(overshoot.evaluate_checked(0.5).is_err());
    let (a,b)=supports();
    let rejected=f::variable_radius_fillet(&a,&b,overshoot,&FilletOptions::default()).unwrap();
    assert!(rejected.surface.is_none()&&rejected.failure.is_some());
    let (a,b)=supports();for r in [0.,-1.,f64::INFINITY] {assert!(f::rolling_ball_fillet(&a,&b,r,&FilletOptions::default()).is_err());}
    let mut options=FilletOptions::default();options.side_a=0.;assert!(f::rolling_ball_fillet(&a,&b,1.,&options).is_err());
    let mut parallel=a.clone();for p in parallel.control_points.iter_mut().flatten() {p[2]+=5.;}
    let failed=f::rolling_ball_fillet(&a,&parallel,1.,&FilletOptions::default()).unwrap();
    assert!(failed.surface.is_none()&&failed.failure.is_some());
}

#[test]
fn station_authored_radii_survive_trace_domain_changes_with_whole_skin_proof(){
    let (a,b)=supports();
    let o=FilletOptions::default();
    let report=f::linear_at_stations(&a,&b,1,[4.,12.],[0.7,1.3],&o).unwrap();
    println!("proposal converged={} iterations={} residual={} law={:?} failure={:?}",report.converged,report.iterations,report.station_residual_estimate_mm,report.normalized_law,report.fillet.failure);
    assert!(report.converged && report.iterations<=8);
    assert!(report.authored_endpoint_error_upper_mm<1e-12);
    let field=report.radius_field;
    assert!((field[1]*4.+field[3]-0.7).abs()<1e-12);
    assert!((field[1]*12.+field[3]-1.3).abs()<1e-12);
    let mut cx=field;cx[3]+=2.;let mut cz=field;cz[3]+=4.;
    let proof=nurbs_core::skin_radius::inspect_affine_field(report.fillet.surface.as_ref().unwrap(),[cx,[0.,1.,0.,0.],cz],field,1e-7,2000000).unwrap();
    println!("station target proposals={} residual={} full_skin={:?}",report.iterations,report.station_residual_estimate_mm,proof.radial_error_upper_mm);
    assert!(proof.within_tolerance);
    assert!(f::linear_at_stations(&a,&b,1,[12.,4.],[0.7,1.3],&o).is_err());
    assert!(f::linear_at_stations(&a,&b,3,[4.,12.],[0.7,1.3],&o).is_err());
}
