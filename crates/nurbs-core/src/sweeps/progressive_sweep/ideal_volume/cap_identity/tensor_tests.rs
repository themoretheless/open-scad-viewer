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
    std::array::from_fn(|i| {
        let (u, v, weights, ku, kv, p) = if i == 0 {
            if multispan {
                (
                    vec![-0.5, 0., 0.5],
                    vec![-0.5, -0.25, 0.25, 0.5],
                    vec![1.; 3],
                    vec![-3., -3., 1., 5., 5.],
                    vec![7., 7., 8., 10., 11., 11.],
                    1,
                )
            } else {
                (
                    vec![-0.5, 0.5],
                    vec![-0.5, 0.5],
                    vec![1.; 2],
                    vec![-3., -3., 5., 5.],
                    vec![7., 7., 11., 11.],
                    1,
                )
            }
        } else {
            (
                vec![-0.5, -0.25, 0.5],
                vec![-0.5, -0.25, 0.5],
                vec![1., 2., 3.],
                vec![17., 17., 17., 29., 29., 29.],
                vec![-7., -7., -7., 13., 13., 13.],
                2,
            )
        };
        Surface {
            degree_u: p,
            degree_v: p,
            knots_u: ku,
            knots_v: kv,
            control_points: u
                .iter()
                .map(|&x| v.iter().map(|&y| vec![x, y, x * y / 8.]).collect())
                .collect(),
            weights: (0..u.len())
                .map(|a| {
                    (0..v.len())
                        .map(|b| if i == 0 { 1. } else { weights[a] * weights[b] })
                        .collect()
                })
                .collect(),
            periodic_u: false,
            periodic_v: false,
        }
    })
}
#[test]
fn different_original_tensor_layouts_own_map_jets_references_and_exact_budget() {
    for multi in [false, true] {
        let c = caps(multi);
        let before = c.clone();
        for s in &c {
            s.validate().unwrap();
        }
        let mut r = ledger();
        r.cells = 5;
        r.exact_work = 7;
        let identity = certify([&c[0], &c[1]], &mut r, 10000, 1000000).unwrap();
        assert!(identity.matches([&c[0], &c[1]]));
        assert!(!identity.matches([&before[0], &c[1]]));
        assert!(r.cells > 5 && r.exact_work > 7);
        let mut exact = ledger();
        exact.cells = 5;
        exact.exact_work = 7;
        assert!(certify([&c[0], &c[1]], &mut exact, r.cells, r.exact_work).is_some());
        assert_eq!((exact.cells, exact.exact_work), (r.cells, r.exact_work));
        for (cells, work) in [
            (r.cells - 1, r.exact_work),
            (r.cells, r.exact_work - 1),
            (r.cells, 7),
        ] {
            let mut short = ledger();
            short.cells = 5;
            short.exact_work = 7;
            assert!(certify([&c[0], &c[1]], &mut short, cells, work).is_none());
            assert!(short.cells <= cells && short.exact_work <= work);
            assert!(
                !short.embedding_certified
                    && !short.regularity_certified
                    && !short.parameter_topology_preserved
            );
        }
        for u in [0_f64, 0.25, 0.5, 0.75, 1.] {
            for v in [0_f64, 0.25, 0.5, 0.75, 1.] {
                let x = u - 0.5;
                let y = v - 0.5;
                let expected = [[x, y, x * y / 8.], [1., 0., y / 8.], [0., 1., x / 8.]];
                for s in &c {
                    let j = cap_jets(s, [[u, u], [v, v]], &mut 0, 1000)
                        .unwrap()
                        .unwrap();
                    for a in 0..3 {
                        for k in 0..3 {
                            assert!(
                                j[a][k].lo <= expected[a][k] + 1e-14
                                    && expected[a][k] - 1e-14 <= j[a][k].hi
                            );
                        }
                    }
                }
                let e = c[1].evaluate(17. + 12. * u, -7. + 20. * v).unwrap();
                let (duu, duv, dvv) = e.second_derivatives().unwrap();
                for a in 0..3 {
                    assert!((e.point[a] - expected[0][a]).abs() < 2e-12);
                    assert!((duu[a] * 144.).abs() < 2e-12);
                    assert!((dvv[a] * 400.).abs() < 2e-12);
                    assert!((duv[a] * 240. - if a == 2 { 0.125 } else { 0. }).abs() < 2e-12);
                }
            }
        }
        let mut altered = c[1].clone();
        altered.control_points[1][1][2] = altered.control_points[1][1][2].next_up();
        assert!(certify([&c[0], &altered], &mut ledger(), 10000, 1000000).is_none());
        assert_eq!(c, before);
        eprintln!(
            "original different tensor layout multi={multi}: cells={} exact={}",
            r.cells - 5,
            r.exact_work - 7
        );
    }
}
#[test]
fn complete_open_original_body_accepts_different_cap_degrees_and_partitions_but_not_rank_loss() {
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
    for multi in [false, true] {
        let c = caps(multi);
        let before = c.clone();
        let mut identity = ledger();
        assert!(certify([&c[0], &c[1]], &mut identity, 10000, 1000000).is_some());
        for atlas in [false, true] {
            let run = |caps, max| {
                if atlas {
                    sweep
                        .certify_ideal_volume_atlas(caps, ledger().projection, max, 1000000)
                        .unwrap()
                } else {
                    sweep
                        .certify_ideal_volume(caps, ledger().projection, max, 1000000)
                        .unwrap()
                }
            };
            let base = run([&c[0], &c[0]], 10000);
            let full = run([&c[0], &c[1]], 10000);
            assert!(base.embedding_certified, "{base:?}");
            assert!(
                full.embedding_certified
                    && full.regularity_certified
                    && full.parameter_topology_preserved,
                "{full:?}"
            );
            assert_eq!(full.cells, base.cells + identity.cells);
            assert_eq!(full.exact_work, base.exact_work + identity.exact_work);
            assert!(run([&c[0], &c[1]], full.cells).embedding_certified);
            let short = run([&c[0], &c[1]], identity.cells - 1);
            assert!(
                !short.embedding_certified
                    && !short.regularity_certified
                    && !short.parameter_topology_preserved
            );
            eprintln!(
                "different tensor layouts original body multi={multi} atlas={atlas}: {full:?}"
            );
        }
        let mut singular = c.clone();
        for cap in &mut singular {
            for p in cap.control_points.iter_mut().flatten() {
                p[0] = 0.;
                p[2] = 0.;
            }
        }
        assert!(certify([&singular[0], &singular[1]], &mut ledger(), 10000, 1000000).is_some());
        let r = sweep
            .certify_ideal_volume_atlas(
                [&singular[0], &singular[1]],
                ledger().projection,
                1000,
                1000000,
            )
            .unwrap();
        assert!(
            !r.embedding_certified && !r.regularity_certified && !r.parameter_topology_preserved,
            "{r:?}"
        );
        assert_eq!(c, before);
    }
}
