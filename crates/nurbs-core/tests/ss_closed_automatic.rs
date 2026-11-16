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

#[test]
fn rational_plane_census_replays_exact_graph_coverage() {
    check_rational_plane_census(false, |_| ());
}

#[test]
fn rotated_rational_plane_census_replays_exact_graph_coverage() {
    check_rational_plane_census(false, |point| {
        point.swap(0, 2);
        point[0] = -point[0];
    });
}

#[test]
fn multispan_rational_plane_census_replays_exact_graph_coverage() {
    check_rational_plane_census(true, |_| ());
}

#[test]
fn rotated_multispan_rational_plane_census_replays_exact_graph_coverage() {
    check_rational_plane_census(true, |point| {
        point.swap(0, 2);
        point[0] = -point[0];
    });
}

fn check_rational_plane_census(multispan: bool, transform: impl Fn(&mut [f64])) {
    let original = fixture::circle();
    let mut plane = original.first().clone();
    // W(u) = 1 + 2u. The homogeneous X numerator is uW(u).
    // Its degree-two controls project to the original affine unit square.
    plane.degree_u = 2;
    plane.knots_u = vec![0., 0., 0., 1., 1., 1.];
    plane.control_points = [0., 0.25, 1.]
        .map(|x| vec![vec![x, 0., 0.], vec![x, 1., 0.]])
        .to_vec();
    plane.weights = vec![vec![1.; 2], vec![2.; 2], vec![3.; 2]];
    if multispan {
        // The exact knot insertion of W(u) = 1/2 + 2u at u = 1/2
        // has dyadic Euclidean controls for both X and Y.
        plane.knots_u = vec![0., 0., 0., 0.5, 1., 1., 1.];
        plane.control_points = [0., 0.125, 0.6875, 1.]
            .map(|x| vec![vec![x, 0., 0.], vec![x, 1., 0.]])
            .to_vec();
        plane.weights = [0.5, 1., 2., 2.5].map(|w| vec![w; 2]).to_vec();
    }
    let mut graph = original.second().clone();
    for surface in [&mut plane, &mut graph] {
        for point in surface.control_points.iter_mut().flatten() {
            transform(point);
        }
    }
    let source = ss::FiberSources::new(&plane, &graph).unwrap();
    let owner = ss::construct_qualified_closed_ss_branches(&source, ss::ClosedSSBuildLimits {
        max_repairs: 24,
        coverage: ss::CoverageLimits { max_depth: 28, ..Default::default() },
        components: ss::MixedComponentDiscoveryLimits {
            transition_proposals: 512,
            transition_depth: 18,
            ..Default::default()
        },
        ..Default::default()
    }).expect("complete original-source rational graph proof");
    assert_eq!(owner.branches().len(), 1);
    assert_eq!(owner.trims().len(), 2);
    let coverage = owner.owner().proof().coverage();
    assert_eq!(coverage.report().reason, "complete-original-graph-reduction");
    assert!(coverage.covers_entire_surfaces());
    assert!(coverage.fibers().iter().all(|fiber| {
        fiber.source().first() == &plane && fiber.source().second() == &graph
    }));
    owner.reverify().unwrap();
    let quotient = coverage.fibers().iter().map(|fiber| {
        let result = fiber.source().certify_quotient(
            fiber.domain(),
            fiber.free_axis(),
            ss::FiberLimits {
                preconditioner: fiber.report().operator.as_ref().map(|op| op.preconditioner()),
                ..Default::default()
            },
        ).unwrap();
        let reason = result.report().reason;
        result.into_fiber().unwrap_or_else(|| panic!("original quotient fiber: {reason}"))
    }).collect();
    let quotient = ss::QualifiedQuotientCoverage::from_nonperiodic_mixed(
        coverage, quotient,
        ss::CoverageLimits { max_rectangles: 1_000_000, max_depth: 28, ..Default::default() },
    ).unwrap();
    assert!(quotient.fibers().iter().all(|fiber| {
        fiber.source().first() == &plane && fiber.source().second() == &graph
    }));
    quotient.reverify().unwrap();
    let cycle = ss::order_closed_fiber_cycle(&owner, 0, ss::ClosedFiberCycleLimits {
        max_start_attempts: 128,
        max_candidates: 8192,
        max_steps: 256,
        ..Default::default()
    }).unwrap();
    let wire = cycle.to_root_wire(Default::default()).unwrap();
    assert_eq!(wire.edges().len(), cycle.arcs().len());
    for (edge, next) in wire.edges().iter().zip(wire.edges().iter().cycle().skip(1)) {
        assert!(edge.end().point().same_selector(next.start().point()));
    }
    for support in 0..2 {
        let winding = ss::qualify_original_ss_winding(&wire, support, [0.5, 0.5], Default::default())
            .unwrap();
        assert_eq!(winding.winding().abs(), 1);
    }
    if multispan {
        assert!(coverage.fibers().iter().any(|fiber| {
            let [lo, hi] = fiber.domain()[0];
            lo < 0.5 && 0.5 < hi
        }));
    }
}

