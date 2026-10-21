use nurbs_core::{surface::Surface,surface_offset::{self,OffsetAssurance}};
fn saddle(delta:f64)->Surface {Surface {degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]],vec![vec![1.,0.,0.],vec![1.,1.,delta]]],weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false}}
#[test]
fn almost_planar_nonplanar_surface_is_not_an_exact_offset_proof() {
    let source=saddle(1e-10);let before=source.clone();
    // The fourth point is off the plane of the first three: nonzero scalar
    // triple product, independent of jet-grid classification tolerances.
    assert_ne!(source.control_points[1][1][2],0.);
    let report=surface_offset::offset(&source,0.1,1e-12).unwrap();
    assert!(!report.exact);assert_eq!(report.assurance(),OffsetAssurance::SampledDeviation);
    assert!(report.max_deviation>0. && report.max_deviation.is_finite());
    assert_eq!(source,before);
}
#[test]
fn identity_has_exact_assurance_and_general_invalid_inputs_are_rejected() {
    let source=saddle(0.2);let report=surface_offset::offset(&source,0.,1e-8).unwrap();
    assert!(report.exact);assert_eq!(report.assurance(),OffsetAssurance::ExactIdentity);
    assert_eq!(report.surface,source);assert_eq!(report.max_deviation,0.);
    for d in [f64::NAN,f64::INFINITY] {assert!(surface_offset::offset(&source,d,1e-8).is_err());}
    for t in [0.,-1.,f64::NAN] {assert!(surface_offset::offset(&source,0.1,t).is_err());}
}

#[test]
fn general_offset_error_is_measured_in_original_authored_uv_domain() {
    let mut source=saddle(0.8);
    source.knots_u=vec![2.,2.,7.,7.];source.knots_v=vec![-3.,-3.,1.,1.];
    let report=surface_offset::offset(&source,0.2,1e-12).unwrap();
    assert_eq!(report.assurance(),OffsetAssurance::SampledDeviation);
    assert!(!report.exact);
    assert_eq!(report.surface.knots_u[report.surface.degree_u],2.);
    assert_eq!(report.surface.knots_v[report.surface.degree_v],-3.);
    for i in 0..=4 {for j in 0..=4 {
        let u=2.+5.*i as f64/4.;let v=-3.+j as f64;
        let e=source.evaluate(u,v).unwrap();let n=e.unit_normal().unwrap();
        let p=report.surface.evaluate(u,v).unwrap().point;
        let error=(0..3).map(|k|(p[k]-e.point[k]-0.2*n[k]).powi(2)).sum::<f64>().sqrt();
        assert!(error<=report.max_deviation+1e-12,"{error} > {}",report.max_deviation);
    }}
}
