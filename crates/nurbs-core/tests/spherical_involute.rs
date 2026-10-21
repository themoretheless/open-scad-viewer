use nurbs_core::involute::{
    SphericalInvoluteSpec as Spec, approximate_spherical_involute, straight_bevel_flank,
};
fn harmonic(angle: f64, t: f64) -> [f64; 3] {
    let (s, c) = angle.sin_cos();
    let a = (1. + s) / 2.;
    let b = (s - 1.) / 2.;
    [
        a * ((1. - s) * t).cos() + b * ((1. + s) * t).cos(),
        a * ((1. - s) * t).sin() + b * ((1. + s) * t).sin(),
        c * (s * t).cos(),
    ]
}
fn norm(p: [f64; 3]) -> f64 {
    p[0].hypot(p[1]).hypot(p[2])
}
#[test]
fn unwinding_source_has_sphere_identity_tangent_and_base_cone_cusp() {
    for angle in [0.05, 0.4, 0.9, 1.5] {
        let spec = Spec {
            base_cone_angle: angle,
            roll: [0., 3.],
        };
        for i in 0..=100 {
            let t = 3. * i as f64 / 100.;
            let (p, d) = spec.evaluate(t).unwrap();
            let oracle = harmonic(angle, t);
            assert!(norm(std::array::from_fn(|k| p[k] - oracle[k])) < 2e-15);
            assert!((norm(p) - 1.).abs() < 1e-15);
            assert!((0..3).map(|k| p[k] * d[k]).sum::<f64>().abs() < 1e-15);
            let speed = angle.cos() * (angle.sin() * t).sin().abs();
            assert!((norm(d) - speed).abs() < 1e-15);
        }
        let (p, d) = spec.evaluate(0.).unwrap();
        assert_eq!(d, [0.; 3]);
        assert_eq!(p, [angle.sin(), 0., angle.cos()]);
    }
}
#[test]
fn cubic_curve_and_radial_surface_match_independent_harmonic_source() {
    for angle in [0.1, 0.5, 1.2] {
        let spec = Spec {
            base_cone_angle: angle,
            roll: [0.2, 2.],
        };
        let flank = straight_bevel_flank(spec, [7., 20.], 1e-3).unwrap();
        assert!(flank.real_arithmetic_error_estimate <= 1e-3);
        assert_eq!(flank.surface.degree_v, 1);
        for i in 0..=200 {
            let u = i as f64 / 200.;
            let q = harmonic(angle, 0.2 + 1.8 * u);
            for v in [0., 0.25, 0.5, 1.] {
                let r = 7. + 13. * v;
                let p = flank.surface.evaluate(u, v).unwrap().point;
                let error = norm(std::array::from_fn(|k| p[k] - r * q[k]));
                assert!(
                    error <= flank.real_arithmetic_error_estimate + 1e-11,
                    "{error}"
                );
            }
        }
        let coarse = approximate_spherical_involute(spec, 20., 1e-2).unwrap();
        assert!(flank.outer_profile.spans > coarse.spans);
    }
}
#[test]
fn rejects_invalid_source_limits_and_degenerate_radial_rulings() {
    let valid = Spec {
        base_cone_angle: 0.5,
        roll: [0., 1.],
    };
    for spec in [
        Spec {
            base_cone_angle: 0.,
            ..valid
        },
        Spec {
            base_cone_angle: f64::NAN,
            ..valid
        },
        Spec {
            roll: [1., 0.],
            ..valid
        },
        Spec {
            roll: [-1., 1.],
            ..valid
        },
        Spec {
            roll: [0., 17.],
            ..valid
        },
    ] {
        assert!(approximate_spherical_involute(spec, 10., 1e-4).is_err());
    }
    for distances in [[0., 10.], [10., 10.], [20., 10.], [1., f64::INFINITY]] {
        assert!(straight_bevel_flank(valid, distances, 1e-4).is_err());
    }
    assert!(approximate_spherical_involute(valid, 10., 1e-30).is_err());
    assert!(approximate_spherical_involute(valid, 10., 0.).is_err());
    assert!(valid.evaluate(2.).is_err());
}

