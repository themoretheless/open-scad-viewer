use brep_core::gear::{self, GearSpec, ProfileEmbeddingLimits as Limits};
#[test]
fn modular_default_gear_profile_exceeds_the_old_global_span_budget() {
    let (certificate, verified) =
        gear::certify_profile_embedding(&GearSpec::default(), Limits::default()).unwrap();
    assert_eq!(verified.teeth(), 20);
    assert_eq!(verified.winding(), &[1]);
    assert_eq!(certificate.local.len(), 20);
    assert_eq!(certificate.far_rays.len(), 170);
    let spans = verified
        .profile()
        .iter()
        .flatten()
        .map(|c| {
            (c.degree..c.control_points.len())
                .filter(|&i| c.knots[i] < c.knots[i + 1])
                .count()
        })
        .sum::<usize>();
    assert!(spans > 128);
    let mut incomplete = certificate.clone();
    incomplete.far_rays.pop();
    assert!(
        gear::verify_profile_embedding(verified.profile(), &incomplete, Limits::default()).is_err()
    );
    let mut source = verified.profile().to_vec();
    source[0]
        .iter_mut()
        .find(|c| c.degree == 5)
        .unwrap()
        .control_points[2][0] += 1e-8;
    assert!(gear::verify_profile_embedding(&source, &certificate, Limits::default()).is_err());
    let mut false_ray = certificate;
    false_ray.far_rays[0] = [1., 1.];
    assert!(
        gear::verify_profile_embedding(verified.profile(), &false_ray, Limits::default()).is_err()
    );
    false_ray.far_rays[0] = [0.; 2];
    assert!(
        gear::verify_profile_embedding(verified.profile(), &false_ray, Limits::default()).is_err()
    );
}
#[test]
fn internal_and_bored_profiles_have_verified_winding_and_radial_nesting() {
    for spec in [
        GearSpec {
            teeth: 8,
            bore: 0.5,
            ..GearSpec::default()
        },
        GearSpec {
            teeth: 40,
            internal: true,
            ..GearSpec::default()
        },
    ] {
        let (_, verified) = gear::certify_profile_embedding(&spec, Limits::default()).unwrap();
        assert_eq!(verified.winding(), &[1, -1]);
        let r = verified.radial_ranges();
        assert!(r[1][1] < r[0][0]);
    }
}
#[test]
fn complete_pair_census_and_shared_work_limits_are_mandatory() {
    for limits in [
        Limits {
            local_certificates: 19,
            ..Limits::default()
        },
        Limits {
            far_pairs: 169,
            ..Limits::default()
        },
        Limits {
            exact_signs: 1,
            ..Limits::default()
        },
    ] {
        assert!(gear::certify_profile_embedding(&GearSpec::default(), limits).is_err());
    }
}

#[test]
fn maximum_tooth_count_has_complete_pair_coverage() {
    let (_, verified) = gear::certify_profile_embedding(
        &GearSpec {
            teeth: 256,
            ..GearSpec::default()
        },
        Limits::default(),
    )
    .unwrap();
    assert_eq!(verified.teeth(), 256);
    assert_eq!(verified.winding(), &[1]);
}

#[test]
fn minimum_tooth_count_uses_local_covers_without_far_pairs() {
    let (certificate, verified) = gear::certify_profile_embedding(
        &GearSpec {
            teeth: 3,
            ..GearSpec::default()
        },
        Limits::default(),
    )
    .unwrap();
    assert!(certificate.far_rays.is_empty());
    assert_eq!(verified.winding(), &[1]);
}
