use brep_core::gear::{BevelBuildLimits, bevel_with_verified_profile};
use nurbs_core::involute::BevelGearSpec;
#[test]
fn default_native_bevel_owns_embedded_original_edges_and_closed_body_accuracy() {
    let owned =
        bevel_with_verified_profile(BevelGearSpec::default(), 1e-6, BevelBuildLimits::default())
            .unwrap();
    assert_eq!(owned.model().validate().unwrap().boundary_edge_count, 0);
    assert_eq!(owned.model().bodies.len(), 1);
    assert!(owned.world_error_upper() <= 1e-6);
    assert!(!owned.solid_geometry_certified());
    assert_eq!(
        owned.closure().closure.status,
        brep_core::watertight::Closure::WithinTolerance
    );
    let original = owned.profile().source().curves();
    let n = original.len();
    for (edge, source) in owned.model().edges[n..2 * n].iter().zip(original) {
        assert_eq!(&edge.curve, source);
    }
}
