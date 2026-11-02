
    use super::*;
    fn fixture(gain: f64) -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            weights: vec![vec![1.; 3]; 3],
            control_points: (0..3)
                .map(|i| {
                    (0..3)
                        .map(|j| {
                            let u = i as f64 / 2.;
                            let v = j as f64 / 2.;
                            let h = if i == 2 { gain } else { 0. };
                            vec![u + v + h, -u + v + h, -2. * v + h]
                        })
                        .collect()
                })
                .collect(),
            periodic_u: false,
            periodic_v: false,
        }
    }
    fn trace() -> Curve {
        Curve::from_polyline(vec![vec![0.1, 0.2], vec![0.2, 0.3]]).unwrap()
    }
    #[test]
    fn exact_scalar_simplification_preserves_uncertainty_and_elevation() {
        let s = fixture(1.);
        let mut pair = Pair::new(&s, &s, 1000000).unwrap();
        let p = vec![I::new(0.1, 0.2).unwrap(), I::new(0.3, 0.4).unwrap()];
        assert_eq!(pair.product(&[I::point(1.)], &p).unwrap(), p);
        let zeros = vec![I::point(0.); 4];
        assert_eq!(pair.product(&zeros, &p).unwrap(), vec![I::point(0.); 5]);
        assert_eq!(pair.sum(&p, &zeros, true).unwrap(), p);
        let elevated = pair.sum(&p, &[I::point(1.); 4], false).unwrap();
        assert_eq!(elevated.len(), 4);
        assert!(elevated[0].lo <= 1.1 && elevated[0].hi >= 1.2);
        let scaled = pair.scale(&zeros, I::point(2.)).unwrap();
        assert_eq!(scaled, zeros);
        assert_eq!(pair.sum(&p, &scaled, false).unwrap(), p);
        let tiny = I::new(-1e-20, 1e-20).unwrap();
        let scaled = pair.scale(&[tiny], I::point(2.)).unwrap();
        assert!(scaled[0].lo < 0. && scaled[0].hi > 0.);
        let uncertain = pair
            .product(&[I::new(0.9, 1.1).unwrap()], &[I::point(2.)])
            .unwrap();
        assert!(uncertain[0].lo <= 1.8 && uncertain[0].hi >= 2.2);
        assert_ne!(uncertain, vec![I::point(2.)]);
    }
    #[test]
    fn exact_curved_graph_has_small_whole_cell_residual() {
        let s = fixture(1.);
        let p = trace();
        let mut pair = Pair::new(&s, &s, 100_000_000).unwrap();
        let bound = pair.bound([&p, &p], [0., 1.], [[2, 2]; 2]).unwrap();
        assert_eq!(bound.same_orientation, Some(true));
        let upper = bound.upper.unwrap();
        assert!(upper[0] < 1e-8 && upper[1] < 1e-7, "{upper:?}");
        assert!(pair.work > 1000 && pair.work <= pair.max_work);
    }
    #[test]
    fn altered_curvature_does_not_receive_small_bound() {
        let s = fixture(1.);
        let changed = fixture(1.125);
        let p = trace();
        let mut pair = Pair::new(&s, &changed, 100_000_000).unwrap();
        let bound = pair
            .bound_adaptive([&p, &p], [0., 1.], [[2, 2]; 2], Some([1e-5, 1e-4]))
            .unwrap();
        assert!(bound.upper.is_none_or(|upper| upper[1] > 1e-4));
    }
    #[test]
    fn cartesian_curved_graph_uses_exact_world_chart_and_refuses_changed_curvature() {
        let world = |gain: f64| {
            let mut s = fixture(gain);
            for i in 0..3 {
                for j in 0..3 {
                    s.control_points[i][j] = vec![
                        i as f64 * 0.5,
                        j as f64 * 0.5,
                        if i == 2 { gain } else { 0. },
                    ];
                }
            }
            s
        };
        let s = world(0.25);
        let p = Curve::from_polyline(vec![vec![0.125, 0.125], vec![0.875, 0.75]]).unwrap();
        let mut pair = Pair::new(&s, &s, 100_000_000).unwrap();
        assert!(pair.cartesian);
        let bound = pair.bound([&p, &p], [0., 1.], [[2, 2]; 2]).unwrap();
        let upper = bound
            .upper
            .expect("whole-cell Cartesian shape must qualify");
        assert!(upper[0] < 1e-8 && upper[1] < 1e-7, "{upper:?}");
        let mut changed = Pair::new(&s, &world(0.375), 100_000_000).unwrap();
        let bound = changed.bound([&p, &p], [0., 1.], [[2, 2]; 2]).unwrap();
        assert!(bound.upper.is_none_or(|u| u[1] > 1e-4));
    }
    #[test]
    fn original_non_affine_chart_corrections_shrink_with_seam_cell() {
        let mut s = fixture(0.);
        for i in 0..3 {
            for j in 0..3 {
                let u = i as f64 * 0.5;
                let v = j as f64 * 0.5;
                // X=u, Y=(1+u^2)v, Z=u^2. A regular non-affine chart.
                let quadratic = if i == 2 { 1. } else { 0. };
                s.control_points[i][j] = vec![u, (1. + quadratic) * v, quadratic];
            }
        }
        let pc = Curve::from_polyline(vec![vec![0.25, 0.25], vec![0.75, 0.25]]).unwrap();
        let mut pair = Pair::new(&s, &s, 100_000_000).unwrap();
        assert!(pair.cartesian);
        let whole = pair.bound([&pc, &pc], [0., 1.], [[2, 2]; 2]).unwrap();
        let small = pair
            .bound([&pc, &pc], [0.5, 0.500001], [[2, 2]; 2])
            .unwrap();
        let small = small
            .upper
            .expect("regular local chart must have bounded shape");
        assert_eq!(whole.same_orientation, Some(true));
        assert!(small[0] < 1e-4 && small[1] < 1e-3, "{small:?}");
        if let Some(whole) = whole.upper {
            assert!(small[1] < whole[1], "{small:?} {whole:?}");
        }
    }
    #[test]
    fn adaptive_graph_bound_retains_correlated_non_affine_cancellation() {
        let mut s = fixture(0.);
        for i in 0..3 {
            for j in 0..3 {
                let q = if i == 2 { 1. } else { 0. };
                s.control_points[i][j] = vec![i as f64 * 0.5, (1. + q) * j as f64 * 0.5, q];
            }
        }
        let p = Curve::from_polyline(vec![vec![0.25, 0.25], vec![0.75, 0.25]]).unwrap();
        let mut pair = Pair::new(&s, &s, 100_000_000).unwrap();
        let proof = pair
            .bound_adaptive([&p, &p], [0., 1.], [[2, 2]; 2], Some([1e-8, 1e-7]))
            .unwrap();
        let upper = proof
            .upper
            .expect("the complete nonlinear chart must qualify");
        assert!(upper[0] < 1e-8 && upper[1] < 1e-7, "{upper:?}");
        assert_eq!(proof.same_orientation, Some(true));
        assert!(pair.work <= pair.max_work);
    }
    #[test]
    fn compiled_original_children_preserve_polynomial_and_rational_jets() {
        use num_rational::BigRational as Q;
        use num_traits::{One, Zero};
        let q = |x: f64| Q::from_float(x).unwrap();
        let surface = fixture(1.);
        for rational in [false, true] {
            let mut p = trace();
            if rational {
                p.weights[1] = 2.;
            }
            let before = p.clone();
            let mut pair = Pair::new(&surface, &surface, 100_000_000).unwrap();
            assert!(!pair.cartesian);
            pair.compositions([&p, &p], [0., 1.], [[2, 2]; 2]).unwrap();
            let start = pair.work;
            let child = pair
                .compositions([&p, &p], [0.25, 0.5], [[2, 2]; 2])
                .unwrap();
            let cached_cost = pair.work - start;
            let mut fresh = Pair::new(&surface, &surface, 100_000_000).unwrap();
            let start = fresh.work;
            fresh
                .compositions([&p, &p], [0.25, 0.5], [[2, 2]; 2])
                .unwrap();
            if !rational {
                assert!(cached_cost < fresh.work - start);
            }
            for t in [0., 0.5, 1.] {
                let clock = q(0.25) + q(0.25) * q(t);
                let one = Q::one() - &clock;
                let weight = q(p.weights[0]) * &one + q(p.weights[1]) * &clock;
                let u = q(p.control_points[0][0]) * q(p.weights[0]) * &one
                    + q(p.control_points[1][0]) * q(p.weights[1]) * &clock;
                let v = q(p.control_points[0][1]) * q(p.weights[0]) * &one
                    + q(p.control_points[1][1]) * q(p.weights[1]) * &clock;
                let w2 = &weight * &weight;
                let w3 = &w2 * &weight;
                let w4 = &w2 * &w2;
                let zero = Q::zero();
                let expected = [
                    [
                        q(2.) * &u * &w3,
                        q(6.) * &v * &w3,
                        q(3.) * &u * &u * &w2,
                        w4.clone(),
                    ],
                    [q(2.) * &w4, zero.clone(), q(6.) * &u * &w3, zero.clone()],
                    [zero.clone(), q(6.) * &w4, zero.clone(), zero.clone()],
                    [zero.clone(), zero.clone(), q(6.) * &w4, zero.clone()],
                    std::array::from_fn(|_| zero.clone()),
                    std::array::from_fn(|_| zero.clone()),
                ];
                for side in &child {
                    assert!(side.position.is_none());
                    for j in 0..6 {
                        for k in 0..4 {
                            let polynomial = if j == 0 {
                                if k == 3 {
                                    &side.weight
                                } else {
                                    let Some(position) = &side.position else {
                                        continue;
                                    };
                                    &position[k]
                                }
                            } else {
                                &side.derivatives[j - 1][k]
                            };
                            let mut row = polynomial.clone();
                            while row.len() > 1 {
                                row = row
                                    .windows(2)
                                    .map(|v| {
                                        v[0].mul(I::point(1. - t))
                                            .unwrap()
                                            .add(v[1].mul(I::point(t)).unwrap())
                                            .unwrap()
                                    })
                                    .collect();
                            }
                            assert!(
                                q(row[0].lo) <= expected[j][k] && expected[j][k] <= q(row[0].hi),
                                "rational={rational} jet={j} k={k} t={t}: {:?}",
                                row[0]
                            );
                        }
                    }
                }
            }
            let mut changed = p.clone();
            changed.control_points[1][0] = changed.control_points[1][0].next_up();
            let changed_jets = pair
                .compositions([&p, &changed], [0.25, 0.5], [[2, 2]; 2])
                .unwrap();
            let mut fresh = Pair::new(&surface, &surface, 100_000_000).unwrap();
            assert_eq!(
                changed_jets,
                fresh
                    .compositions([&p, &changed], [0.25, 0.5], [[2, 2]; 2])
                    .unwrap()
            );
            assert_eq!(p, before);
            pair.max_work = pair.work;
            assert!(
                pair.compositions([&p, &changed], [0.25, 0.5], [[2, 2]; 2])
                    .is_err()
            );
        }
    }
    #[test]
    fn rational_pcurve_chart_does_not_treat_common_weight_as_cartesian_scale() {
        let source = fixture(1.);
        let mut p = trace();
        p.weights[1] = 2.;
        let mut pair = Pair::new(&source, &source, 100_000_000).unwrap();
        let proof = pair
            .bound_adaptive([&p, &p], [0.25, 0.250001], [[2, 2]; 2], Some([1e-5, 1e-4]))
            .unwrap();
        let upper = proof
            .upper
            .expect("rational pcurve graph must remain bounded");
        assert!(upper[0] < 1e-5 && upper[1] < 1e-4, "{upper:?}");
        assert_eq!(proof.same_orientation, Some(true));
        let changed = fixture(1.125);
        let mut pair = Pair::new(&source, &changed, 100_000_000).unwrap();
        let proof = pair
            .bound_adaptive([&p, &p], [0.25, 0.250001], [[2, 2]; 2], Some([1e-5, 1e-4]))
            .unwrap();
        assert!(proof.upper.is_none_or(|u| u[1] > 1e-4));
    }
    #[test]
    fn constant_nonunit_source_weights_keep_original_graph_geometry() {
        let candidate = fixture(1.);
        let mut source = candidate.clone();
        for row in &mut source.weights {
            row.fill(2.);
        }
        for rational in [false, true] {
            let mut p = trace();
            if rational {
                p.weights[1] = 2.;
            }
            let mut pair = Pair::new(&source, &candidate, 100_000_000).unwrap();
            let proof = pair
                .bound_adaptive([&p, &p], [0.25, 0.250001], [[2, 2]; 2], Some([1e-5, 1e-4]))
                .unwrap();
            let upper = proof
                .upper
                .expect("constant nonunit source weights must preserve geometry");
            assert!(upper[0] < 1e-5 && upper[1] < 1e-4, "{upper:?}");
            assert_eq!(proof.same_orientation, Some(true));
        }
    }
    #[test]
    fn no_work_is_not_evidence() {
        let s = fixture(1.);
        assert!(Pair::new(&s, &s, 0).is_err());
    }
    #[test]
    fn rational_source_graph_jets_match_independent_chain_rule() {
        let mut source = fixture(1.);
        for i in 0..3 {
            for j in 0..3 {
                source.weights[i][j] = 1. + (i + 2 * j) as f64 * 0.125;
            }
        }
        let candidate = fixture(1.);
        let mut pair = Pair::new(&source, &candidate, 100_000_000).unwrap();
        let pc = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![0.25, 0.375]; 2],
            weights: vec![1.; 2],
            periodic: false,
        };
        let mut w = Work {
            used: pair.work,
            limit: pair.max_work,
        };
        let h = pair
            .source
            .compose_graph(&pc, [0., 1.], [2, 2], &mut w)
            .unwrap();
        pair.work = w.used;
        assert!(
            h.position.is_some(),
            "Variable original surface weights require position numerators"
        );
        let fields = pair.graph_fields(&h).unwrap();
        let evaluation = source.evaluate(0.25, 0.375).unwrap();
        let (u, v) = evaluation.first_derivatives().unwrap();
        let (uu, uv, vv) = evaluation.second_derivatives().unwrap();
        let project = |p: [f64; 3]| [p[0] - p[1], p[0] + p[1] - 2. * p[2], p[0] + p[1] + p[2]];
        let (u, v) = (project(u), project(v));
        let determinant = u[0] * v[1] - v[0] * u[1];
        let inverse = [
            [v[1] / determinant, -v[0] / determinant],
            [-u[1] / determinant, u[0] / determinant],
        ];
        let gradient =
            std::array::from_fn::<_, 2, _>(|a| u[2] * inverse[0][a] + v[2] * inverse[1][a]);
        let r = [uu, uv, vv].map(|x| {
            let x = project(x);
            x[2] - gradient[0] * x[0] - gradient[1] * x[1]
        });
        let mut expected = vec![gradient[0], gradient[1]];
        for (a, b) in [(0, 0), (0, 1), (1, 1)] {
            expected.push(
                inverse[0][a] * r[0] * inverse[0][b]
                    + (inverse[0][a] * inverse[1][b] + inverse[1][a] * inverse[0][b]) * r[1]
                    + inverse[1][a] * r[2] * inverse[1][b],
            );
        }
        for (field, expected) in fields.iter().zip(expected) {
            let interval = hull(&field.0)
                .unwrap()
                .div_signed(hull(&field.1).unwrap())
                .unwrap();
            assert!(
                expected >= interval.lo - 1e-10 && expected <= interval.hi + 1e-10,
                "{interval:?} versus {expected}"
            );
            assert!(interval.hi - interval.lo < 1e-7, "{interval:?}");
        }
    }
