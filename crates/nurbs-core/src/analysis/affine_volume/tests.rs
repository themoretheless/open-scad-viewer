use super::*;
fn carrier() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-1., -1., 3., 3.],
        knots_v: vec![2., 2., 6., 6.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn pieces() -> Vec<[Pose; 6]> {
    (0..2)
        .map(|i| {
            std::array::from_fn(|j| {
                [
                    [1., 0., 0.],
                    [0., 1., 0.],
                    [0., 0., 1.],
                    [0., 0., (5 * i + j) as f64 / 4.],
                ]
            })
        })
        .collect()
}
fn audit(s: &Surface, p: &[[Pose; 6]], closed: bool, cells: usize, work: u64) -> IdealVolumeReport {
    certify(
        s,
        [0.; 3],
        p,
        closed,
        [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        cells,
        work,
    )
    .unwrap()
}
#[test]
fn actual_affine_volume_proves_regular_global_map_and_all_short_stages_refuse() {
    let s = carrier();
    let p = pieces();
    let old = (s.clone(), p.clone());
    let r = audit(&s, &p, false, 10000, 1000);
    assert!(
        r.embedding_certified && r.regularity_certified && r.parameter_topology_preserved,
        "{r:?}"
    );
    assert!(r.monotonicity_lower.is_none());
    assert!(r.cells > 0 && r.exact_work > 0);
    for (cells, work) in [
        (0, 1000),
        (r.cells - 1, 1000),
        (10000, 0),
        (10000, r.exact_work - 1),
    ] {
        let r = audit(&s, &p, false, cells, work);
        assert!(
            !r.embedding_certified && !r.regularity_certified && !r.parameter_topology_preserved,
            "{r:?}"
        );
    }
    assert_eq!((s, p), old);
}
#[test]
fn literal_pose_identity_and_real_volume_rank_cannot_be_replaced_by_samples() {
    let s = carrier();
    let mut p = pieces();
    p[1][0][3][2] += 1e-12;
    let r = audit(&s, &p, false, 10000, 1000);
    assert_eq!(r.reason, Some("affine-volume-station-identity-unproved"));
    let r = audit(&s, &pieces(), true, 10000, 1000);
    assert_eq!(r.reason, Some("affine-volume-station-identity-unproved"));
    let p = vec![[[[1., 0., 0.], [0., 1., 0.], [0., 0., 1.], [0.; 3]]; 6]];
    // Every A is nonsingular, and the time seam is literal. The filled volume
    // still has rank two because its whole trajectory is stationary.
    let r = audit(&s, &p, true, 1000, 1000);
    assert!(!r.embedding_certified && !r.regularity_certified && !r.parameter_topology_preserved);
}
#[test]
fn normalized_original_rational_nonplanar_carrier_and_variable_pose_are_owned() {
    let mut s = carrier();
    s.weights = vec![vec![1., 2.], vec![2., 4.]];
    s.control_points[1][1][2] = 1. / 128.;
    let mut p = pieces();
    for (i, piece) in p.iter_mut().enumerate() {
        for (j, p) in piece.iter_mut().enumerate() {
            let t = (5 * i + j) as f64 / 16.;
            p[0][0] = 1. + t;
            p[1][1] = 1. + t / 2.;
        }
    }
    let q = charts::cap_jets(&s, [[0., 1.]; 2], &mut 0, 1000)
        .unwrap()
        .unwrap();
    assert!(q[1][0].lo > 0. && q[2][1].lo > 0.);
    assert!(q[1][1].lo.abs().max(q[1][1].hi.abs()) < 1e-10);
    assert!(q[2][0].lo.abs().max(q[2][0].hi.abs()) < 1e-10);
    let r = audit(&s, &p, false, 1000000, 1000);
    assert!(
        r.embedding_certified && r.regularity_certified && r.parameter_topology_preserved,
        "{r:?}"
    );
}
#[test]
fn malformed_geometry_and_oversized_budgets_do_not_masquerade_as_refusal() {
    let s = carrier();
    let p = pieces();
    let projection = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    for (cells, work) in [(1000001, 1000), (10000, 1000001)] {
        assert!(certify(&s, [0.; 3], &p, false, projection, cells, work).is_err());
    }
    let mut bad = p.clone();
    bad[0][2][1][2] = f64::NAN;
    assert!(certify(&s, [0.; 3], &bad, false, projection, 0, 0).is_err());
    let mut bad = s.clone();
    bad.weights[0][0] = 0.;
    assert!(certify(&bad, [0.; 3], &p, false, projection, 0, 0).is_err());
    assert!(certify(&s, [0.; 3], &[], false, projection, 10000, 1000).is_err());
}
#[test]
fn temporal_piece_boundaries_enclose_both_one_sided_derivatives() {
    let p = pieces();
    let s = carrier();
    let mut c = Context {
        carrier: &s,
        origin: [0.; 3],
        pieces: &p,
        poses: BTreeMap::new(),
    };
    let mut out = audit(&s, &p, false, 0, 0);
    let value = c
        .enclose([[0.25, 0.5], [0.125, 0.25], [0.5, 0.5]], &mut out, 10000)
        .unwrap();
    let Enclosed::Value(e) = value else {
        panic!("exact time knot was not enclosed");
    };
    assert!(e.value[2].lo <= 1.25 && e.value[2].hi >= 1.25);
    assert!(e.columns[2][2].lo <= 2.5 && e.columns[2][2].hi >= 2.5);
    assert!(e.columns[0][0].lo <= 1. && e.columns[0][0].hi >= 1.);
    assert!(e.columns[1][1].lo <= 1. && e.columns[1][1].hi >= 1.);
}

#[test]
fn owned_world_orientation_is_signed_and_refuses_short_or_invalid_queries() {
    let s = carrier();
    let p = pieces();
    let sign = jacobian_orientation(&s, [0.; 3], &p, [0.5; 3], 10000).unwrap();
    assert_eq!(sign.sign, Some(1));
    assert!(sign.cells > 0);
    for max in [0, sign.cells - 1] {
        assert!(
            jacobian_orientation(&s, [0.; 3], &p, [0.5; 3], max)
                .unwrap()
                .sign
                .is_none()
        );
    }
    let mut reverse = p.clone();
    for p in reverse.iter_mut().flatten() {
        p[0][0] = -1.;
    }
    assert!(audit(&s, &reverse, false, 10000, 1000).embedding_certified);
    assert_eq!(
        jacobian_orientation(&s, [0.; 3], &reverse, [0.5; 3], 10000)
            .unwrap()
            .sign,
        Some(-1)
    );
    assert!(jacobian_orientation(&s, [0.; 3], &p, [f64::NAN, 0.5, 0.5], 0).is_err());
}
