use nurbs_core::{Result, foundation};
fn main() -> Result<()> {
    let sites = vec![vec![0., 0.], vec![1., 0.], vec![1., 1.]];
    let interpolation = foundation::interpolate_polyline_report(sites, None)?;
    let approximation = foundation::approximate_curve_report(&interpolation.curve, None)?;
    assert!(approximation.hausdorff_error_upper.is_finite());
    assert!(approximation.hausdorff_error_upper >= 1.);
    assert_eq!(
        approximation.curve.control_points,
        interpolation.curve.control_points
    );
    println!(
        "Conservative error bound: {}",
        approximation.hausdorff_error_upper
    );
    Ok(())
}
