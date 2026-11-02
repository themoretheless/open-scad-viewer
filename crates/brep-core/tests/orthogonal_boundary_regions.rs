use brep_core::Model;
use nurbs_core::curve::Curve;
fn rectangle(x: f64, y: f64) -> Vec<Curve> {
    let p = [[x, y], [x + 0.5, y], [x + 0.5, y + 1.], [x, y + 1.]];
    (0..4)
        .map(|i| Curve::from_polyline(vec![p[i].to_vec(), p[(i + 1) % 4].to_vec()]).unwrap())
        .collect()
}
fn verify(m: &Model) {
    assert!(m.validate().unwrap().boundary_edge_count == 0);
    assert!(
        brep_core::boundary_agreement::verify(m, 100000)
            .unwrap()
            .complete
    );
    assert!(brep_core::solid_audit::audit_solid(m).unwrap().ok);
}
#[test]
fn four_blind_pockets_retain_independent_volume_area_and_centroid_on_every_axis() {
    let source = brep_core::cuboid([0.; 3], [12., 8., 3.]).unwrap();
    let profile = (0..4)
        .map(|i| rectangle(1. + 2. * i as f64, 1. + 1.5 * i as f64))
        .collect::<Vec<_>>();
    let tool = brep_core::prism::extrude(&profile, 1., 4.).unwrap();
    let centroid = [
        (288. * 6. - 4. * 4.25) / 284.,
        (288. * 4. - 4. * 3.75) / 284.,
        (288. * 1.5 - 4. * 2.) / 284.,
    ];
    for order in [[0, 1, 2], [1, 2, 0], [2, 0, 1]] {
        let mut matrix = [[0.; 4]; 4];
        matrix[3][3] = 1.;
        for i in 0..3 {
            matrix[i][order[i]] = 1.;
        }
        let a = brep_core::transform::affine(&source, matrix).unwrap();
        let b = brep_core::transform::affine(&tool, matrix).unwrap();
        let result = brep_core::operations::boolean(&a, &b, "difference").unwrap();
        verify(&result);
        let mass = brep_core::analysis::mass_properties(&result, 1e-6, 2000000).unwrap();
        assert!((mass.signed_volume_mm3 - 284.).abs() < 1e-6);
        assert!((mass.surface_area_mm2 - 336.).abs() < 1e-6);
        for i in 0..3 {
            assert!((mass.centroid[i] - centroid[order[i]]).abs() < 1e-6);
        }
        assert!(result.faces.len() < 100);
    }
}
#[test]
fn internal_void_keeps_opposite_shell_orientation_and_six_faces_per_box_boundary() {
    let stock = brep_core::cuboid([0.; 3], [12., 8., 3.]).unwrap();
    let void = brep_core::cuboid([1., 1., 0.5], [11., 7., 2.5]).unwrap();
    let result = brep_core::operations::boolean(&stock, &void, "difference").unwrap();
    verify(&result);
    assert_eq!(result.faces.len(), 12);
    assert_eq!(result.bodies[0].inner_shells.len(), 1);
    let mass = brep_core::analysis::mass_properties(&result, 1e-6, 2000000).unwrap();
    assert!((mass.signed_volume_mm3 - 168.).abs() < 1e-6);
    assert!((mass.surface_area_mm2 - 496.).abs() < 1e-6);
    let filled = brep_core::operations::boolean(&result, &void, "union").unwrap();
    verify(&filled);
    assert_eq!(filled.faces.len(), 6);
    assert!(filled.bodies[0].inner_shells.is_empty());
}
