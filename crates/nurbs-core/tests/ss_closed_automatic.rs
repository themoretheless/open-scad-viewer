use nurbs_core::ss_intersection as ss;
#[path = "support/ss_closed_fiber.rs"]
mod fixture;

#[test]
fn automatic_closed_census_uses_multiple_local_clocks() {
    let source = fixture::circle();
    let owner = ss::construct_qualified_closed_ss_branches(&source, Default::default())
        .expect("complete closed original-source proof");
    assert_eq!(owner.branches().len(), 1);
    assert_eq!(owner.trims().len(), 2);
    let charts = owner.owner().proof().coverage().fibers();
    assert!(
        charts
            .iter()
            .any(|a| charts.iter().any(|b| a.free_axis() != b.free_axis()))
    );
    owner.reverify().unwrap();
}

fn quartic_pair(offset: f64) -> ss::FiberSources {
    let mut a = fixture::plane();
    a.degree_u = 4;
    a.degree_v = 4;
    a.knots_u = vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.];
    a.knots_v = a.knots_u.clone();
    a.control_points = (0..5)
        .map(|i| {
            (0..5)
                .map(|j| {
                    vec![
                        i as f64 / 4.,
                        j as f64 / 4.,
                        if i == 4 { 0.125 } else { 0. },
                    ]
                })
                .collect()
        })
        .collect();
    a.weights = vec![vec![1.; 5]; 5];
    let mut b = a.clone();
    let fourth = [0.0625, -0.0625, 0.0625, -0.0625, 0.0625];
    for i in 0..5 {
        for j in 0..5 {
            b.control_points[i][j][2] += fourth[i] + fourth[j] - offset;
        }
    }
    ss::FiberSources::new(&a, &b).unwrap()
}

