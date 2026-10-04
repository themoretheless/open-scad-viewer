use super::*;

#[test]
fn contact_union_work_exhaustion_keeps_only_known_profile_refusal_bound() {
    let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let profiles=vec![profile;32];
    let path=crate::primitives::line([0.;3],[0.,0.,10.]).unwrap();
    let guide=crate::primitives::line([2.,0.,0.],[2.,0.,10.]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let twist=constant_vector_law([0.;3]).unwrap();
    let opts=Options {max_sections:9,max_deviation:1e-30,..options()};
    let level=MultiSweep::new(&profiles,&path,&scale,&twist,opts).unwrap()
        .with_contact_guide(&guide,0,1.).unwrap().preview_at(9).unwrap();
    assert!(!level.report.accepted);
    assert_eq!(level.report.sampled_control_deviation,0.);
    assert!(!level.report.continuous_bound);
    assert!(level.report.continuous_error_upper.is_none());
    assert!(level.report.known_profile_error_upper.unwrap()>opts.max_deviation);
    assert!(level.report.error_certificate_cells<=10000);
}

#[test]
fn contact_anchor_preserves_original_reference_profile_across_all_contours() {
    let profiles = vec![
        crate::primitives::line([0.5, 0., 0.], [1., 0., 0.]).unwrap(),
        crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap(),
    ];
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let guide = crate::primitives::line([2., 0., 0.], [2., 0., 10.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let multi = MultiSweep::new(&profiles, &path, &scale, &twist, options())
        .unwrap().with_contact_guide(&guide, 1, 1.).unwrap();
    for sweep in &multi.sweeps {
        let (source, parameter) = sweep.contact_source.unwrap();
        assert!(std::ptr::eq(source, &profiles[1]));
        assert_eq!(parameter, 1.);
        assert_eq!(sweep.contact_point, Some([2., 0., 0.]));
        let fitted = sweep.contact_fit_jet([0.25, 0.5], 100).unwrap();
        assert!(fitted.reason.is_none());
        let value = fitted.fit.unwrap().value.unwrap();
        assert!(value[0] <= 1. && 1. <= value[1]);
    }
    let reoriented = multi.with_orientation_guide(&guide).unwrap();
    assert!(reoriented.sweeps.iter().all(|s| s.contact_source.is_none() && s.contact_point.is_none()));
}
fn law(a: f64, b: f64) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![a, 0., 0.], vec![b, 0., 0.]],
        weights: vec![1., 1.],
        periodic: false,
    }
}
fn options() -> Options {
    Options {
        normal: [1., 0., 0.],
        orientation: Orientation::RotationMinimizing,
        spacing: Spacing::Parameter,
        initial_sections: 3,
        max_sections: 129,
        max_deviation: 0.001,
    }
}
#[test]
fn authored_whole_frame_regularity_is_separate_from_sampled_acceptance() {
    use crate::sweeps::progressive_miter::scalar_certificate::Status;
    let profile = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let scale = law(1., 1.);
    let twist = law(0., 0.);
    let axis = constant_vector_law([0., 0., 1.]).unwrap();
    let normal = constant_vector_law([1., 0., 0.]).unwrap();
    let plain = Sweep::new(&profile, &path, &scale, &twist, options()).unwrap();
    assert!(plain.authored_frame_regularity(100).unwrap().is_none());
    let authored = Sweep::new_authored(&profile, &path, &scale, &twist, &axis, &normal, options()).unwrap();
    assert_eq!(authored.authored_frame_regularity(100).unwrap().unwrap().status, Status::Certified);
    assert_eq!(authored.authored_frame_regularity(0).unwrap().unwrap().status, Status::Unresolved);
    assert_eq!(authored.authored_frame_jet_cover(100).unwrap().unwrap().status, Status::Certified);
    assert!(plain.authored_frame_jet_cover(100).unwrap().is_none());
}
fn point(patches: &[Surface], u: f64, v: f64) -> V {
    let patch = patches
        .iter()
        .find(|p| {
            let a = p.knots_u[p.degree_u];
            let b = p.knots_u[p.control_points.len()];
            let c = p.knots_v[p.degree_v];
            let d = p.knots_v[p.control_points[0].len()];
            u >= a && u <= b && v >= c && v <= d
        })
        .unwrap();
    patch.evaluate(u, v).unwrap().point
}

#[test]
fn simultaneous_scale_twist_and_rational_profile_matches_independent_equation() {
    let profile =
        crate::paths::bezier(vec![vec![1., 0., 0.], vec![2., 0., 1.]], Some(vec![1., 2.])).unwrap();
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let result = approximate(
        &profile,
        &path,
        &law(1., 2.),
        &law(0., std::f64::consts::FRAC_PI_2),
        options(),
    )
    .unwrap();
    assert!(result.patches.is_some());
    assert!(result.levels.len() > 1);
    assert!(!result.levels[0].accepted);
    let report = result.levels.last().unwrap();
    assert!(report.accepted && report.sections > 32);
    let patches = result.patches.unwrap();
    assert!(patches.len() > 1);
    for i in 0..report.sections {
        let v = i as f64 / (report.sections - 1) as f64;
        for u in [0., 0.13, 0.5, 0.87, 1.] {
            let alpha = 2. * u / (1. + u);
            let (s, c) = (v * std::f64::consts::FRAC_PI_2).sin_cos();
            let expected = [
                (1. + v) * (1. + alpha) * c,
                (1. + v) * (1. + alpha) * s,
                10. * v + (1. + v) * alpha,
            ];
            let actual = point(&patches, u, v);
            assert!(norm(sub(actual, expected)) < 1e-11);
        }
    }
    // The displayed loft also agrees off station to its sampled budget here.
    for i in 0..101 {
        let v = i as f64 / 100.;
        let (s, c) = (v * std::f64::consts::FRAC_PI_2).sin_cos();
        assert!(
            norm(sub(
                point(&patches, 0., v),
                [(1. + v) * c, (1. + v) * s, 10. * v]
            )) <= options().max_deviation
        );
    }
}

