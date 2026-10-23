use nurbs_core::foundation::fitting::fit_surface_cloud_certified_report as fit;
#[test]
fn rotated_plane_with_correlated_axes_fits_without_pca_collapse() {
    let mut points=Vec::new();for i in 0..=5 {for j in 0..=5 {let u=i as f64/5.;let v=j as f64/5.;points.push([3.+2.*u-v,-2.+2.*u+v,5.+v]);}}
    let report=fit(points.clone(),2,2,None).unwrap();
    assert!(report.certificate.evidence.data_site_error_upper<1e-8);
    // Independent plane: X-Y+2Z=15, independent of PCA chart coordinates.
    for i in 0..=8 {for j in 0..=8 {let p=report.surface.evaluate(i as f64/8.,j as f64/8.).unwrap().point;assert!((p[0]-p[1]+2.*p[2]-15.).abs()<1e-8);}}
    for i in 0..=8 {for j in 0..=8 {
        let p=report.surface.evaluate(i as f64/8.,j as f64/8.).unwrap().point;
        let nearest=points.iter().map(|q|(p[0]-q[0]).hypot(p[1]-q[1]).hypot(p[2]-q[2])).fold(f64::INFINITY,f64::min);
        assert!(nearest<=report.certificate.evidence.hausdorff_error_upper);
    }}
}
#[test]
fn duplicating_sites_does_not_invent_a_smaller_continuum_bound() {
    let points=vec![[0.,0.,0.],[2.,0.,0.],[0.,1.,0.],[2.,1.,0.]];
    let a=fit(points.clone(),2,2,None).unwrap();
    let repeated=(0..16).flat_map(|_|points.clone()).collect();let b=fit(repeated,2,2,None).unwrap();
    assert!(a.certificate.evidence.hausdorff_error_upper>=5_f64.sqrt());
    assert!(b.certificate.evidence.hausdorff_error_upper>=5_f64.sqrt());
    assert!((a.certificate.evidence.hausdorff_error_upper-b.certificate.evidence.hausdorff_error_upper).abs()<1e-8);
}
#[test]
fn degenerate_clouds_and_invalid_inputs_have_typed_rejections() {
    assert!(fit(vec![[1.;3];8],2,2,None).is_err());
    assert!(fit((0..8).map(|i|[i as f64;3]).collect(),2,2,None).is_err());
    assert!(fit(vec![[0.;3];3],2,2,None).is_err());
    let mut bad=vec![[0.;3];4];bad[0][1]=f64::NAN;assert!(fit(bad,2,2,None).is_err());
    assert!(fit(vec![[0.;3];4],9,2,None).is_err());
}
#[test]
fn all_three_axis_aligned_plane_orientations_have_two_chart_axes() {
    for fixed in 0..3 {
        let free:Vec<_>=(0..3).filter(|&k|k!=fixed).collect();
        let mut points=Vec::new();for i in 0..=3 {for j in 0..=3 {
            let mut p=[5.;3];p[free[0]]=2.*i as f64/3.;p[free[1]]=3.*j as f64/3.;points.push(p);
        }}
        let r=fit(points,2,2,None).unwrap();
        assert!(r.certificate.evidence.data_site_error_upper<1e-8);
        assert!((r.surface.evaluate(0.37,0.63).unwrap().point[fixed]-5.).abs()<1e-8);
    }
}
