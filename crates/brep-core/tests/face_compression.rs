use brep_core::{Model, cuboid};

fn refine_u(model: &mut Model, face: usize) {
    let surface = &mut model.faces[face].surface;
    assert_eq!(surface.degree_u, 1);
    assert_eq!(surface.control_points.len(), 2);
    assert!(surface.weights.iter().flatten().all(|&weight| weight == 1.));
    let middle = surface.control_points[0]
        .iter()
        .zip(&surface.control_points[1])
        .map(|(a, b)| a.iter().zip(b).map(|(x, y)| (x + y) * 0.5).collect())
        .collect();
    surface.control_points.insert(1, middle);
    surface.weights.insert(1, surface.weights[0].clone());
    surface
        .knots_u
        .insert(2, (surface.knots_u[1] + surface.knots_u[2]) * 0.5);
}

#[test]
fn selected_trimmed_faces_recover_exact_original_solid() {
    let original = cuboid([0.; 3], [8., 4., 2.]).unwrap();
    let mut refined = original.clone();
    refine_u(&mut refined, 0);
    refine_u(&mut refined, 1);
    let mut expected = original.clone();
    expected.faces[1].surface = refined.faces[1].surface.clone();
    assert_eq!(
        refined.compress_face_surfaces(&[0], 1_000_000).unwrap(),
        Some(expected)
    );
    assert_eq!(
        refined.compress_face_surfaces(&[1, 0], 1_000_000).unwrap(),
        Some(original)
    );
    assert!(
        refined
            .compress_face_surfaces(&[0, 1], 1)
            .unwrap()
            .is_none()
    );
    for indices in [&[0, 0][..], &[6], &[]] {
        assert!(refined.compress_face_surfaces(indices, 1_000_000).is_err());
    }
    assert!(refined.compress_face_surfaces(&[0], 0).is_err());
}

#[test]
fn restores_both_trimmed_caps_of_a_herringbone_sun_with_a_bore() {
    let source = brep_core::gear(&brep_core::GearSpec {
        module: 1.5,
        teeth: 32,
        height: 10.,
        helix_angle_deg: -35.,
        herringbone: true,
        clearance: 0.1,
        backlash: 0.05,
        bore: 42.,
        ..Default::default()
    })
    .unwrap();
    let ends: Vec<_> = source
        .faces
        .iter()
        .enumerate()
        .filter_map(|(i, face)| (!face.holes.is_empty()).then_some(i))
        .collect();
    assert_eq!(ends.len(), 2);
    let mut refined = source.clone();
    for &face in &ends {
        refine_u(&mut refined, face);
    }
    let restored = refined.compress_face_surfaces(&ends, 1_000_000).unwrap();
    assert!(restored.as_ref() == Some(&source));
}
