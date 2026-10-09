use nurbs_core::{Result, curve::Curve, intersection};

fn main() -> Result<()> {
    let first = Curve::from_polyline(vec![vec![-1., 0., 0.], vec![1., 0., 0.]])?;
    let second = Curve::from_polyline(vec![vec![0., -1., 0.], vec![0., 1., 0.]])?;
    let intersection = intersection::intersect_curve_curve_report(&first, &second, None)?;
    for component in &intersection.report.components {
        println!("{:?}: {:?}", component.kind, component.point);
    }
    println!(
        "Unresolved regions: {}",
        intersection.report.unresolved.len()
    );
    Ok(())
}