#[test]
fn iterator_previews_resume_without_promoting_failed_budget() {
    let p = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let scale = law(1., 1.);
    let twist = law(0., TAU);
    let mut opt = options();
    opt.max_sections = 5;
    let mut sweep = Sweep::new(&p, &path, &scale, &twist, opt).unwrap();
    let first = sweep.next().unwrap().unwrap();
    assert!(!first.report.accepted && !first.patches.is_empty());
    assert_eq!(first.report.sections, 3);
    let second = sweep.next().unwrap().unwrap();
    assert_eq!(second.report.sections, 5);
    assert!(sweep.next().is_none());
    let result = approximate(&p, &path, &scale, &twist, opt).unwrap();
    assert!(result.patches.is_none());
    assert_eq!(result.levels.len(), 2);
}

#[test]
fn closed_rmf_with_full_turn_retains_scale_and_twist_seam() {
    let p = crate::primitives::line([5., 0., -1.], [5., 0., 1.]).unwrap();
    let path = crate::primitives::circle([0.; 3], [0., 0., 1.], 5.).unwrap();
    let opt = Options {
        normal: [0., 0., 1.],
        initial_sections: 5,
        max_sections: 257,
        max_deviation: 0.01,
        ..options()
    };
    let result = approximate(&p, &path, &law(1., 1.), &law(0., TAU), opt).unwrap();
    assert!(result.levels.last().unwrap().closed_path);
    let patches = result.patches.unwrap();
    for u in [0., 0.13, 0.5, 0.87, 1.] {
        assert!(norm(sub(point(&patches, u, 0.), point(&patches, u, 1.))) < 1e-12);
    }
    assert!(approximate(&p, &path, &law(1., 2.), &law(0., TAU), opt).is_err());
    assert!(approximate(&p, &path, &law(1., 1.), &law(0., 1.), opt).is_err());
}

#[test]
fn fixed_frenet_and_normal_orientation_are_distinct_and_fail_singular_frames() {
    let path = crate::primitives::circle_arc([0.; 3], [0., 0., 1.], 5., 0., 90.).unwrap();
    let p = crate::primitives::line([5., 0., 1.], [6., 0., 1.]).unwrap();
    let opt = Options {
        normal: [0., 0., 1.],
        max_deviation: 0.01,
        ..options()
    };
    for orientation in [
        Orientation::RotationMinimizing,
        Orientation::FixedNormal,
        Orientation::Frenet,
    ] {
        let result = approximate(
            &p,
            &path,
            &law(1., 1.),
            &law(0., 0.),
            Options { orientation, ..opt },
        )
        .unwrap();
        let q = point(&result.patches.unwrap(), 1., 1.);
        assert!(norm(sub(q, [0., 6., 1.])) < 1e-10);
    }
    let result = approximate(
        &p,
        &path,
        &law(1., 1.),
        &law(0., 0.),
        Options {
            orientation: Orientation::Fixed,
            ..opt
        },
    )
    .unwrap();
    assert!(norm(sub(point(&result.patches.unwrap(), 1., 1.), [1., 5., 1.])) < 1e-10);
    let line = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    assert!(
        approximate(
            &p,
            &line,
            &law(1., 1.),
            &law(0., 0.),
            Options {
                orientation: Orientation::Frenet,
                ..options()
            }
        )
        .is_err()
    );
    assert!(Sweep::new(&p, &line, &law(0., 1.), &law(0., 0.), options()).is_err());
}

#[test]
fn arc_length_stations_and_laws_ignore_nonlinear_rational_path_parameter() {
    let p = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    let mut path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    path.weights = vec![1., 2.];
    let opt = Options {
        spacing: Spacing::ArcLength {
            tolerance: 1e-3,
            max_cells: 100_000,
        },
        initial_sections: 3,
        max_sections: 3,
        max_deviation: 0.005,
        ..options()
    };
    let result = approximate(&p, &path, &law(1., 2.), &law(0., 0.), opt).unwrap();
    let report = result.levels.last().unwrap();
    assert!(report.accepted && report.length_residual_upper.unwrap() <= 1e-3);
    let patches = result.patches.unwrap();
    for v in [0., 0.13, 0.5, 0.87, 1.] {
        assert!(norm(sub(point(&patches, 0., v), [1. + v, 0., 10. * v])) < 1e-3);
    }
}

#[test]
fn dense_profiles_split_without_refitting_and_patch_domains_match() {
    let points = (0..41)
        .map(|i| [1. + i as f64 / 40., 0., 0.])
        .collect::<Vec<_>>();
    let p = crate::primitives::polyline(&points, false).unwrap();
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let result = approximate(&p, &path, &law(1., 1.), &law(0., 0.), options()).unwrap();
    let patches = result.patches.unwrap();
    assert_eq!(patches.len(), 40);
    let [a, b] = p.domain();
    for u in [a, a + 0.13 * (b - a), a + 0.5 * (b - a), b] {
        for v in [0., 0.37, 1.] {
            let expected = p.evaluate(u).unwrap().point;
            assert!(norm(sub(point(&patches, u, v), [expected[0], 0., 10. * v])) < 1e-11);
        }
    }
}

#[test]
fn rejects_kinks_and_invalid_resource_budgets() {
    let p = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    let path = crate::primitives::polyline(&[[0.; 3], [0., 0., 1.], [0., 1., 1.]], false).unwrap();
    assert!(Sweep::new(&p, &path, &law(1., 1.), &law(0., 0.), options()).is_err());
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    for opt in [
        Options {
            max_sections: 1026,
            ..options()
        },
        Options {
            initial_sections: 1,
            ..options()
        },
        Options {
            max_deviation: 0.,
            ..options()
        },
        Options {
            normal: [0.; 3],
            ..options()
        },
    ] {
        assert!(Sweep::new(&p, &path, &law(1., 1.), &law(0., 0.), opt).is_err());
    }
}

