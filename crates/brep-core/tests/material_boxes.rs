use brep_core::closed_solid as solid;
use nurbs_core::surface::Surface;
#[path = "support/closed_solid_policy.rs"]
mod policy;
fn plane(z: f64) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., z], vec![0., 3., z]],
            vec![vec![3., 0., z], vec![3., 3., z]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn directions() -> [[f64; 3]; 3] {
    [[1., 0.317, 0.173], [0.239, 1., 0.419], [0.137, 0.283, 1.]]
}
#[test]
fn entire_original_surface_box_has_one_material_and_straddling_box_refuses() {
    let body = solid::qualify(
        brep_core::cuboid([0.; 3], [2.; 3]).unwrap(),
        policy::policy(),
    )
    .unwrap()
    .into_solid()
    .unwrap();
    let surface = plane(1.);
    let ds = directions();
    let inside = solid::qualify_surface_material_box(
        &body,
        &surface,
        [[0.3, 0.4]; 2],
        &ds,
        Default::default(),
    )
    .unwrap();
    assert!(inside.inside());
    inside.reverify().unwrap();
    let outside = solid::qualify_surface_material_box(
        &body,
        &surface,
        [[0.9, 0.95]; 2],
        &ds,
        Default::default(),
    )
    .unwrap();
    assert!(!outside.inside());
    assert!(
        solid::qualify_surface_material_box(
            &body,
            &surface,
            [[0.60, 0.75], [0.3, 0.4]],
            &ds,
            Default::default()
        )
        .is_err()
    );
    let mut rational = surface;
    rational.weights = vec![vec![1., 2.], vec![3., 5.]];
    let proof = solid::qualify_surface_material_box(
        &body,
        &rational,
        [[0.1, 0.11]; 2],
        &ds,
        Default::default(),
    )
    .unwrap();
    assert!(proof.inside());
    proof.reverify().unwrap();
    assert!(
        solid::qualify_surface_material_box(
            &body,
            &rational,
            [[0.1, 0.11]; 2],
            &ds,
            solid::SurfaceMaterialBoxLimits {
                source: solid::SurfaceMaterialLimits {
                    distance_cells: 1,
                    ..Default::default()
                },
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn retained_surface_distance_proves_void_through_a_real_finite_hole() {
    let profile = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    let hole = vec![
        [0.375, 0.375],
        [0.375, 0.625],
        [0.625, 0.625],
        [0.625, 0.375],
    ];
    let model = brep_core::extrude_polygon_with_holes(&profile, &[hole], -1., 0.).unwrap();
    let face = model
        .faces
        .iter()
        .find(|f| {
            !f.holes.is_empty()
                && f.surface
                    .control_points
                    .iter()
                    .flatten()
                    .all(|p| p[2] == 0.)
        })
        .unwrap()
        .surface
        .clone();
    let body = solid::qualify(model, policy::policy())
        .unwrap()
        .into_solid()
        .unwrap();
    let proof = solid::qualify_surface_material_box(
        &body,
        &face,
        [[0.48, 0.52]; 2],
        &directions(),
        Default::default(),
    )
    .unwrap();
    assert!(!proof.inside());
    assert!(proof.separation_work()[1] > 0);
    proof.reverify().unwrap();
    assert!(
        solid::qualify_surface_material_box(
            &body,
            &face,
            [[0.61, 0.64], [0.49, 0.51]],
            &directions(),
            Default::default()
        )
        .is_err()
    );
}
#[test]
fn all_boolean_operations_classify_entire_finite_face_cells_and_reject_boundary_straddles() {
    use solid::{BooleanMaterialOp as Op, BooleanSourceSide as Side};
    let a = solid::qualify(
        brep_core::cuboid([0.; 3], [2.; 3]).unwrap(),
        policy::policy(),
    )
    .unwrap()
    .into_solid()
    .unwrap();
    let b = solid::qualify(
        brep_core::cuboid([1., -1., -1.], [3.; 3]).unwrap(),
        policy::policy(),
    )
    .unwrap()
    .into_solid()
    .unwrap();
    let face = |body: &solid::QualifiedClosedSolid, x: f64| {
        body.model()
            .faces
            .iter()
            .position(|f| f.surface.control_points.iter().flatten().all(|p| p[0] == x))
            .unwrap()
    };
    for (operation, retain_inside, reverse_inside, retain_outside) in [
        (Op::Union, false, false, true),
        (Op::Intersection, true, false, false),
        (Op::Difference, false, false, true),
        (Op::Xor, true, true, true),
    ] {
        let yes = solid::qualify_boolean_face_box(
            &a,
            &b,
            operation,
            Side::First,
            face(&a, 2.),
            [[0.4, 0.6]; 2],
            &directions(),
            Default::default(),
        )
        .unwrap();
        assert_eq!(yes.retain(), retain_inside);
        assert_eq!(yes.reverse_material_normal(), reverse_inside);
        let no = solid::qualify_boolean_face_box(
            &a,
            &b,
            operation,
            Side::First,
            face(&a, 0.),
            [[0.4, 0.6]; 2],
            &directions(),
            Default::default(),
        )
        .unwrap();
        assert_eq!(no.retain(), retain_outside);
    }
    let subtract = solid::qualify_boolean_face_box(
        &a,
        &b,
        Op::Difference,
        Side::Second,
        face(&b, 1.),
        [[0.48, 0.52]; 2],
        &directions(),
        Default::default(),
    )
    .unwrap();
    assert!(subtract.retain() && subtract.reverse_material_normal());
    subtract.reverify().unwrap();
    let top = a
        .model()
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == 2.)
        })
        .unwrap();
    assert!(
        solid::qualify_boolean_face_box(
            &a,
            &b,
            Op::Union,
            Side::First,
            top,
            [[0.45, 0.55]; 2],
            &directions(),
            Default::default()
        )
        .is_err()
    );
}
#[path = "../../nurbs-core/tests/common/periodic_extrusion.rs"]
mod periodic_fixture;
#[test]
fn original_periodic_upper_alias_keeps_uniform_material_and_source_flags() {
    let body = solid::qualify(
        brep_core::cuboid([-2., -2., -1.], [2., 2., 3.]).unwrap(),
        policy::policy(),
    )
    .unwrap()
    .into_solid()
    .unwrap();
    let source = periodic_fixture::source(1);
    let before = source.clone();
    let proof = solid::qualify_surface_material_box(
        &body,
        &source,
        [[3.98, 4.], [0.49, 0.51]],
        &directions(),
        Default::default(),
    )
    .unwrap();
    assert!(proof.inside());
    assert!(proof.surface().periodic_u);
    assert_eq!(source, before);
    proof.reverify().unwrap();
    let mut broken = source;
    broken.control_points[0][0][0] += 0.125;
    assert!(
        solid::qualify_surface_material_box(
            &body,
            &broken,
            [[3.98, 4.], [0.49, 0.51]],
            &directions(),
            Default::default()
        )
        .is_err()
    );
}