#[test]
fn inclusive_certificate_replays_original_poles_and_exposes_forgery() {
    use nurbs_core::involute::certify_spherical_involute as certify;
    let spec = Spec {
        base_cone_angle: 0.5,
        roll: [0.2, 2.],
    };
    let fit = approximate_spherical_involute(spec, 20., 1e-5).unwrap();
    let report = certify(&fit.curve, spec, 20.).unwrap();
    assert!(report.error_upper >= report.hermite_remainder_upper);
    assert!(report.control_difference_upper > 0. && report.control_difference_upper < 1e-9);
    assert!(report.error_upper < 1e-5);
    let mut forged = fit.curve.clone();
    forged.control_points[1][0] += 0.01;
    assert!(
        certify(&forged, spec, 20.)
            .unwrap()
            .control_difference_upper
            > 0.009
    );
    forged = fit.curve.clone();
    forged.weights[1] = 2.;
    assert!(certify(&forged, spec, 20.).is_err());
    assert!(
        certify(
            &fit.curve,
            Spec {
                roll: [0.2, 9.],
                ..spec
            },
            20.
        )
        .is_err()
    );
    let altered = certify(
        &fit.curve,
        Spec {
            base_cone_angle: 0.6,
            ..spec
        },
        20.,
    )
    .unwrap();
    assert!(altered.error_upper > 0.1);
}

#[test]
fn whole_saved_flank_certificate_includes_radial_scaling_and_refuses_forgery() {
    use nurbs_core::involute::certify_straight_bevel_flank as certify;
    let spec = Spec {
        base_cone_angle: 0.5,
        roll: [0.2, 2.],
    };
    let flank = straight_bevel_flank(spec, [7., 20.], 1e-3).unwrap();
    let report = certify(&flank.surface, spec, [7., 20.]).unwrap();
    assert!(report.error_upper < 1e-3);
    assert!(report.radial_control_difference_upper > 0.);
    let mut altered = flank.surface.clone();
    altered.control_points[1][0][0] += 0.01;
    assert!(certify(&altered, spec, [7., 20.]).unwrap().error_upper > 0.009);
    assert!(
        certify(&flank.surface, spec, [8., 20.])
            .unwrap()
            .error_upper
            > 0.5
    );
    altered = flank.surface.clone();
    altered.weights[1][0] = 2.;
    assert!(certify(&altered, spec, [7., 20.]).is_err());
    assert!(certify(&flank.surface, spec, [0., 20.]).is_err());
}

#[test]
fn qualified_owner_replays_source_and_keeps_the_requested_spatial_tolerance() {
    use nurbs_core::involute::qualified_straight_bevel_flank as construct;
    let mut spec = Spec {
        base_cone_angle: 0.5,
        roll: [0.2, 2.],
    };
    let owned = construct(spec, [7., 20.], 1e-3).unwrap();
    owned.reverify().unwrap();
    assert!(owned.certificate().error_upper <= 1e-3);
    spec.base_cone_angle = 0.7;
    assert_eq!(owned.source().base_cone_angle, 0.5);
    assert_eq!(spec.base_cone_angle, 0.7);
    assert!(construct(spec, [7., 20.], 1e-20).is_err());
    // Small intervals still permit fine tolerances within 32 surface controls.
    let fine = construct(
        Spec {
            base_cone_angle: 0.5,
            roll: [0.2, 0.4],
        },
        [7., 20.],
        1e-7,
    )
    .unwrap();
    fine.reverify().unwrap();
    assert!(fine.certificate().error_upper <= 1e-7);
}

