//! Original spatial source volume; the retained station count is not its proof.
use nurbs_core::{
    curve::Curve,
    progressive_sweep::{MultiSweep, Options, Orientation, Spacing, constant_vector_law},
    surface::Surface,
};
fn source_path(rational: bool, reparameterized: bool) -> Curve {
    let mut p = nurbs_core::paths::bezier(
        vec![
            vec![0., 0., 0.],
            vec![0., 0., 16.],
            vec![0.125, 0., 48.],
            vec![0.25, 0.125, 64.],
        ],
        Some(if rational {
            vec![1., 1.125, 1.25, 1.]
        } else {
            vec![1.; 4]
        }),
    )
    .unwrap();
    if reparameterized {
        for u in &mut p.knots {
            *u = -3. + 10. * *u;
        }
    }
    p
}
fn carrier() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 1. / 64.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn original_spatial_bishop_volume_with_filled_caps_affine_laws_and_original_parameter_units() {
    let profiles = [
        nurbs_core::primitives::line([0.; 3], [1., 0., 0.]).unwrap(),
        nurbs_core::primitives::line([1., 0., 0.], [1., 1., 1. / 64.]).unwrap(),
    ];
    let scale = nurbs_core::primitives::line([1., 0., 0.], [1.125, 0., 0.]).unwrap();
    let twist = nurbs_core::primitives::line([0.; 3], [0.125, 0., 0.]).unwrap();
    let axes = constant_vector_law([1., 1.125, 0.875]).unwrap();
    let center = nurbs_core::primitives::line([0.; 3], [0.125, 0., 0.]).unwrap();
    let cap = carrier();
    for rational in [false, true] {
        for remap in [false, true] {
            for spacing in [
                Spacing::Parameter,
                Spacing::ArcLength {
                    tolerance: 0.001,
                    max_cells: 100000,
                },
            ] {
                let path = source_path(rational, remap);
                let original = path.clone();
                let options = Options {
                    normal: [1., 0., 0.],
                    orientation: Orientation::RotationMinimizing,
                    spacing,
                    initial_sections: 5,
                    max_sections: 9,
                    max_deviation: 0.01,
                };
                let sweep = MultiSweep::new(&profiles, &path, &scale, &twist, options)
                    .unwrap()
                    .with_affine_laws(&axes, &center)
                    .unwrap()
                    .with_spatial_rmf_error_limits(128, 100000, 1000000)
                    .unwrap();
                let projection = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1. / 64.]];
                let report = sweep
                    .certify_ideal_volume([&cap, &cap], projection, 100000, 1000000)
                    .unwrap();
                assert!(
                    report.embedding_certified
                        && report.regularity_certified
                        && report.parameter_topology_preserved,
                    "rational={rational}, remap={remap}, spacing={spacing:?}: {report:?}"
                );
                assert!(report.monotonicity_lower.unwrap() > 0.);
                assert!(report.cells <= 100000 && report.exact_work <= 1000000);
                assert!(
                    !sweep
                        .certify_ideal_volume([&cap, &cap], projection, report.cells - 1, 1000000)
                        .unwrap()
                        .embedding_certified
                );
                assert!(
                    !sweep
                        .certify_ideal_volume([&cap, &cap], projection, 0, 1000000)
                        .unwrap()
                        .embedding_certified
                );
                let mut singular_projection = projection;
                singular_projection[2] = [0.; 3];
                assert!(
                    !sweep
                        .certify_ideal_volume([&cap, &cap], singular_projection, 1000, 1000000)
                        .unwrap()
                        .embedding_certified
                );
                assert_eq!(path, original);
            }
        }
    }
}
#[test]
fn unresolved_original_spatial_transport_never_supplies_partial_volume_credit() {
    let path = source_path(true, false);
    let cap = carrier();
    let profiles = [nurbs_core::primitives::line([0.; 3], [1., 0., 0.]).unwrap()];
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let options = Options {
        normal: [1., 0., 0.],
        orientation: Orientation::RotationMinimizing,
        spacing: Spacing::Parameter,
        initial_sections: 5,
        max_sections: 5,
        max_deviation: 0.01,
    };
    let sweep = MultiSweep::new(&profiles, &path, &scale, &twist, options)
        .unwrap()
        .with_spatial_rmf_error_limits(128, 100000, 1000000)
        .unwrap();
    let p = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1. / 64.]];
    let report = sweep
        .certify_ideal_volume([&cap, &cap], p, 20, 1000000)
        .unwrap();
    assert!(
        !report.embedding_certified
            && !report.regularity_certified
            && !report.parameter_topology_preserved
    );
    assert!(report.monotonicity_lower.is_none());
    assert!(report.cells <= 20);
}

