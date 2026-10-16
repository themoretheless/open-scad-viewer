//! Original pair cover across a half-turn; no retained loft or sampled proof.
use nurbs_core::{
    progressive_sweep::{MultiSweep, Options, Orientation, Spacing, constant_vector_law},
    surface::Surface,
};
fn carrier() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![-0.125, -0.125, 0.], vec![-0.125, 0.125, 0.]],
            vec![vec![0.125, -0.125, 0.], vec![0.125, 0.125, 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn exhaustive_source_pairs_admit_a_half_turn_without_global_monotonicity() {
    let path = nurbs_core::paths::bezier(
        vec![
            vec![0., 0., 0.],
            vec![0., 0., 8.],
            vec![8., 0., 8.],
            vec![8., 0., 0.],
        ],
        None,
    )
    .unwrap();
    let profiles =
        [nurbs_core::primitives::line([-0.125, -0.125, 0.], [0.125, -0.125, 0.]).unwrap()];
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let cap = carrier();
    let original = (path.clone(), cap.clone());
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        Options {
            normal: [0., 1., 0.],
            orientation: Orientation::FixedNormal,
            spacing: Spacing::Parameter,
            initial_sections: 5,
            max_sections: 9,
            max_deviation: 0.01,
        },
    )
    .unwrap();
    let p = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    let audit = |limit| {
        sweep
            .certify_ideal_volume_atlas([&cap, &cap], p, limit, 1000000)
            .unwrap()
    };
    let full = audit(100000);
    assert!(
        full.embedding_certified && full.regularity_certified && full.parameter_topology_preserved,
        "{full:?}"
    );
    assert!(full.monotonicity_lower.is_none());
    assert!(full.boxes > 1 && full.cells <= 100000);
    assert!(audit(full.cells).embedding_certified);
    for limit in [0, full.cells - 1] {
        let refused = audit(limit);
        assert!(
            !refused.embedding_certified
                && !refused.regularity_certified
                && !refused.parameter_topology_preserved,
            "{refused:?}"
        );
        assert!(refused.cells <= limit);
    }
    // Opposite endpoint tangent/transverse columns have a singular average:
    // this source cannot satisfy any constant strong-monotonicity metric.
    assert!(
        !sweep
            .certify_ideal_volume([&cap, &cap], p, 200, 1000000)
            .unwrap()
            .embedding_certified
    );
    assert_eq!((path.clone(), cap.clone()), original);
    eprintln!("half-turn original pair cover: {full:?}");
}
#[test]
fn singular_carrier_and_singular_projection_do_not_receive_atlas_credit() {
    let path = nurbs_core::primitives::line([0.; 3], [0., 0., 8.]).unwrap();
    let profiles = [nurbs_core::primitives::line([0.; 3], [1., 0., 0.]).unwrap()];
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        Options {
            normal: [0., 1., 0.],
            orientation: Orientation::FixedNormal,
            spacing: Spacing::Parameter,
            initial_sections: 5,
            max_sections: 5,
            max_deviation: 0.01,
        },
    )
    .unwrap();
    let mut cap = carrier();
    cap.control_points[1] = cap.control_points[0].clone();
    let identity = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    for (cap, p) in [
        (&cap, identity),
        (&carrier(), [[1., 0., 0.], [0., 1., 0.], [0.; 3]]),
    ] {
        let r = sweep
            .certify_ideal_volume_atlas([cap, cap], p, 400, 1000000)
            .unwrap();
        assert!(
            !r.embedding_certified && !r.regularity_certified && !r.parameter_topology_preserved,
            "{r:?}"
        );
        assert!(r.cells <= 400);
    }
}