#[test]
fn tiny_open_path_is_not_mistaken_for_a_closed_loop() {
    let p = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![1e-15, 0., 0.], vec![2e-15, 0., 0.]],
        weights: vec![1., 1.],
        periodic: false,
    };
    let path = Curve {
        control_points: vec![vec![0.; 3], vec![0., 0., 1e-15]],
        ..p.clone()
    };
    let result = approximate(
        &p,
        &path,
        &law(1., 1.),
        &law(0., 0.),
        Options {
            initial_sections: 2,
            max_sections: 2,
            max_deviation: 1e-28,
            ..options()
        },
    )
    .unwrap();
    assert!(!result.levels[0].closed_path);
    let patches = result.patches.unwrap();
    assert!(norm(sub(point(&patches, 0., 1.), [1e-15, 0., 1e-15])) < 1e-28);
}

#[test]
fn fixed_orientation_accepts_a_piecewise_linear_path_with_corners() {
    let p = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    let path = crate::primitives::polyline(&[[0.; 3], [0., 0., 1.], [0., 1., 1.]], false).unwrap();
    let result = approximate(
        &p,
        &path,
        &law(1., 1.),
        &law(0., 0.),
        Options {
            orientation: Orientation::Fixed,
            max_deviation: 1e-10,
            ..options()
        },
    )
    .unwrap();
    let patches = result.patches.unwrap();
    for v in [0., 0.25, 0.5, 0.75, 1.] {
        let center = path.evaluate(v).unwrap().point;
        assert!(
            norm(sub(
                point(&patches, 0., v),
                [center[0] + 1., center[1], center[2]]
            )) < 1e-11
        );
    }
}

#[test]
fn multiple_profiles_refine_together_and_preserve_authored_boundary_joins() {
    let profiles = vec![
        crate::primitives::line([0.01, 0., 0.], [0.02, 0., 0.]).unwrap(),
        crate::primitives::line([2., 0., 0.], [2., 2., 0.]).unwrap(),
        crate::primitives::line([2., 2., 0.], [0., 2., 0.]).unwrap(),
    ];
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let scale = law(1., 2.);
    let twist = law(0., TAU / 4.);
    let opt = options();
    let small = approximate(&profiles[0], &path, &scale, &twist, opt).unwrap();
    let result = approximate_profiles(&profiles, &path, &scale, &twist, opt).unwrap();
    assert!(result.levels.last().unwrap().sections > small.levels.last().unwrap().sections);
    let ranges = result.profile_patch_ranges.unwrap();
    let patches = result.patches.unwrap();
    for v in [0., 0.123, 0.5, 0.789, 1.] {
        assert!(
            norm(sub(
                point(&patches[ranges[1][0]..ranges[1][1]], 1., v),
                point(&patches[ranges[2][0]..ranges[2][1]], 0., v)
            )) < 2e-14
        );
    }
    for (a, b) in patches[ranges[1][0]..ranges[1][1]]
        .iter()
        .zip(&patches[ranges[2][0]..ranges[2][1]])
    {
        assert_eq!(a.control_points.last().unwrap(), &b.control_points[0]);
        assert_eq!(a.knots_v, b.knots_v);
    }
    let refused = approximate_profiles(
        &profiles,
        &path,
        &scale,
        &twist,
        Options {
            max_sections: 3,
            ..opt
        },
    )
    .unwrap();
    assert!(refused.patches.is_none() && refused.profile_patch_ranges.is_none());
    assert!(approximate_profiles(&[], &path, &scale, &twist, opt).is_err());
    assert!(MultiSweep::new(&vec![profiles[0].clone(); 65], &path, &scale, &twist, opt).is_err());
    let mut preview = MultiSweep::new(&profiles, &path, &scale, &twist, opt).unwrap();
    assert!(!preview.next().unwrap().unwrap().report.accepted);
    assert!(preview.last().unwrap().unwrap().report.accepted);
}

#[test]
fn rational_outer_and_reversed_inner_contours_remain_separate_without_refit() {
    let outer = crate::primitives::circle([0.; 3], [0., 0., 1.], 3.).unwrap();
    let inner = crate::primitives::circle([0.; 3], [0., 0., 1.], 1.)
        .unwrap()
        .reverse()
        .unwrap();
    let profiles = vec![outer, inner];
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let result =
        approximate_profiles(&profiles, &path, &law(1., 2.), &law(0., 0.), options()).unwrap();
    assert_eq!(result.levels.last().unwrap().sections, 3);
    let ranges = result.profile_patch_ranges.unwrap();
    let patches = result.patches.unwrap();
    for (i, profile) in profiles.iter().enumerate() {
        let group = &patches[ranges[i][0]..ranges[i][1]];
        for u in [0., 0.123, 0.5, 0.789, 1.] {
            let source = profile.evaluate(u).unwrap().point;
            for v in [0., 0.17, 0.5, 0.83, 1.] {
                assert!(
                    norm(sub(
                        point(group, u, v),
                        [(1. + v) * source[0], (1. + v) * source[1], 10. * v]
                    )) < 2e-14
                );
            }
        }
        assert_eq!(
            group[0]
                .weights
                .iter()
                .map(|row| row[0])
                .collect::<Vec<_>>(),
            profile.weights
        );
    }
}

