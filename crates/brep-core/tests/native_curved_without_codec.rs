//! Typed computations must remain callable with --no-default-features.
use brep_core::{minkowski3d::curved as c, region_replacement as r};
#[path = "support/seam_graph.rs"]
mod fixture;
#[test]
fn source_atlas_replay_and_mutation_refusal_do_not_require_transport() {
    let source = brep_core::cuboid([0.; 3], [1., 2., 3.]).unwrap();
    let proof = c::certify_source_atlas(&source, 0).unwrap();
    c::verify_source_atlas(&source, &proof).unwrap();
    println!("native-atlas-source={}", proof.source);
    let mut changed = source.clone();
    changed.vertices[0].point[0] = 0.25;
    assert!(c::verify_source_atlas(&changed, &proof).is_err());
    assert_eq!(source.vertices[0].point[0], 0.);
}
#[test]
fn complete_corner_material_and_saved_boundary_proof_remain_native() {
    let source = r::six_edge_corner_stock().unwrap();
    let corner = r::propose_six_edge_corner(&source, [0., 0.25, 0.5]).unwrap();
    let proof = r::inspect_six_edge_graph(&source, &corner).unwrap();
    assert!(proof.material_inside_source);
    assert!(proof.volume_mm3 > 0.);
    assert!(proof.exact_jet_equalities > 0);
}
#[test]
fn seam_boolean_replays_sources_material_and_result_without_transport() {
    use brep_core::operations::{
        certify_seam_normalized_boolean as build, verify_seam_normalized_boolean as verify,
    };
    use nurbs_core::curve::Curve;
    let original = fixture::source();
    let path = brep_core::face_cut::Segment {
        pcurve: Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap(),
        curve: Curve {
            degree: 4,
            knots: [vec![0.; 5], vec![1.; 5]].concat(),
            control_points: (0..5)
                .map(|i| {
                    vec![
                        i as f64 / 4.,
                        i as f64 / 4.,
                        1. + if i == 2 { 0.25 } else { 0. },
                    ]
                })
                .collect(),
            weights: vec![1.; 5],
            periodic: false,
        },
    };
    let divided =
        brep_core::face_cut::partition(&original, 1, [4, 6], &[path], 1e-8, 100000).unwrap();
    let a = divided.model;
    let b = brep_core::cuboid([0.1875, 0.21875, 0.75], [0.3125, 0.34375, 2.]).unwrap();
    for operation in ["difference", "intersection"] {
        let checked = build(&a, &b, operation, 0).unwrap();
        assert_eq!(checked.certificate().merges[0].len(), 1);
        assert_eq!(checked.certificate().merges[0][0].removed_edges.len(), 1);
        assert!(checked.certificate().merges[1].is_empty());
        verify(&a, &b, checked.model(), checked.certificate()).unwrap();
        checked.model().validate_continuous(32768).unwrap();
        assert_eq!(
            &brep_core::boolean(&a, &b, operation).unwrap(),
            checked.model()
        );
        let mut changed = a.clone();
        changed.tolerance_mm *= 2.;
        assert!(verify(&changed, &b, checked.model(), checked.certificate()).is_err());
    }
    assert!(build(&a, &b, "difference", 4).is_err());
    assert!(build(&a, &b, "unsupported", 0).is_err());
    let plain = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    let refusal = build(&plain, &plain, "union", 0).unwrap_err();
    assert_eq!(refusal.code, "BREP_SEAM_BOOLEAN_REFUSED");
    assert!(
        refusal
            .message
            .contains("No exact artificial seam was removed")
    );
}