#[test]
fn regular_path_with_distinct_parameters_at_the_same_point_has_no_global_credit() {
    // Exact original cubic: x=192(t²-t+3/16),
    // z=192(t-1/2)(t²-t+3/16). P(1/4)=P(3/4)=0, but the
    // path velocity never vanishes: x'=0 only at 1/2, where z'=-12.
    let path = nurbs_core::paths::bezier(
        vec![
            vec![36., 0., -18.],
            vec![-28., 0., 26.],
            vec![-28., 0., -26.],
            vec![36., 0., 18.],
        ],
        None,
    )
    .unwrap();
    assert_eq!(path.evaluate(0.25).unwrap().point, vec![0.; 3]);
    assert_eq!(path.evaluate(0.75).unwrap().point, vec![0.; 3]);
    let mut cap = carrier();
    for row in &mut cap.control_points {
        for p in row {
            let u = p[0];
            *p = vec![36. + 11. / 16. * u, p[1], -18. + u];
        }
    }
    // Carrier center is the original path start; its UV center is transported
    // to both crossing points. Its transverse direction (11,0,16) is exactly
    // orthogonal to the original start tangent (-192,0,132).
    let profiles = [nurbs_core::primitives::line(
        cap.control_points[0][0].clone().try_into().unwrap(),
        cap.control_points[1][0].clone().try_into().unwrap(),
    )
    .unwrap()];
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        Options {
            normal: [0., 1., 0.],
            orientation: Orientation::FixedNormal,
            spacing: Spacing::Parameter,
            initial_sections: 5,
            max_sections: 9,
            max_deviation: 0.01,
        },
    )
    .unwrap();
    let report = sweep
        .certify_ideal_volume_atlas(
            [&cap, &cap],
            [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            4000,
            1000000,
        )
        .unwrap();
    assert!(
        !report.embedding_certified
            && !report.regularity_certified
            && !report.parameter_topology_preserved,
        "{report:?}"
    );
    assert!(report.cells <= 4000);
}

fn closed_loop() -> nurbs_core::curve::Curve {
    let quarter = [[4., 0., 0.], [4., 2., 0.], [2., 4., 0.], [0., 4., 0.]];
    let mut controls = Vec::new();
    for k in 0..4 {
        for (j, p) in quarter.iter().enumerate() {
            if k > 0 && j == 0 {
                continue;
            }
            let [x, y, z] = *p;
            controls.push(match k {
                0 => vec![x, y, z],
                1 => vec![-y, x, z],
                2 => vec![-x, -y, z],
                _ => vec![y, -x, z],
            });
        }
    }
    nurbs_core::curve::Curve {
        degree: 3,
        knots: [
            vec![0.; 4],
            vec![0.25; 3],
            vec![0.5; 3],
            vec![0.75; 3],
            vec![1.; 4],
        ]
        .concat(),
        weights: vec![1.; 13],
        control_points: controls,
        periodic: false,
    }
}
#[test]
fn exact_closed_source_uses_a_wrapped_quotient_chart_and_shared_work_limits() {
    let path = closed_loop();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let mut cap = carrier();
    for row in &mut cap.control_points {
        for p in row {
            *p = vec![4. + p[0], 0., p[1]];
        }
    }
    let profiles =
        [nurbs_core::primitives::line([3.875, 0., -0.125], [4.125, 0., -0.125]).unwrap()];
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        Options {
            normal: [0., 0., 1.],
            orientation: Orientation::FixedNormal,
            spacing: Spacing::Parameter,
            initial_sections: 5,
            max_sections: 9,
            max_deviation: 0.01,
        },
    )
    .unwrap();
    let p = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    let audit = |cells, work| {
        sweep
            .certify_ideal_volume_atlas([&cap, &cap], p, cells, work)
            .unwrap()
    };
    let full = audit(100000, 1000000);
    assert!(
        full.embedding_certified && full.regularity_certified && full.parameter_topology_preserved,
        "{full:?}"
    );
    assert!(full.monotonicity_lower.is_none() && full.exact_work > 0);
    assert!(audit(full.cells, full.exact_work).embedding_certified);
    for (cells, work) in [
        (full.cells - 1, 1000000),
        (100000, full.exact_work - 1),
        (0, 1000000),
        (100000, 0),
    ] {
        let r = audit(cells, work);
        assert!(
            !r.embedding_certified && !r.regularity_certified && !r.parameter_topology_preserved,
            "{r:?}"
        );
        assert!(r.cells <= cells && r.exact_work <= work);
        assert!(r.reason.is_some());
    }
    let mut other = cap.clone();
    other.control_points[0][0][2] += 0.001;
    assert_eq!(
        sweep
            .certify_ideal_volume_atlas([&cap, &other], p, 100000, 1000000)
            .unwrap()
            .reason,
        Some("ideal-volume-closed-common-carrier-required")
    );
    assert_eq!(
        sweep
            .certify_ideal_volume([&cap, &cap], p, 100000, 1000000)
            .unwrap()
            .reason,
        Some("ideal-volume-closed-quotient-required")
    );
    eprintln!("closed original quotient pair cover: {full:?}");
}

