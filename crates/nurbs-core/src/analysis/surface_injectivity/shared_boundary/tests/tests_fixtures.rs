    use super::*;
    #[test]
    #[cfg(feature = "codec")]
    fn curved_outer_strips_require_independent_full_range_cover_and_original_budget() {
        let fixture:value_codec::Value=value_codec::from_str(include_str!("../periodic-authored-outer-strips.json")).unwrap();
        let surfaces:[Surface;2]=value_codec::from_value(fixture["surfaces"].clone()).unwrap();
        let ends:[bool;2]=value_codec::from_value(fixture["ends"].clone()).unwrap();
        let original=surfaces.clone();
        assert!(!certify_shared_natural_boundary(&surfaces[0],&surfaces[1],ends,6).unwrap().proven);
        let complete=certify_shared_natural_boundary(&surfaces[0],&surfaces[1],ends,1000).unwrap();
        assert!(complete.proven,"{}",complete.reason);assert!(complete.spans>6 && complete.spans<=1000);
        assert!(certify_shared_natural_boundary(&surfaces[0],&surfaces[1],ends,complete.spans).unwrap().proven);
        for budget in [0,5,6,complete.spans-1] {
            let partial=certify_shared_natural_boundary(&surfaces[0],&surfaces[1],ends,budget).unwrap();
            assert!(!partial.proven);assert!(partial.spans<=budget);
        }
        let mut changed=surfaces[1].clone();changed.control_points[0][0][0]=changed.control_points[0][0][0].next_up();
        assert!(!certify_shared_natural_boundary(&surfaces[0],&changed,ends,1000).unwrap().proven);
        let overlap=certify_shared_natural_boundary(&surfaces[0],&surfaces[0],[true,true],1000).unwrap();
        assert!(!overlap.proven);assert!(overlap.spans<=1000);
        assert_eq!(surfaces,original);
    }
    #[test]
    #[cfg(feature = "codec")]
    fn retained_spatial_edge_regressions_require_full_cover_and_exact_shared_curve() {
        let fixture: value_codec::Value =
            value_codec::from_str(include_str!("../retained-spatial-edges.json"))
                .unwrap();
        for case in fixture["cases"].as_array().unwrap() {
            let surfaces: [Surface; 2] = value_codec::from_value(case["surfaces"].clone()).unwrap();
            let ends: [bool; 2] = value_codec::from_value(case["ends"].clone()).unwrap();
            let original = surfaces.clone();
            let proof =
                certify_shared_natural_boundary(&surfaces[0], &surfaces[1], ends, 6).unwrap();
            assert!(proof.proven, "faces={:?} {proof:?}", case["faces"]);
            assert_eq!(proof.spans, 6);
            assert!(
                !certify_shared_natural_boundary(&surfaces[0], &surfaces[1], ends, 5)
                    .unwrap()
                    .proven
            );
            assert!(
                !certify_shared_natural_boundary(&surfaces[0], &surfaces[0], [ends[0]; 2], 6)
                    .unwrap()
                    .proven
            );
            let mut changed = surfaces[1].clone();
            let row = if ends[1] { changed.degree_u } else { 0 };
            changed.control_points[row][1][0] = changed.control_points[row][1][0].next_up();
            assert!(
                !certify_shared_natural_boundary(&surfaces[0], &changed, ends, 6)
                    .unwrap()
                    .proven
            );
            assert_eq!(surfaces, original);
        }
    }
    #[test]
    fn full_jacobian_hull_can_exclude_singular_means_when_contraction_is_unproved() {
        let j = [
            [
                I::point(1.),
                I {
                    lo: -100.,
                    hi: -0.1,
                },
            ],
            [I { lo: 0.1, hi: 100. }, I::point(1.)],
        ];
        assert!(weighted_contraction(j).unwrap().unwrap() > 1.);
        assert!(jacobian_hull_nonsingular(j).unwrap());
        let bad = [
            j[0],
            [
                I {
                    lo: -100.,
                    hi: -0.1,
                },
                j[1][1],
            ],
        ];
        assert!(!jacobian_hull_nonsingular(bad).unwrap());
        assert!(!jacobian_hull_nonsingular([[I::point(1.); 2]; 2]).unwrap());
    }
    #[test]
    fn spatial_left_inverse_is_validated_against_all_original_coordinates() {
        let j = [
            [I::point(1.), I::point(0.)],
            [I::point(0.), I::point(1.)],
            [I::point(1.), I::point(1.)],
        ];
        assert!(spatial_contraction(j).unwrap().unwrap() < 1e-12);
        assert!(
            spatial_contraction([[I::point(1.); 2]; 3])
                .unwrap()
                .is_none()
        );
        let uncertain = j.map(|row| {
            row.map(|v| I {
                lo: v.lo - 4.,
                hi: v.hi + 4.,
            })
        });
        assert!(spatial_contraction(uncertain).unwrap().unwrap() > 1.);
    }
    #[test]
    fn moving_polynomial_origin_tightens_only_the_unchanged_transverse_derivative() {
        let offsets = [[-0.2, 0., 0.], [-0.2, 0.2, 0.], [0., 0.2, 0.]];
        let mut s = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: offsets
                .into_iter()
                .map(|p| {
                    (0..2)
                        .map(|j| {
                            vec![
                                p[0] + 15. * j as f64,
                                p[1] + 30. * j as f64,
                                p[2] + 45. * j as f64,
                            ]
                        })
                        .collect()
                })
                .collect(),
            weights: vec![
                vec![1.; 2],
                vec![std::f64::consts::FRAC_1_SQRT_2; 2],
                vec![1.; 2],
            ],
            periodic_u: false,
            periodic_v: false,
        };
        let original = s.clone();
        let domain = [[0.875, 1.], [0., 1.]];
        let baseline = section_jacobian(&s, [2, 1], domain).unwrap();
        let improved = joined_jacobian(&s, domain, true).unwrap();
        assert!(improved[0][0].hi - improved[0][0].lo < baseline[0][0].hi - baseline[0][0].lo);
        for (k, expected) in [15., 30., 45.].into_iter().enumerate() {
            assert!(improved[k][1].lo <= expected && expected <= improved[k][1].hi);
        }
        // V-dependent weights invalidate the polynomial translation identity.
        s.weights[0][1] = 0.5;
        let fallback = joined_jacobian(&s, domain, true).unwrap();
        let source = section_jacobian(&s, [2, 1], domain).unwrap();
        for k in 0..3 {
            for axis in 0..2 {
                assert_eq!(
                    [fallback[k][axis].lo, fallback[k][axis].hi],
                    [source[k][axis].lo, source[k][axis].hi]
                );
            }
        }
        assert_eq!(original.control_points, s.control_points);
    }
    #[test]
    fn oblique_strip_proposal_still_requires_strict_original_interval_gap() {
        let a = [0., 4., 8.].map(|x| [I::point(x), I::point(x), I::point(0.)]);
        let b = [0., 4., 8.].map(|x| [I::point(x), I::point(x + 1.), I::point(0.)]);
        assert!(separate(&a, &b).unwrap());
        assert!(!separate(&a, &a).unwrap());
        let mut cross = b;
        cross[1][1] = I::point(3.);
        assert!(!separate(&a, &cross).unwrap());
        let uncertain = b.map(|p| {
            [
                p[0],
                I {
                    lo: p[1].lo - 2.,
                    hi: p[1].hi + 2.,
                },
                p[2],
            ]
        });
        assert!(!separate(&a, &uncertain).unwrap());
    }
    #[test]
    fn weighted_metric_proves_anisotropic_hull_without_ignoring_diagonal_failure() {
        let j = [
            [I::point(1.), I { lo: -4., hi: 4. }],
            [
                I {
                    lo: -0.01,
                    hi: 0.01,
                },
                I::point(1.),
            ],
        ];
        assert!(super::super::contraction(j).unwrap().unwrap() > 1.);
        let upper = weighted_contraction(j).unwrap().unwrap();
        assert!(upper >= 0.2 && upper < 0.201);
        let bad = [[I { lo: -1., hi: 3. }, j[0][1]], j[1]];
        assert!(weighted_contraction(bad).unwrap().unwrap() > 1.);
        assert!(
            weighted_contraction([[I::point(0.); 2]; 2])
                .unwrap()
                .is_none()
        );
    }
    fn face(positive: bool) -> Surface {
        let edge = [
            [0., 0., 0.],
            [0.01, 0.01, 0.3],
            [0.02, -0.01, 0.7],
            [0.05, 0., 1.],
        ];
        let shifted = edge
            .iter()
            .enumerate()
            .map(|(i, p)| {
                vec![
                    p[0] + if positive { 0.2 } else { -0.2 },
                    p[1] + if positive {
                        0.01 * i as f64
                    } else {
                        -0.01 * i as f64
                    },
                    p[2] + if positive {
                        0.01 * i as f64
                    } else {
                        -0.01 * i as f64
                    },
                ]
            })
            .collect();
        let shared = edge.iter().map(|p| p.to_vec()).collect();
        Surface {
            degree_u: 1,
            degree_v: 3,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: if positive {
                vec![shared, shifted]
            } else {
                vec![shifted, shared]
            },
            weights: vec![vec![1.; 4]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn nonplanar_shared_edge_needs_complete_pair_cover_and_joint_contraction() {
        let a = face(false);
        let b = face(true);
        let original = a.clone();
        let proof = certify_shared_natural_boundary(&a, &b, [true, false], 6).unwrap();
        assert!(proof.proven, "{proof:?}");
        assert_eq!(proof.spans, 6);
        assert!(
            !certify_shared_natural_boundary(&a, &b, [true, false], 5)
                .unwrap()
                .proven
        );
        let mut wrong = b.clone();
        wrong.control_points[0][1][0] += f64::EPSILON;
        assert!(
            !certify_shared_natural_boundary(&a, &wrong, [true, false], 6)
                .unwrap()
                .proven
        );
        let mut overlap = b.clone();
        overlap.control_points[1] = a.control_points[0].clone();
        assert!(
            !certify_shared_natural_boundary(&a, &overlap, [true, false], 6)
                .unwrap()
                .proven
        );
        assert_eq!(a, original);
    }

    #[test]
    fn reversed_natural_ends_and_positive_rational_weights_keep_source_proof() {
        let mut a = face(false);
        let mut b = face(true);
        for s in [&mut a, &mut b] {
            s.weights = vec![vec![1., 1.0001, 0.9999, 1.]; 2];
            s.control_points.reverse();
            s.weights.reverse();
        }
        assert!(
            certify_shared_natural_boundary(&a, &b, [false, true], 6)
                .unwrap()
                .proven
        );
        assert!(
            !certify_shared_natural_boundary(&a, &b, [true, false], 6)
                .unwrap()
                .proven
        );
        let mut folded = b.clone();
        folded.control_points[0][1][0] = -1.;
        assert!(
            !certify_shared_natural_boundary(&a, &folded, [false, true], 6)
                .unwrap()
                .proven
        );
    }