fn vector(a: V, b: V, weighted: bool) -> Curve {
    Curve {
        degree: 1,
        knots: vec![2., 2., 6., 6.],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights: if weighted { vec![1., 2.] } else { vec![1., 1.] },
        periodic: false,
    }
}
#[test]
fn anisotropic_scale_center_and_twist_match_independent_rational_equation() {
    let p =
        crate::paths::bezier(vec![vec![1., 2., 1.], vec![2., 3., 1.]], Some(vec![1., 2.])).unwrap();
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let axes = vector([1., 2., 3.], [2., 1., 4.], true);
    let center = vector([0.; 3], [1., -2., 0.5], false);
    let result = approximate_affine_profiles(
        &[p],
        &path,
        &law(1., 2.),
        &law(0., TAU / 4.),
        &axes,
        &center,
        Options {
            max_sections: 257,
            max_deviation: 0.01,
            ..options()
        },
    )
    .unwrap();
    let report = result.levels.last().unwrap();
    assert!(report.accepted);
    let patches = result.patches.unwrap();
    for i in 0..report.sections {
        let v = i as f64 / (report.sections - 1) as f64;
        let beta = 2. * v / (1. + v);
        let (s, c) = (v * TAU / 4.).sin_cos();
        for u in [0., 0.13, 0.5, 0.87, 1.] {
            let alpha = 2. * u / (1. + u);
            let x = (1. + v) * (1. + beta) * (1. + alpha) + v;
            let y = (1. + v) * (2. - beta) * (2. + alpha) - 2. * v;
            let z = (1. + v) * (3. + beta) + 0.5 * v;
            assert!(
                norm(sub(
                    point(&patches, u, v),
                    [c * x - s * y, s * x + c * y, 10. * v + z]
                )) < 1e-11
            );
        }
    }
    assert!(
        approximate_affine_profiles(
            &[crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap()],
            &path,
            &law(1., 1.),
            &law(0., 0.),
            &vector([1., 0., 1.], [1.; 3], false),
            &center,
            options()
        )
        .is_err()
    );
}
#[test]
fn closed_affine_laws_must_match_at_the_seam() {
    let profile = crate::primitives::line([5., 0., -1.], [5., 0., 1.]).unwrap();
    let path = crate::primitives::circle([0.; 3], [0., 0., 1.], 5.).unwrap();
    let opt = Options {
        normal: [0., 0., 1.],
        initial_sections: 5,
        max_sections: 257,
        max_deviation: 0.01,
        ..options()
    };
    let axes = vector([1., 2., 1.], [1., 2., 1.], false);
    let center = vector([0.1, 0., 0.], [0.1, 0., 0.], false);
    let result = approximate_affine_profiles(
        &[profile.clone()],
        &path,
        &law(1., 1.),
        &law(0., TAU),
        &axes,
        &center,
        opt,
    )
    .unwrap();
    let patches = result.patches.unwrap();
    assert!(norm(sub(point(&patches, 0.5, 0.), point(&patches, 0.5, 1.))) < 1e-14);
    for (a, b) in patches[0]
        .control_points
        .iter()
        .zip(&patches.last().unwrap().control_points)
    {
        assert_eq!(&a[0], b.last().unwrap());
    }
    for (a, c) in [
        (vector([1.; 3], [2., 1., 1.], false), center.clone()),
        (axes.clone(), vector([0.; 3], [0.1, 0., 0.], false)),
    ] {
        assert!(
            approximate_affine_profiles(
                &[profile.clone()],
                &path,
                &law(1., 1.),
                &law(0., TAU),
                &a,
                &c,
                opt
            )
            .is_err()
        );
    }
}

#[test]
fn changing_affine_laws_restarts_an_already_accepted_preview() {
    let profile = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let scale = law(1., 1.);
    let twist = law(0., 0.);
    let axes = vector([1.; 3], [3., 1., 1.], true);
    let center = constant_vector_law([0.; 3]).unwrap();
    let profiles = [profile];
    let mut sweep = MultiSweep::new(&profiles, &path, &scale, &twist, options()).unwrap();
    assert!(sweep.next().unwrap().unwrap().report.accepted);
    assert!(sweep.next().is_none());
    let mut changed = sweep.with_affine_laws(&axes, &center).unwrap();
    let first = changed.next().unwrap().unwrap();
    assert_eq!(first.report.sections, options().initial_sections);
    assert!(!first.report.accepted);
    assert!(changed.last().unwrap().unwrap().report.accepted);
}

#[test]
fn authored_full_frames_match_independent_equation_and_restart() {
    let profile = crate::primitives::line([1., 2., 3.], [2., 2., 3.]).unwrap();
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let scale = law(1., 2.);
    let twist = law(0., std::f64::consts::FRAC_PI_2);
    // The authored longitudinal axis is world X, perpendicular to guide Z.
    let axis = constant_vector_law([2., 0., 0.]).unwrap();
    let transverse = Curve {
        degree: 1,
        knots: vec![4., 4., 8., 8.],
        control_points: vec![vec![7., 1., 0.], vec![7., 0., 1.]],
        weights: vec![1., 2.],
        periodic: false,
    };
    let axes = constant_vector_law([2., 3., 4.]).unwrap();
    let center = constant_vector_law([0.5, 1., -1.]).unwrap();
    let mut opts = options();
    opts.max_sections = 513;
    let mut sweep = Sweep::new_authored(&profile, &path, &scale, &twist, &axis, &transverse, opts)
        .unwrap()
        .with_affine_laws(&axes, &center)
        .unwrap();
    let mut accepted = None;
    for level in sweep.by_ref() {
        let level = level.unwrap();
        if level.report.accepted {
            accepted = Some(level);
        }
    }
    let level = accepted.unwrap();
    for i in 0..level.report.sections {
        let f = i as f64 / (level.report.sections - 1) as f64;
        let y = (1. - f) / (1. + f);
        let z = 2. * f / (1. + f);
        let l = y.hypot(z);
        let angle = f * std::f64::consts::FRAC_PI_2;
        let ny = (y * angle.cos() - z * angle.sin()) / l;
        let nz = (y * angle.sin() + z * angle.cos()) / l;
        for u in [0., 0.37, 1.] {
            // initial frame N=Y, B=Z, T=X; q=(2,3,1+u).
            let a = (1. + f) * 4. + 0.5;
            let b = (1. + f) * 9. + 1.;
            let c = (1. + f) * 4. * (1. + u) - 1.;
            let expected = [c, a * ny - b * nz, 10. * f + a * nz + b * ny];
            assert!(norm(sub(point(&level.patches, u, f), expected)) < 1e-10);
        }
    }
    sweep = sweep.with_frame_laws(&axis, &transverse).unwrap();
    assert_eq!(sweep.next().unwrap().unwrap().report.sections, 3);
}

