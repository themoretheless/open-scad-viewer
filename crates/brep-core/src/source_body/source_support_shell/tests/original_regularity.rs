use super::*;
use crate::source_volume::{self, regularity};

#[test]
fn material_body_owns_all_original_face_ranks_with_one_budget() {
    let model = crate::cuboid([0.; 3], [10., 20., 30.]).unwrap();
    let shell = prepare_projective(&model, limits()).unwrap().shell.unwrap();
    let geometry = crate::source_shell_geometry::qualify_projective(shell, geometry_limits())
        .unwrap()
        .geometry
        .unwrap();
    let body = source_volume::qualify(
        geometry,
        source_volume::Limits {
            axis: 0,
            origin: 0.,
            absolute_error: 0.01,
            tolerance_uv: 1e-8,
            cells: 10000,
            spans: 100000,
            domain_cells: 100000,
        },
    )
    .unwrap()
    .body
    .unwrap();
    let original = body.definition().unwrap();
    let report = regularity::qualify(&body, 100000).unwrap();
    let cells = report.cells;
    let proof = report.certificate.expect(report.reason);
    assert!(std::ptr::eq(proof.body(), &body));
    assert_eq!(proof.faces().len(), 6);
    assert!(proof.faces().iter().all(|f| f.spanwise_regular));
    assert_eq!(cells, proof.faces().iter().map(|f| f.cells).sum::<usize>());
    assert!(cells > 0);
    assert!(
        regularity::qualify(&body, cells)
            .unwrap()
            .certificate
            .is_some()
    );
    let short = regularity::qualify(&body, cells - 1).unwrap();
    assert!(short.certificate.is_none());
    assert!(short.cells <= cells - 1);
    assert!(short.uncertain_face.is_some());
    let zero = regularity::qualify(&body, 0).unwrap();
    assert!(zero.certificate.is_none());
    assert_eq!(zero.completed_faces, 0);
    assert_eq!(zero.cells, 0);
    assert!(regularity::qualify(&body, 100001).is_err());
    let smooth = regularity::smooth::qualify(&proof, 2, 100000, 1000000).unwrap();
    let (cells, work) = (smooth.cells, smooth.exact_work);
    let interiors = smooth
        .certificate
        .expect("Every unchanged original box face is C2");
    assert!(std::ptr::eq(interiors.regular(), &proof));
    assert_eq!(interiors.order(), 2);
    assert_eq!(interiors.faces().len(), 6);
    for (face, source) in interiors
        .faces()
        .iter()
        .zip(body.geometry().shell().faces())
    {
        assert!(std::ptr::eq(face.source(), source[0].edges()[0].surface()));
    }
    assert!(cells > 0 && work > 0);
    assert!(
        regularity::smooth::qualify(&proof, 2, cells, work)
            .unwrap()
            .certificate
            .is_some()
    );
    for (cells, work) in [(cells - 1, work), (cells, work - 1), (0, work), (cells, 0)] {
        let short = regularity::smooth::qualify(&proof, 2, cells, work).unwrap();
        assert!(short.certificate.is_none());
        assert!(short.cells <= cells && short.exact_work <= work);
    }
    assert_eq!(body.definition().unwrap(), original);
}

#[test]
fn embedded_original_body_owns_a_geometric_face_clock_change() {
    for tilt in [0., 0.25] {
        check_geometric_face_clock_change(tilt);
    }
}

fn check_geometric_face_clock_change(tilt: f64) {
    let mut model = crate::cuboid([0.; 3], [2.; 3]).unwrap();
    let source = &mut model.faces[0].surface;
    let first = source.control_points[0][0].clone();
    let u = source.control_points[1][0].clone();
    let v = source.control_points[0][1].clone();
    // Greville controls reproduce the original affine map on every side.
    // One interior control changes only the speed along the first axis.
    source.control_points = [0., 0.25, 0.75, 1.]
        .into_iter()
        .enumerate()
        .map(|(i, t)| {
            [0., 0.5, 1.]
                .into_iter()
                .enumerate()
                .map(|(j, r)| {
                    let t = t + if i == 1 && j == 1 { 1. / 16. } else { 0. };
                    (0..3)
                        .map(|k| first[k] + t * (u[k] - first[k]) + r * (v[k] - first[k]))
                        .collect()
                })
                .collect()
        })
        .collect();
    source.degree_u = 2;
    source.degree_v = 2;
    source.knots_u = vec![0., 0., 0., 0.5, 1., 1., 1.];
    source.knots_v = vec![0., 0., 0., 1., 1., 1.];
    source.weights = vec![vec![1.; 3]; 4];
    let shear = |p: &mut [f64]| p[2] += tilt * p[0] + tilt * p[1];
    for vertex in &mut model.vertices {
        shear(&mut vertex.point);
    }
    for edge in &mut model.edges {
        for point in &mut edge.curve.control_points {
            shear(point);
        }
    }
    for face in &mut model.faces {
        for point in face.surface.control_points.iter_mut().flatten() {
            shear(point);
        }
    }
    assert_embedded_face_g2(&model, 8.);
}

