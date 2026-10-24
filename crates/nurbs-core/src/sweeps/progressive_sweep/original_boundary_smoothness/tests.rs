use super::*;
fn path(spatial: bool) -> Curve {
    let mut p = vec![
        [0., 0., 0.],
        [1., 0., 0.],
        [2., 1., 0.],
        [3., 3., 1.],
        [2., 4., 6.],
        [-2., 4., -4.],
        [-3., 3., -1.],
        [-2., 1., 0.],
        [-1., 0., 0.],
        [0., 0., 0.],
    ];
    if !spatial {
        for p in &mut p {
            p[2] = 0.;
        }
    }
    Curve {
        degree: 9,
        knots: [vec![-3.; 10], vec![5.; 10]].concat(),
        control_points: p.iter().map(|p| p.to_vec()).collect(),
        weights: vec![1.; 10],
        periodic: false,
    }
}
fn profiles() -> Vec<Curve> {
    let p = [
        [1., 0.],
        [1., 1.],
        [0., 1.],
        [-1., 1.],
        [-1., 0.],
        [-1., -1.],
        [0., -1.],
        [1., -1.],
        [1., 0.],
    ];
    (0..4)
        .map(|i| Curve {
            degree: 2,
            knots: vec![7., 7., 7., 11., 11., 11.],
            control_points: p[2 * i..=2 * i + 2]
                .iter()
                .map(|p| vec![0., p[0] / 128., p[1] / 128.])
                .collect(),
            weights: vec![1., 0.5, 1.],
            periodic: false,
        })
        .collect()
}
fn limits(order: usize) -> OriginalBoundarySmoothnessLimits {
    OriginalBoundarySmoothnessLimits {
        order,
        cells: 100000,
        exact_work: 1000000,
        products: 1000000,
    }
}
fn options(mode: usize, spacing: Spacing) -> Options {
    Options {
        normal: [0., 0., 1.],
        orientation: if mode == 1 {
            Orientation::Fixed
        } else {
            Orientation::RotationMinimizing
        },
        spacing,
        initial_sections: 5,
        max_sections: 129,
        max_deviation: 1.,
    }
}
#[test]
fn original_closed_profiles_laws_and_frame_own_all_periodic_differentials() {
    for mode in 0..3 {
        for spacing in [
            Spacing::Parameter,
            Spacing::ArcLength {
                tolerance: 0.001,
                max_cells: 100000,
            },
        ] {
            let p = path(spacing == Spacing::Parameter);
            let profiles = profiles();
            let scale = constant_vector_law([1., 0., 0.]).unwrap();
            let twist = constant_vector_law([0.; 3]).unwrap();
            let axes = constant_vector_law([1., 1.125, 0.875]).unwrap();
            let center = constant_vector_law([1. / 1024., 0., 0.]).unwrap();
            let axis = Curve {
                degree: 8,
                knots: [vec![2.; 9], vec![6.; 9]].concat(),
                weights: vec![1.; 9],
                periodic: false,
                control_points: p
                    .control_points
                    .windows(2)
                    .map(|p| (0..3).map(|k| 9. * (p[1][k] - p[0][k])).collect())
                    .collect(),
            };
            let normal = constant_vector_law([0., 0., 1.]).unwrap();
            let mut guide = p.clone();
            for p in &mut guide.control_points {
                p[2] += 1.;
            }
            let mut s = MultiSweep::new(&profiles, &p, &scale, &twist, options(mode, spacing))
                .unwrap()
                .with_affine_laws(&axes, &center)
                .unwrap();
            if mode == 0 {
                s = s
                    .with_spatial_rmf_error_limits(1024, 100000, 1000000)
                    .unwrap();
            }
            if mode == 1 {
                s = s.with_frame_laws(&axis, &normal).unwrap();
            }
            if mode == 2 {
                s = s.with_orientation_guide(&guide).unwrap();
            }
            let proof = s
                .certify_original_closed_boundary_smoothness(&[4], limits(2))
                .unwrap();
            eprintln!("original G2 differentials {mode}/{spacing:?}: {proof:?}");
            assert!(
                proof.differential_compatibility_certified,
                "{mode}/{spacing:?}: {proof:?}"
            );
            assert_eq!(proof.inspected_profile_joins, 4);
            for stage in 0..6 {
                let mut l = limits(2);
                match stage {
                    0 => l.cells = 0,
                    1 => l.cells = proof.cells - 1,
                    2 => l.exact_work = 0,
                    3 => l.exact_work = proof.exact_work - 1,
                    4 => l.products = 0,
                    _ => l.products = proof.products - 1,
                }
                let short = s
                    .certify_original_closed_boundary_smoothness(&[4], l)
                    .unwrap();
                assert!(
                    !short.differential_compatibility_certified,
                    "{mode}/{spacing:?}/{stage}: {short:?}"
                );
                assert!(
                    short.cells <= l.cells
                        && short.exact_work <= l.exact_work
                        && short.products <= l.products
                );
            }
        }
    }
}
#[test]
fn original_source_g1_does_not_hide_curve_curvature_or_motion_jet_jumps() {
    let p = path(true);
    let mut profiles = profiles();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    profiles[0].weights[1] = 0.75;
    let audit = |profiles: &[Curve], twist: &Curve, order| {
        MultiSweep::new(profiles, &p, &scale, twist, options(0, Spacing::Parameter))
            .unwrap()
            .with_spatial_rmf_error_limits(1024, 100000, 1000000)
            .unwrap()
            .certify_original_closed_boundary_smoothness(&[4], limits(order))
            .unwrap()
    };
    assert!(audit(&profiles, &twist, 1).differential_compatibility_certified);
    assert!(!audit(&profiles, &twist, 2).differential_compatibility_certified);
    let saved = profiles.clone();
    profiles[0].control_points[1][1] = (1. / 128_f64).next_up();
    assert!(!audit(&profiles, &twist, 1).differential_compatibility_certified);
    profiles = saved;
    profiles[0].weights[1] = 0.5;
    let twist = Curve {
        degree: 5,
        knots: [vec![0.; 6], vec![1.; 6]].concat(),
        control_points: [0., 0., 1. / 64., 0., 0., 0.]
            .map(|x| vec![x, 0., 0.])
            .to_vec(),
        weights: vec![1.; 6],
        periodic: false,
    };
    assert!(audit(&profiles, &twist, 1).differential_compatibility_certified);
    assert!(!audit(&profiles, &twist, 2).differential_compatibility_certified);
}
#[test]
fn original_source_smoothness_checks_partition_and_limits() {
    let p = path(true);
    let profiles = profiles();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let s = MultiSweep::new(
        &profiles,
        &p,
        &scale,
        &twist,
        options(0, Spacing::Parameter),
    )
    .unwrap();
    for partition in [vec![], vec![0, 4], vec![3], vec![65]] {
        assert!(
            s.certify_original_closed_boundary_smoothness(&partition, limits(2))
                .is_err()
        );
    }
    let mut l = limits(2);
    l.exact_work = 1000001;
    assert!(
        s.certify_original_closed_boundary_smoothness(&[4], l)
            .is_err()
    );
    l = limits(3);
    assert!(
        s.certify_original_closed_boundary_smoothness(&[4], l)
            .is_err()
    );
}
