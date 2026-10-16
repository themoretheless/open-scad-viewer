//! Original closed spatial Bishop source volume, including its seam quotient.
use nurbs_core::{
    curve::Curve,
    progressive_sweep::{MultiSweep, Options, Orientation, Spacing, constant_vector_law},
    surface::Surface,
};
fn path(rational: bool) -> Curve {
    Curve {
        degree: 9,
        knots: [vec![-3.; 10], vec![5.; 10]].concat(),
        control_points: vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![2., 1., 0.],
            vec![3., 3., 1.],
            vec![2., 4., 6.],
            vec![-2., 4., -4.],
            vec![-3., 3., -1.],
            vec![-2., 1., 0.],
            vec![-1., 0., 0.],
            vec![0., 0., 0.],
        ],
        weights: if rational {
            vec![1., 1., 1., 1., 1.25, 1.125, 1., 1., 1., 1.]
        } else {
            vec![1.; 10]
        },
        periodic: false,
    }
}
fn cap() -> Surface {
    let r = 1. / 128.;
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-1., -1., 3., 3.],
        knots_v: vec![2., 2., 6., 6.],
        control_points: vec![
            vec![vec![0., -r, -r], vec![0., -r, r]],
            vec![vec![0., r, -r], vec![0., r, r]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn corrected_closed_spatial_original_volume_has_global_quotient_embedding() {
    let path = path(false);
    let cap = cap();
    let r = 1. / 128.;
    let profiles = [nurbs_core::primitives::line([0., -r, -r], [0., r, -r]).unwrap()];
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        Options {
            normal: [0., 0., 1.],
            orientation: Orientation::RotationMinimizing,
            spacing: Spacing::Parameter,
            initial_sections: 5,
            max_sections: 17,
            max_deviation: 0.01,
        },
    )
    .unwrap()
    .with_spatial_rmf_error_limits(1024, 100000, 1000000)
    .unwrap();
    let projection = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    assert!(
        sweep
            .certify_ideal_volume_atlas([&cap, &cap], projection, 1000001, 1000000)
            .is_err()
    );
    assert!(
        sweep
            .certify_ideal_volume([&cap, &cap], projection, 100001, 1000000)
            .is_err()
    );
    let p = sweep
        .certify_ideal_volume_atlas([&cap, &cap], projection, 100000, 1000000)
        .unwrap();
    eprintln!("closed spatial original: {p:?}");
    assert!(
        p.embedding_certified && p.regularity_certified && p.parameter_topology_preserved,
        "{p:?}"
    );
    assert!(p.monotonicity_lower.is_none());
    let short = sweep
        .certify_ideal_volume_atlas([&cap, &cap], projection, p.cells - 1, 1000000)
        .unwrap();
    assert!(
        !short.embedding_certified
            && !short.regularity_certified
            && !short.parameter_topology_preserved
    );
}

#[test]
fn closed_spatial_matrix_preserves_rational_caps_affine_laws_and_parameter_arc_units() {
    let proof_cells = 300000;
    let r = 1. / 128.;
    let profiles = [nurbs_core::primitives::line([0., -r, -r], [0., r, -r]).unwrap()];
    let scalar = |points: [f64; 5], domain: [f64; 2]| Curve {
        degree: 4,
        knots: [vec![domain[0]; 5], vec![domain[1]; 5]].concat(),
        control_points: points.map(|x| vec![x, 0., 0.]).to_vec(),
        weights: vec![1.; 5],
        periodic: false,
    };
    let scale = scalar([1., 1., 1.125, 1., 1.], [2., 6.]);
    let twist = scalar([0., 0., 1. / 64., 0., 0.], [7., 11.]);
    let axes = constant_vector_law([1., 1.125, 0.875]).unwrap();
    let center = constant_vector_law([1. / 1024., 1. / 2048., 0.]).unwrap();
    let mut cap = cap();
    cap.weights = vec![vec![1., 1.125], vec![1.25, 1.]];
    cap.control_points[1][1][0] = 1. / 65536.;
    for rational in [false, true] {
        let path = path(rational);
        let original = (path.clone(), cap.clone());
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
                    normal: [0., 0., 1.],
                    orientation: Orientation::RotationMinimizing,
                    spacing,
                    initial_sections: 5,
                    max_sections: 17,
                    max_deviation: 0.01,
                },
            )
            .unwrap()
            .with_affine_laws(&axes, &center)
            .unwrap()
            .with_spatial_rmf_error_limits(2048, 100000, 1000000)
            .unwrap();
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
            let p = audit(proof_cells, 1000000);
            eprintln!("closed spatial rational={rational}/{spacing:?}: {p:?}");
            assert!(
                p.embedding_certified && p.regularity_certified && p.parameter_topology_preserved,
                "{p:?}"
            );
            assert!(audit(p.cells, p.exact_work).embedding_certified);
            for (cells, work) in [
                (p.cells - 1, 1000000),
                (proof_cells, p.exact_work - 1),
                (0, 1000000),
                (proof_cells, 0),
            ] {
                let short = audit(cells, work);
                assert!(
                    !short.embedding_certified
                        && !short.regularity_certified
                        && !short.parameter_topology_preserved,
                    "{short:?}"
                );
                assert!(short.cells <= cells && short.exact_work <= work && short.reason.is_some());
            }
        }
        assert_eq!((path.clone(), cap.clone()), original);
    }
}

#[test]
fn repeated_spatial_loop_has_no_global_credit_despite_regular_closed_bishop_field() {
    let mut twice = path(false);
    twice
        .control_points
        .extend(path(false).control_points.into_iter().skip(1));
    twice.weights = vec![1.; 19];
    twice.knots = [vec![0.; 10], vec![0.5; 9], vec![1.; 10]].concat();
    assert_eq!(twice.evaluate(0.).unwrap().point, vec![0.; 3]);
    assert_eq!(twice.evaluate(0.5).unwrap().point, vec![0.; 3]);
    let cap = cap();
    let r = 1. / 128.;
    let profiles = [nurbs_core::primitives::line([0., -r, -r], [0., r, -r]).unwrap()];
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &twice,
        &scale,
        &twist,
        Options {
            normal: [0., 0., 1.],
            orientation: Orientation::RotationMinimizing,
            spacing: Spacing::Parameter,
            initial_sections: 5,
            max_sections: 17,
            max_deviation: 0.01,
        },
    )
    .unwrap()
    .with_spatial_rmf_error_limits(1024, 100000, 1000000)
    .unwrap();
    assert!(
        sweep
            .certify_closed_path_frame_smoothness(2, 100000, 1000000)
            .unwrap()
            .closed_source_frame_smoothness_certified
    );
    let transport = nurbs_core::progressive_sweep::certify_original_rmf_transport(
        &twice,
        [0., 0., 1.],
        1024,
        100000,
        1000000,
        true,
    )
    .unwrap();
    assert!(transport.cover.is_some() && transport.arc_corrected_normals.is_some());
    let p = sweep
        .certify_ideal_volume_atlas(
            [&cap, &cap],
            [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            20000,
            1000000,
        )
        .unwrap();
    assert!(
        !p.embedding_certified && !p.regularity_certified && !p.parameter_topology_preserved,
        "{p:?}"
    );
    assert!(p.cells <= 20000 && p.reason.is_some());
}
