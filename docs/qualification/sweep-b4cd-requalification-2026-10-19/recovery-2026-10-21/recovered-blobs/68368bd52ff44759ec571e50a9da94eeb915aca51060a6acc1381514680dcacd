fn main() -> nurbs_core::Result<()> {
    use nurbs_core::{patches, primitives, surface_measure};
    let plane = patches::plane([3., -2., 5.], [3., 0., 1.], [0., 4., 2.])?;
    let report = surface_measure::area(&plane, 1e-9, 1)?;
    assert!(report.within_tolerance);
    println!(
        "oblique panel area {:?}, error <= {}",
        report.bounds, report.error_upper
    );
    let cylinder = primitives::cylinder([0.; 3], 2., 3.)?;
    let report = surface_measure::area(&cylinder, 0.03, 32768)?;
    assert!(report.within_tolerance);
    println!(
        "cylinder lateral area {:?}, error <= {}, {} cell enclosures",
        report.bounds, report.error_upper, report.cells
    );
    Ok(())
}