#[test]
fn closed_quotient_matrix_covers_authored_and_guided_frames_arc_laws_and_rational_carriers() {
    use nurbs_core::curve::Curve;
    let mut path = closed_loop();
    for u in &mut path.knots {
        *u = -3. + 8. * *u;
    }
    let axis = Curve {
        degree: 2,
        knots: [
            vec![2.; 3],
            vec![3.; 2],
            vec![4.; 2],
            vec![5.; 2],
            vec![6.; 3],
        ]
        .concat(),
        control_points: vec![
            vec![0., 24., 0.],
            vec![-24., 24., 0.],
            vec![-24., 0., 0.],
            vec![-24., -24., 0.],
            vec![0., -24., 0.],
            vec![24., -24., 0.],
            vec![24., 0., 0.],
            vec![24., 24., 0.],
            vec![0., 24., 0.],
        ],
        weights: vec![1.; 9],
        periodic: false,
    };
    let mut guide = path.clone();
    for p in &mut guide.control_points {
        p[2] = 1.;
    }
    let normal = constant_vector_law([0., 0., 1.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let axes = constant_vector_law([1., 1.125, 0.875]).unwrap();
    let center = constant_vector_law([0.03125, 0., 0.]).unwrap();
    let mut cap = carrier();
    for row in &mut cap.control_points {
        for p in row {
            *p = vec![4. + p[0], 0., p[1]];
        }
    }
    cap.weights = vec![vec![1., 1.125], vec![1.25, 1.]];
    cap.knots_u = vec![2., 2., 6., 6.];
    cap.knots_v = vec![-3., -3., 5., 5.];
    let profiles =
        [nurbs_core::primitives::line([3.875, 0., -0.125], [4.125, 0., -0.125]).unwrap()];
    let original = (path.clone(), guide.clone(), axis.clone(), cap.clone());
    for (mode, spacing) in [
        (
            0,
            Spacing::ArcLength {
                tolerance: 0.001,
                max_cells: 100000,
            },
        ),
        (1, Spacing::Parameter),
        (
            1,
            Spacing::ArcLength {
                tolerance: 0.001,
                max_cells: 100000,
            },
        ),
        (2, Spacing::Parameter),
    ] {
        let mut sweep = MultiSweep::new(
            &profiles,
            &path,
            &scale,
            &twist,
            Options {
                normal: [0., 0., 1.],
                orientation: if mode == 1 {
                    Orientation::Fixed
                } else {
                    Orientation::FixedNormal
                },
                spacing,
                initial_sections: 5,
                max_sections: 9,
                max_deviation: 0.01,
            },
        )
        .unwrap()
        .with_affine_laws(&axes, &center)
        .unwrap();
        if mode == 1 {
            sweep = sweep.with_frame_laws(&axis, &normal).unwrap();
        }
        if mode == 2 {
            sweep = sweep.with_orientation_guide(&guide).unwrap();
        }
        let audit = |cells, work| {
            sweep
                .certify_ideal_volume_atlas(
                    [&cap, &cap],
                    [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                    cells,
                    work,
                )
                .unwrap()
        };
        let r = audit(100000, 1000000);
        assert!(
            r.embedding_certified && r.regularity_certified && r.parameter_topology_preserved,
            "mode={mode}/{spacing:?}: {r:?}"
        );
        assert!(!audit(r.cells - 1, 1000000).embedding_certified);
        // A failed plane proposal can be skipped when exact work is scarce.
        // Exhaust the mandatory original frame proof, not a whole-search trace.
        let (frame_proven, frame_work) = if mode == 1 {
            let p=sweep.certify_closed_authored_frame_smoothness(1,100000,1000000).unwrap();
            (p.closed_source_frame_smoothness_certified,p.exact_work)
        } else if mode == 2 {
            let p=sweep.certify_closed_guided_frame_smoothness(1,100000,1000000).unwrap();
            (p.closed_source_frame_smoothness_certified,p.exact_work)
        } else {
            let p=sweep.certify_closed_path_frame_smoothness(1,100000,1000000).unwrap();
            (p.closed_source_frame_smoothness_certified,p.exact_work)
        };
        assert!(frame_proven && frame_work>0 && frame_work<=r.exact_work);
        let short=audit(100000,frame_work-1);
        assert!(!short.embedding_certified && !short.regularity_certified
            && !short.parameter_topology_preserved,"{short:?}");
        assert!(short.cells<=100000 && short.exact_work<frame_work);
        eprintln!("closed matrix {mode}/{spacing:?}: {r:?}");
    }
    assert_eq!(
        (path.clone(), guide.clone(), axis.clone(), cap.clone()),
        original
    );
}

#[test]
fn near_closed_source_and_locally_regular_double_cover_do_not_receive_quotient_credit() {
    let mut path = closed_loop();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let profiles =
        [nurbs_core::primitives::line([3.875, 0., -0.125], [4.125, 0., -0.125]).unwrap()];
    let mut cap = carrier();
    for row in &mut cap.control_points {
        for p in row {
            *p = vec![4. + p[0], 0., p[1]];
        }
    }
    let options = Options {
        normal: [0., 0., 1.],
        orientation: Orientation::FixedNormal,
        spacing: Spacing::Parameter,
        initial_sections: 5,
        max_sections: 9,
        max_deviation: 0.01,
    };
    let p = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    path.control_points.last_mut().unwrap()[0] = 4_f64.next_up();
    let near = MultiSweep::new(&profiles, &path, &scale, &twist, options).unwrap();
    assert!(near.source_is_closed().unwrap());
    let r = near
        .certify_ideal_volume_atlas([&cap, &cap], p, 100000, 1000000)
        .unwrap();
    assert!(!r.embedding_certified && !r.regularity_certified && !r.parameter_topology_preserved);
    assert_eq!(r.reason, Some("ideal-volume-exact-source-seam-unproved"));
    let mut twice = closed_loop();
    twice
        .control_points
        .extend(closed_loop().control_points.into_iter().skip(1));
    twice.weights = vec![1.; 25];
    twice.knots = vec![0.; 4];
    for i in 1..8 {
        twice.knots.extend(vec![i as f64 / 8.; 3]);
    }
    twice.knots.extend(vec![1.; 4]);
    assert_eq!(
        twice.evaluate(0.).unwrap().point,
        twice.evaluate(0.5).unwrap().point
    );
    assert!(
        nurbs_core::curve_regularity::inspect(&twice, 100000)
            .unwrap()
            .spanwise_regular
    );
    let double = MultiSweep::new(&profiles, &twice, &scale, &twist, options).unwrap();
    assert!(
        double
            .certify_closed_path_frame_smoothness(1, 100000, 1000000)
            .unwrap()
            .closed_source_frame_smoothness_certified
    );
    let r = double
        .certify_ideal_volume_atlas([&cap, &cap], p, 5000, 1000000)
        .unwrap();
    assert!(
        !r.embedding_certified && !r.regularity_certified && !r.parameter_topology_preserved,
        "{r:?}"
    );
    assert!(r.cells <= 5000 && r.reason.is_some());
}

#[test]
fn open_spatial_atlas_covers_original_arc_bishop_jets_and_distinct_moving_carriers() {
    let mut path = nurbs_core::paths::bezier(
        vec![
            vec![0.; 3],
            vec![0., 0., 16.],
            vec![0.125, 0., 48.],
            vec![0.25, 0.125, 64.],
        ],
        Some(vec![1., 1.125, 1.25, 1.]),
    )
    .unwrap();
    for u in &mut path.knots {
        *u = -3. + 8. * *u;
    }
    let a = carrier();
    let mut b = a.clone();
    b.weights = vec![vec![1., 1.125], vec![1.25, 1.]];
    b.knots_u = vec![2., 2., 6., 6.];
    b.knots_v = vec![-3., -3., 5., 5.];
    for (i, row) in b.control_points.iter_mut().enumerate() {
        for (j, p) in row.iter_mut().enumerate() {
            p[2] = (1. + (i * j) as f64) / 256.;
        }
    }
    let profiles =
        [nurbs_core::primitives::line([-0.125, -0.125, 0.], [0.125, -0.125, 0.]).unwrap()];
    let scale = nurbs_core::primitives::line([1., 0., 0.], [1.125, 0., 0.]).unwrap();
    let twist = nurbs_core::primitives::line([0.; 3], [0.125, 0., 0.]).unwrap();
    let axes = constant_vector_law([1., 1.125, 0.875]).unwrap();
    let center = nurbs_core::primitives::line([0.; 3], [0.125, 0., 0.]).unwrap();
    let before = (path.clone(), a.clone(), b.clone());
    for spacing in [
        Spacing::Parameter,
        Spacing::ArcLength {
            tolerance: 0.001,
            max_cells: 100000,
        },
    ] {
        let sweep = MultiSweep::new(
            &profiles,
            &path,
            &scale,
            &twist,
            Options {
                normal: [1., 0., 0.],
                orientation: Orientation::RotationMinimizing,
                spacing,
                initial_sections: 5,
                max_sections: 9,
                max_deviation: 0.01,
            },
        )
        .unwrap()
        .with_affine_laws(&axes, &center)
        .unwrap()
        .with_spatial_rmf_error_limits(128, 100000, 1000000)
        .unwrap();
        let audit = |cells| {
            sweep
                .certify_ideal_volume_atlas(
                    [&a, &b],
                    [[1., 0., 0.], [0., 1., 0.], [0., 0., 1. / 64.]],
                    cells,
                    1000000,
                )
                .unwrap()
        };
        let r = audit(100000);
        assert!(
            r.embedding_certified && r.regularity_certified && r.parameter_topology_preserved,
            "{spacing:?}: {r:?}"
        );
        assert!(!audit(r.cells - 1).embedding_certified);
        assert!(audit(r.cells).embedding_certified);
        eprintln!("spatial moving carrier {spacing:?}: {r:?}");
    }
    assert_eq!((path, a, b), before);
}

#[test]
fn original_closed_spatial_bishop_field_is_c1_c2_with_periodic_holonomy_rates() {
    use nurbs_core::{curve::Curve, progressive_sweep::Sweep};
    let path = Curve {
        degree: 7,
        knots: [vec![2.; 8], vec![5.; 8]].concat(),
        control_points: vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![2., 1., 0.],
            vec![3., 3., 1.],
            vec![-3., 3., -1.],
            vec![-2., 1., 0.],
            vec![-1., 0., 0.],
            vec![0., 0., 0.],
        ],
        weights: vec![1.; 8],
        periodic: false,
    };
    let twist = Curve {
        degree: 5,
        knots: [vec![7.; 6], vec![9.; 6]].concat(),
        control_points: [0., 0.125, 0.25, -0.25, -0.125, 0.]
            .into_iter()
            .map(|x| vec![x, 0., 0.])
            .collect(),
        weights: vec![1.; 6],
        periodic: false,
    };
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let profile = nurbs_core::primitives::line([0., 0., 0.125], [0., 0., 0.25]).unwrap();
    let original = (path.clone(), twist.clone());
    for spacing in [
        Spacing::Parameter,
        Spacing::ArcLength {
            tolerance: 0.001,
            max_cells: 100000,
        },
    ] {
        let options = Options {
            normal: [0., 0., 1.],
            orientation: Orientation::RotationMinimizing,
            spacing,
            initial_sections: 5,
            max_sections: 17,
            max_deviation: 0.01,
        };
        let base = || Sweep::new(&profile, &path, &scale, &twist, options).unwrap();
        let sweep = base()
            .with_spatial_rmf_error_limits(128, 100000, 1000000)
            .unwrap();
        for order in [1, 2] {
            let audit = |cells, work| {
                sweep
                    .certify_closed_path_frame_smoothness(order, cells, work)
                    .unwrap()
            };
            let r = audit(100000, 1000000);
            assert!(
                r.closed_source_frame_smoothness_certified,
                "{spacing:?}/{order}: {r:?}"
            );
            assert!(audit(r.cells, r.exact_work).closed_source_frame_smoothness_certified);
            for (cells, work) in [
                (r.cells - 1, 1000000),
                (100000, r.exact_work - 1),
                (0, 1000000),
                (100000, 0),
            ] {
                let short = audit(cells, work);
                assert!(!short.closed_source_frame_smoothness_certified, "{short:?}");
                assert!(short.cells <= cells && short.exact_work <= work);
            }
            // Existing nonspatial policy still requires its own planar/line
            // owner; the original spatial ODE premise cannot be borrowed.
            assert!(
                !base()
                    .certify_closed_path_frame_smoothness(order, 100000, 1000000)
                    .unwrap()
                    .closed_source_frame_smoothness_certified
            );
        }
    }
    let mut damaged = path.clone();
    damaged.control_points[3][2] = 1_f64.next_up();
    let options = Options {
        normal: [0., 0., 1.],
        orientation: Orientation::RotationMinimizing,
        spacing: Spacing::Parameter,
        initial_sections: 5,
        max_sections: 17,
        max_deviation: 0.01,
    };
    let damaged = Sweep::new(&profile, &damaged, &scale, &twist, options)
        .unwrap()
        .with_spatial_rmf_error_limits(128, 100000, 1000000)
        .unwrap();
    assert!(
        damaged
            .certify_closed_path_frame_smoothness(1, 100000, 1000000)
            .unwrap()
            .closed_source_frame_smoothness_certified
    );
    assert!(
        !damaged
            .certify_closed_path_frame_smoothness(2, 100000, 1000000)
            .unwrap()
            .closed_source_frame_smoothness_certified
    );
    let parallel = Sweep::new(
        &profile,
        &path,
        &scale,
        &twist,
        Options {
            normal: [1., 0., 0.],
            ..options
        },
    )
    .unwrap()
    .with_spatial_rmf_error_limits(128, 100000, 1000000)
    .unwrap();
    assert!(
        !parallel
            .certify_closed_path_frame_smoothness(1, 100000, 1000000)
            .unwrap()
            .closed_source_frame_smoothness_certified
    );
    assert_eq!((path, twist), original);
}
