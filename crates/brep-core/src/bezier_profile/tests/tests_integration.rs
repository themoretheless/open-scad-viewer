    use planar_geometry::path::{BezierPath, PathSegment};
    fn concave_path() -> BezierPath {
        BezierPath {
            start: [0., 0.],
            closed: true,
            segments: vec![
                PathSegment::Cubic {
                    c1: [2., 0.],
                    c2: [4., 0.],
                    to: [6., 0.],
                },
                PathSegment::Line { to: [6., 2.] },
                PathSegment::Line { to: [2., 2.] },
                PathSegment::Line { to: [2., 6.] },
                PathSegment::Line { to: [0., 6.] },
                PathSegment::Line { to: [0., 0.] },
            ],
        }
    }
    #[test]
    fn curved_partial_overlap_boolean_retains_source_cubics() {
        let top = crate::Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![1., 2.], vec![2., 1.], vec![3., 3.]],
            weights: vec![1.; 4],
            periodic: false,
        };
        let region = |c: crate::Curve| {
            let a = c.control_points[0].clone();
            let b = c.control_points.last().unwrap().clone();
            vec![
                crate::Curve::from_polyline(vec![vec![a[0], -2.], vec![b[0], -2.]]).unwrap(),
                crate::Curve::from_polyline(vec![vec![b[0], -2.], b]).unwrap(),
                c.reverse().unwrap(),
                crate::Curve::from_polyline(vec![a.clone(), vec![a[0], -2.]]).unwrap(),
            ]
        };
        let a = vec![region(top.clone())];
        let b = vec![region(top.trim(0.25, 0.75).unwrap())];
        let aa = super::validate_profile(&a, 1e-7).unwrap();
        let bb = super::validate_profile(&b, 1e-7).unwrap();
        for (op, expected) in [("union", aa), ("intersection", bb), ("difference", aa - bb)] {
            let result = super::boolean(&a, &b, op, 1e-7).unwrap_or_else(|e| panic!("{op}: {e:?}"));
            assert!((super::validate_profile(&result, 1e-7).unwrap() - expected).abs() < 1e-5);
            assert!(result.iter().flatten().any(|c| c.degree == 3));
        }
        let normalized = super::normalize(&[a[0].clone(), b[0].clone()], 1e-7).unwrap();
        assert_eq!(normalized.len(), 2);
        assert!((super::validate_profile(&normalized, 1e-7).unwrap() - (aa - bb)).abs() < 1e-5);
    }
    #[test]
    fn partial_curved_coincidence_with_different_injective_axes_keeps_material() {
        let c = crate::Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![6., -2.], vec![11., -1.], vec![7., 11.]],
            weights: vec![1.; 4],
            periodic: false,
        };
        let ac = c.trim(0., 0.625).unwrap();
        let bc = c.trim(0.375, 1.).unwrap();
        let line = |a: Vec<f64>, b: Vec<f64>| crate::Curve::from_polyline(vec![a, b]).unwrap();
        let ap = ac.control_points[0].clone();
        let az = ac.control_points.last().unwrap().clone();
        let bp = bc.control_points[0].clone();
        let bz = bc.control_points.last().unwrap().clone();
        let a = vec![vec![
            line(vec![ap[0], -3.], vec![az[0], -3.]),
            line(vec![az[0], -3.], az),
            ac.reverse().unwrap(),
            line(ap.clone(), vec![ap[0], -3.]),
        ]];
        let b = vec![vec![
            line(vec![-2., bp[1]], bp),
            bc,
            line(bz.clone(), vec![-2., bz[1]]),
            line(vec![-2., bz[1]], vec![-2., -0.5625]),
        ]];
        let aa = super::validate_profile(&a, 1e-7).unwrap();
        let bb = super::validate_profile(&b, 1e-7).unwrap();
        let union = super::boolean(&a, &b, "union", 1e-7).unwrap();
        let intersection = super::boolean(&a, &b, "intersection", 1e-7).unwrap();
        let difference = super::boolean(&a, &b, "difference", 1e-7).unwrap();
        let ia = super::validate_profile(&intersection, 1e-7).unwrap();
        assert!((super::validate_profile(&union, 1e-7).unwrap() + ia - aa - bb).abs() < 1e-5);
        assert!((super::validate_profile(&difference, 1e-7).unwrap() + ia - aa).abs() < 1e-5);
        crate::prism::extrude(&union, 0., 2.)
            .unwrap()
            .validate()
            .unwrap();
        let xy = |t: f64| {
            [
                18. * t - 3. * t * t - 8. * t * t * t,
                -6. * t + 9. * t * t + 8. * t * t * t,
            ]
        };
        let inverse = |axis: usize, value: f64, mut lo: f64, mut hi: f64| {
            for _ in 0..60 {
                let mid = (lo + hi) * 0.5;
                if xy(mid)[axis] < value {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            (lo + hi) * 0.5
        };
        for i in 0..20 {
            for j in 0..20 {
                let p = [-2. + i as f64 * 0.57 + 0.137, -3. + j as f64 * 0.69 + 0.193];
                let ina = p[0] > 0.
                    && p[0] < 8.125
                    && p[1] > -3.
                    && p[1] < xy(inverse(0, p[0], 0., 0.625))[1];
                let inb = p[1] > -0.5625
                    && p[1] < 11.
                    && p[0] > -2.
                    && p[0] < xy(inverse(1, p[1], 0.375, 1.))[0];
                for (result, expected) in [
                    (&union, ina || inb),
                    (&intersection, ina && inb),
                    (&difference, ina && !inb),
                ] {
                    let spans = result.iter().flatten().cloned().collect::<Vec<_>>();
                    assert_eq!(
                        super::point_inside(&spans, p, 1e-7).unwrap(),
                        expected,
                        "{p:?}"
                    );
                }
            }
        }
    }
    #[test]
    fn tangent_cusp_components_use_departure_order_instead_of_equal_tangents() {
        let lobe = |sign: f64| {
            crate::sketch::bezier_wire(BezierPath {
                start: [0., 0.],
                closed: true,
                segments: vec![
                    PathSegment::Cubic {
                        c1: [sign * 2. / 3., 0.],
                        c2: [sign * 4. / 3., 4. / 3.],
                        to: [sign * 2., 4.],
                    },
                    PathSegment::Line {
                        to: [sign * 2., -4.],
                    },
                    PathSegment::Cubic {
                        c1: [sign * 4. / 3., -4. / 3.],
                        c2: [sign * 2. / 3., 0.],
                        to: [0., 0.],
                    },
                ],
            })
            .unwrap()
        };
        let mut source = lobe(1.);
        source.extend(lobe(-1.));
        let result = super::normalize(&[source], 1e-7).unwrap();
        assert_eq!(result.len(), 2);
        assert!((super::validate_profile(&result, 1e-7).unwrap() - 32. / 3.).abs() < 1e-5);
        assert!(result.iter().flatten().filter(|c| c.degree == 3).count() >= 4);
    }
    #[test]
    fn zero_first_jets_at_self_touching_cusps_keep_separate_regions() {
        let lobe = |sign: f64| {
            crate::sketch::bezier_wire(BezierPath {
                start: [0., 0.],
                closed: true,
                segments: vec![
                    PathSegment::Cubic {
                        c1: [0., 0.],
                        c2: [sign, 0.],
                        to: [sign * 2., 4.],
                    },
                    PathSegment::Line {
                        to: [sign * 2., -4.],
                    },
                    PathSegment::Cubic {
                        c1: [sign, 0.],
                        c2: [0., 0.],
                        to: [0., 0.],
                    },
                ],
            })
            .unwrap()
        };
        let mut source = lobe(1.);
        source.extend(lobe(-1.));
        let result = super::normalize(&[source], 1e-7).unwrap();
        assert_eq!(result.len(), 2);
        assert!((super::validate_profile(&result, 1e-7).unwrap() - 11.2).abs() < 1e-5);
        crate::prism::extrude(&result, 0., 2.)
            .unwrap()
            .validate()
            .unwrap();
    }
    #[test]
    fn diagonal_self_touching_cusps_keep_the_same_topology_after_affine_transform() {
        let lobe = |sign: f64| {
            crate::sketch::bezier_wire(BezierPath {
                start: [0., 0.],
                closed: true,
                segments: vec![
                    PathSegment::Cubic {
                        c1: [0., 0.],
                        c2: [sign, 0.],
                        to: [sign * 2., 4.],
                    },
                    PathSegment::Line {
                        to: [sign * 2., -4.],
                    },
                    PathSegment::Cubic {
                        c1: [sign, 0.],
                        c2: [0., 0.],
                        to: [0., 0.],
                    },
                ],
            })
            .unwrap()
        };
        let mut wire = lobe(1.);
        wire.extend(lobe(-1.));
        for c in &mut wire {
            for p in &mut c.control_points {
                let [x, y] = [p[0], p[1]];
                p[0] = 17. + x - y;
                p[1] = -9. + x + y;
            }
        }
        let result = super::normalize(&[wire], 1e-7).unwrap();
        assert_eq!(result.len(), 2);
        assert!((super::validate_profile(&result, 1e-7).unwrap() - 22.4).abs() < 1e-5);
    }
    #[test]
    fn monotone_cubic_with_reversing_controls_keeps_exact_profile_and_overlap() {
        use crate::Curve;
        let c = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![1., 1.1], vec![0.9, 0.8], vec![1.9, 2.1]],
            weights: vec![1.; 4],
            periodic: false,
        };
        let region = |c: &Curve| {
            let a = c.control_points[0].clone();
            let b = c.control_points.last().unwrap().clone();
            let line = |a, b| Curve::from_polyline(vec![a, b]).unwrap();
            vec![
                line(vec![a[0], -1.], vec![b[0], -1.]),
                line(vec![b[0], -1.], b),
                c.reverse().unwrap(),
                line(a.clone(), vec![a[0], -1.]),
            ]
        };
        let a = vec![region(&c)];
        assert!((super::validate_profile(&a, 1e-7).unwrap() - 3.811).abs() < 1e-10);
        let normalized = super::normalize(&a, 1e-7).unwrap();
        assert!((super::validate_profile(&normalized, 1e-7).unwrap() - 3.811).abs() < 1e-10);
        crate::prism::extrude(&normalized, 0., 1.5)
            .unwrap()
            .validate()
            .unwrap();
        let b = vec![region(&c.trim(0.25, 0.75).unwrap())];
        let aa = super::validate_profile(&a, 1e-7).unwrap();
        let bb = super::validate_profile(&b, 1e-7).unwrap();
        for (op, area) in [("union", aa), ("intersection", bb), ("difference", aa - bb)] {
            let result = super::boolean(&a, &b, op, 1e-7).unwrap();
            assert!((super::validate_profile(&result, 1e-7).unwrap() - area).abs() < 1e-6);
            assert!(result.iter().flatten().any(|c| c.degree == 3));
        }
    }
    #[test]
    fn affine_curved_tangency_boolean_preserves_both_retained_carriers() {
        use crate::Curve;
        let region = |ys: [f64; 4]| {
            let c = Curve {
                degree: 3,
                knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
                control_points: (0..4).map(|i| vec![10. * i as f64, ys[i]]).collect(),
                weights: vec![1.; 4],
                periodic: false,
            };
            let line = |a, b| Curve::from_polyline(vec![a, b]).unwrap();
            let mut wire = vec![
                line(vec![0., -2.], vec![30., -2.]),
                line(vec![30., -2.], vec![30., ys[3]]),
                c.reverse().unwrap(),
                line(vec![0., ys[0]], vec![0., -2.]),
            ];
            for c in &mut wire {
                for p in &mut c.control_points {
                    let [x, y] = [p[0], p[1]];
                    p[0] = 17. + x - y;
                    p[1] = -9. + x + y;
                }
            }
            wire
        };
        let a = vec![region([0., 0., 1., 3.])];
        let b = vec![region([1., -1., 1., 7.])];
        assert!((super::validate_profile(&a, 1e-7).unwrap() - 180.).abs() < 1e-6);
        assert!((super::validate_profile(&b, 1e-7).unwrap() - 240.).abs() < 1e-6);
        for (operation, area) in [("union", 240.), ("intersection", 180.), ("difference", 0.)] {
            let result = super::boolean(&a, &b, operation, 1e-7).unwrap();
            assert!((super::validate_profile(&result, 1e-7).unwrap() - area).abs() < 1e-5);
            if area > 0. {
                assert!(result.iter().flatten().any(|c| c.degree == 3));
            }
        }
    }
    #[test]
    fn interior_semicubical_cusp_splits_original_curve_without_chords() {
        let c = crate::Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![vec![3., -1.], vec![-1., 1.], vec![-1., -1.], vec![3., 1.]],
            weights: vec![1.; 4],
            periodic: false,
        };
        let line = crate::Curve::from_polyline(vec![vec![3., 1.], vec![3., -1.]]).unwrap();
        let result = super::normalize(&[vec![c, line]], 1e-7).unwrap();
        assert_eq!(result.len(), 1);
        assert!((super::validate_profile(&result, 1e-7).unwrap() - 2.4).abs() < 1e-5);
        assert_eq!(result[0].iter().filter(|c| c.degree == 3).count(), 2);
        crate::prism::extrude(&result, 0., 2.)
            .unwrap()
            .validate()
            .unwrap();
    }
    #[test]
    fn non_dyadic_cusp_keeps_its_material_area() {
        let c = crate::Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![vec![1., -1.], vec![-1., 2.], vec![0., -4.], vec![4., 8.]],
            weights: vec![1.; 4],
            periodic: false,
        };
        let line = crate::Curve::from_polyline(vec![vec![4., 8.], vec![1., -1.]]).unwrap();
        let result = super::normalize(&[vec![c, line]], 1e-7).unwrap();
        assert_eq!(result.len(), 1);
        assert!((super::validate_profile(&result, 1e-7).unwrap() - 2.7).abs() < 1e-5);
    }
    #[test]
    fn a_retraced_spur_cancels_without_removing_the_curved_region() {
        let mut wire =
            crate::sketch::bezier_wire(BezierPath::from_circle([0., 0.], 2.).unwrap()).unwrap();
        let expected = super::validate_single_wire(&wire, 1e-7).unwrap().abs();
        let spur = crate::Curve::from_polyline(vec![vec![2., 0.], vec![3., 1.]]).unwrap();
        wire.extend([spur.clone(), spur.reverse().unwrap()]);
        let result = super::normalize(&[wire], 1e-7).unwrap();
        assert_eq!(result.len(), 1);
        assert!((super::validate_profile(&result, 1e-7).unwrap() - expected).abs() < 1e-5);
    }
    #[test]
    fn constant_cubic_passes_have_no_even_odd_material() {
        let point = crate::Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![vec![2., 0.]; 4],
            weights: vec![1.; 4],
            periodic: false,
        };
        assert!(
            super::normalize(&[vec![point.clone()]], 1e-7)
                .unwrap()
                .is_empty()
        );
        let mut circle =
            crate::sketch::bezier_wire(BezierPath::from_circle([0., 0.], 2.).unwrap()).unwrap();
        let expected = super::validate_single_wire(&circle, 1e-7).unwrap().abs();
        circle.push(point);
        let result = super::normalize(&[circle], 1e-7).unwrap();
        assert!((super::validate_profile(&result, 1e-7).unwrap() - expected).abs() < 1e-5);
    }
    #[test]
    fn retraced_straight_cubic_with_nonlinear_speed_cancels_a_line() {
        let a = crate::sketch::bezier_wire(BezierPath {
            start: [0., 0.],
            closed: true,
            segments: vec![
                PathSegment::Cubic {
                    c1: [0.5, 0.],
                    c2: [2.5, 0.],
                    to: [3., 0.],
                },
                PathSegment::Line { to: [3., 2.] },
                PathSegment::Line { to: [0., 2.] },
                PathSegment::Line { to: [0., 0.] },
            ],
        })
        .unwrap();
        let b = crate::sketch::bezier_wire(
            BezierPath::from_polygon(&[[0., 0.], [3., 0.], [3., 2.], [0., 2.]], true).unwrap(),
        )
        .unwrap();
        assert!(
            super::normalize(&[a.clone(), b.clone()], 1e-7)
                .unwrap()
                .is_empty()
        );
        let result = super::normalize(&[a, b.clone(), b], 1e-7).unwrap();
        assert_eq!(result.len(), 1);
        assert!((super::validate_profile(&result, 1e-7).unwrap() - 6.).abs() < 1e-5);
    }
    #[test]
    fn repeated_curved_boundaries_cancel_by_even_odd_multiplicity() {
        let a = crate::sketch::bezier_wire(BezierPath::from_circle([0., 0.], 2.).unwrap()).unwrap();
        let expected = super::validate_single_wire(&a, 1e-7).unwrap().abs();
        assert!(
            super::normalize(&[a.clone(), a.clone()], 1e-7)
                .unwrap()
                .is_empty()
        );
        let reversed = a
            .iter()
            .rev()
            .map(|c| c.reverse().unwrap())
            .collect::<Vec<_>>();
        let result = super::normalize(&[a.clone(), reversed, a.clone()], 1e-7).unwrap();
        assert_eq!(result.len(), 1);
        assert!((super::validate_profile(&result, 1e-7).unwrap() - expected).abs() < 1e-5);
    }
    #[test]
    fn third_order_contact_boolean_preserves_cubic_cusp_and_area() {
        let a = vec![
            crate::sketch::bezier_wire(BezierPath {
                start: [0., -2.],
                closed: true,
                segments: vec![
                    PathSegment::Line { to: [3., -2.] },
                    PathSegment::Line { to: [3., 1.] },
                    PathSegment::Cubic {
                        c1: [2., -1.],
                        c2: [1., 1.],
                        to: [0., -1.],
                    },
                    PathSegment::Line { to: [0., -2.] },
                ],
            })
            .unwrap(),
        ];
        let b = vec![
            crate::sketch::bezier_wire(
                BezierPath::from_polygon(&[[0., -2.], [3., -2.], [3., 0.], [0., 0.]], true)
                    .unwrap(),
            )
            .unwrap(),
        ];
        for (op, expected) in [
            ("union", 6.375),
            ("intersection", 5.625),
            ("difference", 0.375),
        ] {
            let result = super::boolean(&a, &b, op, 1e-7).unwrap_or_else(|e| panic!("{op}: {e:?}"));
            let area = super::validate_profile(&result, 1e-7).unwrap();
            assert!((area - expected).abs() < 1e-5, "{op}: {area}");
            assert!(result.iter().flatten().any(|c| c.degree == 3));
        }
    }
    #[test]
    fn partial_line_cubic_overlap_deduplicates_different_degrees() {
        let a = vec![
            crate::sketch::bezier_wire(BezierPath {
                start: [0., 0.],
                closed: true,
                segments: vec![
                    PathSegment::Cubic {
                        c1: [2., 0.],
                        c2: [4., 0.],
                        to: [6., 0.],
                    },
                    PathSegment::Line { to: [6., 4.] },
                    PathSegment::Line { to: [0., 4.] },
                    PathSegment::Line { to: [0., 0.] },
                ],
            })
            .unwrap(),
        ];
        let b = vec![
            crate::sketch::bezier_wire(
                BezierPath::from_polygon(&[[2., 0.], [8., 0.], [8., 4.], [2., 4.]], true).unwrap(),
            )
            .unwrap(),
        ];
        for (op, expected) in [("union", 32.), ("intersection", 16.), ("difference", 8.)] {
            let result = super::boolean(&a, &b, op, 1e-7).unwrap_or_else(|e| panic!("{op}: {e:?}"));
            assert!((super::validate_profile(&result, 1e-7).unwrap() - expected).abs() < 1e-5);
        }
    }
    #[test]
    fn partial_collinear_cubic_overlap_keeps_union_and_difference_area() {
        let rectangle = |x: f64| {
            crate::sketch::bezier_wire(BezierPath {
                start: [x, 0.],
                closed: true,
                segments: vec![
                    PathSegment::Cubic {
                        c1: [x + 2., 0.],
                        c2: [x + 4., 0.],
                        to: [x + 6., 0.],
                    },
                    PathSegment::Line { to: [x + 6., 4.] },
                    PathSegment::Line { to: [x, 4.] },
                    PathSegment::Line { to: [x, 0.] },
                ],
            })
            .unwrap()
        };
        let a = vec![rectangle(0.)];
        let b = vec![rectangle(2.)];
        for (op, area) in [("union", 32.), ("intersection", 16.), ("difference", 8.)] {
            let result = super::boolean(&a, &b, op, 1e-7).unwrap_or_else(|e| panic!("{op}: {e:?}"));
            assert!((super::validate_profile(&result, 1e-7).unwrap() - area).abs() < 1e-5);
        }
    }
    #[test]
    fn external_endpoint_tangency_has_empty_intersection() {
        let a = vec![
            crate::sketch::bezier_wire(BezierPath::from_circle([0., 0.], 5.).unwrap()).unwrap(),
        ];
        let b = vec![
            crate::sketch::bezier_wire(BezierPath::from_circle([10., 0.], 5.).unwrap()).unwrap(),
        ];
        let united = super::boolean(&a, &b, "union", 1e-7).unwrap();
        assert_eq!(united.len(), 2);
        assert!(
            super::boolean(&a, &b, "intersection", 1e-7)
                .unwrap()
                .is_empty()
        );
        let result = super::boolean(&a, &b, "difference", 1e-7).unwrap();
        assert!(
            (super::validate_profile(&result, 1e-7).unwrap()
                - super::signed_area(&a[0], 1e-7).unwrap().abs())
            .abs()
                < 1e-5
        );
    }
    #[test]
    fn self_crossing_even_odd_profile_splits_into_retained_components() {
        let path = BezierPath {
            start: [0., 0.],
            closed: true,
            segments: vec![
                PathSegment::Cubic {
                    c1: [4. / 3., 4. / 3.],
                    c2: [8. / 3., 8. / 3.],
                    to: [4., 4.],
                },
                PathSegment::Line { to: [0., 4.] },
                PathSegment::Line { to: [4., 0.] },
                PathSegment::Line { to: [0., 0.] },
            ],
        };
        let wire = crate::sketch::bezier_wire(path).unwrap();
        let result = super::normalize(&[wire], 1e-7).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.iter().flatten().any(|c| c.degree == 3));
        assert!((super::validate_profile(&result, 1e-7).unwrap() - 8.).abs() < 1e-6);
        crate::prism::extrude(&result, 0., 2.)
            .unwrap()
            .validate()
            .unwrap();
    }
    #[test]
    fn mixed_cubic_and_rational_circle_boolean_retains_both_carriers() {
        let a = vec![
            crate::sketch::bezier_wire(BezierPath::from_circle([0., 0.], 5.).unwrap()).unwrap(),
        ];
        let mut circle = crate::sketch::circle_wire(5.).unwrap();
        for c in &mut circle {
            for p in &mut c.control_points {
                p[0] += 3.;
            }
        }
        let b = vec![circle];
        for op in ["union", "intersection", "difference"] {
            let result = super::boolean(&a, &b, op, 1e-7).unwrap_or_else(|e| panic!("{op}: {e:?}"));
            assert!(result.iter().flatten().any(|c| c.degree == 3));
            assert!(result.iter().flatten().any(|c| c.degree == 2));
            assert!(super::validate_profile(&result, 1e-7).unwrap() > 0.);
            crate::prism::extrude(&result, 0., 2.)
                .unwrap()
                .validate()
                .unwrap();
        }
    }
    #[test]
    fn transverse_bezier_booleans_retain_cubic_fragments_and_close_solids() {
        let a = vec![
            crate::sketch::bezier_wire(BezierPath::from_circle([0., 0.], 5.).unwrap()).unwrap(),
        ];
        let b = vec![
            crate::sketch::bezier_wire(BezierPath::from_circle([3., 0.], 5.).unwrap()).unwrap(),
        ];
        let mut areas = Vec::new();
        for op in ["union", "intersection", "difference"] {
            let loops = super::boolean(&a, &b, op, 1e-7).unwrap_or_else(|e| panic!("{op}: {e:?}"));
            let area = super::validate_profile(&loops, 1e-7).unwrap();
            areas.push(area);
            assert!(loops.iter().flatten().all(|c| c.degree == 3));
            let solid = crate::prism::extrude(&loops, 0., 2.).unwrap();
            solid.validate().unwrap();
        }
        let original = super::signed_area(&a[0], 1e-7).unwrap().abs();
        assert!((areas[0] + areas[1] - 2. * original).abs() < 1e-5);
        assert!((areas[2] + areas[1] - original).abs() < 1e-5);
        assert!(super::boolean(&a, &b, "xor", 1e-7).is_err());
    }
    #[test]
    fn identical_bezier_booleans_do_not_duplicate_boundaries() {
        let a = vec![
            crate::sketch::bezier_wire(BezierPath::from_circle([0., 0.], 5.).unwrap()).unwrap(),
        ];
        assert_eq!(super::boolean(&a, &a, "union", 1e-7).unwrap()[0].len(), 4);
        assert!(
            super::boolean(&a, &a, "difference", 1e-7)
                .unwrap()
                .is_empty()
        );
        assert!(super::boolean(&a, &a, "xor", 1e-7).unwrap().is_empty());
    }
    #[test]
    fn concave_holes_use_separated_hull_winding_without_replacing_curves() {
        let hole = BezierPath::from_circle([1., 4.], 0.3).unwrap();
        let solid =
            crate::sketch::extrude_bezier_profiles(vec![concave_path(), hole], 2., 0., None)
                .unwrap();
        solid.validate().unwrap();
        assert_eq!(solid.bodies.len(), 1);
        assert!(solid.faces.iter().any(|f| f.holes.len() == 1));
        assert_eq!(
            solid.edges.iter().filter(|e| e.curve.degree == 3).count(),
            10
        );
    }
    #[test]
    fn overlapping_bounds_do_not_imply_nested_or_intersecting_profiles() {
        let outside = BezierPath::from_circle([4., 4.], 0.5).unwrap();
        let solid =
            crate::sketch::extrude_bezier_profiles(vec![concave_path(), outside], 2., 0., None)
                .unwrap();
        solid.validate().unwrap();
        assert_eq!(solid.bodies.len(), 2);
        assert!(solid.faces.iter().all(|f| f.holes.is_empty()));
    }
    #[test]
    fn edited_smooth_joint_uses_proved_adaptive_separator() {
        let path = BezierPath::from_polygon(&[[0., 0.], [20., 0.], [20., 20.], [0., 20.]], true)
            .unwrap()
            .smooth()
            .unwrap()
            .set_anchor_position(1, [30., 0.])
            .unwrap();
        let solid =
            crate::sketch::extrude(crate::sketch::Profile::Bezier(path), true, 5., 0., None)
                .unwrap();
        solid.validate().unwrap();
        assert_eq!(solid.faces.len(), 6);
    }
    #[test]
    fn compound_holes_and_nested_islands_keep_exact_curves() {
        let paths = [10., 4., 1.].map(|r| BezierPath::from_circle([0., 0.], r).unwrap());
        let solid = crate::sketch::extrude_bezier_profiles(paths.into(), -3., 1., None).unwrap();
        solid.validate().unwrap();
        assert_eq!(solid.bodies.len(), 2);
        assert_eq!(solid.faces.len(), 16);
        assert_eq!(
            solid.edges.iter().filter(|e| e.curve.degree == 3).count(),
            24
        );
        assert!(solid.faces.iter().any(|f| f.holes.len() == 1));
        assert!(
            solid
                .vertices
                .iter()
                .all(|v| v.point[2] == -2. || v.point[2] == 1.)
        );
    }
    #[test]
    fn compound_contacts_and_unproved_containment_are_refused() {
        let a = BezierPath::from_circle([0., 0.], 10.).unwrap();
        let b = BezierPath::from_circle([8., 0.], 3.).unwrap();
        assert!(crate::sketch::extrude_bezier_profiles(vec![a, b], 2., 0., None).is_err());
    }
    #[test]
    fn exact_cubic_extrusion_retains_controls_and_cap_trims() {
        let path = BezierPath::from_circle([0., 0.], 10.).unwrap().reverse();
        let solid = crate::sketch::extrude(
            crate::sketch::Profile::Bezier(path.clone()),
            true,
            5.,
            0.,
            None,
        )
        .unwrap();
        solid.validate().unwrap();
        assert_eq!(solid.faces.len(), 6);
        let cubic = solid.edges.iter().find(|e| e.curve.degree == 3).unwrap();
        let PathSegment::Cubic { c1, c2, to } = path.segments[0] else {
            panic!()
        };
        assert_eq!(
            cubic.curve.control_points,
            vec![
                vec![path.start[0], path.start[1], 0.],
                vec![c1[0], c1[1], 0.],
                vec![c2[0], c2[1], 0.],
                vec![to[0], to[1], 0.]
            ]
        );
        assert!(
            solid
                .loops
                .iter()
                .flat_map(|l| &l.coedges)
                .any(|e| e.pcurve.degree == 3)
        );
    }
    #[test]
    fn accepts_clockwise_path_and_negative_height() {
        let path = BezierPath::from_circle([0., 0.], 2.).unwrap();
        let solid =
            crate::sketch::extrude(crate::sketch::Profile::Bezier(path), true, -3., 1., None)
                .unwrap();
        assert!(
            solid
                .vertices
                .iter()
                .all(|v| v.point[2] == 1. || v.point[2] == -2.)
        );
    }
    #[test]
    fn rejects_crossing_and_unresolved_boundaries_without_chording() {
        let path = BezierPath::from_polygon(&[[0., 0.], [10., 10.], [0., 10.], [10., 0.]], true)
            .unwrap()
            .smooth()
            .unwrap();
        assert!(
            crate::sketch::extrude(crate::sketch::Profile::Bezier(path), true, 3., 0., None)
                .is_err()
        );
    }
