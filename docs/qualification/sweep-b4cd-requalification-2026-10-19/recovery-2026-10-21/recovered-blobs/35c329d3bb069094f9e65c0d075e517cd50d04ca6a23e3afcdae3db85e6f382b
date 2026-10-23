fn main() -> nurbs_core::Result<()> {
    use nurbs_core::{
        primitives,
        surface::{self, Axis},
        surface_edit,
    };
    let curve = primitives::circle_arc([0.; 3], [0., 0., 1.], 5., 0., 180.)?;
    let elevated = curve.elevate(4)?;
    let split = elevated.split(0.4)?;
    let shell = surface::extrude(&curve, [0., 0., 10.])?;
    let shell = surface_edit::insert(&shell, Axis::V, 0.5, 1)?;
    let parts = surface_edit::decompose(&shell)?;
    let switched = surface_edit::transpose(&shell)?;
    switched.validate()?;
    println!(
        "{} curve parts, {} Bezier surface patches",
        split.len(),
        parts.len()
    );
    Ok(())
}
