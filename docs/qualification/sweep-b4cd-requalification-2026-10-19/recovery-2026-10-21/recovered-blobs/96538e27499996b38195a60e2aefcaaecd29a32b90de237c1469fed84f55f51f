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

#[test]
fn cylinder_cap_push_preserves_circles_and_opposite_cap_under_placement() {
    use brep_core::{cylinder, operations::push_planar_face, transform::affine};
    let stock = cylinder(3., 5.).unwrap();
    for z in [0., 5.] {
        let cap = stock
            .faces
            .iter()
            .position(|f| f.surface.control_points.iter().flatten().all(|p| p[2] == z))
            .unwrap();
        for distance in [20., -2.] {
            let pushed = push_planar_face(&stock, cap, distance).unwrap();
            pushed.validate().unwrap();
            for (before, after) in stock.vertices.iter().zip(&pushed.vertices) {
                assert!((before.point[0] - after.point[0]).abs() < 1e-8);
                assert!((before.point[1] - after.point[1]).abs() < 1e-8);
                let expected = before.point[2]
                    + if before.point[2] == z {
                        if z == 0. { -distance } else { distance }
                    } else {
                        0.
                    };
                assert!((after.point[2] - expected).abs() < 1e-8);
            }
            for (before, after) in stock.edges.iter().zip(&pushed.edges) {
                assert_eq!(before.curve.weights, after.curve.weights);
                assert_eq!(before.curve.degree, after.curve.degree);
            }
            let placement = [
                [0., 0., 1., 17.],
                [1., 0., 0., -8.],
                [0., 1., 0., 4.],
                [0., 0., 0., 1.],
            ];
            let placed = affine(&stock, placement).unwrap();
            let actual = push_planar_face(&placed, cap, distance).unwrap();
            let expected = affine(&pushed, placement).unwrap();
            for (a, b) in actual.vertices.iter().zip(&expected.vertices) {
                for k in 0..3 {
                    assert!((a.point[k] - b.point[k]).abs() < 1e-8);
                }
            }
            // The result stays recognizable for subsequent edits.
            push_planar_face(&actual, cap, 1.).unwrap();
        }
        assert!(push_planar_face(&stock, cap, -5.).is_err());
        assert!(push_planar_face(&stock, cap, -6.).is_err());
    }
    let side = stock
        .faces
        .iter()
        .position(|f| f.surface.degree_u > 1)
        .unwrap();
    assert!(push_planar_face(&stock, side, 2.).is_err());
    assert!(push_planar_face(&stock, usize::MAX, 2.).is_err());
}