#[test]
fn windowed_closed_census_replays_the_exact_reduced_product() {
    let original = fixture::circle();
    let mut plane = original.first().clone();
    plane.knots_u = vec![-0.125, -0.125, 1.125, 1.125];
    for (row, x) in plane.control_points.iter_mut().zip([-0.125, 1.125]) {
        for point in row {
            point[0] = x;
        }
    }
    let source = ss::FiberSources::new(&plane, original.second()).unwrap();
    let window = [[0.125, 0.875], [0.0625, 0.875], [0.0625, 0.9375], [0.125, 0.9375]];
    let owner =
        ss::construct_qualified_closed_ss_branches_in_box(&source, window, Default::default())
            .expect("complete closed intersection in the restricted product");
    assert_eq!(owner.branches().len(), 1);
    assert_eq!(owner.trims().len(), 2);
    let coverage = owner.owner().proof().coverage();
    assert_eq!(coverage.domain(), window);
    assert_eq!(coverage.report().reason, "complete-original-graph-reduction");
    assert!(!coverage.covers_entire_surfaces());
    assert!(!coverage.report().requested_entire_surfaces);
    assert!(coverage.fibers().iter().all(|fiber| {
        fiber.source().first() == source.first() && fiber.source().second() == source.second()
    }));
    owner.reverify().unwrap();

    let charts: Vec<_> = coverage.fibers().iter().map(|native| {
        let preconditioner = native.report().operator.as_ref().map(|op| op.preconditioner());
        native.source().certify_quotient(
            native.domain(),
            native.free_axis(),
            ss::FiberLimits { preconditioner, ..Default::default() },
        ).unwrap().into_fiber().expect("original quotient chart")
    }).collect();
    let limits = ss::CoverageLimits { max_rectangles: 1_000_000, ..Default::default() };
    let retained =
        ss::QualifiedQuotientCoverage::from_nonperiodic_mixed(coverage, charts.clone(), limits)
            .unwrap();
    assert_eq!(retained.domain(), window);
    assert!(retained.nodes().is_empty());
    assert_eq!(
        retained.root_enclosures().unwrap(),
        retained.fibers().iter().enumerate().map(|(index, fiber)| {
            (index, fiber.domain())
        }).collect::<Vec<_>>(),
    );
    retained.reverify().unwrap();

    let mut reordered = charts;
    let index = reordered.iter().position(|chart| {
        chart.domain() != reordered[0].domain() || chart.free_axis() != reordered[0].free_axis()
    }).unwrap();
    reordered.swap(0, index);
    assert!(ss::QualifiedQuotientCoverage::from_nonperiodic_mixed(
        coverage, reordered, limits,
    ).is_err());
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
            max_repairs: 129,
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
            max_charts: 193,
            ..Default::default()
        },
        ss::ClosedSSBuildLimits {
            proposal_rectangles: 250001,
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

#[test]
fn automatic_contact_aligns_unequal_degrees_and_world_coordinate_order() {
    let high = quartic_pair(0.);
    let mut a = fixture::plane();
    // The first coordinate is also affine on the plane, but only the
    // second and third coordinates form a shared affine projection.
    for (i, row) in a.control_points.iter_mut().enumerate() {
        for p in row {
            p[2] = i as f64 / 8.;
            p.rotate_right(1);
        }
    }
    let mut b = high.second().clone();
    for (i, row) in b.control_points.iter_mut().enumerate() {
        for p in row {
            p[2] -= if i == 4 { 0.125 } else { 0. };
            p[2] += i as f64 / 32.;
            p.rotate_right(1);
        }
    }
    for reverse in [false, true] {
        let pair = if reverse { ss::FiberSources::new(&b, &a) } else { ss::FiberSources::new(&a, &b) }.unwrap();
        let owner = ss::construct_qualified_ss_branches(&pair, Default::default()).unwrap();
        let ss::QualifiedSurfaceIntersection::IsolatedGraphContact(contact) = owner else {
            panic!("Expected the isolated contact");
        };
        assert_eq!(contact.specification().center, [0.5, 0.5]);
        assert_eq!(contact.specification().powers, [4, 4]);
        assert_eq!(contact.source().first(), pair.first());
        contact.reverify().unwrap();
    }
    b.control_points[2][2][0] += 1. / 1024.;
    let perturbed = ss::FiberSources::new(&a, &b).unwrap();
    assert!(ss::discover_isolated_graph_contact(&perturbed).unwrap().is_none());
}

#[test]
fn automatic_contact_preserves_nondyadic_original_uv() {
    use num_rational::BigRational as Q;

    let mut a = fixture::plane();
    a.knots_u = vec![2., 2., 5., 5.];
    a.knots_v = vec![1., 1., 3., 3.];
    let mut b = quartic_pair(0.).second().clone();
    b.knots_u = vec![-1.; 5];
    b.knots_u.extend([1.; 5]);
    b.knots_v = vec![4.; 5];
    b.knots_v.extend([8.; 5]);
    let u = [1., -2., 4., -8., 16.];
    let v = [1., -1., 1., -1., 1.];
    for (i, row) in b.control_points.iter_mut().enumerate() {
        for (j, point) in row.iter_mut().enumerate() {
            point[2] = u[i] + v[j];
        }
    }
    let source = ss::FiberSources::new(&a, &b).unwrap();
    let owner = ss::construct_qualified_ss_branches(&source, Default::default()).unwrap();
    let ss::QualifiedSurfaceIntersection::IsolatedGraphContact(contact) = owner else {
        panic!("Expected one exact isolated contact");
    };
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    assert_eq!(contact.exact_specification().center, [q(1, 3), q(1, 2)]);
    assert_eq!(contact.exact_specification().coefficients, [q(81, 1), q(16, 1)]);
    assert_eq!(contact.native_parameters(), [q(3, 1), q(2, 1), q(-1, 3), q(6, 1)]);
    contact.reverify().unwrap();
    let mut damaged = contact.exact_specification().clone();
    damaged.center[0] += q(1, 1024);
    assert!(ss::qualify_exact_isolated_graph_contact(&source, damaged).is_err());

    // The second surface now uses (V, 1-U) for the same world patch.
    b.control_points = (0..5)
        .map(|i| (0..5).map(|j| b.control_points[4 - j][i].clone()).collect())
        .collect();
    std::mem::swap(&mut b.knots_u, &mut b.knots_v);
    let remapped = ss::FiberSources::new(&a, &b).unwrap();
    let contact = ss::discover_isolated_graph_contact(&remapped).unwrap().unwrap();
    assert_eq!(contact.native_parameters(), [q(3, 1), q(2, 1), q(6, 1), q(1, 3)]);
    contact.reverify().unwrap();

    let reversed = ss::FiberSources::new(&b, &a).unwrap();
    let contact = ss::discover_isolated_graph_contact(&reversed).unwrap().unwrap();
    assert_eq!(contact.native_parameters(), [q(6, 1), q(1, 3), q(3, 1), q(2, 1)]);
    contact.reverify().unwrap();

    // W(u) = 1 + 2u and X(u) = uW(u) give the same affine clock.
    a.degree_u = 2;
    a.knots_u = vec![2., 2., 2., 5., 5., 5.];
    a.control_points = [0., 0.25, 1.]
        .map(|x| vec![vec![x, 0., 0.], vec![x, 1., 0.]])
        .to_vec();
    a.weights = [1., 2., 3.].map(|weight| vec![weight; 2]).to_vec();
    for (first, second, expected) in [
        (&a, &b, [q(3, 1), q(2, 1), q(6, 1), q(1, 3)]),
        (&b, &a, [q(6, 1), q(1, 3), q(3, 1), q(2, 1)]),
    ] {
        let source = ss::FiberSources::new(first, second).unwrap();
        let owner = ss::construct_qualified_ss_branches(&source, Default::default()).unwrap();
        let ss::QualifiedSurfaceIntersection::IsolatedGraphContact(contact) = owner else {
            panic!("Expected one exact contact on the rational support");
        };
        assert_eq!(contact.native_parameters(), expected);
        assert_eq!(contact.exact_specification().powers, [4, 4]);
        contact.reverify().unwrap();
    }
    a.control_points[1][0][0] += 1. / 1024.;
    let changed = ss::FiberSources::new(&a, &b).unwrap();
    assert!(ss::discover_isolated_graph_contact(&changed).unwrap().is_none());
}

#[test]
fn mixed_flat_contact_requires_strict_definiteness_of_the_original_gap() {
    use num_rational::BigRational as Q;
    let a = fixture::plane();
    let mut b = quartic_pair(0.).second().clone();
    b.degree_u = 2;
    b.knots_u = vec![0., 0., 0., 1., 1., 1.];
    b.weights = vec![vec![1.; 5]; 3];
    let u = [3., -3., 3.];
    let v = [3., -3., 3., -3., 3.];
    let cross_u = [-1., 0., 1.];
    let cross_v = [3., 0., -1., 0., 3.];
    for weight in [-1., 1., 2., 3.] {
        b.control_points = (0..3).map(|i| (0..5).map(|j| {
            vec![i as f64 / 2., j as f64 / 4., u[i] + v[j] + weight * cross_u[i] * cross_v[j]]
        }).collect()).collect();
        let pair = ss::FiberSources::new(&a, &b).unwrap();
        if weight >= 2. {
            assert!(ss::discover_isolated_graph_contact(&pair).unwrap().is_none());
            continue;
        }
        let owner = ss::construct_qualified_ss_branches(&pair, Default::default()).unwrap();
        let ss::QualifiedSurfaceIntersection::IsolatedGraphContact(contact) = owner else {
            panic!("Expected the definite mixed contact");
        };
        assert_eq!(contact.specification().center, [0.5, 0.5]);
        assert_eq!(contact.exact_specification().powers, [2, 4]);
        assert_eq!(contact.mixed_coefficient(), &Q::from_integer((24 * weight as i64).into()));
        contact.reverify().unwrap();
    }
}

#[test]
fn isolated_contact_is_invariant_under_nonsingular_world_shear() {
    let pair = quartic_pair(0.);
    let mut a = pair.first().clone();
    let mut b = pair.second().clone();
    for surface in [&mut a, &mut b] {
        for point in surface.control_points.iter_mut().flatten() {
            let [x, y, z] = [point[0], point[1], point[2]];
            *point = vec![x + y + z, x + 2. * y + z, 2. * x + y + 3. * z];
        }
    }
    let source = ss::FiberSources::new(&a, &b).unwrap();
    let owner = ss::construct_qualified_ss_branches(&source, Default::default()).unwrap();
    let ss::QualifiedSurfaceIntersection::IsolatedGraphContact(contact) = owner else {
        panic!("Expected exact contact in the sheared projection");
    };
    assert_eq!(contact.specification().center, [0.5, 0.5]);
    assert_eq!(contact.specification().powers, [4, 4]);
    assert_eq!(contact.source().second(), &b);
    contact.reverify().unwrap();

    b.control_points[2][2][2] += 1. / 1024.;
    let perturbed = ss::FiberSources::new(&a, &b).unwrap();
    assert!(ss::discover_isolated_graph_contact(&perturbed).unwrap().is_none());
}

#[test]
fn convex_gap_proof_covers_coupled_polynomial_by_exact_subdivision() {
    use num_rational::BigRational as Q;

    let a = fixture::plane();
    let mut b = a.clone();
    b.degree_u = 2;
    b.degree_v = 2;
    b.knots_u = vec![0., 0., 0., 1., 1., 1.];
    b.knots_v = b.knots_u.clone();
    b.weights = vec![vec![1.; 3]; 3];
    let quadratic = [1., -1., 1.];
    b.control_points = (0..3)
        .map(|i| (0..3).map(|j| {
            let (u, v) = (quadratic[i], quadratic[j]);
            vec![i as f64 / 2., j as f64 / 2., u + v + u * v / 2.]
        }).collect())
        .collect();
    let center = || std::array::from_fn(|_| Q::new(1.into(), 2.into()));
    for (a, b) in [(&a, &b), (&b, &a)] {
        let pair = ss::FiberSources::new(a, b).unwrap();
        let error = ss::qualify_convex_graph_contact(&pair, center(), 1).unwrap_err();
        assert_eq!(error.code, "NURBS_RESOURCE_LIMIT");
        let owner = ss::qualify_convex_graph_contact(&pair, center(), 256).unwrap();
        assert_eq!(owner.native_parameters(), std::array::from_fn(|_| Q::new(1.into(), 2.into())));
        owner.reverify().unwrap();
        assert!(ss::qualify_convex_graph_contact(&pair, [Q::new(1.into(), 3.into()), center()[1].clone()], 256).is_err());
    }
    for point in b.control_points.iter_mut().flatten() {
        point[2] += 1. / 1024.;
    }
    let pair = ss::FiberSources::new(&a, &b).unwrap();
    assert!(ss::qualify_convex_graph_contact(&pair, center(), 256).is_err());
}

#[test]
fn dispatcher_discovers_a_nondyadic_stationary_contact_in_a_coupled_gap() {
    use num_rational::BigRational as Q;

    let a = fixture::plane();
    let mut b = a.clone();
    b.degree_u = 2;
    b.degree_v = 2;
    b.knots_u = vec![0., 0., 0., 1., 1., 1.];
    b.knots_v = b.knots_u.clone();
    b.weights = vec![vec![1.; 3]; 3];
    let u = [1., -2., 4.];
    let v = [1., -1., 1.];
    b.control_points = (0..3).map(|i| (0..3).map(|j| {
        vec![i as f64 / 2., j as f64 / 2., u[i] + v[j] + u[i] * v[j] / 8.]
    }).collect()).collect();
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    for (first, second) in [(&a, &b), (&b, &a)] {
        let pair = ss::FiberSources::new(first, second).unwrap();
        let intersection = ss::construct_qualified_ss_branches(&pair, Default::default()).unwrap();
        assert!(intersection.covers_entire_surfaces());
        let ss::QualifiedSurfaceIntersection::ConvexGraphContact(owner) = intersection else {
            panic!("Expected automatic whole-support convex contact");
        };
        assert_eq!(owner.native_parameters(), [q(1,3),q(1,2),q(1,3),q(1,2)]);
        owner.reverify().unwrap();
    }
    for p in b.control_points.iter_mut().flatten() {
        p[2] += 1. / 1024.;
    }
    let pair = ss::FiberSources::new(&a, &b).unwrap();
    assert!(ss::discover_convex_graph_contact(&pair, 256).unwrap().is_none());
}

#[test]
fn convex_contact_allows_zero_curvature_on_the_support_boundary() {
    use num_rational::BigRational as Q;

    let a = fixture::plane();
    let mut b = a.clone();
    b.degree_u = 4;
    b.degree_v = 2;
    b.knots_u = vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.];
    b.knots_v = vec![0., 0., 0., 1., 1., 1.];
    b.weights = vec![vec![1.; 3]; 5];
    // u⁴ - u/2 + 3/16 + (2v-1)² is uniquely zero at (1/2, 1/2).
    b.control_points = (0..5).map(|i| (0..3).map(|j| {
        let quartic = if i == 4 { 1. } else { 0. };
        vec![i as f64 / 4., j as f64 / 2., quartic - i as f64 / 8. + 3./16. + [1.,-1.,1.][j]]
    }).collect()).collect();
    let pair = ss::FiberSources::new(&a, &b).unwrap();
    let contact = ss::discover_convex_graph_contact(&pair, 1).unwrap().unwrap();
    assert_eq!(contact.native_parameters(), std::array::from_fn(|_| Q::new(1.into(), 2.into())));
    contact.reverify().unwrap();
}

