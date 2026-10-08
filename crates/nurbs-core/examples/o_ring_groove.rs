fn main() -> nurbs_core::Result<()> {
    let curve = nurbs_core::engineering_profiles::o_ring_groove([0.; 3], 8., 5., 1.)?;
    let length = nurbs_core::curve_measure::length(&curve, 1e-4, 8192)?;
    let wall = nurbs_core::surface::extrude(&curve, [0., 0., 2.])?;
    let area = nurbs_core::surface_measure::area(&wall, 1e-3, 16384)?;
    assert!(length.within_tolerance && area.within_tolerance);
    println!(
        "O-ring groove perimeter {:?}, wall area {:?}",
        length.bounds, area.bounds
    );
    Ok(())
}