#[test]
fn wide_fine_flank_uses_original_parameter_patches_with_exact_common_rows() {
    use nurbs_core::involute::{
        qualified_straight_bevel_flank as single, qualified_straight_bevel_patches as construct,
    };
    let spec = Spec {
        base_cone_angle: 0.5,
        roll: [0.2, 2.],
    };
    assert!(single(spec, [7., 20.], 1e-6).is_err());
    let owned = construct(spec, [7., 20.], 1e-6, 16).unwrap();
    owned.reverify().unwrap();
    assert!(owned.patches().len() > 1);
    assert!(owned.certificate().error_upper <= 1e-6);
    for pair in owned.patches().windows(2) {
        assert_eq!(
            pair[0].control_points.last(),
            pair[1].control_points.first()
        );
        assert_eq!(pair[0].knots_u.last(), pair[1].knots_u.first());
    }
    for surface in owned.patches() {
        assert!(surface.control_points.len() <= 32);
        let a = surface.knots_u[3];
        let b = surface.knots_u[surface.control_points.len()];
        for i in 0..=20 {
            let u = if i == 20 {
                b
            } else {
                a + (b - a) * i as f64 / 20.
            };
            let q = harmonic(0.5, 0.2 + 1.8 * u);
            for v in [0., 0.3, 1.] {
                let p = surface.evaluate(u, v).unwrap().point;
                let r = 7. + 13. * v;
                assert!(
                    norm(std::array::from_fn(|k| p[k] - r * q[k]))
                        <= owned.certificate().error_upper + 1e-11
                );
            }
        }
    }
}
#[test]
fn patch_certificate_rejects_gaps_reordering_changed_seams_and_radial_rows() {
    use nurbs_core::involute::{
        certify_straight_bevel_patches as certify, qualified_straight_bevel_patches as construct,
    };
    let spec = Spec {
        base_cone_angle: 0.5,
        roll: [0.2, 2.],
    };
    let owned = construct(spec, [7., 20.], 1e-6, 16).unwrap();
    let mut patches = owned.patches().to_vec();
    patches.swap(0, 1);
    assert!(certify(&patches, spec, [7., 20.]).is_err());
    patches = owned.patches().to_vec();
    patches[1].control_points[0][0][0] += 0.001;
    assert!(certify(&patches, spec, [7., 20.]).is_err());
    patches = owned.patches().to_vec();
    patches[0].control_points[1][0][0] += 0.01;
    assert!(certify(&patches, spec, [7., 20.]).unwrap().error_upper > 0.009);
    assert!(certify(&owned.patches()[1..], spec, [7., 20.]).is_err());
    assert!(
        certify(owned.patches(), spec, [8., 20.])
            .unwrap()
            .error_upper
            > 0.5
    );
    assert!(construct(spec, [7., 20.], 1e-6, 1).is_err());
    assert!(construct(spec, [7., 20.], 1e-6, 0).is_err());
}

#[test]
fn patched_flanks_cover_cusp_small_and_large_cones_at_the_requested_accuracy() {
    use nurbs_core::involute::qualified_straight_bevel_patches as construct;
    for (angle, radii, end, tolerance) in [
        (0.05, [0.01, 0.1], 3., 1e-7),
        (0.5, [7., 100.], 2., 1e-6),
        (1.5, [10., 20.], 3., 1e-7),
    ] {
        let spec = Spec {
            base_cone_angle: angle,
            roll: [0., end],
        };
        let owned = construct(spec, radii, tolerance, 16).unwrap();
        owned.reverify().unwrap();
        assert!(owned.certificate().error_upper <= tolerance);
        for surface in owned.patches() {
            let a = surface.knots_u[3];
            let b = surface.knots_u[surface.control_points.len()];
            for u in [a, (a + b) / 2., b] {
                let q = harmonic(angle, end * u);
                for v in [0., 0.5, 1.] {
                    let p = surface.evaluate(u, v).unwrap().point;
                    let r = radii[0] + (radii[1] - radii[0]) * v;
                    assert!(
                        norm(std::array::from_fn(|k| p[k] - r * q[k]))
                            <= owned.certificate().error_upper + 1e-10
                    );
                }
            }
        }
    }
}