#[test]
fn flat_quartic_loop_has_a_complete_original_graph_reduction() {
    let source = quartic_pair(1. / 256.);
    let owner = ss::construct_qualified_closed_ss_branches(
        &source,
        ss::ClosedSSBuildLimits {
            seed_steps: 32,
            max_repairs: 8,
            max_charts: 64,
            components: ss::MixedComponentDiscoveryLimits {
                max_bridges: 63,
                max_pairs: 2016,
                ..Default::default()
            },
            coverage: ss::CoverageLimits {
                max_depth: 32,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(owner.branches().len(), 1);
    assert_eq!(owner.trims().len(), 2);
    assert_eq!(
        owner.owner().proof().coverage().report().reason,
        "complete-original-graph-reduction"
    );
    owner.reverify().unwrap();
}

#[test]
fn singular_isolated_quartic_contact_is_not_a_regular_closed_branch() {
    assert!(
        ss::construct_qualified_closed_ss_branches(&quartic_pair(0.), Default::default()).is_err()
    );
}

#[test]
fn missing_chart_coverage_and_exhausted_proposals_cannot_publish_a_closed_owner() {
    for limits in [
        ss::ClosedSSBuildLimits {
            max_seeds: 1,
            max_repairs: 0,
            ..Default::default()
        },
        ss::ClosedSSBuildLimits {
            max_charts: 1,
            ..Default::default()
        },
        ss::ClosedSSBuildLimits {
            proposal_rectangles: 1,
            ..Default::default()
        },
    ] {
        assert!(ss::construct_qualified_closed_ss_branches(&fixture::circle(), limits).is_err());
    }
}

#[test]
fn quartic_flat_singular_contact_has_an_exact_complete_point_certificate() {
    let source = quartic_pair(0.);
    let spec = ss::IsolatedGraphContactSpec {
        center: [0.5, 0.5],
        powers: [4, 4],
        coefficients: [1., 1.],
    };
    let owner = ss::qualify_isolated_graph_contact(&source, spec).unwrap();
    assert!(owner.covers_entire_surfaces());
    owner.reverify().unwrap();
    for changed in [
        ss::IsolatedGraphContactSpec {
            center: [0.5, 0.25],
            ..spec
        },
        ss::IsolatedGraphContactSpec {
            powers: [2, 4],
            ..spec
        },
        ss::IsolatedGraphContactSpec {
            coefficients: [1., -1.],
            ..spec
        },
    ] {
        assert!(ss::qualify_isolated_graph_contact(&source, changed).is_err());
    }
    let mut damaged = source.second().clone();
    damaged.control_points[2][2][2] = damaged.control_points[2][2][2].next_up();
    let damaged = ss::FiberSources::new(source.first(), &damaged).unwrap();
    assert!(ss::qualify_isolated_graph_contact(&damaged, spec).is_err());
    assert!(ss::qualify_isolated_graph_contact(&quartic_pair(1. / 256.), spec).is_err());
}

#[test]
fn invalid_work_limits_refuse_without_an_owner() {
    for limits in [
        ss::ClosedSSBuildLimits {
            max_repairs: 65,
            ..Default::default()
        },
        ss::ClosedSSBuildLimits {
            seed_steps: 0,
            ..Default::default()
        },
        ss::ClosedSSBuildLimits {
            max_seeds: 4097,
            ..Default::default()
        },
        ss::ClosedSSBuildLimits {
            max_charts: 65,
            ..Default::default()
        },
        ss::ClosedSSBuildLimits {
            proposal_rectangles: 0,
            ..Default::default()
        },
    ] {
        assert!(ss::construct_qualified_closed_ss_branches(&fixture::circle(), limits).is_err());
    }
}

#[test]
fn isolated_contact_replay_supports_operand_reversal_and_rejects_projection_damage() {
    let source = quartic_pair(0.);
    let reversed = ss::FiberSources::new(source.second(), source.first()).unwrap();
    let spec = ss::IsolatedGraphContactSpec {
        center: [0.5, 0.5],
        powers: [4, 4],
        coefficients: [-1., -1.],
    };
    ss::qualify_isolated_graph_contact(&reversed, spec)
        .unwrap()
        .reverify()
        .unwrap();
    let mut bad = source.second().clone();
    bad.control_points[2][2][0] = bad.control_points[2][2][0].next_up();
    let bad = ss::FiberSources::new(source.first(), &bad).unwrap();
    assert!(
        ss::qualify_isolated_graph_contact(
            &bad,
            ss::IsolatedGraphContactSpec {
                coefficients: [1., 1.],
                ..spec
            }
        )
        .is_err()
    );
}

#[test]
fn smooth_nonconic_loop_has_a_complete_original_source_owner() {
    let source = quartic_pair(0.);
    let mut a = source.first().clone();
    a.degree_v = 2;
    a.knots_v = vec![0., 0., 0., 1., 1., 1.];
    for row in &mut a.control_points {
        row.truncate(3);
        for (j, p) in row.iter_mut().enumerate() {
            p[1] = j as f64 / 2.;
        }
    }
    a.weights = vec![vec![1.; 3]; 5];
    let mut b = a.clone();
    let fourth = [0.0625, -0.0625, 0.0625, -0.0625, 0.0625];
    let square_u = [0.75, 0., -0.25, 0., 0.75];
    let square_v = [0.25, -0.25, 0.25];
    for i in 0..5 {
        for j in 0..3 {
            b.control_points[i][j][2] += fourth[i] + square_u[i] + square_v[j] - 0.0625;
        }
    }
    let source = ss::FiberSources::new(&a, &b).unwrap();
    assert!(ss::qualify_quadratic_graph_ss(&source, Default::default()).is_err());
    let owner = ss::construct_qualified_closed_ss_branches(
        &source,
        ss::ClosedSSBuildLimits {
            seed_steps: 32,
            coverage: ss::CoverageLimits {
                max_depth: 32,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(owner.branches().len(), 1);
    owner.reverify().unwrap();
}

#[test]
fn exact_graph_shear_keeps_original_carriers_and_root_parameter_ownership() {
    let original = quartic_pair(1. / 256.);
    let source =
        ss::FiberSources::with_exact_graph_shear(original.first(), original.second()).unwrap();
    assert_eq!(source.first(), original.first());
    assert_eq!(source.second(), original.second());
    let f = source
        .certify_contracted(
            [
                [0.46875, 0.53125],
                [0.245, 0.255],
                [0.45, 0.55],
                [0.245, 0.255],
            ],
            ss::FiberAxis::FirstU,
            100000,
        )
        .unwrap()
        .into_fiber()
        .unwrap();
    assert!(
        f.report()
            .operator
            .as_ref()
            .unwrap()
            .uses_exact_graph_shear()
    );
    f.reverify().unwrap();
    let p = f.at(0.5, Default::default()).unwrap().into_fiber().unwrap();
    p.reverify().unwrap();
    let roots = p.report().root_image.unwrap();
    for (box_, x) in roots.into_iter().zip([0.25, 0.5, 0.25]) {
        assert!(box_[0] <= x && x <= box_[1]);
    }
}

#[test]
fn public_dispatch_discovers_quartic_contact_without_supplied_specification() {
    let source = quartic_pair(0.);
    for source in [
        source.clone(),
        ss::FiberSources::new(source.second(), source.first()).unwrap(),
    ] {
        let before = (source.first().clone(), source.second().clone());
        let contact = ss::discover_isolated_graph_contact(&source)
            .unwrap()
            .unwrap();
        assert_eq!(contact.specification().center, [0.5, 0.5]);
        assert_eq!(contact.specification().powers, [4, 4]);
        contact.reverify().unwrap();
        let owner = ss::construct_qualified_ss_branches(&source, Default::default()).unwrap();
        assert!(matches!(
            owner,
            ss::QualifiedSurfaceIntersection::IsolatedGraphContact(_)
        ));
        assert!(owner.covers_entire_surfaces());
        owner.reverify().unwrap();
        assert_eq!(before, (source.first().clone(), source.second().clone()));
    }
}
#[test]
fn discovered_contact_never_absorbs_a_nonzero_gap_mixed_residual_or_projection_damage() {
    assert!(
        ss::discover_isolated_graph_contact(&quartic_pair(1. / 256.))
            .unwrap()
            .is_none()
    );
    let source = quartic_pair(0.);
    for axis in [0, 1, 2] {
        let mut b = source.second().clone();
        b.control_points[2][2][axis] = b.control_points[2][2][axis].next_up();
        let changed = ss::FiberSources::new(source.first(), &b).unwrap();
        assert!(
            ss::discover_isolated_graph_contact(&changed)
                .unwrap()
                .is_none()
        );
    }
}
#[test]
fn public_dispatch_builds_hidden_quartic_loop_with_complete_census_and_owned_trims() {
    let source = quartic_pair(1. / 256.);
    let limits = ss::QualifiedSSBuildLimits {
        closed: ss::ClosedSSBuildLimits {
            seed_steps: 32,
            max_repairs: 8,
            components: ss::MixedComponentDiscoveryLimits {
                max_bridges: 63,
                max_pairs: 2016,
                ..Default::default()
            },
            coverage: ss::CoverageLimits {
                max_depth: 32,
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    };
    let owner = ss::construct_qualified_ss_branches(&source, limits).unwrap();
    let ss::QualifiedSurfaceIntersection::RegularClosedLoops(ref closed) = owner else {
        panic!("Expected complete regular closed ownership")
    };
    assert_eq!(closed.branches().len(), 1);
    assert_eq!(closed.trims().len(), 2);
    assert!(owner.covers_entire_surfaces());
    owner.reverify().unwrap();
    assert!(closed.owner().proof().coverage().fibers().iter().all(|f|f.source().first()==source.first()&&f.source().second()==source.second()));
}

#[test]
fn automatic_closed_census_finds_both_disconnected_hidden_ovals() {
    let source = fixture::two_ovals();
    let owner = ss::construct_qualified_closed_ss_branches(
        &source,
        ss::ClosedSSBuildLimits {
            seed_steps: 32,
            max_repairs: 32,
            coverage: ss::CoverageLimits {
                max_depth: 32,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(owner.branches().len(), 2);
    assert_eq!(owner.trims().len(), 4);
    assert!(owner.owner().proof().coverage().covers_entire_surfaces());
    owner.reverify().unwrap();
}

#[test]
fn discovery_uses_original_shifted_contact_and_normalized_domains_with_independent_gauges() {
    let source = quartic_pair(0.);
    let mut a = source.first().clone();
    let mut b = a.clone();
    let u = [1., -3., 9., -27., 81.].map(|x| x / 256.);
    let v = [81., -27., 9., -3., 1.].map(|x| x / 256.);
    for i in 0..5 {
        for j in 0..5 {
            b.control_points[i][j][2] += 16. * u[i] + v[j] / 16.;
        }
    }
    a.knots_u = vec![2.; 5];
    a.knots_u.extend([6.; 5]);
    a.knots_v = vec![-4.; 5];
    a.knots_v.extend([4.; 5]);
    b.knots_u = vec![-7.; 5];
    b.knots_u.extend([-3.; 5]);
    b.knots_v = vec![10.; 5];
    b.knots_v.extend([12.; 5]);
    a.weights = vec![vec![3.; 5]; 5];
    b.weights = vec![vec![7.; 5]; 5];
    let source = ss::FiberSources::new(&a, &b).unwrap();
    let owner = ss::discover_isolated_graph_contact(&source)
        .unwrap()
        .unwrap();
    assert_eq!(
        owner.specification(),
        ss::IsolatedGraphContactSpec {
            center: [0.25, 0.75],
            powers: [4, 4],
            coefficients: [16., 1. / 16.]
        }
    );
    owner.reverify().unwrap();
    let public = ss::construct_qualified_ss_branches(&source, Default::default()).unwrap();
    assert!(matches!(
        public,
        ss::QualifiedSurfaceIntersection::IsolatedGraphContact(_)
    ));
    public.reverify().unwrap();
}

#[test]
fn automatic_contact_recognition_checks_all_supported_even_orders() {
    for degree in [2, 4, 6, 8] {
        let mut a = fixture::plane();
        a.degree_u = degree;
        a.degree_v = degree;
        a.knots_u = vec![0.; degree + 1];
        a.knots_u.extend(vec![1.; degree + 1]);
        a.knots_v = a.knots_u.clone();
        a.control_points = (0..=degree)
            .map(|i| (0..=degree).map(|j| vec![i as f64, j as f64, 0.]).collect())
            .collect();
        a.weights = vec![vec![3.; degree + 1]; degree + 1];
        let mut b = a.clone();
        b.weights = vec![vec![7.; degree + 1]; degree + 1];
        for i in 0..=degree {
            for j in 0..=degree {
                b.control_points[i][j][2] = (if i % 2 == 0 { 1. } else { -1. })
                    * 2f64.powi(-(degree as i32))
                    + (if j % 2 == 0 { 2. } else { -2. }) * 2f64.powi(-(degree as i32));
            }
        }
        let source = ss::FiberSources::new(&a, &b).unwrap();
        let o = ss::discover_isolated_graph_contact(&source)
            .unwrap()
            .unwrap();
        assert_eq!(
            o.specification(),
            ss::IsolatedGraphContactSpec {
                center: [0.5, 0.5],
                powers: [degree, degree],
                coefficients: [1., 2.]
            }
        );
        o.reverify().unwrap();
        let public = ss::construct_qualified_ss_branches(&source, Default::default()).unwrap();
        public.reverify().unwrap();
    }
}
