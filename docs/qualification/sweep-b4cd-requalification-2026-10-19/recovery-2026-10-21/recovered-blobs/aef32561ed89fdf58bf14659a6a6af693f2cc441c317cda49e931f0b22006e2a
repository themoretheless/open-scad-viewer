use nurbs_core::{Result, foundation};

fn main() -> Result<()> {
    let sites = vec![vec![0., 0.], vec![0.5, 0.], vec![1., 0.]];
    let fitted = foundation::fit_curve_points_report(sites.clone(), 2, None)?;
    assert_eq!(fitted.curve.degree, 1);
    assert!(fitted.certificate.data_site_error_upper < 1e-12);
    let cloud = foundation::fit_curve_cloud_certified_report(sites, 2, None)?;
    assert_eq!(cloud.certificate.evidence.rank, 2);
    assert!(cloud.certificate.evidence.hausdorff_error_upper.is_finite());
    let grid = vec![
        vec![[0., 0., 0.], [0., 1., 0.]],
        vec![[1., 0., 0.], [1., 1., 0.]],
    ];
    let interpolated = foundation::interpolate_surface_grid_report(grid.clone(), false, None)?;
    assert!(!interpolated.certificate.fitting);
    assert_eq!(interpolated.certificate.rank, 4);
    let cloud = foundation::fit_surface_cloud_certified_report(
        grid.into_iter().flatten().collect(),
        2,
        2,
        None,
    )?;
    assert_eq!(cloud.certificate.evidence.rank, 4);
    assert!(cloud.certificate.evidence.data_site_error_upper < 1e-12);
    println!(
        "Curve and surface fits, interpolation and cloud certificates available directly in Rust"
    );
    Ok(())
}