#[test]
fn rank_one_stationary_hessian_distinguishes_a_point_from_a_contact_line() {
    use num_rational::BigRational as Q;

    let a = fixture::plane();
    let mut b = a.clone();
    b.degree_u = 4;
    b.degree_v = 2;
    b.knots_u = vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.];
    b.knots_v = vec![0., 0., 0., 1., 1., 1.];
    b.weights = vec![vec![1.; 3]; 5];
    let u = [1., -1., 1., -1., 1.];
    let v = [1., -1., 1.];
    b.control_points = (0..5)
        .map(|i| (0..3).map(|j| {
            vec![i as f64 / 4., j as f64 / 2., u[i] + v[j] + u[i] * v[j] / 8.]
        }).collect())
        .collect();
    for (first, second) in [(&a, &b), (&b, &a)] {
        let pair = ss::FiberSources::new(first, second).unwrap();
        let owner = ss::construct_qualified_ss_branches(&pair, Default::default()).unwrap();
        let ss::QualifiedSurfaceIntersection::ConvexGraphContact(contact) = owner else {
            panic!("Expected a rank-one isolated graph contact");
        };
        assert_eq!(contact.native_parameters(), std::array::from_fn(|_| Q::new(1.into(), 2.into())));
        contact.reverify().unwrap();
    }

    for row in &mut b.control_points {
        for (j, point) in row.iter_mut().enumerate() {
            point[2] = v[j];
        }
    }
    let line = ss::FiberSources::new(&a, &b).unwrap();
    assert!(ss::discover_convex_graph_contact(&line, 256).unwrap().is_none());
}

