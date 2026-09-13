use brep_core::{cuboid, operations::shell_planar};

#[test]
fn shell_rejects_a_cavity_wholly_outside_the_original_body() {
    let stock = cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
    let openings: Vec<_> = stock
        .faces
        .iter()
        .enumerate()
        .filter_map(|(i, face)| {
            (!face
                .surface
                .control_points
                .iter()
                .flatten()
                .all(|point| point[2] == 0.))
            .then_some(i)
        })
        .collect();
    assert_eq!(openings.len(), 5);
    for thickness in [10., 11.] {
        let result = shell_planar(&stock, &openings, thickness);
        assert!(
            result.is_err(),
            "A shell whose remaining wall is as thick as or thicker than the entire body must fail, not return unchanged stock"
        );
    }
}

#[test]
fn shell_accepts_a_thick_single_wall_with_positive_cavity() {
    let stock = cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
    let openings: Vec<_> = stock
        .faces
        .iter()
        .enumerate()
        .filter_map(|(i, face)| {
            (!face
                .surface
                .control_points
                .iter()
                .flatten()
                .all(|point| point[2] == 0.))
            .then_some(i)
        })
        .collect();
    let result = shell_planar(&stock, &openings, 9.).unwrap();
    let properties = brep_core::analysis::mass_properties(&result, 1e-7, 200_000).unwrap();
    assert!((properties.signed_volume_mm3 - 900.).abs() < 1e-6);
}