#[test]
fn original_arc_volume_covers_authored_fixed_fixed_normal_frenet_and_planar_rmf_frames() {
    let profiles = [nurbs_core::primitives::line([0.; 3], [1., 0., 0.]).unwrap()];
    let cap = carrier();
    let path = nurbs_core::paths::bezier(
        vec![
            vec![0.; 3],
            vec![0., 0., 64. / 3.],
            vec![0.125 / 3., 0., 128. / 3.],
            vec![0.125, 0., 64.],
        ],
        None,
    )
    .unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let axis = constant_vector_law([0., 0., 1.]).unwrap();
    let normal = nurbs_core::primitives::line([1., 0., 0.], [1., 0.125, 0.]).unwrap();
    for orientation in [
        Orientation::Fixed,
        Orientation::FixedNormal,
        Orientation::Frenet,
        Orientation::RotationMinimizing,
    ] {
        for authored in [false, true] {
            if authored && orientation != Orientation::Fixed {
                continue;
            }
            let o = Options {
                normal: [0., 1., 0.],
                orientation,
                spacing: Spacing::ArcLength {
                    tolerance: 0.001,
                    max_cells: 100000,
                },
                initial_sections: 5,
                max_sections: 9,
                max_deviation: 0.01,
            };
            let mut sweep = MultiSweep::new(&profiles, &path, &scale, &twist, o).unwrap();
            if authored {
                sweep = sweep.with_frame_laws(&axis, &normal).unwrap();
            }
            let p = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1. / 64.]];
            let report = sweep
                .certify_ideal_volume([&cap, &cap], p, 100000, 1000000)
                .unwrap();
            assert!(
                report.embedding_certified,
                "{orientation:?}, authored={authored}: {report:?}"
            );
            assert!(
                !sweep
                    .certify_ideal_volume([&cap, &cap], p, report.cells - 1, 1000000)
                    .unwrap()
                    .embedding_certified
            );
        }
    }
}
#[test]
fn original_arc_guides_and_contact_keep_exact_phase_identity_and_owned_anchor() {
    let profiles = [nurbs_core::primitives::line([0.; 3], [1., 0., 0.]).unwrap()];
    let cap = carrier();
    let path = nurbs_core::paths::bezier(
        vec![
            vec![0.; 3],
            vec![0., 0., 16.],
            vec![0., 0., 48.],
            vec![0., 0., 64.],
        ],
        Some(vec![1., 1.125, 1.25, 1.]),
    )
    .unwrap();
    let mut guide = path.clone();
    for p in &mut guide.control_points {
        p[0] += 1.;
    }
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let options = Options {
        normal: [1., 0., 0.],
        orientation: Orientation::RotationMinimizing,
        spacing: Spacing::ArcLength {
            tolerance: 0.001,
            max_cells: 100000,
        },
        initial_sections: 5,
        max_sections: 9,
        max_deviation: 0.01,
    };
    for contact in [false, true] {
        let sweep = MultiSweep::new(&profiles, &path, &scale, &twist, options).unwrap();
        let sweep = if contact {
            sweep.with_contact_guide(&guide, 0, 1.).unwrap()
        } else {
            sweep.with_orientation_guide(&guide).unwrap()
        };
        let p = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1. / 64.]];
        let report = sweep
            .certify_ideal_volume([&cap, &cap], p, 100000, 1000000)
            .unwrap();
        assert!(report.embedding_certified, "contact={contact}: {report:?}");
        assert!(
            !sweep
                .certify_ideal_volume([&cap, &cap], p, report.cells - 1, 1000000)
                .unwrap()
                .embedding_certified
        );
    }
}

#[test]
fn curved_guides_with_different_rational_weights_use_independent_original_arc_maps() {
    let path = source_path(false, true);
    let mut guide = source_path(true, false);
    for p in &mut guide.control_points {
        p[0] += 1.;
    }
    for u in &mut guide.knots {
        *u = 11. + 8. * *u;
    }
    let original = guide.clone();
    let profiles = [nurbs_core::primitives::line([0.; 3], [1., 0., 0.]).unwrap()];
    let cap = carrier();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let options = Options {
        normal: [1., 0., 0.],
        orientation: Orientation::RotationMinimizing,
        spacing: Spacing::ArcLength {
            tolerance: 0.001,
            max_cells: 100000,
        },
        initial_sections: 5,
        max_sections: 9,
        max_deviation: 0.01,
    };
    let sweep = MultiSweep::new(&profiles, &path, &scale, &twist, options)
        .unwrap()
        .with_orientation_guide(&guide)
        .unwrap();
    let p = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1. / 64.]];
    let report = sweep
        .certify_ideal_volume([&cap, &cap], p, 100000, 1000000)
        .unwrap();
    assert!(report.embedding_certified, "{report:?}");
    assert!(
        !sweep
            .certify_ideal_volume([&cap, &cap], p, report.cells - 1, 1000000)
            .unwrap()
            .embedding_certified
    );
    assert_eq!(guide, original);
}