#[test]
fn stationary_hessian_of_rank_zero_uses_the_whole_boundary_to_isolate_contact() {
    use num_rational::BigRational as Q;

    let a = fixture::plane();
    let mut b = quartic_pair(0.).second().clone();
    let fourth = [1., -1., 1., -1., 1.];
    for (i, row) in b.control_points.iter_mut().enumerate() {
        for (j, point) in row.iter_mut().enumerate() {
            point[2] = fourth[i] + fourth[j] + fourth[i] * fourth[j] / 8.;
        }
    }
    for (first, second) in [(&a, &b), (&b, &a)] {
        let pair = ss::FiberSources::new(first, second).unwrap();
        assert_eq!(
            ss::discover_convex_graph_contact(&pair, 3).unwrap_err().code,
            "NURBS_RESOURCE_LIMIT",
        );
        let result = ss::construct_qualified_ss_branches(&pair, Default::default()).unwrap();
        let ss::QualifiedSurfaceIntersection::ConvexGraphContact(contact) = result else {
            panic!("Expected the rank-zero convex contact");
        };
        assert_eq!(contact.native_parameters(), std::array::from_fn(|_| Q::new(1.into(), 2.into())));
        contact.reverify().unwrap();
    }

    for (i, row) in b.control_points.iter_mut().enumerate() {
        for point in row {
            point[2] = fourth[i];
        }
    }
    let line = ss::FiberSources::new(&a, &b).unwrap();
    assert!(ss::discover_convex_graph_contact(&line, 256).unwrap().is_none());
}

