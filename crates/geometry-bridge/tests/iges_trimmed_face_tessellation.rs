#[test]
fn tessellates_iges_faces_with_one_closed_boundary_edge() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/rush-iges");
    for name in [
        "boundary-pair",
        "model-only",
        "graph-boundary",
        "grid-boundary",
        "weighted-grid",
    ] {
        let source = std::fs::read_to_string(root.join(format!("{name}.igs"))).unwrap();
        let (model, _, _) = cad_step::import_iges_v2(&source).unwrap();
        assert_eq!(model.validate().unwrap().boundary_edge_count, 1);
        let mesh = geometry_bridge::brep::nurbs(&model, 8)
            .unwrap_or_else(|error| panic!("{name}: {}", error.message));
        assert!(!mesh.face_ids.is_empty(), "{name}");
    }
}
