use brep_core::gear::{
    ProfileEmbeddingLimits as Limits, certify_bevel_profile as construct,
    verify_bevel_profile_projection as verify,
};
use nurbs_core::involute::BevelGearSpec as Spec;
#[test]
fn original_spherical_profile_is_embedded_through_exact_xy_projection() {
    for teeth in [20, 100] {
        let owned = construct(
            Spec {
                teeth,
                ..Spec::default()
            },
            1e-6,
            16,
            4096,
            8192,
            Limits::default(),
        )
        .unwrap();
        owned.reverify().unwrap();
        assert_eq!(owned.certificate().teeth, teeth);
        assert_eq!(
            owned.certificate().curves_per_tooth,
            if teeth == 20 { 6 } else { 4 }
        );
        for (a, b) in owned
            .grouped_profile()
            .iter()
            .zip(owned.projected_profile())
        {
            assert_eq!(a.knots, b.knots);
            assert_eq!(a.weights, b.weights);
            for (a, b) in a.control_points.iter().zip(&b.control_points) {
                assert_eq!(&a[..2], &b[..]);
            }
        }
        let checked = verify(
            owned.source(),
            owned.projected_profile(),
            owned.certificate(),
            Limits::default(),
        )
        .unwrap();
        assert_eq!(checked.winding(), &[1]);
    }
}
#[test]
fn original_source_projection_and_every_modular_witness_remain_bound() {
    let owned = construct(Spec::default(), 1e-6, 16, 4096, 8192, Limits::default()).unwrap();
    let mut altered = owned.projected_profile().to_vec();
    altered[0].control_points[1][0] += 0.001;
    assert!(
        verify(
            owned.source(),
            &altered,
            owned.certificate(),
            Limits::default()
        )
        .is_err()
    );
    let mut receipt = owned.certificate().clone();
    receipt.far_rays.pop();
    assert!(
        verify(
            owned.source(),
            owned.projected_profile(),
            &receipt,
            Limits::default()
        )
        .is_err()
    );
    let mut limits = Limits::default();
    limits.local_certificates = 0;
    assert!(construct(Spec::default(), 1e-6, 16, 4096, 8192, limits).is_err());
}
