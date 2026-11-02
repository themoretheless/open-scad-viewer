use super::*;
fn cap(end: bool) -> Surface {
    let degree = if end { 2 } else { 1 };
    Surface {
        degree_u: degree,
        degree_v: degree,
        knots_u: [
            vec![if end { 17. } else { -3. }; degree + 1],
            vec![if end { 25. } else { 5. }; degree + 1],
        ]
        .concat(),
        knots_v: [
            vec![if end { -7. } else { 7. }; degree + 1],
            vec![if end { 1. } else { 11. }; degree + 1],
        ]
        .concat(),
        control_points: (0..=degree)
            .map(|i| {
                (0..=degree)
                    .map(|j| {
                        vec![
                            i as f64 / degree as f64,
                            j as f64 / degree as f64,
                            if end && i == 1 && j == 1 {
                                1. / 64.
                            } else {
                                0.
                            },
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: (0..=degree)
            .map(|i| {
                (0..=degree)
                    .map(|j| if end && i == 1 && j == 1 { 2. } else { 1. })
                    .collect()
            })
            .collect(),
        periodic_u: false,
        periodic_v: false,
    }
}
fn pieces() -> Vec<RationalPiece> {
    (0..2)
        .map(|i| RationalPiece {
            controls: (0..2)
                .map(|j| {
                    [
                        [1., 0., 0.],
                        [0., 1., 0.],
                        [0., 0., 1.],
                        [0., 0., (i + j) as f64],
                    ]
                })
                .collect(),
            weights: vec![1.; 2],
        })
        .collect()
}
const P: [[f64; 3]; 3] = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
#[test]
fn independent_rational_caps_retain_global_map_jets_and_every_budget() {
    let a = cap(false);
    let b = cap(true);
    let pieces = pieces();
    let before = (a.clone(), b.clone(), pieces.clone());
    let run =
        |cells, exact| certify_rational_caps([&a, &b], [0.; 3], &pieces, P, cells, exact).unwrap();
    let full = run(100000, 1000000);
    assert!(
        full.embedding_certified && full.regularity_certified && full.parameter_topology_preserved,
        "{full:?}"
    );
    assert!(run(full.cells, full.exact_work).embedding_certified);
    let short = run(full.cells - 1, full.exact_work);
    assert!(!short.embedding_certified && short.cells <= full.cells - 1);
    let short = run(full.cells, full.exact_work - 1);
    assert!(!short.embedding_certified && short.exact_work <= full.exact_work - 1);
    let r =
        jacobian_orientation_rational_caps([&a, &b], [0.; 3], &pieces, [0.5; 3], 100000).unwrap();
    assert_eq!(r.sign, Some(1));
    assert!(
        jacobian_orientation_rational_caps([&a, &b], [0.; 3], &pieces, [0.5; 3], r.cells - 1)
            .unwrap()
            .sign
            .is_none()
    );
    assert_eq!(a, before.0);
    assert_eq!(b, before.1);
    for (a, b) in pieces.iter().zip(&before.2) {
        assert_eq!(a.controls, b.controls);
        assert_eq!(a.weights, b.weights);
    }
    eprintln!(
        "paired affine caps: cells={} exact={} orientationCells={}",
        full.cells, full.exact_work, r.cells
    );
}
#[test]
fn folded_pair_or_broken_motion_never_gains_volume_credit() {
    let a = cap(false);
    let mut b = cap(true);
    let mut pieces = pieces();
    for p in b.control_points.iter_mut().flatten() {
        p[0] = 1. - p[0];
    }
    let r = certify_rational_caps([&a, &b], [0.; 3], &pieces, P, 256, 1000000).unwrap();
    assert!(!r.embedding_certified && !r.parameter_topology_preserved);
    pieces[1].controls[0][3][2] += 1. / 64.;
    let r = certify_rational_caps([&a, &a], [0.; 3], &pieces, P, 100000, 1000000).unwrap();
    assert!(!r.embedding_certified);
    assert_eq!(r.reason, Some("affine-volume-station-identity-unproved"));
}

#[test]
fn paired_cap_time_jet_survives_exact_column_factor_coordinates() {
    let a = cap(false);
    let mut b = a.clone();
    for p in b.control_points.iter_mut().flatten() { p[1] += 0.25; }
    let pieces = vec![RationalPiece {
        controls: vec![
            [[1.,0.,0.],[0.5,1.,0.],[0.,0.,1.],[0.,0.,0.]],
            [[1.,0.,0.],[0.5,1.,0.],[0.,0.,1.],[0.,0.,2.]],
        ], weights: vec![1.;2],
    }];
    let coordinates = ProofCoordinates {
        rows: P, offset: [0.;3], determinant: [1.,1.],
        column_factor: Some(ColumnFactor {
            base: 0, dependent: 1, scale: 0.5, constant: [0.,1.,0.],
        }),
    };
    let mut context = Context {
        carrier: &a, second: Some(&b), difference: None,
        uv_window: [[0.,1.];2], origin: [0.;3],
        pieces: Motion::Rational(&pieces), poses: BTreeMap::new(),
        coordinates: Some(coordinates), separable: None,
    };
    let mut out = IdealVolumeReport {
        embedding_certified: false, regularity_certified: false,
        parameter_topology_preserved: false, cells: 0, exact_work: 0,
        boxes: 0, monotonicity_lower: None, projection: P, reason: None,
    };
    // Independent derivative of F=(u+.5(v+t/4),v+t/4,2t).
    // In particular dx/dt=1/8 must not disappear in proof coordinates.
    for t in [0.,0.25,0.5,1.] {
        let Enclosed::Value(e) = context.enclose([[0.5;2],[0.5;2],[t;2]], &mut out, 10000).unwrap()
            else { panic!("point enclosure unavailable"); };
        for (bound, expected) in e.columns[2].iter().zip([0.125,0.25,2.]) {
            assert!(bound.lo <= expected && expected <= bound.hi, "{bound:?} misses {expected}");
            assert!(bound.hi-bound.lo < 1e-10);
        }
    }
}