#[test]
fn flat_contact_search_recovers_nondyadic_center_and_splits_through_it() {
    use num_rational::BigRational as Q;

    let mut a = fixture::plane();
    a.knots_u = vec![2., 2., 5., 5.];
    let mut b = quartic_pair(0.).second().clone();
    b.knots_u = vec![-1.; 5];
    b.knots_u.extend([1.; 5]);
    let u = [1., -2., 4., -8., 16.];
    let v = [1., -1., 1., -1., 1.];
    for (i, row) in b.control_points.iter_mut().enumerate() {
        for (j, point) in row.iter_mut().enumerate() {
            point[2] = u[i] + v[j] + u[i] * v[j] / 512.;
        }
    }
    let q = |n: i64, d: i64| Q::new(n.into(), d.into());
    for (first, second, expected) in [
        (&a, &b, [q(3, 1), q(1, 2), q(-1, 3), q(1, 2)]),
        (&b, &a, [q(-1, 3), q(1, 2), q(3, 1), q(1, 2)]),
    ] {
        let pair = ss::FiberSources::new(first, second).unwrap();
        let owner = ss::construct_qualified_ss_branches(&pair, Default::default()).unwrap();
        let ss::QualifiedSurfaceIntersection::ConvexGraphContact(contact) = owner else {
            panic!("Expected the off-center rank-zero contact");
        };
        assert_eq!(contact.center(), &[q(1, 3), q(1, 2)]);
        assert_eq!(contact.native_parameters(), expected);
        contact.reverify().unwrap();
    }
}