#[test]
fn authored_frames_refuse_sampled_zero_parallel_and_inconsistent_closed_seam() {
    let profile = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    let path = crate::primitives::line([0.; 3], [0., 0., 1.]).unwrap();
    let scale = law(1., 1.);
    let twist = law(0., 0.);
    let axis = constant_vector_law([0., 0., 1.]).unwrap();
    let parallel = constant_vector_law([0., 0., 2.]).unwrap();
    let zero = constant_vector_law([0.; 3]).unwrap();
    for (a, n) in [(&axis, &parallel), (&zero, &axis)] {
        let mut s = Sweep::new_authored(&profile, &path, &scale, &twist, a, n, options()).unwrap();
        assert!(s.next().unwrap().is_err());
        assert!(s.next().is_none());
    }
    let circle = crate::primitives::circle([0.; 3], [0., 0., 1.], 5.).unwrap();
    let changing = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![1., 0., 0.], vec![0., 1., 0.]],
        weights: vec![1., 1.],
        periodic: false,
    };
    let mut opts = options();
    opts.initial_sections = 5;
    let mut s =
        Sweep::new_authored(&profile, &circle, &scale, &twist, &axis, &changing, opts).unwrap();
    assert!(
        s.next()
            .unwrap()
            .unwrap_err()
            .to_string()
            .contains("authored frame endpoints")
    );
}

#[test]
fn authored_shared_grid_and_closed_frame_seam_preserve_retained_sections() {
    let profile = crate::primitives::line([5., 0., 1.], [5., 0., 2.]).unwrap();
    let path = crate::primitives::circle([0.; 3], [0., 0., 1.], 5.).unwrap();
    let scale = law(1., 1.);
    let twist = law(0., TAU);
    let axis = constant_vector_law([0., 1., 0.]).unwrap();
    let normal = constant_vector_law([0., 0., 1.]).unwrap();
    let mut opts = options();
    opts.initial_sections = 5;
    opts.orientation = Orientation::Fixed;
    let sweep = Sweep::new_authored(&profile, &path, &scale, &twist, &axis, &normal, opts).unwrap();
    let sections = sweep.sections_at(9).unwrap();
    assert_eq!(
        sections.first().unwrap().control_points,
        sections.last().unwrap().control_points
    );
    let profiles = vec![profile.clone(), profile];
    let mut multi = MultiSweep::new(&profiles, &path, &scale, &twist, opts)
        .unwrap()
        .with_frame_laws(&axis, &normal)
        .unwrap();
    let level = multi.next().unwrap().unwrap();
    assert_eq!(level.profile_patch_ranges.len(), 2);
    let [a, b] = level.profile_patch_ranges[0];
    let [c, d] = level.profile_patch_ranges[1];
    assert_eq!(b - a, d - c);
    for (left, right) in level.patches[a..b].iter().zip(&level.patches[c..d]) {
        assert_eq!(left.control_points, right.control_points);
    }
    assert!(level.report.closed_path);
}

#[cfg(feature = "transport")]
#[test]
fn authored_frame_transport_requires_both_laws_and_preserves_metadata() {
    use value_codec::json;
    let profile = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    let path = crate::primitives::line([0.; 3], [0., 0., 2.]).unwrap();
    let axis = constant_vector_law([0., 1., 0.]).unwrap();
    let normal = constant_vector_law([1., 0., 0.]).unwrap();
    let request = json!({"op":"surface_progressive_sweep", "profile":profile,
        "path":path,"scale":law(1.,1.),"twist":law(0.,0.),
        "orientation":"authored","frame_axis":axis,"frame_normal":normal,
        "normal":[1.,0.,0.],"spacing":"parameter","initial_sections":3,
        "max_sections":17,"max_deviation":0.001});
    let result = crate::transport::dispatch(request.clone()).unwrap();
    assert_eq!(result["report"]["accepted"], json!(true));
    assert_eq!(result["report"]["continuousBound"], json!(true));
    assert_eq!(result["report"]["roundingCertified"], json!(true));
    assert!(result["report"]["continuousErrorUpper"].as_f64().unwrap()<0.001);
    assert_eq!(result["report"]["continuousErrorScope"],json!("retained-patches-relative-to-original-profile-transport"));
    let mut incomplete = request;
    incomplete["frame_normal"] = json!(null);
    assert!(crate::transport::dispatch(incomplete).is_err());
}

#[test]
fn authored_multi_profile_level_shares_error_work_and_discards_partial_bound() {
    let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let profiles=vec![profile;32];
    let path=crate::primitives::line([0.;3],[0.,0.,10.]).unwrap();
    let axis=constant_vector_law([0.,0.,1.]).unwrap();
    let normal=constant_vector_law([1.,0.,0.]).unwrap();
    let scale=law(1.,1.);
    let twist=law(0.,0.);
    let options=Options {orientation:Orientation::Fixed,initial_sections:3,max_sections:9,..options()};
    let small=MultiSweep::new(&profiles[..2],&path,&scale,&twist,options).unwrap().with_frame_laws(&axis,&normal).unwrap().preview_at(9).unwrap();
    assert!(small.report.continuous_bound);
    assert!(small.report.continuous_error_upper.unwrap()<options.max_deviation);
    let large=MultiSweep::new(&profiles,&path,&scale,&twist,options).unwrap().with_frame_laws(&axis,&normal).unwrap().preview_at(9).unwrap();
    assert!(!large.report.continuous_bound);
    assert!(large.report.continuous_error_upper.is_none());
    assert!(large.report.error_certificate_cells<=10000);
    assert!(large.report.error_certificate_reason.is_some());
    assert!(large.report.known_profile_error_upper.is_some());
    let strict=Options {max_deviation:1e-30,..options};
    let rejected=MultiSweep::new(&profiles,&path,&scale,&twist,strict).unwrap().with_frame_laws(&axis,&normal).unwrap().preview_at(9).unwrap();
    assert_eq!(rejected.report.sampled_control_deviation,0.);
    assert!(!rejected.report.accepted);
    assert!(!rejected.report.continuous_bound);
    assert!(rejected.report.continuous_error_upper.is_none());
    assert!(rejected.report.known_profile_error_upper.unwrap()>strict.max_deviation);
}

