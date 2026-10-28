use super::*;
fn ledger() -> IdealVolumeReport {
    IdealVolumeReport {
        embedding_certified: false,
        regularity_certified: false,
        parameter_topology_preserved: false,
        cells: 0,
        exact_work: 0,
        boxes: 0,
        monotonicity_lower: None,
        projection: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        reason: None,
    }
}
fn caps(multispan: bool) -> [Surface; 2] {
    let x: &[f64] = if multispan {
        &[-0.5, 0., 0.5]
    } else {
        &[-0.5, 0.5]
    };
    std::array::from_fn(|i| {
        let (u, v, k) = if i == 0 {
            ([-3., 5.], [7., 11.], 1.)
        } else {
            ([17., 29.], [-7., 13.], 2.)
        };
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: if multispan {
                vec![u[0], u[0], (u[0] + u[1]) / 2., u[1], u[1]]
            } else {
                vec![u[0], u[0], u[1], u[1]]
            },
            knots_v: vec![v[0], v[0], v[1], v[1]],
            control_points: x
                .iter()
                .map(|&x| {
                    [-0.5, 0.5]
                        .iter()
                        .map(|&y| vec![x, y, x * y / 8.])
                        .collect()
                })
                .collect(),
            weights: (0..x.len())
                .map(|j| vec![(j + 1) as f64 * k, 1.5 * (j + 1) as f64 * k])
                .collect(),
            periodic_u: false,
            periodic_v: false,
        }
    })
}
#[test]
fn original_affine_tensor_identity_owns_multispan_knots_weights_and_one_ledger() {
    for multispan in [false, true] {
        let c = caps(multispan);
        let before = c.clone();
        for s in &c {
            s.validate().unwrap();
        }
        let mut r = ledger();
        r.cells = 7;
        r.exact_work = 11;
        let proof = certify([&c[0], &c[1]], &mut r, 10000, 1000000).unwrap();
        assert!(proof.matches([&c[0], &c[1]]));
        assert!(!proof.matches([&c[0], &before[1]]));
        assert!(r.cells > 7 && r.exact_work > 11);
        let mut exact = ledger();
        exact.cells = 7;
        exact.exact_work = 11;
        assert!(certify([&c[0], &c[1]], &mut exact, r.cells, r.exact_work).is_some());
        assert_eq!((exact.cells, exact.exact_work), (r.cells, r.exact_work));
        for (cells, work) in [
            (7, r.exact_work),
            (r.cells - 1, r.exact_work),
            (r.cells, 11),
            (r.cells, r.exact_work - 1),
        ] {
            let mut short = ledger();
            short.cells = 7;
            short.exact_work = 11;
            assert!(certify([&c[0], &c[1]], &mut short, cells, work).is_none());
            assert!(short.cells <= cells && short.exact_work <= work);
            assert!(
                !short.embedding_certified
                    && !short.regularity_certified
                    && !short.parameter_topology_preserved
            );
        }
        let mut literal = ledger();
        assert!(certify([&c[0], &c[0]], &mut literal, 0, 0).is_some());
        assert_eq!((literal.cells, literal.exact_work), (0, 0));
        assert_eq!(c, before);
        eprintln!(
            "original normalized carrier identity multispan={multispan}: cells={} exact={}",
            r.cells - 7,
            r.exact_work - 11
        );
    }
}
#[test]
fn near_weight_knot_or_control_agreement_never_supplies_exact_normalized_identity() {
    for multispan in [false, true] {
        let c = caps(multispan);
        for mutation in 0..3 {
            let mut b = c[1].clone();
            match mutation {
                0 => b.weights[0][1] = b.weights[0][1].next_up(),
                1 => b.control_points[0][0][2] = b.control_points[0][0][2].next_up(),
                _ => {
                    if !multispan {
                        continue;
                    }
                    b.knots_u[2] = b.knots_u[2].next_up();
                }
            }
            b.validate().unwrap();
            assert!(certify([&c[0], &b], &mut ledger(), 10000, 1000000).is_none());
        }
    }
    // Ordinary floating products underflow to zero in both terms. The exact
    // authored-leaf determinant still sees that the weight ratios differ.
    let mut c = caps(false);
    c[0].weights = vec![vec![1e-300; 2]; 2];
    c[1].weights = c[0].weights.clone();
    c[1].weights[1][1] = 1e-300_f64.next_up();
    assert_eq!(c[0].weights[0][0] * c[1].weights[1][1], 0.);
    assert_eq!(c[1].weights[0][0] * c[0].weights[1][1], 0.);
    assert!(certify([&c[0], &c[1]], &mut ledger(), 10000, 1000000).is_none());
}
#[test]
fn normalized_rational_cap_first_and_second_derivatives_preserve_original_units() {
    let c = caps(false);
    let before = c.clone();
    assert!(certify([&c[0], &c[1]], &mut ledger(), 10000, 1000000).is_some());
    for u in [0_f64, 0.125, 0.5, 0.875, 1.] {
        for v in [0_f64, 0.25, 0.75, 1.] {
            let x = -0.5 + 2. * u / (1. + u);
            let y = -0.5 + 1.5 * v / (1. + 0.5 * v);
            let dx = 2. / (1. + u).powi(2);
            let dy = 1.5 / (1. + 0.5 * v).powi(2);
            let dxx = -4. / (1. + u).powi(3);
            let dyy = -1.5 / (1. + 0.5 * v).powi(3);
            let expected = [
                [x, y, x * y / 8.],
                [dx, 0., dx * y / 8.],
                [0., dy, x * dy / 8.],
                [dxx, 0., dxx * y / 8.],
                [0., 0., dx * dy / 8.],
                [0., dyy, x * dyy / 8.],
            ];
            for s in &c {
                let lo = [s.knots_u[1], s.knots_v[1]];
                let w = [s.knots_u[2] - lo[0], s.knots_v[2] - lo[1]];
                let e = s.evaluate(lo[0] + w[0] * u, lo[1] + w[1] * v).unwrap();
                let (du, dv) = e.first_derivatives().unwrap();
                let (duu, duv, dvv) = e.second_derivatives().unwrap();
                let normalized = [
                    e.point,
                    du.map(|x| x * w[0]),
                    dv.map(|x| x * w[1]),
                    duu.map(|x| x * w[0] * w[0]),
                    duv.map(|x| x * w[0] * w[1]),
                    dvv.map(|x| x * w[1] * w[1]),
                ];
                for j in 0..6 {
                    for k in 0..3 {
                        assert!((normalized[j][k] - expected[j][k]).abs() < 2e-12);
                    }
                }
                let jets = cap_jets(s, [[u, u], [v, v]], &mut 0, 1000)
                    .unwrap()
                    .unwrap();
                for j in 0..3 {
                    for k in 0..3 {
                        assert!(
                            jets[j][k].lo <= expected[j][k] + 1e-14
                                && expected[j][k] - 1e-14 <= jets[j][k].hi
                        );
                        assert!(jets[j][k].hi - jets[j][k].lo < 1e-10);
                    }
                }
            }
        }
    }
    assert_eq!(c, before);
}
fn open_original_body(multispan: bool) {
    let c = caps(multispan);
    let before = c.clone();
    let profiles = [crate::primitives::line([-0.5, -0.5, 0.03125], [0.5, -0.5, -0.03125]).unwrap()];
    let path = crate::primitives::line([0.; 3], [0., 0., 1.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let axis = constant_vector_law([0., 0., 1.]).unwrap();
    let normal = constant_vector_law([1., 0., 0.]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        Options {
            normal: [1., 0., 0.],
            orientation: Orientation::Fixed,
            spacing: Spacing::Parameter,
            initial_sections: 3,
            max_sections: 9,
            max_deviation: 0.05,
        },
    )
    .unwrap()
    .with_frame_laws(&axis, &normal)
    .unwrap();
    let mut work = ledger();
    assert!(certify([&c[0], &c[1]], &mut work, 10000, 1000000).is_some());
    for atlas in [false, true] {
        let audit = |caps, max| {
            if atlas {
                sweep.certify_ideal_volume_atlas(caps, ledger().projection, max, 1000000)
            } else {
                sweep.certify_ideal_volume(caps, ledger().projection, max, 1000000)
            }
            .unwrap()
        };
        let base = audit([&c[0], &c[0]], 10000);
        let same = audit([&c[0], &c[1]], 10000);
        assert!(base.embedding_certified, "{base:?}");
        assert!(
            same.embedding_certified
                && same.regularity_certified
                && same.parameter_topology_preserved,
            "{same:?}"
        );
        assert_eq!(same.cells, base.cells + work.cells);
        assert_eq!(same.exact_work, base.exact_work + work.exact_work);
        assert!(audit([&c[0], &c[1]], same.cells).embedding_certified);
        for max in [0, same.cells - 1] {
            let short = audit([&c[0], &c[1]], max);
            assert!(
                !short.embedding_certified
                    && !short.regularity_certified
                    && !short.parameter_topology_preserved,
                "{short:?}"
            );
            assert!(short.cells <= max);
        }
        eprintln!(
            "normalized rational original body multispan={multispan} atlas={atlas}: {same:?}"
        );
    }
    assert_eq!(c, before);
}

#[test]
fn rational_normalized_cap_identity_preserves_the_complete_open_original_body() {
    for multispan in [false, true] {
        open_original_body(multispan);
    }
}
