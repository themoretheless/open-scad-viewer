use nurbs_core::thread::{self, Hand, Kind, Spec};
#[test]
fn full_nominal_thread_bounds_cover_hands_starts_and_internal_clearance() {
    for (hand, kind, starts) in [
        (Hand::Right, Kind::External, 1),
        (Hand::Left, Kind::External, 2),
        (Hand::Right, Kind::Internal, 2),
    ] {
        let spec = Spec {
            hand,
            kind,
            starts,
            turns: 2,
            ..Spec::default()
        };
        let (source, certificate) = thread::patches_certified(spec, 2560).unwrap();
        assert!(source.rounding_certified && certificate.rounding_included);
        assert!(certificate.error_upper <= spec.error_budget);
        assert_eq!(certificate.patches, 20 * starts * 2);
        let crest = spec.diameter / 2.
            + if kind == Kind::External {
                -spec.clearance / 2.
            } else {
                spec.clearance / 2.
            };
        let root = crest - 3_f64.sqrt() * spec.pitch * 5. / 16.;
        let phase = [0., 1. / 16., 3. / 8., 5. / 8., 15. / 16., 1.];
        let radii = [crest, crest, root, root, crest, crest];
        for p in &source.patches {
            for u in [0., 0.3, 1.] {
                for v in [0., 0.4, 1.] {
                    let angle = (p.quarter as f64 + v)
                        * std::f64::consts::FRAC_PI_2
                        * if hand == Hand::Right { 1. } else { -1. };
                    let radius =
                        (1. - u) * radii[p.profile_segment] + u * radii[p.profile_segment + 1];
                    let z = p.start as f64 * spec.pitch
                        + (p.turn as f64 + (p.quarter as f64 + v) / 4.)
                            * spec.pitch
                            * starts as f64
                        + ((1. - u) * phase[p.profile_segment] + u * phase[p.profile_segment + 1])
                            * spec.pitch;
                    let q = p.surface.evaluate(u, v).unwrap().point;
                    let error = (q[0] - radius * angle.cos())
                        .hypot(q[1] - radius * angle.sin())
                        .hypot(q[2] - z);
                    assert!(error <= certificate.error_upper + 1e-11);
                }
            }
        }
    }
}
#[test]
fn omitted_relabelled_and_modified_patches_do_not_inherit_certification() {
    let spec = Spec::default();
    let source = thread::patches(spec).unwrap();
    let mut omitted = source.clone();
    omitted.patches.pop();
    assert!(thread::certify_patches(spec, &omitted, 2560).is_err());
    let mut role = source.clone();
    role.patches[0].quarter = 1;
    assert!(thread::certify_patches(spec, &role, 2560).is_err());
    let mut changed = source.clone();
    changed.patches[0].surface.control_points[0][1][0] += 0.01;
    let report = thread::certify_patches(spec, &changed, 2560).unwrap();
    assert!(report.error_upper > 0.009);
    assert!(thread::certify_patches(spec, &source, 19).is_err());
    assert!(
        thread::patches_certified(
            Spec {
                error_budget: 1e-20,
                ..spec
            },
            2560
        )
        .is_err()
    );
}
