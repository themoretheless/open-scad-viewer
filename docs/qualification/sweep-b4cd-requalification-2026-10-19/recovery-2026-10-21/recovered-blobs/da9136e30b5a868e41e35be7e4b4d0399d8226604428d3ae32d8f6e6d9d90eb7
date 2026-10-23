//! Run with `cargo run -p nurbs-core --no-default-features --example engineering_profiles`.
fn main() -> nurbs_core::Result<()> {
    use nurbs_core::engineering_profiles::{annular_sector, capsule, rounded_rectangle};
    let body = rounded_rectangle([0.; 3], 80., 50., 6.)?;
    let slot = capsule([0.; 3], 20., 4.)?;
    let flange = annular_sector([0.; 3], [0., 0., 1.], 10., 20., 0., 360.)?;
    // Profiles are directly usable by native extrusion; no JSON or viewer.
    let wall = nurbs_core::surface::extrude(&body, [0., 0., 15.])?;
    wall.validate()?;
    slot.validate()?;
    flange.validate()?;
    println!("rounded body: {} controls; capsule: {} controls; flange: {} angular controls", body.control_points.len(), slot.control_points.len(), flange.control_points.len());
    Ok(())
}
