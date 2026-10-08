fn main() -> nurbs_core::Result<()> {
    use nurbs_core::{curve_measure, engineering_profiles as profile, surface, surface_measure};
    let profiles = [
        ("keyway", profile::keyway([0.; 3], 6., 5.)?),
        ("T-slot", profile::t_slot([0.; 3], 4., 10., 3., 8.)?),
        ("dovetail", profile::dovetail([0.; 3], 4., 10., 3.)?),
        ("square", profile::square_fastener([0.; 3], 6.)?),
        ("hex", profile::hex_fastener([0.; 3], 6.)?),
        ("V-belt", profile::poly_v_belt([0.; 3], 3, 4., 2., 3.)?),
        (
            "toothed belt",
            profile::toothed_belt(
                [0.; 3],
                profile::ToothedBelt {
                    teeth: 3,
                    pitch: 6.,
                    base_width: 4.,
                    tip_width: 2.,
                    back_thickness: 2.,
                    tooth_height: 3.,
                },
            )?,
        ),
    ];
    for (name, curve) in profiles {
        let edges = curve.control_points.len() - 1;
        let perimeter = curve_measure::length(&curve, 1e-8, edges)?;
        let wall = surface::extrude(&curve, [0., 0., 7.])?;
        let area = surface_measure::area(&wall, 1e-7, edges)?;
        assert!(perimeter.within_tolerance && area.within_tolerance);
        println!(
            "{name}: {edges} edges, perimeter {:?}, extrusion wall area {:?}",
            perimeter.bounds, area.bounds
        );
    }
    Ok(())
}
