use super::*;
#[test]
fn independent_irrational_roots_on_multispan_uv_uses_keep_owned_plane_relation() {
    let (sa, sb, mut a, mut b, mut world) = fixture();
    // x(q)=(q+q*q)/2 has strictly positive derivative. Its roots for
    // x=1/4 and x=1/2 are irrational; no binary64 parameter is supplied.
    world.degree = 2;
    world.knots = vec![-5., -5., -5., 3., 3., 3.];
    world.control_points = vec![vec![0., 0., 0.], vec![0.25, 0., 0.], vec![1., 0., 0.]];
    world.weights = vec![1.; 3];
    a.degree = 2;
    a.knots = vec![2., 2., 2., 3., 4., 4., 4.];
    a.control_points = [0., 0.125, 0.625, 1.]
        .into_iter()
        .map(|x| vec![x, 0.])
        .collect();
    a.weights = vec![1.; 4];
    b.degree = 2;
    b.knots = vec![10., 10., 10., 12., 14., 18., 18., 18.];
    b.control_points = [0., 0.0625, 0.25, 0.625, 1.]
        .into_iter()
        .map(|x| vec![x, 0.])
        .collect();
    b.weights = vec![1.; 5];
    let root = |surface: &Surface, curve: &Curve, x: f64, selector: [f64; 2]| {
        let cutter = Curve::from_polyline(vec![vec![x, -1.], vec![x, 1.]]).unwrap();
        let report = crate::source_contact_point::qualify(
            surface,
            curve,
            &cutter,
            [selector, [0.4, 0.6]],
            10000,
        )
        .unwrap();
        Endpoint::Crossing {
            point: report.point.expect(report.reason),
            role: Role::Boundary,
        }
    };
    let fa = Fragment::new(
        &sa,
        &a,
        root(&sa, &a, 0.25, [2.73, 2.74]),
        root(&sa, &a, 0.5, [3.23, 3.24]),
    )
    .unwrap();
    let fb = Fragment::new(
        &sb,
        &b,
        root(&sb, &b, 0.5, [14.94, 14.95]),
        root(&sb, &b, 0.25, [12.92, 12.94]),
    )
    .unwrap();
    let plane = |x| [[x, 0., 0.], [x, 1., 0.], [x, 0., 1.]];
    let ranges = [[[0., 1.], [1., 1.]]; 2];
    let planes = [Some(plane(0.25)), Some(plane(0.5))];
    let mut split = world.clone();
    split.knots = vec![-5., -5., -5., -1., 3., 3., 3.];
    split.control_points = [0., 0.125, 0.625, 1.]
        .into_iter()
        .map(|x| vec![x, 0., 0.])
        .collect();
    split.weights = vec![1.; 4];
    for world in [world, split] {
        let report = qualify(&world, [&fa, &fb], ranges, planes, 100000000, 10000).unwrap();
        let edge = report.edge.expect(report.reason);
        assert!(report.driver_cells > 0);
        assert_eq!(report.root_checks, 2);
        assert!(!edge.covers_complete_canonical_source());
        assert!(edge.uses().iter().all(|f| {
            f.endpoints()
                .iter()
                .all(|e| matches!(e, Endpoint::Crossing { .. }))
        }));
        let restored = crate::source_shared_edge_restore::restore(
            edge.definition(),
            crate::source_shared_edge_restore::Limits {
                mapping_cells_per_use: 10000,
                exact_work: 100000000,
                driver_cells: 10000,
            },
        )
        .unwrap();
        assert_eq!(
            restored.edge.expect(restored.reason).definition(),
            edge.definition()
        );
        assert!(
            qualify(&world, [&fa, &fb], ranges, planes, 100000000, 0)
                .unwrap()
                .edge
                .is_none()
        );
        assert!(
            qualify(&world, [&fa, &fb], ranges, [None; 2], 100000000, 10000)
                .unwrap()
                .edge
                .is_none()
        );
        assert!(
            qualify(
                &world,
                [&fa, &fb],
                ranges,
                [planes[0], Some(plane(0.5 + 1e-12))],
                100000000,
                10000
            )
            .unwrap()
            .edge
            .is_none()
        );
    }
}
#[test]
fn local_plane_uniqueness_matches_one_branch_and_refuses_opposite_branch_roots() {
    for oblique in [false, true] {
        let (mut sa, mut sb, mut a, mut b, mut world) = fixture();
        // x(q)=(2q-1)^2 crosses each chosen plane twice globally.
        world.degree = 2;
        world.knots = vec![-5., -5., -5., 3., 3., 3.];
        world.control_points = vec![vec![1., 0., 0.], vec![-1., 0., 0.], vec![1., 0., 0.]];
        world.weights = vec![1.; 3];
        a.degree = 2;
        a.knots = vec![2., 2., 2., 3., 4., 4., 4.];
        a.control_points = [1., 0., 0., 1.].into_iter().map(|x| vec![x, 0.]).collect();
        a.weights = vec![1.; 4];
        b.degree = 2;
        b.knots = vec![10., 10., 10., 12., 14., 18., 18., 18.];
        b.control_points = [1., 0.5, 0., 0., 1.]
            .into_iter()
            .map(|x| vec![x, 0.])
            .collect();
        b.weights = vec![1.; 5];
        if oblique {
            // Apply the same exact affine shear to both original surfaces
            // and their canonical curve; the UV root owners stay unchanged.
            let shear = |p: &mut Vec<f64>| {
                let (x, y) = (p[0], p[1]);
                p[0] = x + y;
                p[1] = x + 2. * y;
            };
            for p in &mut world.control_points {
                shear(p);
            }
            for surface in [&mut sa, &mut sb] {
                for row in &mut surface.control_points {
                    for p in row {
                        shear(p);
                    }
                }
            }
        }
        let root = |surface: &Surface, curve: &Curve, x: f64, selector: [f64; 2]| {
            let cutter = Curve::from_polyline(vec![vec![x, -1.], vec![x, 1.]]).unwrap();
            let report = crate::source_contact_point::qualify(
                surface,
                curve,
                &cutter,
                [selector, [0.4, 0.6]],
                10000,
            )
            .unwrap();
            Endpoint::Crossing {
                point: report.point.expect(report.reason),
                role: Role::Boundary,
            }
        };
        let fa = Fragment::new(
            &sa,
            &a,
            root(&sa, &a, 0.5, [2.29, 2.30]),
            root(&sa, &a, 0.125, [2.64, 2.65]),
        )
        .unwrap();
        let fb = Fragment::new(
            &sb,
            &b,
            root(&sb, &b, 0.125, [12.58, 12.59]),
            root(&sb, &b, 0.5, [11.17, 11.18]),
        )
        .unwrap();
        let plane = |x| {
            if oblique {
                [[x, x, 0.], [x + 1., x + 2., 0.], [x, x, 1.]]
            } else {
                [[x, 0., 0.], [x, 1., 0.], [x, 0., 1.]]
            }
        };
        let planes = [Some(plane(0.5)), Some(plane(0.125))];
        let ranges = [[[0., 1.], [1., 1.]]; 2];
        assert!(
            !nurbs_core::curve_axis_driver::certify(&world, 0, 128)
                .unwrap()
                .monotonic_proven
        );
        let report = qualify(&world, [&fa, &fb], ranges, planes, 100000000, 10000).unwrap();
        let edge = report.edge.expect(report.reason);
        assert_eq!(report.root_checks, 2);
        assert!(report.driver_cells > 0);
        let replay = crate::source_shared_edge_restore::restore(
            edge.definition(),
            crate::source_shared_edge_restore::Limits {
                mapping_cells_per_use: 10000,
                exact_work: 100000000,
                driver_cells: 10000,
            },
        )
        .unwrap();
        assert_eq!(
            replay.edge.expect(replay.reason).definition(),
            edge.definition()
        );
        // A fixed original parameter must match the owned root on the same
        // branch, even when that plane also meets another canonical branch.
        let mixed_a = Fragment::new(
            &sa,
            &a,
            root(&sa, &a, 0.25, [2.49, 2.51]),
            root(&sa, &a, 0.125, [2.64, 2.65]),
        )
        .unwrap();
        let mixed_b = Fragment::new(
            &sb,
            &b,
            root(&sb, &b, 0.125, [12.58, 12.59]),
            Endpoint::Parameter(12.),
        )
        .unwrap();
        let mixed_planes = [Some(plane(0.25)), Some(plane(0.125))];
        let mixed = qualify(
            &world,
            [&mixed_a, &mixed_b],
            ranges,
            mixed_planes,
            100000000,
            10000,
        )
        .unwrap();
        let mixed_edge = mixed.edge.expect(mixed.reason);
        let restored = crate::source_shared_edge_restore::restore(
            mixed_edge.definition(),
            crate::source_shared_edge_restore::Limits {
                mapping_cells_per_use: 10000,
                exact_work: 100000000,
                driver_cells: 10000,
            },
        )
        .unwrap();
        assert_eq!(
            restored.edge.expect(restored.reason).definition(),
            mixed_edge.definition()
        );
        let mixed_wrong = Fragment::new(
            &sb,
            &b,
            root(&sb, &b, 0.125, [12.58, 12.59]),
            Endpoint::Parameter(16.),
        )
        .unwrap();
        assert!(
            qualify(
                &world,
                [&mixed_a, &mixed_wrong],
                ranges,
                mixed_planes,
                100000000,
                256
            )
            .unwrap()
            .edge
            .is_none()
        );
        // Same physical plane/point, but the other canonical branch must not
        // be merged into the left branch by an overlap or midpoint heuristic.
        let wrong = Fragment::new(
            &sb,
            &b,
            root(&sb, &b, 0.125, [15.41, 15.42]),
            root(&sb, &b, 0.5, [11.17, 11.18]),
        )
        .unwrap();
        assert!(
            qualify(&world, [&fa, &wrong], ranges, planes, 100000000, 256)
                .unwrap()
                .edge
                .is_none()
        );
        assert!(
            qualify(&world, [&fa, &fb], ranges, planes, 100000000, 0)
                .unwrap()
                .edge
                .is_none()
        );
        assert!(
            qualify(&world, [&fa, &fb], ranges, [None; 2], 100000000, 10000)
                .unwrap()
                .edge
                .is_none()
        );
    }
}
#[test]
fn linear_roots_on_different_source_coverage_keep_exact_canonical_parameters() {
    let (sa, sb, a, b, world) = fixture();
    let point = |s: &Surface, c: &Curve, x: f64, selector: [f64; 2]| {
        let cut = Curve::from_polyline(vec![vec![x, -1.], vec![x, 1.]]).unwrap();
        let r = crate::source_contact_point::qualify(s, c, &cut, [selector, [0.4, 0.6]], 10000)
            .unwrap();
        assert!(r.point.is_some(), "{}", r.reason);
        Endpoint::Crossing {
            point: r.point.unwrap(),
            role: Role::Boundary,
        }
    };
    let fa = Fragment::new(
        &sa,
        &a,
        point(&sa, &a, 0.4, [2.7, 2.9]),
        point(&sa, &a, 0.6, [3.1, 3.3]),
    )
    .unwrap();
    let fb = Fragment::new(
        &sb,
        &b,
        point(&sb, &b, 0.6, [15.5, 15.7]),
        point(&sb, &b, 0.4, [12.3, 12.5]),
    )
    .unwrap();
    let mut split_world = world.clone();
    split_world.knots = vec![-5., -5., -1., 3., 3.];
    split_world.control_points.insert(1, vec![0.5, 0., 0.]);
    split_world.weights.insert(1, 1.);
    for world in [world, split_world] {
        let r = qualify(&world, [&fa, &fb], RANGES, [None, None], 100_000_000, 0).unwrap();
        assert!(r.edge.is_some(), "{}", r.reason);
        assert_eq!(r.driver_cells, 0);
        let e = r.edge.unwrap();
        let replay = crate::source_shared_edge_restore::restore(
            e.definition(),
            crate::source_shared_edge_restore::Limits {
                mapping_cells_per_use: 10000,
                exact_work: 100000000,
                driver_cells: 10000,
            },
        )
        .unwrap();
        assert_eq!(
            replay.edge.expect(replay.reason).definition(),
            e.definition()
        );
        assert!(e.uses().iter().all(|f| {
            f.endpoints()
                .iter()
                .all(|p| matches!(p, Endpoint::Crossing { .. }))
        }));
        let wrong = Fragment::new(
            &sb,
            &b,
            point(&sb, &b, 0.6 + 1e-12, [15.5, 15.7]),
            point(&sb, &b, 0.4, [12.3, 12.5]),
        )
        .unwrap();
        assert!(
            qualify(&world, [&fa, &wrong], RANGES, [None, None], 100_000_000, 0)
                .unwrap()
                .edge
                .is_none()
        );
    }
}
#[test]
fn nonlinear_root_fixed_end_has_exact_candidate_without_plane_hints() {
    let (sa, _, _, _, _) = fixture();
    let uv = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let world = Curve {
        control_points: uv
            .control_points
            .iter()
            .map(|p| vec![p[0], p[1], 0.])
            .collect(),
        ..uv.clone()
    };
    let sb = Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: uv.knots.clone(),
        control_points: (0..2)
            .map(|z| {
                uv.control_points
                    .iter()
                    .map(|p| vec![p[0], p[1], z as f64])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let side = Curve::from_polyline(vec![vec![0., 0.], vec![0., 1.]]).unwrap();
    let cut = Curve::from_polyline(vec![vec![0., 0.859375], vec![1., 0.859375]]).unwrap();
    let point =
        crate::source_contact_point::qualify(&sa, &uv, &cut, [[0.6, 0.65], [0.58, 0.63]], 10000)
            .unwrap()
            .point
            .unwrap();
    let definition = point.definition();
    let fa = Fragment::new(
        &sa,
        &uv,
        Endpoint::Parameter(0.),
        Endpoint::Crossing {
            point: point.clone(),
            role: Role::Boundary,
        },
    )
    .unwrap();
    let fb = Fragment::new(
        &sb,
        &side,
        Endpoint::Parameter(0.625),
        Endpoint::Parameter(0.),
    )
    .unwrap();
    let ranges = [[[0., 1.], [1., 1.]]; 2];
    assert!(
        qualify(&world, [&fa, &fb], ranges, [None; 2], 100_000_000, 0)
            .unwrap()
            .edge
            .is_none()
    );
    let candidates = [[None, Some([0.625, 0.609375])], [None, None]];
    let report = qualify_with_candidates(
        &world,
        [&fa, &fb],
        ranges,
        [None; 2],
        candidates,
        100_000_000,
        0,
    )
    .unwrap();
    assert!(report.edge.is_some(), "{}", report.reason);
    let edge = report.edge.unwrap();
    crate::source_edge_restriction::assert_replay(&edge);
    let restriction = crate::source_edge_restriction::Restriction::from_edge(&edge);
    let b = restriction.parameter_bounds().unwrap()[1];
    assert!(b[0] <= 0.625 && 0.625 <= b[1]);
    let b = restriction.endpoint_boxes(100).unwrap()[1];
    for (axis, expected) in [0.609375, 0.859375, 0.].into_iter().enumerate() {
        assert!(b[axis][0] <= expected && expected <= b[axis][1]);
    }
    let restore = |value| {
        crate::source_shared_edge_restore::restore(
            value,
            crate::source_shared_edge_restore::Limits {
                mapping_cells_per_use: 10000,
                exact_work: 100_000_000,
                driver_cells: 10000,
            },
        )
        .unwrap()
    };
    let restored = restore(edge.definition()).edge.unwrap();
    assert_eq!(restored.definition(), edge.definition());
    assert_eq!(restored.ranges(), edge.ranges());
    assert_eq!(restored.reversed(), edge.reversed());
    let mut displaced = edge.definition();
    displaced["recipe"]["candidates"][0][1][0] = value_codec::json!(0.625 + 1e-12);
    displaced["certificateQualified"] = value_codec::json!(true);
    assert!(restore(displaced).edge.is_none());

    match &edge.uses()[0].endpoints()[1] {
        Endpoint::Crossing { point, .. } => assert_eq!(point.definition(), definition),
        _ => panic!("root expression was replaced"),
    }
    assert!(
        qualify_with_candidates(
            &world,
            [&fa, &fb],
            ranges,
            [None; 2],
            [[None, Some([0.625 + 1e-12, 0.609375])], [None, None]],
            100_000_000,
            0
        )
        .unwrap()
        .edge
        .is_none()
    );
    assert!(
        crate::source_root_parameter::verify(&point, [0.6, 0.609375], cad_predicates::MAX_WORK)
            .unwrap()
            .parameters
            .is_none()
    );
    assert!(
        crate::source_root_parameter::verify(&point, [0.625, 0.609375], 1)
            .unwrap()
            .parameters
            .is_none()
    );
}
