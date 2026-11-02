use brep_core::feature_ops::freeform::{self, IsoTrace};
use nurbs_core::{
    constrained_surface::{Constraint, Options},
    patches,
    surface::{Axis, Surface},
    surface_join::Decision,
};
fn supports() -> [Surface; 2] {
    let a = patches::bezier(
        (0..3)
            .map(|i| {
                (0..2)
                    .map(|j| vec![i as f64 / 2., j as f64, if i == 2 { 0.3 } else { 0. }])
                    .collect()
            })
            .collect(),
        None,
    )
    .unwrap();
    let b = patches::bezier(
        vec![
            vec![vec![-1., 0., 0.], vec![-1., 0., 2.]],
            vec![vec![2., 0., 0.], vec![2., 0., 2.]],
        ],
        None,
    )
    .unwrap();
    [a, b]
}
#[test]
fn nonplanar_face_replacement_preserves_owned_edges_and_actual_trim_g2() {
    let source = brep_core::cuboid([0.; 3], [8.; 3]).unwrap();
    let before = source.clone();
    let face = source
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == 8.)
        })
        .unwrap();
    let e = source.faces[face].surface.evaluate(0.5, 0.5).unwrap();
    let mut p = e.point;
    p[2] -= 0.2;
    let result = freeform::replace_constrained(
        &source,
        face,
        &[Constraint::Point {
            uv: [0.5; 2],
            point: p,
        }],
        Options::default(),
        [source.tolerance_mm, 1e-6, 1e-6],
        1e-8,
        100000,
    )
    .unwrap();
    assert_eq!(source, before);
    assert_eq!(result.model().edges, source.edges);
    assert_eq!(result.model().vertices, source.vertices);
    assert!(result.interior().certified);
    assert!(result.audit().ok);
    assert_eq!(result.seams().len(), 4);
    for seam in result.seams() {
        assert!(seam.positions.proven);
        assert_eq!(seam.geometric.decision, Decision::WithinTolerance);
    }
    let actual = result.model().faces[face]
        .surface
        .evaluate(0.5, 0.5)
        .unwrap()
        .point;
    assert!((actual[2] - p[2]).abs() < 1e-8);
    assert!(
        freeform::replace_constrained(
            &source,
            face,
            &[Constraint::Point {
                uv: [0., 0.5],
                point: p
            }],
            Options::default(),
            [source.tolerance_mm, 1e-6, 1e-6],
            1e-8,
            100000
        )
        .is_err()
    );
    assert_eq!(source, before);
}
#[test]
fn freeform_chamfer_sheet_has_proven_original_contact_and_retained_curvature() {
    let [a, b] = supports();
    let traces = [
        IsoTrace {
            axis: Axis::V,
            at: 0.2,
            along: [0., 1.],
            reverse: false,
        },
        IsoTrace {
            axis: Axis::V,
            at: 0.3,
            along: [1. / 3., 2. / 3.],
            reverse: false,
        },
    ];
    let result = freeform::chamfer_sheet([&a, &b], traces, 1e-7, 100000).unwrap();
    result.model().validate().unwrap();
    assert!(result.model().bodies.is_empty());
    assert!(result.contacts().iter().all(|r| r.proven));
    assert!(result.interior().certified);
    assert!(result.model().faces[0].surface.degree_u >= 2);
}
#[test]
fn freeform_offset_and_rolling_ball_produce_explicit_open_sheet_results() {
    let [a, b] = supports();
    let offset = freeform::offset_sheet(&a, 0.05, 1e-3, 10000).unwrap();
    offset.model.validate().unwrap();
    assert!(offset.model.bodies.is_empty());
    assert!(offset.geometry.minimum_stretch() > 0.);
    assert!(offset.sampled_deviation_mm <= 1e-3);
    let ball =
        nurbs_core::offset::freeform_blend::RollingBall::new([&a, &b], 0.2, [1., -1.], 0, 1e-10)
            .unwrap();
    let fillet =
        freeform::fillet_sheet_proposal(&ball, [0.1, 0.9], [0.1, 0.2, 0.5, 0.3], 5, 1e-7).unwrap();
    fillet.model.validate().unwrap();
    assert!(fillet.model.bodies.is_empty());
    assert_eq!(fillet.sections.len(), 5);
    assert!(fillet.maximum_section_residual_mm < 1e-10);
}
