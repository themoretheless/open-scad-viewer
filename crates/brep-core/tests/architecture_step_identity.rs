#[test]
fn translated_box_v9_preserves_authored_topology_identity() {
    let source = brep_core::cuboid([2.,3.,4.],[7.,11.,13.]).unwrap();
    let (text,_,_) = brep_core::export_step_v9(&source).unwrap();
    let (_,_,report) = brep_core::import_step_v9(&text).unwrap();
    assert!(report.identity.preserved, "{:?}", report.identity);
}
