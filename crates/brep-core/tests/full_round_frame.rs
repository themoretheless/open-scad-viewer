use brep_core::{Model, feature_ops::full_round, transform::affine};

fn selected(m: &Model) -> [usize; 3] {
    [(0, 0.), (1, 8.), (0, 4.)].map(|(axis, value)| {
        m.faces
            .iter()
            .position(|f| {
                f.surface
                    .control_points
                    .iter()
                    .flatten()
                    .all(|p| (p[axis] - value).abs() < 1e-9)
            })
            .unwrap()
    })
}

#[test]
fn full_round_is_covariant_under_rigid_frames_and_reflections() {
    let source = brep_core::cuboid([0.; 3], [4., 8., 6.]).unwrap();
    let faces = selected(&source);
    let baseline = full_round(&source, faces).unwrap();
    for reflection in [1., -1.] {
        let (s, c) = 0.73_f64.sin_cos();
        let (t, d) = (-0.41_f64).sin_cos();
        let matrix = [
            [reflection * c, -s * d, s * t, 17.],
            [reflection * s, c * d, -c * t, -23.],
            [0., t, d, 11.],
            [0., 0., 0., 1.],
        ];
        let placed = affine(&source, matrix).unwrap();
        let before = placed.clone();
        let actual = full_round(&placed, faces).unwrap();
        let expected = affine(&baseline.model, matrix).unwrap();
        assert_eq!(placed, before);
        assert!((actual.radius_mm - 2.).abs() < 1e-10);
        assert!(actual.audit.ok);
        assert_eq!(actual.tangency.model(), &actual.model);
        assert_eq!(actual.tangency.seams().len(), 3);
        assert_eq!(actual.tangency.envelopes().len(), 2);
        for (face, proof) in actual.tangency.envelopes() {
            assert!(proof.radius().error_upper() <= actual.model.tolerance_mm);
            let original =
                nurbs_core::surface_edit::transpose(&actual.model.faces[*face].surface).unwrap();
            assert_eq!(proof.radius().surface(), &original);
        }
        actual.model.validate().unwrap();
        // Compare geometric sets: reflection reverses the local axial frame,
        // so generated topology ordering need not match.
        for face in &expected.faces {
            for p in face.surface.control_points.iter().flatten() {
                assert!(
                    actual
                        .model
                        .faces
                        .iter()
                        .flat_map(|f| f.surface.control_points.iter().flatten())
                        .any(|q| p
                            .iter()
                            .zip(q)
                            .map(|(a, b)| (a - b).powi(2))
                            .sum::<f64>()
                            .sqrt()
                            < 1e-8)
                );
            }
        }
    }
}

#[test]
fn full_round_refuses_sheared_cuboid_without_mutation() {
    let source = brep_core::cuboid([0.; 3], [4., 8., 6.]).unwrap();
    let faces = selected(&source);
    let sheared = affine(
        &source,
        [
            [1., 0.2, 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let before = sheared.clone();
    assert!(full_round(&sheared, faces).is_err());
    assert_eq!(sheared, before);
}
