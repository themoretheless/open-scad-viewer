use brep_core::gear::{GearSpec, ProfileEmbeddingLimits as Limits, gear_with_verified_profile};
#[test]
fn four_native_gear_families_bind_verified_profiles_to_actual_bodies() {
    for (helix, herringbone, internal) in [
        (0., false, false),
        (20., false, false),
        (-20., false, false),
        (20., true, false),
        (0., false, true),
    ] {
        let gear = gear_with_verified_profile(
            &GearSpec {
                teeth: if internal { 40 } else { 8 },
                helix_angle_deg: helix,
                herringbone,
                internal,
                ..GearSpec::default()
            },
            Limits::default(),
        )
        .unwrap();
        assert_eq!(gear.model().validate().unwrap().boundary_edge_count, 0);
        assert_eq!(gear.model().bodies.len(), 1);
        assert!(gear.geometry().profile_error_upper.unwrap() <= 1e-6);
        assert!(gear.geometry().ideal_wall_error_upper.unwrap() <= 1.01e-4);
        assert!(!gear.solid_geometry_certified());
        assert_eq!(
            gear.profile().winding(),
            if internal { &[1, -1][..] } else { &[1][..] }
        );
    }
}
#[test]
fn incomplete_proofs_and_invalid_specs_never_return_a_verified_gear() {
    assert!(
        gear_with_verified_profile(
            &GearSpec::default(),
            Limits {
                local_certificates: 0,
                ..Limits::default()
            }
        )
        .is_err()
    );
    assert!(
        gear_with_verified_profile(
            &GearSpec {
                module: f64::NAN,
                ..GearSpec::default()
            },
            Limits::default()
        )
        .is_err()
    );
}
