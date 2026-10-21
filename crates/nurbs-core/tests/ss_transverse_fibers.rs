use nurbs_core::{
    ss_intersection::{FiberAxis, FiberLimits, FiberSources, FiberStatus},
    surface::Surface,
};
fn parabola(rational: bool) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..2)
            .map(|i| {
                vec![
                    vec![i as f64, 0., 0.],
                    vec![i as f64, 0.5, 0.],
                    vec![i as f64, 1., 1.],
                ]
            })
            .collect(),
        weights: vec![
            if rational {
                vec![1., 2., 4.]
            } else {
                vec![1.; 3]
            };
            2
        ],
        periodic_u: false,
        periodic_v: false,
    }
}
fn plane(z: f64, multispan: bool) -> Surface {
    let xs = if multispan {
        vec![0., 0.5, 1.]
    } else {
        vec![0., 1.]
    };
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: if multispan {
            vec![0., 0., 0.5, 1., 1.]
        } else {
            vec![0., 0., 1., 1.]
        },
        knots_v: vec![0., 0., 1., 1.],
        control_points: xs
            .iter()
            .map(|&x| vec![vec![x, 0., z], vec![x, 1., z]])
            .collect(),
        weights: vec![vec![1.; 2]; xs.len()],
        periodic_u: false,
        periodic_v: false,
    }
}
fn domain() -> [[f64; 2]; 4] {
    [[0.2, 0.4], [0.4, 0.6], [0.1, 0.5], [0.3, 0.7]]
}
#[test]
fn a_uniform_nonlinear_fiber_owns_the_original_surfaces_and_every_free_value() {
    let mut a = parabola(false);
    let original = a.clone();
    let b = plane(0.25, false);
    let sources = FiberSources::new(&a, &b).unwrap();
    let result = sources
        .certify(domain(), FiberAxis::FirstU, FiberLimits::default())
        .unwrap();
    assert_eq!(
        result.report().status,
        FiberStatus::UniformUniqueRoot,
        "{:?}",
        result.report()
    );
    assert_eq!(result.report().rectangles, 4);
    assert!(result.report().contraction_upper.unwrap() < 1.);
    let fiber = result.into_fiber().unwrap();
    fiber.reverify().unwrap();
    for u in [0.2, 0.25, 0.3, 0.35, 0.4] {
        let point = fiber.at(u, FiberLimits::default()).unwrap();
        assert_eq!(point.report().status, FiberStatus::UniformUniqueRoot);
        let image = point.report().root_image.unwrap();
        for (i, x) in [0.5, u, 0.5].into_iter().enumerate() {
            assert!(image[i][0] <= x && x <= image[i][1]);
        }
        let p = a.evaluate(u, 0.5).unwrap().point;
        let q = b.evaluate(u, 0.5).unwrap().point;
        assert!(p.iter().zip(q).all(|(x, y)| (x - y).abs() < 1e-14));
    }
    assert!(fiber.at(0.5, FiberLimits::default()).is_err());
    for p in a.control_points.iter_mut().flatten() {
        p[2] += 10.;
    }
    assert_eq!(fiber.source().first(), &original);
    fiber.reverify().unwrap();
    assert_eq!(
        FiberSources::new(&a, &b)
            .unwrap()
            .certify(domain(), FiberAxis::FirstU, Default::default())
            .unwrap()
            .report()
            .status,
        FiberStatus::Excluded
    );
}
#[test]
fn rational_weights_are_original_data_and_unknown_root_is_not_a_sampled_seed() {
    let a = parabola(true);
    let b = plane(0.25, false);
    let sources = FiberSources::new(&a, &b).unwrap();
    let d = [[0.2, 0.4], [0.32, 0.35], [0.1, 0.5], [0.4, 0.6]];
    let result = sources
        .certify(d, FiberAxis::FirstU, Default::default())
        .unwrap();
    assert_eq!(
        result.report().status,
        FiberStatus::UniformUniqueRoot,
        "{:?}",
        result.report()
    );
    let v = nurbs_core::interval_eval::Interval::point(1.)
        .div(nurbs_core::interval_eval::Interval::point(3.))
        .unwrap();
    let image = result.report().root_image.unwrap();
    assert!(image[0][0] <= v.lo && v.hi <= image[0][1]);
    for u in [0.2, 0.3, 0.4] {
        let p = a.evaluate(u, 1. / 3.).unwrap().point;
        let q = b.evaluate(u, 0.5).unwrap().point;
        assert!(p.iter().zip(q).all(|(x, y)| (x - y).abs() < 1e-14));
    }
    result.fiber().unwrap().reverify().unwrap();
}
#[test]
fn source_knots_and_swapped_free_axis_preserve_complete_fiber_census() {
    let a = parabola(false);
    let b = plane(0.25, true);
    let sources = FiberSources::new(&a, &b).unwrap();
    let d = [[0.2, 0.8], [0.4, 0.6], [0.1, 0.9], [0.3, 0.7]];
    let r = sources
        .certify(d, FiberAxis::FirstU, Default::default())
        .unwrap();
    assert_eq!(
        r.report().status,
        FiberStatus::UniformUniqueRoot,
        "{:?}",
        r.report()
    );
    assert_eq!(r.report().rectangles, 6);
    let limited = sources
        .certify(
            d,
            FiberAxis::FirstU,
            FiberLimits {
                max_rectangles: 5,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(limited.report().status, FiberStatus::Unresolved);
    assert!(limited.fiber().is_none());
    let reversed = FiberSources::new(&plane(0.25, false), &a).unwrap();
    let r = reversed
        .certify(
            [[0.1, 0.5], [0.3, 0.7], [0.2, 0.4], [0.4, 0.6]],
            FiberAxis::SecondU,
            Default::default(),
        )
        .unwrap();
    assert_eq!(
        r.report().status,
        FiberStatus::UniformUniqueRoot,
        "{:?}",
        r.report()
    );
    let image = r.report().root_image.unwrap();
    for (i, x) in [0.3, 0.5, 0.5].into_iter().enumerate() {
        assert!(image[i][0] <= x && x <= image[i][1]);
    }
}
#[test]
fn exclusion_tangency_missing_work_and_false_preconditioners_have_distinct_outcomes() {
    let a = parabola(false);
    let disjoint = FiberSources::new(&a, &plane(2., false))
        .unwrap()
        .certify(domain(), FiberAxis::FirstU, Default::default())
        .unwrap();
    assert_eq!(disjoint.report().status, FiberStatus::Excluded);
    assert!(disjoint.fiber().is_none());
    let tangent = FiberSources::new(&a, &plane(0., false))
        .unwrap()
        .certify(
            [[0.2, 0.4], [0., 0.1], [0.1, 0.5], [0., 0.2]],
            FiberAxis::FirstU,
            Default::default(),
        )
        .unwrap();
    assert_eq!(tangent.report().status, FiberStatus::Unresolved);
    assert!(tangent.fiber().is_none());
    let sources = FiberSources::new(&a, &plane(0.25, false)).unwrap();
    for limits in [
        FiberLimits {
            max_rectangles: 1,
            ..Default::default()
        },
        FiberLimits {
            max_rectangles: 3,
            ..Default::default()
        },
        FiberLimits {
            preconditioner: Some([[0.; 3]; 3]),
            ..Default::default()
        },
    ] {
        let r = sources
            .certify(domain(), FiberAxis::FirstU, limits)
            .unwrap();
        assert_eq!(r.report().status, FiberStatus::Unresolved);
        assert!(r.fiber().is_none());
    }
    assert!(
        sources
            .certify(
                domain(),
                FiberAxis::FirstU,
                FiberLimits {
                    max_rectangles: 0,
                    ..Default::default()
                }
            )
            .is_err()
    );
    assert!(
        sources
            .certify(
                domain(),
                FiberAxis::FirstU,
                FiberLimits {
                    preconditioner: Some([[f64::NAN; 3]; 3]),
                    ..Default::default()
                }
            )
            .is_err()
    );
    let mut d = domain();
    d[1] = [0.5, 0.5];
    assert!(
        sources
            .certify(d, FiberAxis::FirstU, Default::default())
            .is_err()
    );
    let mut d = domain();
    d[0] = [-0.1, 0.4];
    assert!(
        sources
            .certify(d, FiberAxis::FirstU, Default::default())
            .is_err()
    );
}

#[test]
fn krawczyk_excludes_a_correlated_empty_box_that_world_coordinate_hulls_do_not_exclude() {
    let a = parabola(false);
    let mut b = plane(0.1, false);
    for row in &mut b.control_points {
        row[1][2] = 1.1;
    }
    // Equality of y would require t=v; then zA=v²<=v but zB=t+0.1>v.
    let r = FiberSources::new(&a, &b)
        .unwrap()
        .certify(
            [[0.2, 0.4], [0.6, 0.8], [0.1, 0.5], [0., 1.]],
            FiberAxis::FirstU,
            Default::default(),
        )
        .unwrap();
    assert!(
        r.report()
            .residual_box
            .unwrap()
            .iter()
            .all(|r| r[0] <= 0. && 0. <= r[1])
    );
    assert_eq!(r.report().status, FiberStatus::Excluded, "{:?}", r.report());
    assert_eq!(r.report().reason, "original-krawczyk-exclusion");
    assert!(r.fiber().is_none());
}
#[test]
fn a_free_coordinate_is_a_proposal_and_periodic_endpoints_require_actual_exact_seam_data() {
    let sources = FiberSources::new(&parabola(false), &plane(0.25, false)).unwrap();
    for axis in [FiberAxis::FirstV, FiberAxis::SecondV] {
        let r = sources.certify(domain(), axis, Default::default()).unwrap();
        assert_eq!(r.report().status, FiberStatus::Unresolved);
        assert!(r.fiber().is_none());
    }
    let c = nurbs_core::curve::Curve {
        degree: 1,
        knots: vec![-1., 0., 1., 2., 3., 4., 5.],
        control_points: vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![1., 1., 0.],
            vec![0., 1., 0.],
            vec![0., 0., 0.],
        ],
        weights: vec![1.; 5],
        periodic: true,
    };
    let mut s = nurbs_core::surface::extrude(&c, [0., 0., 2.]).unwrap();
    let b = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![-1., -1., 1., 1.],
        control_points: vec![
            vec![vec![-0.5, -1., 1.], vec![-0.5, 1., 1.]],
            vec![vec![0.5, -1., 1.], vec![0.5, 1., 1.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let d = [[3.6, 4.], [0.3, 0.7], [0.3, 0.7], [-0.1, 0.6]];
    let good = FiberSources::new(&s, &b)
        .unwrap()
        .certify(d, FiberAxis::FirstU, Default::default())
        .unwrap();
    assert_eq!(
        good.report().status,
        FiberStatus::UniformUniqueRoot,
        "{:?}",
        good.report()
    );
    good.fiber().unwrap().reverify().unwrap();
    let end = good.fiber().unwrap().at(4., Default::default()).unwrap();
    assert_eq!(end.report().status, FiberStatus::UniformUniqueRoot);
    s.knots_u[0] = (-1f64).next_up();
    s.validate().unwrap();
    let bad = FiberSources::new(&s, &b)
        .unwrap()
        .certify(d, FiberAxis::FirstU, Default::default())
        .unwrap();
    assert_eq!(bad.report().status, FiberStatus::Unresolved);
    assert_eq!(bad.report().reason, "periodic-upper-seam-unproven");
}

#[test]
fn a_parameter_fiber_does_not_turn_a_singular_supports_point_image_into_a_curve() {
    let a = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![-1., -1., 1., 1.],
        control_points: vec![vec![vec![0., -1., 0.], vec![0., 1., 0.]]; 2],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let b = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-1., -1., 1., 1.],
        knots_v: vec![-1., -1., 1., 1.],
        control_points: vec![
            vec![vec![-1., 0., -1.], vec![-1., 0., 1.]],
            vec![vec![1., 0., -1.], vec![1., 0., 1.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let r = FiberSources::new(&a, &b)
        .unwrap()
        .certify(
            [[0., 1.], [-0.5, 0.5], [-0.5, 0.5], [-0.5, 0.5]],
            FiberAxis::FirstU,
            Default::default(),
        )
        .unwrap();
    assert_eq!(r.report().status, FiberStatus::UniformUniqueRoot);
    assert_eq!(r.report().support_regularity_proven, [false, true]);
    assert_eq!(
        a.evaluate(0., 0.).unwrap().point,
        a.evaluate(1., 0.).unwrap().point
    );
    r.fiber().unwrap().reverify().unwrap();
}
