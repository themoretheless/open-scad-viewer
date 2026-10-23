use nurbs_core::worm::{
    Hand, Spec, certify_trim_candidates as verify, trimmed_patches_certified as construct,
};
#[test]
fn complete_original_worm_inventory_has_independently_verified_finite_slab_coverage() {
    for hand in [Hand::Right, Hand::Left] {
        let spec = Spec {
            starts: 2,
            turns: 2,
            hand,
            ..Spec::default()
        };
        let owned = construct(spec, [1., 8.], 1e-7, 1e-5, 2560).unwrap();
        owned.reverify().unwrap();
        let c = owned.coverage();
        assert_eq!(c.source_patches, 80);
        assert_eq!(c.retained_patches + c.excluded_patches, 80);
        assert!(c.exact_affine_partition && c.stored_uv_isotopy_certified);
        assert!(c.axial_error_band_upper <= 1e-5);
        assert!(c.retained_patches > 0 && c.excluded_patches > 0);
        for candidate in &owned.trims().candidates {
            assert!(
                candidate.retained_z_bounds[0] >= 1. - 1e-5
                    && candidate.retained_z_bounds[1] <= 8. + 1e-5
            );
            let polygon = candidate.pcurve(1e-7).unwrap();
            for point in polygon.curve.control_points {
                let z = candidate
                    .patch
                    .surface
                    .evaluate(point[0], point[1])
                    .unwrap()
                    .point[2];
                assert!(z >= 1. - 1e-5 && z <= 8. + 1e-5);
            }
        }
    }
}
#[test]
fn slab_coverage_rejects_missing_duplicated_or_changed_actual_original_patches() {
    let owned = construct(Spec::default(), [0.5, 3.], 1e-7, 1e-5, 2560).unwrap();
    let mut changed = owned.trims().clone();
    changed.candidates.pop();
    assert!(verify(owned.source(), &changed, 1e-7, 1e-5, 2560).is_err());
    changed = owned.trims().clone();
    changed.candidates[0].patch.surface.control_points[0][1][2] += 0.001;
    assert!(verify(owned.source(), &changed, 1e-7, 1e-5, 2560).is_err());
    changed = owned.trims().clone();
    changed.candidates[0].exact_uv_vertices.pop();
    assert!(verify(owned.source(), &changed, 1e-7, 1e-5, 2560).is_err());
    changed = owned.trims().clone();
    changed.candidates[0].affine_z_coefficients[2] += 0.001;
    assert!(verify(owned.source(), &changed, 1e-7, 1e-5, 2560).is_err());
    assert!(construct(Spec::default(), [3., 0.5], 1e-7, 1e-5, 2560).is_err());
    assert!(construct(Spec::default(), [0.5, 3.], 1e-7, 1e-20, 2560).is_err());
}