#[test]
fn local_curvature_and_exterior_gap_prove_a_nonconvex_contact() {
    let a = fixture::plane();
    let mut b = quartic_pair(0.).second().clone();
    b.degree_v = 2;
    b.knots_v = vec![0., 0., 0., 1., 1., 1.];
    b.weights = vec![vec![1.; 3]; 5];
    let squared_u = [3., 0., -1., 0., 3.];
    let quartic_u = [1., -1., 1., -1., 1.];
    let squared_v = [1., -1., 1.];
    let net = |quartic_scale: f64| {
        (0..5)
            .map(|i| (0..3).map(|j| {
                vec![i as f64 / 4., j as f64 / 2.,
                    squared_u[i] - quartic_scale * quartic_u[i] + squared_v[j] / 8.]
            }).collect())
            .collect()
    };
    // With x = 2u - 1, 3x² - 3x⁴/4 stays positive away from zero
    // even though its second derivative is negative at x = ±1.
    b.control_points = net(0.75);
    for (first, second) in [(&a, &b), (&b, &a)] {
        let pair = ss::FiberSources::new(first, second).unwrap();
        let result = ss::construct_qualified_ss_branches(&pair, Default::default()).unwrap();
        let ss::QualifiedSurfaceIntersection::ConvexGraphContact(owner) = result else {
            panic!("Expected a locally convex graph contact");
        };
        assert_eq!(owner.center().each_ref().map(ToString::to_string), ["1/2", "1/2"]);
        owner.reverify().unwrap();
    }

    b.control_points = net(6.);
    let pair = ss::FiberSources::new(&a, &b).unwrap();
    assert!(ss::discover_convex_graph_contact(&pair, 256).unwrap().is_none());
}

