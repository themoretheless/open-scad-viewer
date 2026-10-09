use nurbs_core::{Result, curve::Curve, foundation, surface, trim_point};

fn main() -> Result<()> {
    let curve = Curve::from_polyline(vec![vec![0., 0.], vec![10., 0.]])?;
    let projection = foundation::project_curve_report(&curve, &[4., 0.], None)?;
    println!("Projection: {:?}", projection.status);
    if let Some(index) = projection.winner_index {
        let candidate = &projection.candidates[index];
        println!("Parameter interval: {:?}", candidate.parameter_interval);
    }
    let trimmed = trim_point::trim_at_point_report(&curve, &[4., 0.], "start", 0.01)?;
    println!("Retained curve domain: {:?}", trimmed.curve.domain());
    let edge = Curve::from_polyline(vec![vec![0., 0., 0.], vec![10., 0., 0.]])?;
    let patch = surface::extrude(&edge, [0., 10., 0.])?;
    let surface_projection = foundation::project_surface_report(&patch, [3., 4., 2.], None)?;
    println!("Surface projection: {:?}", surface_projection.status);
    println!(
        "Uniqueness evidence: {:?}",
        surface_projection.uniqueness_proof
    );
    Ok(())
}