#[test]
fn orientation_guide_matches_projected_rational_correspondence_and_restarts() {
    let profile = crate::primitives::line([1., 0., 0.], [2., 0., 1.]).unwrap();
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let guide = Curve {
        degree: 1,
        knots: vec![4., 4., 8., 8.],
        control_points: vec![vec![1., 0., 0.], vec![0., 1., 10.]],
        weights: vec![1., 2.],
        periodic: false,
    };
    let scale = law(1., 2.);
    let twist = law(0., 0.);
    let mut sweep = Sweep::new(&profile, &path, &scale, &twist, options())
        .unwrap()
        .with_orientation_guide(&guide)
        .unwrap();
    let mut final_level = None;
    for level in sweep.by_ref() {
        let level = level.unwrap();
        if level.report.accepted {
            final_level = Some(level);
        }
    }
    let level = final_level.unwrap();
    for i in 0..level.report.sections {
        let f = i as f64 / (level.report.sections - 1) as f64;
        let y = 2. * f / (1. + f);
        let x = (1. - f) / (1. + f);
        let length = x.hypot(y);
        for u in [0., 0.37, 1.] {
            let expected = [
                (1. + f) * (1. + u) * x / length,
                (1. + f) * (1. + u) * y / length,
                10. * f + (1. + f) * u,
            ];
            assert!(norm(sub(point(&level.patches, u, f), expected)) < 1e-10);
        }
    }
    sweep = sweep.with_orientation_guide(&guide).unwrap();
    assert_eq!(sweep.next().unwrap().unwrap().report.sections, 3);
}

#[test]
fn orientation_guide_arc_length_correspondence_and_degeneracy_refusals() {
    let profile = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let guide = crate::paths::bezier(
        vec![vec![1., 0., 0.], vec![1., 0., 10.]],
        Some(vec![1., 4.]),
    )
    .unwrap();
    let scale = law(1., 1.);
    let twist = law(0., 0.);
    let mut opts = options();
    opts.max_sections = 5;
    opts.spacing = Spacing::ArcLength {
        tolerance: 0.001,
        max_cells: 100000,
    };
    let mut sweep = Sweep::new(&profile, &path, &scale, &twist, opts)
        .unwrap()
        .with_orientation_guide(&guide)
        .unwrap();
    let level = sweep.next().unwrap().unwrap();
    assert!(level.report.accepted);
    assert!(level.report.length_residual_upper.unwrap() <= 0.001);
    for f in [0., 0.5, 1.] {
        assert!(norm(sub(point(&level.patches, 0., f), [1., 0., 10. * f])) < 0.002);
    }
    let mut coincident = Sweep::new(&profile, &path, &scale, &twist, options())
        .unwrap()
        .with_orientation_guide(&path)
        .unwrap();
    assert!(coincident.next().unwrap().is_err());
    opts.orientation = Orientation::Fixed;
    assert!(
        Sweep::new(&profile, &path, &scale, &twist, opts)
            .unwrap()
            .with_orientation_guide(&guide)
            .is_err()
    );
}

#[test]
fn orientation_guide_closed_seam_and_shared_contours() {
    let path = crate::primitives::circle([0.; 3], [0., 0., 1.], 5.).unwrap();
    let guide = crate::primitives::circle([0., 0., 1.], [0., 0., 1.], 5.).unwrap();
    let profile = crate::primitives::line([5., 0., 1.], [5., 0., 2.]).unwrap();
    let scale = law(1., 1.);
    let twist = law(0., TAU);
    let mut opts = options();
    opts.initial_sections = 5;
    let sweep = Sweep::new(&profile, &path, &scale, &twist, opts)
        .unwrap()
        .with_orientation_guide(&guide)
        .unwrap();
    let sections = sweep.sections_at(9).unwrap();
    assert_eq!(sections[0].control_points, sections[8].control_points);
    let profiles = vec![profile.clone(), profile.clone()];
    let mut multi = MultiSweep::new(&profiles, &path, &scale, &twist, opts)
        .unwrap()
        .with_orientation_guide(&guide)
        .unwrap();
    let level = multi.next().unwrap().unwrap();
    let [a, b] = level.profile_patch_ranges[0];
    let [c, d] = level.profile_patch_ranges[1];
    assert_eq!(b - a, d - c);
    for (left, right) in level.patches[a..b].iter().zip(&level.patches[c..d]) {
        assert_eq!(left.control_points, right.control_points);
    }
    let changing = crate::primitives::line([5., 0., 1.], [5., 0., -1.]).unwrap();
    let mut invalid = Sweep::new(&profile, &path, &scale, &twist, opts)
        .unwrap()
        .with_orientation_guide(&changing)
        .unwrap();
    assert!(invalid.next().unwrap().is_err());
}

#[cfg(feature = "transport")]
#[test]
fn orientation_guide_transport_refuses_conflicting_frames() {
    use value_codec::json;
    let p = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let guide = crate::primitives::line([1., 0., 0.], [1., 0., 10.]).unwrap();
    let request = json!({"op":"surface_progressive_sweep","profile":p,"path":path,
        "orientation_guide":guide,"scale":law(1.,1.),"twist":law(0.,0.),
        "orientation":"rmf","normal":[1.,0.,0.],"spacing":"parameter",
        "initial_sections":3,"max_sections":5,"max_deviation":0.001});
    let result = crate::transport::dispatch(request.clone()).unwrap();
    assert_eq!(result["report"]["accepted"], json!(true));
    for mode in ["fixed", "authored"] {
        let mut conflict = request.clone();
        conflict["orientation"] = json!(mode);
        assert!(crate::transport::dispatch(conflict).is_err());
    }
}

