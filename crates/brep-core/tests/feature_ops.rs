use brep_core::{
    Model,
    aag::Aag,
    aag_fillets::find_fillet_chains,
    feature_ops::{self, PatternMode},
};
use nurbs_core::foundation::guards::Budget;
fn budget() -> Budget {
    Budget::new(10_000_000, 16, 60_000).unwrap()
}
fn cap(model: &Model, axis: usize, value: f64) -> usize {
    model
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| (p[axis] - value).abs() < 1e-9)
        })
        .unwrap()
}
fn volume(m: &Model) -> f64 {
    brep_core::analysis::mass_properties(m, 1e-6, 2_000_000)
        .unwrap()
        .signed_volume_mm3
}
#[test]
fn full_round_consumes_the_middle_face_and_keeps_analytic_cylinder() {
    let source = brep_core::cuboid([0.; 3], [4., 8., 6.]).unwrap();
    let before = source.clone();
    let selected = [
        cap(&source, 0, 0.),
        cap(&source, 1, 8.),
        cap(&source, 0, 4.),
    ];
    let report = feature_ops::full_round(&source, selected).unwrap();
    assert_eq!(source, before);
    assert_eq!(report.radius_mm, 2.);
    assert!(report.audit.ok);
    report.model.validate().unwrap();
    assert!(
        report
            .model
            .faces
            .iter()
            .any(|f| f.surface.degree_u > 1 || f.surface.degree_v > 1)
    );
    let expected = (4. * 6. + std::f64::consts::PI * 2.) * 6.;
    assert!((volume(&report.model) - expected).abs() < 1e-4);
    assert!(feature_ops::full_round(&source, [selected[0], selected[0], selected[2]]).is_err());
    let too_thin = brep_core::cuboid([0.; 3], [4., 1., 6.]).unwrap();
    assert!(
        feature_ops::full_round(
            &too_thin,
            [
                cap(&too_thin, 0, 0.),
                cap(&too_thin, 1, 1.),
                cap(&too_thin, 0, 4.)
            ]
        )
        .is_err()
    );
}
#[test]
fn full_round_handles_translated_frames_and_both_middle_normals() {
    for width_axis in 0..3 {
        let middle_axis = (width_axis + 1) % 3;
        let extrusion_axis = (width_axis + 2) % 3;
        let min = [-7., 11., -3.];
        let mut max = min;
        max[width_axis] += 4.;
        max[middle_axis] += 8.;
        max[extrusion_axis] += 6.;
        let source = brep_core::cuboid(min, max).unwrap();
        for middle in [min[middle_axis], max[middle_axis]] {
            let result = feature_ops::full_round(
                &source,
                [
                    cap(&source, width_axis, min[width_axis]),
                    cap(&source, middle_axis, middle),
                    cap(&source, width_axis, max[width_axis]),
                ],
            )
            .unwrap();
            assert!(result.audit.ok);
            let expected = (24. + 2. * std::f64::consts::PI) * 6.;
            assert!((volume(&result.model) - expected).abs() < 1e-4);
        }
    }
}
#[test]
fn reblend_recognized_chain_changes_radius_with_atomic_failure() {
    let sharp = brep_core::cuboid([0.; 3], [10., 8., 6.]).unwrap();
    let edge = sharp
        .edges
        .iter()
        .position(|e| {
            let a = sharp.vertices[e.vertices[0]].point;
            let b = sharp.vertices[e.vertices[1]].point;
            (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9
        })
        .unwrap();
    let (rounded, _) = brep_core::analytic_fillet(&sharp, edge, 1.).unwrap();
    let before = rounded.clone();
    let mut graph = Aag::build(&rounded, &budget()).unwrap();
    graph.attach_face_attrs(&rounded, &budget()).unwrap();
    let chains = find_fillet_chains(&rounded, &graph, &budget()).unwrap();
    assert_eq!(chains.len(), 1);
    let report = feature_ops::reblend_faces(&rounded, &chains[0].faces, 2., &budget()).unwrap();
    assert_eq!(rounded, before);
    assert!(report.audit.ok);
    assert_eq!(report.new_radius_mm, 2.);
    assert!(!report.reconstructed_edges.is_empty());
    let expected = 480. - (1. - std::f64::consts::FRAC_PI_4) * 4. * 6.;
    assert!((volume(&report.model) - expected).abs() < 1e-4);
    assert!(feature_ops::reblend_faces(&rounded, &chains[0].faces, -1., &budget()).is_err());
    assert_eq!(rounded, before);
}
#[test]
fn selected_boss_face_patterns_as_joined_and_cut_topology() {
    let base = brep_core::cuboid([0.; 3], [10., 8., 2.]).unwrap();
    let boss = brep_core::cuboid([1., 1., 2.], [3., 3., 3.]).unwrap();
    let source = brep_core::operations::boolean(&base, &boss, "union").unwrap();
    let before = source.clone();
    let face = cap(&source, 2, 3.);
    let placement = [
        [1., 0., 0., 4.],
        [0., 1., 0., 0.],
        [0., 0., 1., 0.],
        [0., 0., 0., 1.],
    ];
    let join = feature_ops::pattern_faces(
        &source,
        &[face],
        -1.,
        &[placement],
        PatternMode::Join,
        &budget(),
    )
    .unwrap();
    assert_eq!(source, before);
    assert_eq!(join.instances, 1);
    assert!(join.audit.ok);
    assert!((volume(&join.model) - 168.).abs() < 1e-5);
    let cut = feature_ops::pattern_faces(
        &source,
        &[face],
        -2.,
        &[placement],
        PatternMode::Cut,
        &budget(),
    )
    .unwrap();
    assert!(cut.audit.ok);
    assert!((volume(&cut.model) - 160.).abs() < 1e-5);
    assert!(
        feature_ops::pattern_faces(
            &source,
            &[face, face],
            -1.,
            &[placement],
            PatternMode::Join,
            &budget()
        )
        .is_err()
    );
    assert_eq!(source, before);
}
