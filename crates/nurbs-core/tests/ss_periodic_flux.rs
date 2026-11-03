use nurbs_core::{interval_eval::Interval as I,surface_flux};
#[path="common/periodic_extrusion.rs"]mod fixture;
#[test]
fn original_periodic_chart_flux_keeps_seam_identity_and_source_flags() {
    let source=fixture::source(1);let before=source.clone();let mut total=I::point(0.);
    for i in 0..128 {
        let r=surface_flux::bound(&source,[[i as f64/32.,(i+1) as f64/32.],[0.,1.]],0,0.,4).unwrap();
        let b=r.bound.unwrap();total=total.add(I::new(b.integral[0],b.integral[1]).unwrap()).unwrap();
    }
    // Independent circular-cylinder value; the binary rational weight's
    // deviation from sqrt(1/2) is far below this enclosing interval's width.
    let expected=2.*std::f64::consts::PI;
    assert!(total.lo<expected && expected<total.hi,"{total:?}");
    assert!(total.hi-total.lo<0.05,"{total:?}");assert_eq!(source,before);
    assert!(surface_flux::bound(&source,[[0.,4.],[0.,1.]],0,0.,0).unwrap().bound.is_none());
    let mut broken=source;broken.control_points[0][0][0]+=0.125;
    assert!(surface_flux::bound(&broken,[[0.,4.],[0.,1.]],0,0.,4).is_err());
}
#[test]
fn exact_polynomial_flux_uses_original_nonunit_clocks_and_shared_work() {
    use nurbs_core::surface::Surface;
    let plane=Surface{degree_u:1,degree_v:1,knots_u:vec![2.,2.,5.,5.],knots_v:vec![-4.,-4.,-2.,-2.],
        control_points:vec![vec![vec![0.,0.,5.],vec![0.,3.,5.]],vec![vec![2.,0.,5.],vec![2.,3.,5.]]],
        weights:vec![vec![3.;2];2],periodic_u:false,periodic_v:false};
    let report=surface_flux::polynomial_bound(&plane,[[2.,5.],[-4.,-2.]],2,0.,1,Default::default()).unwrap();
    assert_eq!(report.integral,Some([30.,30.]));assert_eq!(report.spans,1);assert!(report.work>0);
    let exhausted=surface_flux::polynomial_bound(&plane,[[2.,5.],[-4.,-2.]],2,0.,1,
        nurbs_core::ss_intersection::ExactSsAlgebraLimits{max_work:1,..Default::default()}).unwrap();
    assert!(exhausted.integral.is_none());assert_eq!(exhausted.work,1);assert_eq!(exhausted.spans,1);
    let no_span=surface_flux::polynomial_bound(&plane,[[2.,5.],[-4.,-2.]],2,0.,0,Default::default()).unwrap();
    assert!(no_span.integral.is_none());assert_eq!(no_span.work,0);
    let mut rational=plane;rational.weights[0][0]=4.;
    let fallback=surface_flux::polynomial_bound(&rational,[[2.,5.],[-4.,-2.]],2,0.,1,Default::default()).unwrap();
    assert!(fallback.integral.is_none());assert_eq!(fallback.work,0);
    let mut polynomial=fixture::source(1);for row in &mut polynomial.weights {for w in row {*w=1.;}}
    let periodic=surface_flux::polynomial_bound(&polynomial,[[0.,4.],[0.,1.]],0,0.,4,Default::default()).unwrap().integral.unwrap();
    let expected=20./3.;assert!(periodic[0]<=expected && expected<=periodic[1],"{periodic:?}");
}
