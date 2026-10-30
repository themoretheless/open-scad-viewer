#[test]
fn translated_box_v9_preserves_authored_topology_identity() {
    let source = brep_core::cuboid([2., 3., 4.], [7., 11., 13.]).unwrap();
    let (text, _, _) = cad_step::export_step_v9(&source).unwrap();
    let (imported, _, report) = cad_step::import_step_v9(&text).unwrap();
    assert!(report.identity.preserved, "{:?}", report.identity);
    assert_eq!(report.identity.created_count, 0);
    assert_eq!(report.identity.lost_count, 0);
    let expected = &source.1.faces;
    let actual = &imported.1.faces;
    assert_eq!(expected.len(), actual.len());
    assert!(expected.iter().all(|id| actual.contains(id)));
}