#[test]
fn contact_guide_fits_rational_anchor_at_every_retained_station() {
    let profile =
        crate::paths::bezier(vec![vec![0., 0., 0.], vec![2., 0., 0.]], Some(vec![1., 2.])).unwrap();
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let guide = crate::primitives::line([2., 0., 0.], [0., 4., 10.]).unwrap();
    let scale = law(1., 3.);
    let twist = law(0., 0.);
    let mut sweep = Sweep::new(&profile, &path, &scale, &twist, options())
        .unwrap()
        .with_contact_guide(&guide, 1.)
        .unwrap();
    let mut final_level = None;
    for level in sweep.by_ref() {
        let level = level.unwrap();
        if level.report.accepted {
            final_level = Some(level);
        }
    }
    let level = final_level.unwrap();
    for i in 0..level.report.sections {
        let f = i as f64 / (level.report.sections - 1) as f64;
        assert!(
            norm(sub(
                point(&level.patches, 1., f),
                [2. * (1. - f), 4. * f, 10. * f]
            )) < 1e-10
        );
        assert!(norm(sub(point(&level.patches, 0., f), [0., 0., 10. * f])) < 1e-10);
        let expected = [2. * (1. - f) * 2. / 3., 4. * f * 2. / 3., 10. * f];
        assert!(norm(sub(point(&level.patches, 0.5, f), expected)) < 1e-10);
    }
    let shifted = crate::primitives::line([2., 0., 1.], [2., 0., 11.]).unwrap();
    let mut invalid = Sweep::new(&profile, &path, &scale, &twist, options())
        .unwrap()
        .with_contact_guide(&shifted, 1.)
        .unwrap();
    assert!(invalid.next().unwrap().is_err());
    let nonzero = law(0., 1.);
    assert!(
        Sweep::new(&profile, &path, &scale, &nonzero, options())
            .unwrap()
            .with_contact_guide(&guide, 1.)
            .is_err()
    );
}

#[test]
fn shared_contact_anchor_preserves_hole_widths_and_restarts_aggregate_grid() {
    let profiles = vec![
        crate::primitives::line([0.; 3], [2., 0., 0.]).unwrap(),
        crate::paths::bezier(vec![vec![0., 0., 0.], vec![1., 0., 0.]], Some(vec![1., 2.])).unwrap(),
    ];
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let rail = crate::primitives::line([2., 0., 0.], [0., 4., 10.]).unwrap();
    let scale = law(1., 3.);
    let twist = law(0., 0.);
    let mut sweep = MultiSweep::new(&profiles, &path, &scale, &twist, options())
        .unwrap()
        .with_contact_guide(&rail, 0, 1.)
        .unwrap();
    let mut level = sweep.next().unwrap().unwrap();
    assert!(level.report.continuous_bound);
    assert!(!level.report.accepted);
    assert!(level.report.continuous_error_upper.unwrap() > level.report.budget);
    while !level.report.accepted {
        level = sweep.next().unwrap().unwrap();
    }
    for i in 0..level.report.sections {
        let f = i as f64 / (level.report.sections - 1) as f64;
        for (index, ratio) in [(0, 1.), (1, 0.5)] {
            let [a, b] = level.profile_patch_ranges[index];
            let actual = point(&level.patches[a..b], 1., f);
            assert!(
                norm(sub(
                    actual,
                    [2. * (1. - f) * ratio, 4. * f * ratio, 10. * f]
                )) < 1e-10
            );
        }
    }
    assert!(sweep.next().is_none());
    sweep = sweep.with_contact_guide(&rail, 0, 1.).unwrap();
    assert_eq!(sweep.next().unwrap().unwrap().report.sections, 3);
    assert!(
        MultiSweep::new(&profiles, &path, &scale, &twist, options())
            .unwrap()
            .with_contact_guide(&rail, 2, 1.)
            .is_err()
    );
    assert!(
        MultiSweep::new(&profiles, &path, &scale, &twist, options())
            .unwrap()
            .with_contact_guide(&rail, 0, 2.)
            .is_err()
    );
    let mut wrong = MultiSweep::new(&profiles, &path, &scale, &twist, options())
        .unwrap()
        .with_contact_guide(&rail, 1, 1.)
        .unwrap();
    assert!(wrong.next().unwrap().is_err());
}

#[test]
fn contact_closed_seam_refuses_matching_normals_with_mismatched_widths() {
    let path = crate::primitives::circle([0.; 3], [0., 0., 1.], 5.).unwrap();
    let mut guide = path.clone();
    for p in &mut guide.control_points {
        p[2] = 3.;
    }
    guide.control_points.last_mut().unwrap()[2] = 4.;
    let profile = crate::primitives::line([5., 0., 3.], [5., 0., 2.]).unwrap();
    let scale = law(1., 1.);
    let twist = law(0., 0.);
    let mut opts = options();
    opts.initial_sections = 5;
    let mut sweep = Sweep::new(&profile, &path, &scale, &twist, opts)
        .unwrap()
        .with_contact_guide(&guide, 0.)
        .unwrap();
    assert!(
        sweep
            .next()
            .unwrap()
            .unwrap_err()
            .to_string()
            .contains("rail widths must agree")
    );
}

#[test]
fn independent_preview_levels_preserve_iterator_state_and_admission() {
    let profile =
        crate::paths::bezier(vec![vec![1., 0., 0.], vec![2., 0., 1.]], Some(vec![1., 2.])).unwrap();
    let profiles = vec![profile.clone(), profile.clone()];
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let scale = law(1., 2.);
    let twist = law(0., std::f64::consts::FRAC_PI_2);
    let mut sweep = Sweep::new(&profile, &path, &scale, &twist, options()).unwrap();
    let preview = sweep.preview_at(5).unwrap();
    assert!(!preview.report.accepted);
    assert!(!preview.patches.is_empty());
    assert!(sweep.preview_at(2).is_err());
    assert!(sweep.preview_at(130).is_err());
    assert_eq!(sweep.next().unwrap().unwrap().report.sections, 3);
    let next = sweep.next().unwrap().unwrap();
    assert_eq!(next.report, preview.report);
    assert_eq!(
        next.patches[0].control_points,
        preview.patches[0].control_points
    );
    let mut multi = MultiSweep::new(&profiles, &path, &scale, &twist, options()).unwrap();
    let preview = multi.preview_at(5).unwrap();
    assert_eq!(preview.profile_patch_ranges.len(), 2);
    assert!(multi.preview_at(2).is_err());
    assert!(multi.preview_at(130).is_err());
    assert_eq!(multi.next().unwrap().unwrap().report.sections, 3);
    assert_eq!(multi.next().unwrap().unwrap().report, preview.report);
    #[cfg(feature = "transport")]
    {
        use value_codec::{Serialize, json};
        let encoded = preview.to_value();
        assert_eq!(encoded["preview"], json!(true));
        assert_eq!(encoded["report"]["accepted"], json!(false));
        assert_eq!(encoded["report"]["continuousBound"], json!(false));
    }
}