#[test]
fn nonplanar_original_cap_owns_g2_with_a_varying_acceleration_jump() {
    use nurbs_core::{
        continuity::{
            ProjectiveSeam, inspect_projective_seam_collection_with_rational_scale_proposals,
        },
        editing::exact_surface_knot_insertion::tile_tensor_surface,
    };
    let mut model = crate::cuboid([0.; 3], [3., 2., 2.]).unwrap();
    let shear = |p: &mut [f64]| p[2] += p[0] * p[1] / 8.;
    for vertex in &mut model.vertices {
        shear(&mut vertex.point);
    }
    for edge in &mut model.edges {
        for point in &mut edge.curve.control_points {
            shear(point);
        }
    }
    for face in &mut model.faces {
        for point in face.surface.control_points.iter_mut().flatten() {
            shear(point);
        }
    }
    // On the bottom cap x = 3v and z = xy/8. Retiming y inside the
    // cap preserves every world edge. Degree elevation and multiplication
    // by 3v give exact dyadic control coordinates.
    let source = &mut model.faces[0].surface;
    source.degree_u = 2;
    source.degree_v = 3;
    source.knots_u = vec![0., 0., 0., 0.5, 1., 1., 1.];
    source.knots_v = vec![0., 0., 0., 0., 1., 1., 1., 1.];
    source.weights = vec![vec![1.; 4]; 4];
    source.control_points = [0., 0.5, 1.5, 2.]
        .into_iter()
        .enumerate()
        .map(|(i, y)| {
            let shift = if i == 1 { 3. / 32. } else { 0. };
            (0..4)
                .map(|j| {
                    let y_elevated = y + if matches!(j, 1 | 2) {
                        2. * shift / 3.
                    } else {
                        0.
                    };
                    let y_product = y + if j == 2 { shift } else { 0. };
                    vec![j as f64, y_elevated, j as f64 * y_product / 8.]
                })
                .collect()
        })
        .collect();

    let tiled = tile_tensor_surface(source, &[0.5], &[], 1_000_000).unwrap();
    let patches = tiled.patches.expect("Exact dyadic strips");
    let strip = inspect_projective_seam_collection_with_rational_scale_proposals(
        &patches,
        &[ProjectiveSeam {
            patches: [0, 1],
            boundaries: ["uMax".into(), "uMin".into()],
            order: 2,
            normal_scale: 1.,
        }],
        1_000_000,
    )
    .unwrap();
    assert!(strip.seams[0].affine_parameter_identity);
    assert!(!strip.certified);

    let mut changed = source.clone();
    changed.control_points[3][1][2] += 1. / 16.;
    let inspect = crate::sweep_smoothness::complete::interiors::geometric_joins;
    assert!(
        nurbs_core::analysis::surface_continuity::qualify(&changed, 1, 100, 1_000_000)
            .unwrap()
            .certificate
            .is_some()
    );
    assert_eq!(inspect(source, 2, 100, 100_000_000).unwrap().0, Some(1));
    assert!(inspect(&changed, 2, 100, 1_000_000).unwrap().0.is_none());
    assert_embedded_face_g2(&model, 12.);
}

fn assert_embedded_face_g2(model: &crate::Model, expected_volume: f64) {
    let assembly = prepare_projective(model, limits()).unwrap();
    let shell = assembly.shell.expect(assembly.reason);
    let geometry =
        crate::source_shell_geometry::qualify_projective(shell, geometry_limits()).unwrap();
    let geometry = geometry.geometry.expect(geometry.reason);
    let volume = source_volume::qualify(
        geometry,
        source_volume::Limits {
            axis: 2,
            origin: 0.,
            absolute_error: 0.01,
            tolerance_uv: 1e-8,
            cells: 10000,
            spans: 100000,
            domain_cells: 100000,
        },
    )
    .unwrap();
    let body = volume.body.expect(volume.reason);
    assert!((body.volume()[0]..=body.volume()[1]).contains(&expected_volume));
    let before = body.definition().unwrap();
    let rank = regularity::qualify(&body, 100000).unwrap();
    let rank = rank.certificate.expect(rank.reason);
    let smooth = regularity::smooth::qualify(&rank, 2, 100000, 1000000).unwrap();
    let (cells, work) = (smooth.cells, smooth.exact_work);
    let faces = smooth
        .certificate
        .expect("The original face has an exact geometric join");
    let surface = body.geometry().shell().faces()[0][0].edges()[0].surface();
    assert!(std::ptr::eq(faces.faces()[0].source(), surface));
    assert_eq!(faces.faces()[0].geometric_joins(), 1);
    assert!(
        nurbs_core::analysis::surface_continuity::qualify(surface, 2, 100000, 1000000)
            .unwrap()
            .certificate
            .is_none()
    );
    let short = regularity::smooth::qualify(&rank, 2, cells, work - 1).unwrap();
    assert!(short.certificate.is_none());
    assert!(short.exact_work < work);
    assert_eq!(body.definition().unwrap(), before);
}
