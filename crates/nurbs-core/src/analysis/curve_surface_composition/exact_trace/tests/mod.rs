
    use super::*;
    fn work() -> Work {
        Work {
            used: 0,
            limit: 100_000,
        }
    }
    #[test]
    fn directed_conversion_including_subnormal_and_rational_thirds() {
        for q in [
            Q::new(1.into(), 3.into()),
            -Q::new(1.into(), 3.into()),
            authored(f64::from_bits(1)).unwrap() / Q::from_integer(2.into()),
            Q::zero(),
        ] {
            let i = enclosure(&q).unwrap();
            assert!(authored(i.lo).unwrap() <= q && authored(i.hi).unwrap() >= q);
            assert!(i.hi - i.lo <= 2. * i.hi.abs().max(f64::MIN_POSITIVE) * f64::EPSILON);
        }
    }
    #[test]
    fn exact_narrow_cell_subtraction_retains_zero_and_one() {
        let a = 0.5f64;
        let b = a.next_up();
        let c = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![a, a], vec![b, b]],
            weights: vec![1., 1.],
            periodic: false,
        };
        let h = pcurve(&c, [0., 1.], [[a, b], [a, b]], &mut work()).unwrap();
        for k in 0..2 {
            assert_eq!(h[k], vec![I::point(0.), I::point(1.)]);
        }
    }
    #[test]
    fn boundary_restriction_uses_only_exact_unit_polynomial_coordinates() {
        let net: Net = (0..3)
            .map(|i| (0..4).map(|j| [I::point((i * 4 + j) as f64); 4]).collect())
            .collect();
        for axis in 0..2 {
            for end in 0..2 {
                let mut h = vec![vec![I::point(0.25), I::point(0.75)]; 3];
                h[axis] = vec![I::point(end as f64); 2];
                h[2] = vec![I::point(1.); 2];
                let (reduced, d) =
                    restrict_polynomial_boundaries(net.clone(), [2, 3], &h, true, &mut work())
                        .unwrap();
                assert_eq!(d[axis], 0);
                assert_eq!(d[1 - axis], [2, 3][1 - axis]);
                if axis == 0 {
                    assert_eq!(reduced, vec![net[end * 2].clone()]);
                } else {
                    assert_eq!(
                        reduced,
                        net.iter().map(|row| vec![row[end * 3]]).collect::<Net>()
                    );
                }
                let (unchanged, d) =
                    restrict_polynomial_boundaries(net.clone(), [2, 3], &h, false, &mut work())
                        .unwrap();
                assert_eq!(unchanged, net);
                assert_eq!(d, [2, 3]);
                h[axis][0] = I::new(-1e-20, 1e-20).unwrap();
                assert_eq!(
                    restrict_polynomial_boundaries(net.clone(), [2, 3], &h, true, &mut work())
                        .unwrap()
                        .1,
                    [2, 3]
                );
            }
        }
        let h = vec![
            vec![I::point(0.); 2],
            vec![I::point(1.); 2],
            vec![I::point(1.); 2],
        ];
        let (corner, d) =
            restrict_polynomial_boundaries(net.clone(), [2, 3], &h, true, &mut work()).unwrap();
        assert_eq!(corner, vec![vec![net[0][3]]]);
        assert_eq!(d, [0, 0]);
        let mut none = Work { used: 0, limit: 0 };
        assert!(restrict_polynomial_boundaries(net, [2, 3], &h, true, &mut none).is_err());
        assert_eq!(none.used, 0);
    }
    #[test]
    fn original_boundary_jets_retain_transverse_and_mixed_curvature() {
        let s = crate::surface::Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
            control_points: (0..3)
                .map(|i| {
                    (0..3)
                        .map(|j| {
                            let u = i as f64 * 0.5;
                            let v = j as f64 * 0.5;
                            let a = if i == 2 { 1. } else { 0. };
                            let b = if j == 2 { 1. } else { 0. };
                            vec![u * (1. + v), v + u * b, a + b + a * b]
                        })
                        .collect()
                })
                .collect(),
        };
        let before = s.clone();
        for axis in 0..2 {
            for end in [0., 1.] {
                let points = [0.125, 0.875].map(|t| {
                    if axis == 0 {
                        vec![end, t]
                    } else {
                        vec![t, end]
                    }
                });
                let p = Curve::from_polyline(points.to_vec()).unwrap();
                let mut w = Work {
                    used: 0,
                    limit: 1_000_000,
                };
                let support = Support::new_in_frame(&s, &mut w, true).unwrap();
                let jets = support.compose(&p, [0., 1.], [2, 2], &mut w).unwrap();
                for t in [0., 0.5, 1.] {
                    let free = 0.125 + 0.75 * t;
                    let (u, v) = if axis == 0 { (end, free) } else { (free, end) };
                    let expected = [
                        [u * (1. + v), v + u * v * v, u * u + v * v + u * u * v * v],
                        [1. + v, v * v, 2. * u * (1. + v * v)],
                        [u, 1. + 2. * u * v, 2. * v * (1. + u * u)],
                        [0., 0., 2. * (1. + v * v)],
                        [1., 2. * v, 4. * u * v],
                        [0., 2. * u, 2. * (1. + u * u)],
                    ];
                    for jet in 0..6 {
                        for k in 0..3 {
                            let mut row = jets[jet][k].clone();
                            while row.len() > 1 {
                                row = row
                                    .windows(2)
                                    .map(|r| {
                                        r[0].mul(I::point(1. - t))
                                            .unwrap()
                                            .add(r[1].mul(I::point(t)).unwrap())
                                            .unwrap()
                                    })
                                    .collect();
                            }
                            assert!(
                                row[0].lo <= expected[jet][k] && row[0].hi >= expected[jet][k],
                                "axis={axis} end={end} jet={jet} k={k} t={t}: {:?} vs {}",
                                row[0],
                                expected[jet][k]
                            );
                        }
                    }
                }
                assert!(w.used > 0 && w.used < w.limit);
            }
        }
        assert_eq!(s, before);
    }
    #[test]
    fn reversed_trace_retains_exact_clock_complement_below_binary64_spacing() {
        let p = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
        let tiny = 2f64.powi(-100);
        assert_eq!(1. - tiny, 1., "control must expose rounded complement loss");
        let mut w = work();
        let reversed = pcurve_oriented(&p, [0., tiny], [[0., 1.]; 2], &mut w, true).unwrap();
        assert_eq!(reversed[0][0], I::point(1.));
        let endpoint = reversed[0][1];
        let exact = Q::one() - authored(tiny).unwrap();
        assert!(authored(endpoint.lo).unwrap() <= exact && exact <= authored(endpoint.hi).unwrap());
        assert!(
            endpoint.lo < endpoint.hi,
            "exact complement must not collapse to rounded one"
        );
        assert_eq!(p.control_points, vec![vec![0., 0.], vec![1., 0.]]);
    }
    #[test]
    fn original_rational_pcurve_spans_use_exact_extraction_without_refitting() {
        let c = Curve {
            degree: 2,
            knots: vec![2., 2., 2., 4., 7., 7., 7.],
            control_points: vec![
                vec![0.25, 0.],
                vec![0.3, 0.1],
                vec![0.7, 0.2],
                vec![0.9, 0.4],
            ],
            weights: vec![1., 0.5, 2., 1.],
            periodic: false,
        };
        let before = c.clone();
        for range in [[0., 0.25], [0.5, 1.]] {
            let mut ledger = work();
            let h = pcurve(&c, range, [[0., 1.]; 2], &mut ledger).unwrap();
            for t in [0., 0.125, 0.5, 0.875, 1.] {
                let mut value = Vec::new();
                for coeffs in &h {
                    let mut row = coeffs.clone();
                    while row.len() > 1 {
                        row = row
                            .windows(2)
                            .map(|p| p[0].mul(I::point(1. - t))?.add(p[1].mul(I::point(t))?))
                            .collect::<Result<Vec<_>>>()
                            .unwrap();
                    }
                    value.push(row[0]);
                }
                let actual = c
                    .evaluate(2. + 5. * (range[0] + (range[1] - range[0]) * t))
                    .unwrap();
                for k in 0..2 {
                    let bound = value[k].div_signed(value[2]).unwrap();
                    assert!(
                        bound.lo - actual.rounding_bound <= actual.point[k]
                            && actual.point[k] <= bound.hi + actual.rounding_bound
                    );
                }
            }
            assert!(ledger.used > 0 && ledger.used <= ledger.limit);
        }
        assert_eq!(c, before);
        assert!(pcurve(&c, [0.25, 0.75], [[0., 1.]; 2], &mut work()).is_err());
        let mut small = Work { used: 0, limit: 1 };
        assert!(pcurve(&c, [0., 0.25], [[0., 1.]; 2], &mut small).is_err());
        assert!(small.used <= small.limit);
        let jump = Curve {
            degree: 1,
            knots: vec![0., 0., 0.5, 0.5, 1., 1.],
            control_points: vec![vec![0., 0.], vec![0.25, 0.], vec![0.75, 0.], vec![1., 0.]],
            weights: vec![1.; 4],
            periodic: false,
        };
        assert!(pcurve(&jump, [0., 0.25], [[0., 1.]; 2], &mut work()).is_err());
    }
    #[test]
    fn extraction_of_quadratic_c2_span_is_partition_of_unity() {
        let k = [0., 0., 0., 0.3, 0.8, 1., 1., 1.];
        for span in 2..5 {
            let matrix = extraction(&k, 2, span, &mut work()).unwrap();
            for row in matrix {
                assert_eq!(row.iter().cloned().sum::<Q>(), Q::one());
                assert!(row.iter().all(|x| *x >= Q::zero()));
            }
        }
    }
    #[test]
    fn resource_exhaustion_does_not_return_coefficients() {
        let mut w = Work { used: 0, limit: 1 };
        assert!(extraction(&[0., 0., 0., 1., 1., 1.], 2, 2, &mut w).is_err());
        assert!(w.used <= w.limit);
    }
    #[test]
    fn composed_original_multispan_derivatives_reproduce_quadratic() {
        // S=(u,v,u^2), quadratic clamped u with one simple knot.
        // u^2 has B-spline coefficients product of its two Greville knots.
        let knots = vec![0., 0., 0., 0.25, 1., 1., 1.];
        let gu = [0., 0.125, 0.625, 1.];
        let z = [0., 0., 0.25, 1.];
        let s = crate::surface::Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: knots,
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..4)
                .map(|i| (0..2).map(|j| vec![gu[i], j as f64, z[i]]).collect())
                .collect(),
            weights: vec![vec![1.; 2]; 4],
            periodic_u: false,
            periodic_v: false,
        };
        let c = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![0.1, 0.25], vec![0.2, 0.75]],
            weights: vec![1.; 2],
            periodic: false,
        };
        let mut w = work();
        let support = Support::new(&s, &mut w).unwrap();
        let hulls = support.hulls().unwrap();
        assert_eq!(hulls[3][2], I::point(2.));
        let jets = support.compose(&c, [0., 1.], [2, 1], &mut w).unwrap();
        let initial_work = w.used;
        let replay = support.compose(&c, [0., 1.], [2, 1], &mut w).unwrap();
        assert_eq!(
            jets, replay,
            "A cached span must retain original coefficient enclosures"
        );
        assert!(
            w.used > initial_work,
            "Cache hits still consume composition work"
        );
        let mut exhausted = Work {
            used: w.used,
            limit: w.used,
        };
        assert!(
            support
                .compose(&c, [0., 1.], [2, 1], &mut exhausted)
                .is_err()
        );
        // Z=x+y+z, so Z_uu=2 and Z_uv=Z_vv=0, W=1.
        for i in &jets[3][2] {
            assert!(i.lo <= 2. && i.hi >= 2., "{i:?}");
        }
        for index in [4, 5] {
            for k in 0..4 {
                for i in &jets[index][k] {
                    assert!(i.lo <= 0. && i.hi >= 0.);
                }
            }
        }
        // An independent evaluation of composed first derivative at endpoints.
        for (endpoint, u) in [0.1, 0.2].into_iter().enumerate() {
            let i = if endpoint == 0 {
                0
            } else {
                jets[1][2].len() - 1
            };
            assert!(jets[1][2][i].lo <= 1. + 2. * u && jets[1][2][i].hi >= 1. + 2. * u);
        }
    }
