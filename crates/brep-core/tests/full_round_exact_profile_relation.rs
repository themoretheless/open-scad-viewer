use brep_core::feature_ops::full_round_curved::build;
use brep_core::profile_certificate::material_relation::Limits;

#[test]
fn full_round_exact_profile_relation_uses_actual_source_and_result_curves() {
    let source = brep_core::cuboid([0.; 3], [4., 8., 6.]).unwrap();
    let faces = [(0, 0.), (1, 8.), (0, 4.)].map(|(axis, value)| {
        source
            .faces
            .iter()
            .position(|f| {
                f.surface
                    .control_points
                    .iter()
                    .flatten()
                    .all(|p| p[axis] == value)
            })
            .unwrap()
    });
    let rounded = build(&source, faces).unwrap();
    let report = rounded
        .inspect_source_profile_relation(1e-12, Limits::default())
        .unwrap();
    assert_eq!(report.source(), &source);
    assert_eq!(report.result(), rounded.model());
    assert!(report.relation().result_is_subset());
    assert!(!report.relation().removed().is_empty());
    let admitted = report.require_no_addition().unwrap();
    assert_eq!(admitted.source(), &source);
    assert_eq!(admitted.result(), rounded.model());
    admitted.certificate().relation().reverify().unwrap();
}
