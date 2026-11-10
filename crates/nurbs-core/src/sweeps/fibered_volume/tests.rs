use super::*;
fn carrier() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 7., 7.],
        knots_v: vec![-3., -3., 5., 5.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn proposal() -> Proposal {
    Proposal {
        source_normal: [0., 0., 1.],
        world_rows: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
    }
}
fn limits() -> Limits {
    Limits {
        cells: 10000,
        exact_work: 1000000,
    }
}
fn motion() -> Vec<RationalPiece> {
    vec![RationalPiece {
        controls: [1., 1.25, 1.5, 1.5, 1.25, 1.]
            .into_iter()
            .zip([0., 0., 0., 1., 1., 1.])
            .map(|(a, z)| [[a, 0., 0.], [0., a, 0.], [0.; 3], [0., 0., z]])
            .collect(),
        weights: vec![1.; 6],
    }]
}
#[test]
fn original_filled_volume_is_globally_embedded_despite_singular_end_parameter_jacobians() {
    let q = carrier();
    let pieces = motion();
    let before = (q.clone(), pieces[0].controls.clone());
    let r = certify(&q, [0.; 3], &pieces, proposal(), limits()).unwrap();
    assert!(
        r.embedding_certified && r.parameter_topology_preserved && r.spanwise_interior_regular,
        "{r:?}"
    );
    assert_eq!(r.height_direction, Some(1));
    assert_eq!(r.endpoint_height_flat_orders, Some([2, 2]));
    assert_eq!(r.known_singular_parameter_ends, [true; 2]);
    assert_eq!((q, pieces[0].controls.clone()), before);
    // The older strict cube owner must keep its stronger regularity contract.
    let strict = crate::affine_volume::certify_rational(
        &before.0,
        [0.; 3],
        &pieces,
        false,
        proposal().world_rows,
        100,
        1000000,
    )
    .unwrap();
    assert!(
        !strict.regularity_certified,
        "singular closed-cube parameterization got strict regularity"
    );
    eprintln!(
        "original flat-end volume exact work={} cells={} endpoint orders={:?}",
        r.exact_work, r.cells, r.endpoint_height_flat_orders
    );
}
#[test]
fn oblique_material_world_planes_and_original_nonuniform_rational_weights_are_supported() {
    let mut q = carrier();
    for row in &mut q.control_points {
        for x in row {
            x[0] += 2.;
            x[1] += 3.;
            x[2] = -x[0];
        }
    }
    let mut pieces = motion();
    for (pose, b) in pieces[0]
        .controls
        .iter_mut()
        .zip([0., 0.25, 0.5, 0.5, 0.25, 0.])
    {
        pose[0][2] = pose[0][0];
        pose[3][0] = b;
        pose[3][2] += b;
    }
    pieces[0].weights = vec![1., 0.3, 2., 0.7, 3., 2.];
    let p = Proposal {
        source_normal: [0.7, 0., 0.7],
        world_rows: [[1., 0., 0.], [0., 1., 0.], [-0.3, 0., 0.3]],
    };
    let r = certify(&q, [2., 3., -2.], &pieces, p, limits()).unwrap();
    assert!(r.embedding_certified, "{r:?}");
    assert_eq!(r.endpoint_height_flat_orders, Some([2, 2]));
    for pose in &mut pieces[0].controls {
        pose[3][0] = -pose[3][0];
        pose[3][2] = -pose[3][2];
    }
    let reversed = certify(&q, [2., 3., -2.], &pieces, p, limits()).unwrap();
    assert!(reversed.embedding_certified, "{reversed:?}");
    assert_eq!(reversed.height_direction, Some(-1));
}
#[test]
fn curved_rational_frames_need_complete_rank_covers_and_respect_shared_cell_work() {
    // Binary-rational quarter rotation: cos/sin use [1,1,2] weights.
    let mut p = RationalPiece {
        controls: Vec::new(),
        weights: vec![1., 1., 2.],
    };
    for (c, s, z) in [(1., 0., 0.), (1., 1., 0.), (0., 1., 1.)] {
        p.controls
            .push([[c, s, 0.], [-s, c, 0.], [0.; 3], [0., 0., z]]);
    }
    let r = certify(&carrier(), [0.; 3], &[p.clone()], proposal(), limits()).unwrap();
    assert!(r.embedding_certified, "{r:?}");
    assert!(r.frames[0].cells > 1);
    assert_eq!(r.endpoint_height_flat_orders, Some([1, 0]));
    for cells in [0, r.cells - 1] {
        let short = certify(
            &carrier(),
            [0.; 3],
            &[p.clone()],
            proposal(),
            Limits { cells, ..limits() },
        )
        .unwrap();
        assert!(
            !short.embedding_certified
                && !short.parameter_topology_preserved
                && short.cells <= cells
        );
    }
}
#[test]
fn folds_frame_collapses_nonplanarity_and_disconnected_motion_never_get_global_credit() {
    for case in 0..5 {
        let mut q = carrier();
        let mut pieces = motion();
        match case {
            0 => {
                q.degree_u = 3;
                q.knots_u = vec![2., 2., 2., 2., 7., 7., 7., 7.];
                q.control_points = [0., 2., -1., 1.]
                    .into_iter()
                    .map(|x| vec![vec![x, 0., 0.], vec![x, 1., 0.]])
                    .collect();
                q.weights = vec![vec![1.; 2]; 4];
            }
            1 => {
                for (i, p) in pieces[0].controls.iter_mut().enumerate() {
                    p[0][0] = if i < 3 { 1. } else { -1. };
                }
            }
            2 => q.control_points[1][1][2] = f64::from_bits(1),
            3 => {
                let mut next = pieces[0].clone();
                for p in &mut next.controls {
                    p[3][2] += 1.;
                }
                next.controls[0][0][0] = 2.;
                pieces.push(next);
            }
            4 => pieces[0].controls[1][0][2] = f64::from_bits(1),
            _ => unreachable!(),
        }
        let r = certify(&q, [0.; 3], &pieces, proposal(), limits()).unwrap();
        assert!(
            !r.embedding_certified
                && !r.parameter_topology_preserved
                && !r.spanwise_interior_regular,
            "case {case}: {r:?}"
        );
    }
}
#[test]
fn every_exact_budget_refuses_and_invalid_tail_inputs_precede_zero_budget_handling() {
    let q = carrier();
    let pieces = motion();
    let r = certify(&q, [0.; 3], &pieces, proposal(), limits()).unwrap();
    for exact_work in [0, r.exact_work - 1] {
        let x = certify(
            &q,
            [0.; 3],
            &pieces,
            proposal(),
            Limits {
                exact_work,
                ..limits()
            },
        )
        .unwrap();
        assert!(
            !x.embedding_certified && !x.parameter_topology_preserved && x.exact_work <= exact_work
        );
    }
    let mut invalid = pieces;
    *invalid[0].weights.last_mut().unwrap() = 0.;
    assert!(
        certify(
            &q,
            [0.; 3],
            &invalid,
            proposal(),
            Limits {
                cells: 0,
                exact_work: 0
            }
        )
        .is_err()
    );
}
#[test]
fn insufficient_height_control_order_is_unresolved_even_for_a_monotone_curve() {
    // h'=3-6.6t+3.9t²>0, but this sufficient coefficient-order test cannot prove it.
    let pieces = vec![RationalPiece {
        controls: [0., 1., 0.9, 1.]
            .into_iter()
            .map(|z| [[1., 0., 0.], [0., 1., 0.], [0.; 3], [0., 0., z]])
            .collect(),
        weights: vec![1.; 4],
    }];
    let r = certify(&carrier(), [0.; 3], &pieces, proposal(), limits()).unwrap();
    assert!(!r.embedding_certified);
    assert_eq!(r.reason, Some("fiber-coefficient-premises-unproved"));
}
