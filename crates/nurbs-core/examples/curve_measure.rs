//! Direct Rust arc measurements; no transport feature or viewer is required.
fn main() -> nurbs_core::Result<()> {
    use nurbs_core::{curve::Curve, curve_measure as measure};
    let path = Curve::from_polyline(vec![
        vec![0., 0., 0.],
        vec![10., 0., 0.],
        vec![10., 10., 0.],
    ])?;
    let total = measure::length(&path, 1e-7, 128)?;
    let point = measure::point_at_length(&path, 15., 1e-6, 1024)?;
    let divisions = measure::divide_by_length(&path, 4, 1e-6, 4096)?;
    assert!(total.within_tolerance && point.within_tolerance && divisions.within_tolerance);
    println!(
        "length bounds {:?}; point at 15 {:?}; {} equal-distance stations",
        total.bounds,
        point.point,
        divisions.points.len()
    );
    let circle = nurbs_core::primitives::circle([0.; 3], [0., 0., 1.], 3.)?;
    let circumference = measure::length(&circle, 1e-4, 16384)?;
    assert!(circumference.within_tolerance);
    println!(
        "circle length bounds {:?}; certified error <= {}; {} cell enclosures",
        circumference.bounds, circumference.error_upper, circumference.cells
    );
    Ok(())
}