#[test]
fn corrected_frenet_continues_across_a_cubic_inflection_and_straight_path() {
    let path = crate::paths::bezier(
        vec![vec![0.,0.,0.],vec![1.,1.,0.],vec![2.,-1.,0.],vec![3.,0.,0.]],
        Some(vec![1.;4]),
    ).unwrap();
    let profile = crate::primitives::line([0.,0.,1.],[0.,0.,2.]).unwrap();
    let scale = law(1.,1.); let twist = law(0.,0.);
    let opts = Options {orientation:Orientation::CorrectedFrenet,normal:[0.,0.,1.],..options()};
    let sweep = Sweep::new(&profile,&path,&scale,&twist,opts).unwrap();
    let sections = sweep.sections_at(9).unwrap();
    // A planar inflection must not turn the profile's constant binormal offset.
    for (i,section) in sections.iter().enumerate() {
        let position = path.evaluate(i as f64 / 8.).unwrap().point;
        for (j,p) in section.control_points.iter().enumerate() {
            assert!((p[0]-position[0]).abs()<1e-10);
            assert!((p[1]-position[1]).abs()<1e-10);
            assert!((p[2]-(j+1) as f64).abs()<1e-10);
        }
    }
    assert!(Sweep::new(&profile,&path,&scale,&twist,Options{orientation:Orientation::Frenet,..opts}).unwrap().sections_at(9).is_err());
    let straight = crate::primitives::line([0.;3],[0.,0.,10.]).unwrap();
    let planar = crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let result = approximate(&planar,&straight,&scale,&twist,Options{normal:[1.,0.,0.],..opts}).unwrap();
    assert!(result.levels.last().unwrap().accepted);
    assert!(norm(sub(point(&result.patches.unwrap(),1.,1.),[2.,0.,10.]))<1e-10);
}


#[test]
fn corrected_frenet_resolves_zero_initial_curvature_from_the_first_principal_normal() {
    let path = crate::paths::bezier(vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![2.,0.,0.],vec![3.,1.,0.]], None).unwrap();
    let profile = crate::primitives::line([0.,0.,1.],[0.,0.,2.]).unwrap();
    let scale=law(1.,1.); let twist=law(0.,0.);
    let sweep=Sweep::new(&profile,&path,&scale,&twist,Options{orientation:Orientation::CorrectedFrenet,normal:[0.,0.,1.],..options()}).unwrap();
    let sections=sweep.sections_at(9).unwrap();
    for section in sections { assert!((section.control_points[1][2]-2.).abs()<1e-10); }
}


#[test]
fn corrected_frenet_closes_a_rational_circle_with_full_turn_twist() {
    let path=crate::primitives::circle([0.;3],[0.,0.,1.],5.).unwrap();
    let profile=crate::primitives::line([5.,0.,-1.],[5.,0.,1.]).unwrap();
    let opts=Options{orientation:Orientation::CorrectedFrenet,normal:[0.,0.,1.],initial_sections:5,max_sections:257,max_deviation:0.01,..options()};
    let result=approximate(&profile,&path,&law(1.,1.),&law(0.,TAU),opts).unwrap();
    assert!(result.levels.last().unwrap().accepted);
    assert!(result.levels.last().unwrap().closed_path);
    let patches=result.patches.unwrap();
    for u in [0.,0.13,0.5,0.87,1.] { assert!(norm(sub(point(&patches,u,0.),point(&patches,u,1.)))<1e-12); }
    let scale=law(1.,1.); let twist=law(0.,TAU);
    let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap();
    let sections=sweep.sections_at(result.levels.last().unwrap().sections).unwrap();
    assert_eq!(sections.first().unwrap().control_points,sections.last().unwrap().control_points);
    assert_eq!(sections.first().unwrap().weights,sections.last().unwrap().weights);
    assert!(approximate(&profile,&path,&law(1.,1.),&law(0.,1.),opts).is_err());
}

#[test]
fn corrected_frenet_handles_a_c1_straight_to_curved_join_without_a_defined_second_derivative() {
    let path=Curve{degree:3,knots:vec![0.,0.,0.,0.,0.5,0.5,1.,1.,1.,1.],
        control_points:vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![2.,0.,0.],vec![3.,0.,0.],vec![4.,1.,0.],vec![5.,2.,0.]],
        weights:vec![1.;6],periodic:false};
    path.validate().unwrap();
    assert!(path.evaluate(0.5).unwrap().d2.is_none());
    let profile=crate::primitives::line([0.,0.,1.],[0.,0.,2.]).unwrap();
    let scale=law(1.,1.); let twist=law(0.,0.);
    let opts=Options{orientation:Orientation::CorrectedFrenet,normal:[0.,0.,1.],..options()};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap();
    for (i,section) in sweep.sections_at(9).unwrap().iter().enumerate() {
        let position=path.evaluate(i as f64/8.).unwrap().point;
        for (j,p) in section.control_points.iter().enumerate() {
            assert!(norm(sub([p[0],p[1],p[2]],[position[0],position[1],(j+1) as f64]))<1e-10);
        }
    }
    assert!(Sweep::new(&profile,&path,&scale,&twist,Options{orientation:Orientation::Frenet,..opts}).unwrap().sections_at(9).is_err());
}
