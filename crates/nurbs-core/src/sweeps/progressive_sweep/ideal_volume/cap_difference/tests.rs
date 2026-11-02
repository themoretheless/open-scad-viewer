use super::*;
fn caps(rational: bool) -> [Surface; 2] {
    std::array::from_fn(|end| {
        let (u, v) = if end == 0 {
            ([-3., 5.], [7., 11.])
        } else {
            ([17., 25.], [-7., 1.])
        };
        let mut s = Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: [vec![u[0]; 3], vec![u[1]; 3]].concat(),
            knots_v: [vec![v[0]; 3], vec![v[1]; 3]].concat(),
            control_points: (0..3)
                .map(|i| {
                    (0..3)
                        .map(|j| {
                            vec![
                                i as f64 / 2.,
                                j as f64 / 2.,
                                if end == 1 && i == 1 && j == 1 {
                                    1. / 64.
                                } else {
                                    0.
                                },
                            ]
                        })
                        .collect()
                })
                .collect(),
            weights: vec![vec![if end == 0 { 1. } else { 2. }; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        if end == 1 && rational {
            s.weights[1][1] *= 65. / 64.;
        }
        s
    })
}
#[test]
fn original_rational_difference_retains_zero_components_and_raw_domain_jets() {
    for rational in [false, true] {
        let c = caps(rational);
        let before = c.clone();
        let pair = [&c[0], &c[1]];
        let mut spent = 0;
        let d = Difference::prepare(pair, &mut spent, 10000)
            .unwrap()
            .unwrap();
        assert_eq!(spent, 45);
        let mut exact = 0;
        assert!(
            Difference::prepare(pair, &mut exact, spent)
                .unwrap()
                .is_some()
        );
        assert_eq!(exact, spent);
        let mut short = 0;
        assert!(
            Difference::prepare(pair, &mut short, spent - 1)
                .unwrap()
                .is_none()
        );
        assert!(short < spent);
        let a = cap_jets(&c[0], [[0., 1.]; 2], &mut spent, 10000)
            .unwrap()
            .unwrap();
        let bound = d
            .enclose(pair, a, [[0., 1.]; 2], &mut spent, 10000)
            .unwrap()
            .unwrap();
        for axis in 0..2 {
            assert!(bound[0][axis].lo > -0.02 && bound[0][axis].hi < 0.02);
        }
        if !rational {
            for j in 0..3 {
                for k in 0..2 {
                    assert!(bound[j][k].lo > -1e-12 && bound[j][k].hi < 1e-12);
                }
            }
        }
        for u in [0., 0.25, 0.5, 0.75, 1.] {
            for v in [0., 0.25, 0.5, 0.75, 1.] {
                let a = cap_jets(&c[0], [[u, u], [v, v]], &mut spent, 10000)
                    .unwrap()
                    .unwrap();
                let b = cap_jets(&c[1], [[u, u], [v, v]], &mut spent, 10000)
                    .unwrap()
                    .unwrap();
                for j in 0..3 {
                    for k in 0..3 {
                        let reference = b[j][k].sub(a[j][k]).unwrap();
                        assert!(
                            bound[j][k].lo <= reference.hi && reference.lo <= bound[j][k].hi,
                            "rational={rational} uv={u},{v} jet={j} component={k} bound={:?} reference={reference:?}",
                            bound[j][k]
                        );
                    }
                }
            }
        }
        let mut exhausted = 0;
        assert!(
            d.enclose(pair, a, [[0., 1.]; 2], &mut exhausted, 0)
                .unwrap()
                .is_none()
        );
        assert_eq!(exhausted, 0);
        let foreign = c[0].clone();
        assert!(
            d.enclose([&foreign, &c[1]], a, [[0., 1.]; 2], &mut spent, 10000)
                .unwrap()
                .is_none()
        );
        assert_eq!(c, before);
    }
}

#[test]
fn restricted_original_homogeneous_jets_sharpen_unequal_weight_difference() {
    let c = caps(true);
    let pair = [&c[0], &c[1]];
    let mut spent = 0;
    let d = Difference::prepare(pair, &mut spent, 10000)
        .unwrap()
        .unwrap();
    let whole = cap_jets(&c[0], [[0., 1.]; 2], &mut spent, 10000)
        .unwrap()
        .unwrap();
    let broad = d
        .enclose(pair, whole, [[0., 1.]; 2], &mut spent, 10000)
        .unwrap()
        .unwrap();
    let point = cap_jets(&c[0], [[0.5; 2]; 2], &mut spent, 10000)
        .unwrap()
        .unwrap();
    let narrow = d
        .enclose(pair, point, [[0.5; 2]; 2], &mut spent, 10000)
        .unwrap()
        .unwrap();
    for k in 0..2 {
        assert!(broad[0][k].hi - broad[0][k].lo > 0.001);
        assert!(narrow[0][k].lo > -1e-10 && narrow[0][k].hi < 1e-10);
    }
    for uv in [[[0.25, 0.5], [0.5, 0.75]], [[1.; 2]; 2]] {
        let a = cap_jets(&c[0], uv, &mut spent, 10000).unwrap().unwrap();
        let bound = d.enclose(pair, a, uv, &mut spent, 10000).unwrap().unwrap();
        for u in uv[0] {
            for v in uv[1] {
                let a = cap_jets(&c[0], [[u; 2], [v; 2]], &mut spent, 10000)
                    .unwrap()
                    .unwrap();
                let b = cap_jets(&c[1], [[u; 2], [v; 2]], &mut spent, 10000)
                    .unwrap()
                    .unwrap();
                for j in 0..3 {
                    for k in 0..3 {
                        let reference = b[j][k].sub(a[j][k]).unwrap();
                        assert!(bound[j][k].lo <= reference.hi && reference.lo <= bound[j][k].hi);
                    }
                }
            }
        }
    }
}