#[test]
fn bernstein_corners_certify_flat_contact_even_with_indefinite_nearby_curvature() {
    let a = fixture::plane();
    let mut b = quartic_pair(0.).second().clone();
    let fourth = [1., -1., 1., -1., 1.];
    let three_squared = [3., 0., -1., 0., 3.];
    // x⁴ + y⁴ + 9x²y² has an indefinite Hessian along x=y≠0,
    // but all three summands are nonnegative and vanish together only at zero.
    for (i, row) in b.control_points.iter_mut().enumerate() {
        for (j, point) in row.iter_mut().enumerate() {
            point[2] = fourth[i] + fourth[j] + three_squared[i] * three_squared[j];
        }
    }
    for (first, second) in [(&a, &b), (&b, &a)] {
        let pair = ss::FiberSources::new(first, second).unwrap();
        let result = ss::construct_qualified_ss_branches(&pair, Default::default()).unwrap();
        let ss::QualifiedSurfaceIntersection::ConvexGraphContact(contact) = result else {
            panic!("Expected whole-support isolation by Bernstein sign");
        };
        assert_eq!(contact.center().each_ref().map(ToString::to_string), ["1/2", "1/2"]);
        contact.reverify().unwrap();
    }
}

#[test]
fn automatic_flat_contact_can_lie_on_an_original_edge_or_corner() {
    use num_rational::BigRational as Q;
    let a = fixture::plane();
    let mut b = quartic_pair(0.).second().clone();
    b.knots_u = vec![-1.; 5];
    b.knots_u.extend([1.; 5]);
    let u = [0., 0., 0., 0., 1.];
    for (coefficients, v_center) in [(u, Q::from_integer(0.into())), ([1.,-1.,1.,-1.,1.], Q::new(1.into(), 2.into()))] {
        for (i, row) in b.control_points.iter_mut().enumerate() {
            for (j, point) in row.iter_mut().enumerate() {
                point[2] = u[i] + coefficients[j] + u[i] * coefficients[j] / 8.;
            }
        }
        let pair = ss::FiberSources::new(&a, &b).unwrap();
        let result = ss::construct_qualified_ss_branches(&pair, Default::default()).unwrap();
        let ss::QualifiedSurfaceIntersection::ConvexGraphContact(contact) = result else {
            panic!("Expected a stationary boundary contact");
        };
        assert_eq!(contact.native_parameters(), [
            Q::from_integer(0.into()), v_center.clone(),
            Q::from_integer((-1).into()), v_center,
        ]);
        contact.reverify().unwrap();
    }
}
