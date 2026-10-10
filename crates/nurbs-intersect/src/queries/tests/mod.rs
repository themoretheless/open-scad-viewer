use super::*;
#[test]
fn indeterminate_interval_arithmetic_never_proves_a_sign() {
    let a = Interval {
        lo: 0.,
        hi: f64::INFINITY,
    }
    .mul(Interval::exact(0.));
    let b = Interval::exact(f64::INFINITY).add(Interval::exact(f64::NEG_INFINITY));
    for bound in [a, b] {
        assert_eq!(bound.lo, f64::NEG_INFINITY);
        assert_eq!(bound.hi, f64::INFINITY);
        assert_eq!(bound.sign(), 0);
    }
}
#[test]
fn ruled_range_subdivision_admits_interior_and_boundary_but_refuses_uncertainty() {
    // Mixed control signs need subdivision, although the polynomial stays negative.
    assert!(admit_ruled_parameter_range(&[[-1., 1.], [0.25, 2.25], [-1., 1.]]).is_ok());
    assert!(admit_ruled_parameter_range(&[[0., 2.]; 3]).is_ok());
    assert!(admit_ruled_parameter_range(&[[2., 0.]; 3]).is_ok());
    // Exact dyadic cancellation resolves this boundary tangency without
    // manufacturing an interval error for 0+x, 1*x or x+(-x).
    assert!(admit_ruled_parameter_range(&[[-1., 1.], [1., 3.], [-1., 1.]]).is_ok());
    let uncertain = admit_ruled_parameter_bounds(vec![
        [
            Interval {
                lo: -1e-16,
                hi: 1e-16
            },
            Interval::exact(2.)
        ];
        3
    ])
    .unwrap_err();
    assert_eq!(uncertain.code, "BREP_INTERSECTION_UNRESOLVED");
    assert!(uncertain.to_string().contains("subdivision limit"));
    assert!(admit_ruled_parameter_range(&[[f64::INFINITY, 0.]; 3]).is_err());
}
#[test]
fn residual_construction_cancellation_cannot_authorize_a_ruled_trace() {
    let source = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..2)
            .map(|i| {
                vec![
                    vec![268435456., 1e-8, i as f64],
                    vec![268435456., 4., i as f64],
                ]
            })
            .collect(),
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    // The exact input equation x+y=268435456 has no point on this surface.
    // Rounded point residuals nevertheless vanish on the lower boundary.
    for (axis, surface) in [source.clone(), transpose_surface(&source)]
        .into_iter()
        .enumerate()
    {
        let plane = Plane {
            normal: [1., 1., 0.],
            offset: 268435456.,
        };
        let trace = if axis == 0 {
            SurfaceTrace::Ruled {
                surface,
                plane,
                u_interval: [0., 1.],
            }
        } else {
            SurfaceTrace::RuledU {
                surface,
                plane,
                v_interval: [0., 1.],
            }
        };
        assert_eq!(trace.evaluate(0.5).unwrap().plane_residual, 0.);
        assert_eq!(
            trace.to_curve().unwrap_err().code,
            "BREP_INTERSECTION_UNRESOLVED"
        );
        assert_eq!(
            trace.to_curve_segments().unwrap_err().code,
            "BREP_INTERSECTION_UNRESOLVED"
        );
    }
}
#[test]
fn plane_normalization_preserves_extreme_equivalent_equations() {
    for scale in [f64::from_bits(1), f64::MIN_POSITIVE, 1., f64::MAX] {
        let p = Plane {
            normal: [scale, scale, 0.],
            offset: scale,
        }
        .normalized()
        .unwrap();
        assert!((p.normal[0].hypot(p.normal[1]) - 1.).abs() < 3e-16);
        assert!((p.offset - std::f64::consts::FRAC_1_SQRT_2).abs() < 2e-16);
        assert_eq!(p.distance([1., 0., 0.]), 0.);
    }
    let p = Plane {
        normal: [f64::from_bits(1); 3],
        offset: f64::from_bits(2),
    }
    .normalized()
    .unwrap();
    assert!((p.offset - 2. / 3_f64.sqrt()).abs() < 3e-16);
    // offset/scale overflows here, while the normalized offset is finite.
    let p = Plane {
        normal: [0.75; 3],
        offset: f64::MAX,
    }
    .normalized()
    .unwrap();
    assert!(p.offset.is_finite());
    assert!((p.offset / f64::MAX - 1. / (0.75 * 3_f64.sqrt())).abs() < 3e-16);
    assert!(
        Plane {
            normal: [f64::from_bits(1), 0., 0.],
            offset: 1.
        }
        .normalized()
        .is_err()
    );
    assert!(
        Plane {
            normal: [f64::NAN, 1., 0.],
            offset: 0.
        }
        .normalized()
        .is_err()
    );
}
#[test]
fn curve_plane_roots_are_invariant_under_extreme_equation_scaling() {
    let source = Curve::from_polyline(vec![vec![0., 0., 0.], vec![2., 0., 0.]]).unwrap();
    for magnitude in [f64::from_bits(1), f64::MIN_POSITIVE, 1., f64::MAX] {
        for sign in [-1., 1.] {
            let scale = sign * magnitude;
            let report = curve_plane(
                &source,
                Plane {
                    normal: [scale, scale, 0.],
                    offset: scale,
                },
                Options::default(),
            )
            .unwrap();
            assert_eq!(report.coverage, Coverage::NumericallyResolved);
            let roots = points(&report);
            assert_eq!(roots.len(), 1);
            assert_eq!(roots[0].parameter, 0.5);
            assert_eq!(roots[0].point, [1., 0., 0.]);
            assert_eq!(roots[0].plane_residual, 0.);
            assert!(!report.permits_topology_change());
        }
    }
}
fn plane(z: f64) -> Plane {
    Plane {
        normal: [0., 0., 1.],
        offset: z,
    }
}
fn bezier(z: &[f64]) -> Curve {
    let degree = z.len() - 1;
    Curve {
        degree,
        knots: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
        control_points: z
            .iter()
            .enumerate()
            .map(|(i, z)| vec![i as f64 / degree as f64, 0., *z])
            .collect(),
        weights: vec![1.; z.len()],
        periodic: false,
    }
}
fn points(report: &Report<CurvePlaneComponent>) -> Vec<&CurvePoint> {
    report
        .components
        .iter()
        .filter_map(|c| {
            if let CurvePlaneComponent::Point(p) = c {
                Some(p)
            } else {
                None
            }
        })
        .collect()
}
#[test]
fn roots_are_isolated_without_seed_sampling_and_residuals_are_checked() {
    // z(t) = (t-.2)(t-.5)(t-.8): three intersections, including
    // an exact subdivision boundary root that must not be duplicated.
    let curve = bezier(&[-0.08, 0.14, -0.14, 0.08]);
    let report = curve_plane(&curve, plane(0.), Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    let roots = points(&report);
    assert_eq!(roots.len(), 3, "{report:?}");
    for (root, expected) in roots.iter().zip([0.2, 0.5, 0.8]) {
        assert!((root.parameter - expected).abs() < 1e-8);
        assert!(root.plane_residual < 1e-9);
        assert!(
            root.parameter_interval[0] <= expected + 1e-14
                && root.parameter_interval[1] >= expected - 1e-14
        );
    }
    assert!(!report.permits_topology_change());
}
#[test]
fn tangencies_and_work_exhaustion_are_not_reported_as_empty() {
    let tangent =
        curve_plane(&bezier(&[0.25, -0.25, 0.25]), plane(0.), Options::default()).unwrap();
    assert_eq!(tangent.coverage, Coverage::Incomplete);
    assert!(
        tangent
            .unresolved
            .iter()
            .any(|u| u.reason == UnresolvedReason::TangencyOrMultipleRoot)
    );
    let limited = curve_plane(
        &bezier(&[-0.08, 0.14, -0.14, 0.08]),
        plane(0.),
        Options {
            max_boxes: 1,
            ..Options::default()
        },
    )
    .unwrap();
    assert_eq!(limited.coverage, Coverage::Incomplete);
    assert_eq!(limited.boxes_visited, 1);
    assert!(
        limited
            .unresolved
            .iter()
            .all(|u| u.reason == UnresolvedReason::BudgetExceeded)
    );
    assert!(
        curve_plane(&bezier(&[1., 2.]), plane(0.), Options::default())
            .unwrap()
            .components
            .is_empty()
    );
    assert!(matches!(
        curve_plane(&bezier(&[0., 0.]), plane(0.), Options::default())
            .unwrap()
            .components[0],
        CurvePlaneComponent::Overlap { .. }
    ));
}
#[test]
fn surface_budget_visits_other_spans_before_refining_difficult_branches() {
    let surface = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [-2., -2., -2., 5., -2.]
            .into_iter()
            .enumerate()
            .map(|(i, z)| {
                vec![
                    vec![i as f64, 0., z],
                    vec![i as f64, 1., if i == 3 { 5. } else { 2. }],
                ]
            })
            .collect(),
        weights: vec![vec![1., 1.]; 5],
        periodic_u: false,
        periodic_v: false,
    };
    let report = surface_plane(
        &surface,
        plane(0.),
        Options {
            max_boxes: 8,
            ..Options::default()
        },
    )
    .unwrap();
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert!(report.boxes_visited <= 8);
    assert!(
        report.components.iter().any(|component| matches!(component,
        SurfacePlaneComponent::Curve {parameter_box,..} if *parameter_box==[0.,0.5,0.,1.])),
        "The simple first span was starved: {report:?}"
    );
}
#[test]
fn curve_budget_admits_later_span_endpoint_before_deep_refinement() {
    let curve = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
        control_points: [1., -2., 1., 1., 0.]
            .into_iter()
            .enumerate()
            .map(|(i, z)| vec![i as f64, 0., z])
            .collect(),
        weights: vec![1.; 5],
        periodic: false,
    };
    let report = curve_plane(
        &curve,
        plane(0.),
        Options {
            max_boxes: 4,
            ..Options::default()
        },
    )
    .unwrap();
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert!(report.boxes_visited <= 4);
    assert!(
        points(&report).iter().any(|p| p.parameter == 1.),
        "Later endpoint was starved"
    );
}
#[test]
fn finite_curve_segment_queries_keep_parameters_and_refusals() {
    let curve = bezier(&[-1., 1.]);
    let report = curve_segment(&curve, [0., 0., 0.], [1., 0., 0.], Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
    assert!(!report.permits_topology_change());
    assert_eq!(report.components.len(), 1);
    let CurveSegmentComponent::Point {
        curve: p,
        segment_parameter,
        ..
    } = &report.components[0]
    else {
        panic!("Expected point")
    };
    assert!((p.parameter - 0.5).abs() < 1e-10);
    assert!((segment_parameter - 0.5).abs() < 1e-10);
    let skew = curve_segment(&curve, [0., 1., 0.], [1., 1., 0.], Options::default()).unwrap();
    assert!(skew.components.is_empty());
    assert_eq!(skew.coverage, Coverage::NumericallyResolved);
    let outside =
        curve_segment(&curve, [2., 0., 0.], [3., 0., 0.], Options::default()).unwrap();
    assert!(outside.components.is_empty());
    let line = bezier(&[0., 0.]);
    assert!(matches!(
        curve_segment(&line, [0., 0., 0.], [1., 0., 0.], Options::default())
            .unwrap()
            .components[0],
        CurveSegmentComponent::Overlap {
            curve_interval: [0., 1.]
        }
    ));
    let partial =
        curve_segment(&line, [0.25, 0., 0.], [0.75, 0., 0.], Options::default()).unwrap();
    assert_eq!(partial.coverage, Coverage::NumericallyResolved);
    assert!(matches!(
        partial.components[0],
        CurveSegmentComponent::Overlap {
            curve_interval: [0.25, 0.75]
        }
    ));
    let limited = curve_segment(
        &line,
        [0., 0., 0.],
        [1., 0., 0.],
        Options {
            max_boxes: 1,
            ..Options::default()
        },
    )
    .unwrap();
    assert_eq!(limited.boxes_visited, 1);
    assert_eq!(limited.coverage, Coverage::Incomplete);
    assert!(curve_segment(&line, [0.; 3], [0.; 3], Options::default()).is_err());
}
#[test]
fn rational_linear_overlap_clipping_inverts_weights_and_keeps_endpoint_contacts() {
    let mut line = bezier(&[0., 0.]);
    line.weights = vec![1., 3.];
    for (start, end) in [
        ([0.25, 0., 0.], [0.75, 0., 0.]),
        ([0.75, 0., 0.], [0.25, 0., 0.]),
    ] {
        let report = curve_segment(&line, start, end, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
        let CurveSegmentComponent::Overlap { curve_interval } = report.components[0] else {
            panic!("Expected overlap")
        };
        assert!((curve_interval[0] - 0.1).abs() < 1e-12);
        assert!((curve_interval[1] - 0.5).abs() < 1e-12);
    }
    let report = curve_segment(&line, [1., 0., 0.], [2., 0., 0.], Options::default()).unwrap();
    assert!(
        matches!(&report.components[0],CurveSegmentComponent::Point {curve,segment_parameter,..}
        if curve.parameter==1. && *segment_parameter==0.)
    );
    let quadratic = bezier(&[0., 0., 0.]);
    let unresolved = curve_segment(
        &quadratic,
        [0.25, 0., 0.],
        [0.75, 0., 0.],
        Options::default(),
    )
    .unwrap();
    assert_eq!(unresolved.coverage, Coverage::NumericallyResolved);
}
#[test]
fn shared_knot_segment_contact_has_one_parameter_event() {
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 0.5, 1., 1.],
        control_points: vec![vec![-1., 0., 0.], vec![0., 0., 0.], vec![-1., 0., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let report = curve_segment(&curve, [0., 0., 0.], [1., 0., 0.], Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
    assert_eq!(
        report.components.len(),
        1,
        "Duplicate shared-knot contact: {report:?}"
    );
    assert!(
        matches!(&report.components[0],CurveSegmentComponent::Point {curve,segment_parameter,..}
        if curve.parameter==0.5 && *segment_parameter==0.)
    );
    let repeated = Curve {
        degree: 1,
        knots: vec![0., 0., 0.25, 0.5, 0.75, 1., 1.],
        control_points: vec![
            vec![-1., 0., 0.],
            vec![0., 0., 0.],
            vec![-1., 0., 0.],
            vec![0., 0., 0.],
            vec![-1., 0., 0.],
        ],
        weights: vec![1.; 5],
        periodic: false,
    };
    let report =
        curve_segment(&repeated, [0., 0., 0.], [1., 0., 0.], Options::default()).unwrap();
    let parameters = report
        .components
        .iter()
        .map(|c| match c {
            CurveSegmentComponent::Point { curve, .. } => curve.parameter,
            _ => panic!("Expected point"),
        })
        .collect::<Vec<_>>();
    assert_eq!(parameters, vec![0.25, 0.75]);
}
#[test]
fn quadratic_coincident_clipping_uses_original_nonlinear_parameters() {
    let curve = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0., 0.], vec![0., 0., 0.], vec![1., 0., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let report =
        curve_segment(&curve, [0.25, 0., 0.], [0.5625, 0., 0.], Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert_eq!(report.components.len(), 1);
    assert!(matches!(
        report.components[0],
        CurveSegmentComponent::Overlap {
            curve_interval: [0.5, 0.75]
        }
    ));
    let uncertain =
        curve_segment(&curve, [0.3, 0., 0.], [0.6, 0., 0.], Options::default()).unwrap();
    assert_eq!(uncertain.coverage, Coverage::Incomplete);
    assert!(!uncertain.unresolved.is_empty());
    assert!(uncertain.boxes_visited <= Options::default().max_boxes);
}
#[test]
fn coincident_backtracking_curve_keeps_disconnected_parameter_intervals() {
    // x(t)=4t(1-t): the same spatial segment is visited twice.
    let curve = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0., 0.], vec![2., 0., 0.], vec![0., 0., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let report =
        curve_segment(&curve, [0.4375, 0., 0.], [0.75, 0., 0.], Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    let intervals = report
        .components
        .iter()
        .map(|c| match c {
            CurveSegmentComponent::Overlap { curve_interval } => *curve_interval,
            _ => panic!("Unexpected isolated point"),
        })
        .collect::<Vec<_>>();
    assert_eq!(intervals, vec![[0.125, 0.25], [0.75, 0.875]]);
    let uncertain =
        curve_segment(&curve, [0.3, 0., 0.], [0.7, 0., 0.], Options::default()).unwrap();
    assert_eq!(uncertain.coverage, Coverage::Incomplete);
    for q in [0.3_f64, 0.7] {
        for root in [(1. - (1. - q).sqrt()) / 2., (1. + (1. - q).sqrt()) / 2.] {
            assert!(
                uncertain
                    .unresolved
                    .iter()
                    .any(|r| root >= r.parameter_box[0] && root <= r.parameter_box[1]),
                "Missing boundary-root band for {root}: {uncertain:?}"
            );
        }
    }
    for c in uncertain.components {
        if let CurveSegmentComponent::Overlap {
            curve_interval: [a, b],
        } = c
        {
            for i in 0..=20 {
                let t = a + (b - a) * i as f64 / 20.;
                let x = 4. * t * (1. - t);
                assert!((0.3..=0.7).contains(&x));
            }
        }
    }
}
#[test]
fn coplanar_curve_is_clipped_to_affine_surface_domain() {
    let curve = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0., 0.], vec![0.5, 0., 0.], vec![1., 1., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 4., 4.],
        knots_v: vec![-1., -1., 3., 3.],
        control_points: vec![
            vec![vec![0.25, 0., 0.], vec![0.25, 1., 0.]],
            vec![vec![0.75, 0., 0.], vec![0.75, 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let report = curve_surface(&curve, &surface, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert_eq!(report.components.len(), 1);
    assert!(matches!(
        report.components[0],
        CurveSurfaceComponent::Overlap {
            curve_interval: [0.25, 0.75]
        }
    ));
    let mut edge = curve.clone();
    for p in &mut edge.control_points {
        p[1] = 0.;
    }
    let report = curve_surface(&edge, &surface, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
    assert!(matches!(
        report.components[0],
        CurveSurfaceComponent::Overlap {
            curve_interval: [0.25, 0.75]
        }
    ));
    let limited = curve_surface(
        &curve,
        &surface,
        Options {
            max_boxes: 2,
            ..Options::default()
        },
    )
    .unwrap();
    assert_eq!(limited.coverage, Coverage::Incomplete);
    assert!(limited.boxes_visited <= 2);
}
#[test]
fn affine_clipping_preserves_shear_and_single_corner_contacts() {
    let mut curve = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0., 0.], vec![0.5, 0., 0.], vec![1., 1., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let mut surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0.25, 0., 0.], vec![0.25, 1., 0.]],
            vec![vec![0.75, 0., 0.], vec![0.75, 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    for point in &mut curve.control_points {
        point[0] += point[1];
    }
    for point in surface.control_points.iter_mut().flatten() {
        point[0] += point[1];
    }
    let report = curve_surface(&curve, &surface, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert!(matches!(
        report.components[0],
        CurveSurfaceComponent::Overlap {
            curve_interval: [0.25, 0.75]
        }
    ));
    let surface = Surface {
        control_points: vec![
            vec![vec![0., 0., 0.], vec![1., 1., 0.]],
            vec![vec![1., 0., 0.], vec![2., 1., 0.]],
        ],
        ..surface
    };
    let diagonal = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 1., 0.], vec![0., -1., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let report = curve_surface(&diagonal, &surface, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert_eq!(report.components.len(), 1);
    let CurveSurfaceComponent::Point { curve, uv, .. } = &report.components[0] else {
        panic!("Expected corner")
    };
    assert_eq!(curve.parameter, 0.5);
    assert_eq!(*uv, [0., 0.]);
}
#[test]
fn finite_affine_surface_pair_retains_both_parameterizations() {
    let a = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![-1., -1., 0.], vec![-1., 1., 0.]],
            vec![vec![1., -1., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let b = Surface {
        control_points: vec![
            vec![vec![-0.5, 0., -1.], vec![-0.5, 0., 1.]],
            vec![vec![0.5, 0., -1.], vec![0.5, 0., 1.]],
        ],
        ..a.clone()
    };
    for (first, second) in [(&a, &b), (&b, &a)] {
        let report = surface_surface(first, second, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::Complete, "{report:?}");
        assert!(!report.permits_topology_change());
        assert_eq!(report.components.len(), 1);
        let SurfaceSurfaceComponent::Curve { first, second, .. } = &report.components[0] else {
            panic!("Expected intersection segment")
        };
        for i in 0..=20 {
            let p = first.evaluate(i as f64 / 20.).unwrap();
            let q = second.evaluate(i as f64 / 20.).unwrap();
            assert!(
                p.point
                    .iter()
                    .zip(q.point)
                    .all(|(a, b)| (a - b).abs() < 1e-12)
            );
            assert_eq!(p.point[1], 0.);
            assert_eq!(p.point[2], 0.);
            assert!(p.point[0] >= -0.5 && p.point[0] <= 0.5);
        }
    }
    assert_eq!(
        surface_surface(&a, &a, Options::default())
            .unwrap()
            .coverage,
        Coverage::Complete
    );
    let limited = surface_surface(
        &a,
        &b,
        Options {
            max_boxes: 1,
            ..Options::default()
        },
    )
    .unwrap();
    assert_eq!(limited.coverage, Coverage::Incomplete);
    assert_eq!(limited.boxes_visited, 1);
}
#[test]
fn coplanar_affine_pairs_keep_area_edge_and_point_dimensions() {
    let square = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![-1., -1., 0.], vec![-1., 1., 0.]],
            vec![vec![1., -1., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let diamond = Surface {
        control_points: vec![
            vec![vec![0., -1.5, 0.], vec![-1.5, 0., 0.]],
            vec![vec![1.5, 0., 0.], vec![0., 1.5, 0.]],
        ],
        ..square.clone()
    };
    let report = surface_surface(&square, &diamond, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::Complete, "{report:?}");
    assert!(!report.permits_topology_change());
    let SurfaceSurfaceComponent::Overlap {
        points,
        first_boundary,
        second_boundary,
        ..
    } = &report.components[0]
    else {
        panic!("Expected polygon")
    };
    assert_eq!(points.len(), 8);
    assert_eq!(first_boundary.len(), 8);
    assert_eq!(second_boundary.len(), 8);
    let area = (0..points.len())
        .map(|i| {
            let a = points[i];
            let b = points[(i + 1) % points.len()];
            a[0] * b[1] - a[1] * b[0]
        })
        .sum::<f64>()
        / 2.;
    assert!((area - 3.5).abs() < 1e-12);
    for (dx, dy, kind) in [(2., 0., 1), (2., 2., 0), (3., 0., -1)] {
        let mut target = square.clone();
        for p in target.control_points.iter_mut().flatten() {
            p[0] += dx;
            p[1] += dy;
        }
        let report = surface_surface(&square, &target, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::Complete, "{report:?}");
        assert!(!report.permits_topology_change());
        match kind {
            1 => assert!(matches!(
                report.components[0],
                SurfaceSurfaceComponent::Curve { .. }
            )),
            0 => assert!(matches!(
                report.components[0],
                SurfaceSurfaceComponent::Point { .. }
            )),
            _ => assert!(report.components.is_empty()),
        }
    }
    assert_eq!(
        surface_surface(
            &square,
            &diamond,
            Options {
                max_boxes: 3,
                ..Options::default()
            }
        )
        .unwrap()
        .coverage,
        Coverage::Incomplete
    );
}
#[test]
fn coplanar_pair_correspondence_survives_swap_uv_reversal_and_knot_scaling() {
    let square = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![-1., -1., 0.], vec![-1., 1., 0.]],
            vec![vec![1., -1., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let diamond = Surface {
        control_points: vec![
            vec![vec![0., -1.5, 0.], vec![-1.5, 0., 0.]],
            vec![vec![1.5, 0., 0.], vec![0., 1.5, 0.]],
        ],
        ..square.clone()
    };
    for swap in [false, true] {
        for reverse in [false, true] {
            for transpose in [false, true] {
                let (mut a, mut b) = if swap {
                    (diamond.clone(), square.clone())
                } else {
                    (square.clone(), diamond.clone())
                };
                if reverse {
                    a.control_points.reverse();
                    a.weights.reverse();
                }
                if transpose {
                    b = transpose_surface(&b);
                }
                a.knots_u = a.knots_u.iter().map(|t| -3. + 8. * t).collect();
                b.knots_v = b.knots_v.iter().map(|t| 10. + 4. * t).collect();
                for row in &mut b.weights {
                    for w in row {
                        *w *= 32.;
                    }
                }
                let report = surface_surface(&a, &b, Options::default()).unwrap();
                assert_eq!(
                    report.coverage,
                    Coverage::Complete,
                    "swap={swap} reverse={reverse} transpose={transpose}: {report:?}"
                );
                assert!(!report.permits_topology_change());
                let SurfaceSurfaceComponent::Overlap {
                    first_boundary,
                    second_boundary,
                    points,
                    ..
                } = &report.components[0]
                else {
                    panic!("Expected area")
                };
                assert_eq!(points.len(), 8);
                let area = (0..8)
                    .map(|i| {
                        let p = points[i];
                        let q = points[(i + 1) % 8];
                        p[0] * q[1] - p[1] * q[0]
                    })
                    .sum::<f64>()
                    .abs()
                    / 2.;
                assert!((area - 3.5).abs() < 1e-11);
                for ((u, v), point) in first_boundary.iter().zip(second_boundary).zip(points) {
                    let p = a.evaluate(u[0], u[1]).unwrap().point;
                    let q = b.evaluate(v[0], v[1]).unwrap().point;
                    for axis in 0..3 {
                        assert!((p[axis] - q[axis]).abs() < 1e-11);
                        assert!((p[axis] - point[axis]).abs() < 1e-11);
                    }
                    assert!(point[0].abs() <= 1. + 1e-12 && point[1].abs() <= 1. + 1e-12);
                    assert!(point[0].abs() + point[1].abs() <= 1.5 + 1e-12);
                }
            }
        }
    }
}
#[test]
fn multispan_ruled_conversion_preserves_seams_and_parameterization() {
    let surface = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 0., 4.]],
            vec![vec![1., 2., 0.], vec![1., 2., 4.]],
            vec![vec![2., 0., 0.], vec![2., 0., 4.]],
        ],
        weights: vec![vec![1., 2.]; 3],
        periodic_u: false,
        periodic_v: false,
    }
    .edit_axis(nurbs_core::surface::Axis::U, |c| {
        c.insert(0.25, 1)?.insert(0.75, 1)
    })
    .unwrap();
    let dense = surface
        .trim([0., 0.25, 0., 1.])
        .unwrap()
        .edit_axis(nurbs_core::surface::Axis::U, |c| {
            let mut curve = c.elevate(12)?;
            for i in 1..20 {
                curve = curve.insert(i as f64 / 80., 1)?;
            }
            Ok(curve)
        })
        .unwrap();
    let oversized = SurfaceTrace::Ruled {
        surface: dense,
        plane: plane(2.),
        u_interval: [0., 0.25],
    };
    assert!(
        oversized
            .to_curve()
            .unwrap_err()
            .to_string()
            .contains("256 control points")
    );
    for reverse in [false, true] {
        let trace = SurfaceTrace::Ruled {
            surface: surface.clone(),
            plane: plane(2.),
            u_interval: if reverse { [1., 0.] } else { [0., 1.] },
        };
        let curve = trace.to_curve().unwrap();
        assert_eq!(curve.degree, 4);
        assert_eq!(curve.control_points.len(), 13);
        for i in 0..=100 {
            let t = i as f64 / 100.;
            let expected = trace.evaluate(t).unwrap().point;
            let actual = curve.evaluate(t).unwrap().point;
            assert!(
                expected
                    .iter()
                    .zip(actual)
                    .all(|(a, b)| (a - b).abs() < 1e-10)
            );
        }
    }
}
#[test]
fn multispan_conversion_handles_varying_boundary_weights_and_oblique_cuts() {
    let surface = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..5)
            .map(|i| {
                let x = i as f64 / 2.;
                vec![vec![x, 0., 0.], vec![x, 0., 4.]]
            })
            .collect(),
        weights: vec![
            vec![1., 2.],
            vec![2., 3.],
            vec![3., 4.],
            vec![2., 5.],
            vec![1., 3.],
        ],
        periodic_u: false,
        periodic_v: false,
    };
    let mut shifted = surface.clone();
    shifted.knots_u = shifted
        .knots_u
        .iter()
        .map(|&u| if u == 0.5 { -1. } else { -3. + 8. * u })
        .collect();
    shifted.knots_v = shifted.knots_v.iter().map(|&v| 10. + 4. * v).collect();
    for source in [shifted.clone(), transpose_surface(&shifted)] {
        for reverse in [false, true] {
            let interval = if reverse { [4., -2.] } else { [-2., 4.] };
            let plane = Plane {
                normal: [0.25, 0., 1.],
                offset: 2.,
            };
            let trace = if source.degree_v == 1 {
                SurfaceTrace::Ruled {
                    surface: source.clone(),
                    plane,
                    u_interval: interval,
                }
            } else {
                SurfaceTrace::RuledU {
                    surface: source.clone(),
                    plane,
                    v_interval: interval,
                }
            };
            let pieces = trace.to_curve_segments().unwrap();
            let middle = if reverse { 5. / 6. } else { 1. / 6. };
            assert_eq!(pieces.len(), 2);
            assert_eq!(pieces[0].domain(), [0., middle]);
            assert_eq!(pieces[1].domain(), [middle, 1.]);
            for curve in pieces {
                let [lo, hi] = curve.domain();
                for i in 0..=100 {
                    let t = if i == 100 {
                        hi
                    } else {
                        lo + (hi - lo) * (i as f64 / 100.)
                    };
                    let p = curve.evaluate(t).unwrap().point;
                    let q = trace.evaluate(t).unwrap().point;
                    assert!(p.iter().zip(q).all(|(a, b)| (a - b).abs() < 1e-10));
                    assert!((0.25 * p[0] + p[2] - 2.).abs() < 1e-10);
                }
            }
        }
    }
    for surface in [surface.clone(), transpose_surface(&surface)] {
        for reverse in [false, true] {
            let interval = if reverse { [1., 0.] } else { [0., 1.] };
            let plane = Plane {
                normal: [0.25, 0., 1.],
                offset: 2.,
            };
            let trace = if surface.degree_v == 1 {
                SurfaceTrace::Ruled {
                    surface: surface.clone(),
                    plane,
                    u_interval: interval,
                }
            } else {
                SurfaceTrace::RuledU {
                    surface: surface.clone(),
                    plane,
                    v_interval: interval,
                }
            };
            let curve = trace.to_curve().unwrap();
            for i in 0..=100 {
                let t = i as f64 / 100.;
                let p = curve.evaluate(t).unwrap().point;
                let q = trace.evaluate(t).unwrap().point;
                assert!(p.iter().zip(q).all(|(a, b)| (a - b).abs() < 1e-10));
                assert!((0.25 * p[0] + p[2] - 2.).abs() < 1e-10);
            }
        }
    }
}
#[test]
fn multispan_uv_diagonal_joins_identical_polynomial_endpoints() {
    let mut source = flat();
    source.control_points[1][1][2] = 1.;
    let refined = source
        .edit_axis(nurbs_core::surface::Axis::U, |c| {
            c.insert(0.25, 1)?.insert(0.75, 1)
        })
        .unwrap()
        .edit_axis(nurbs_core::surface::Axis::V, |c| c.insert(0.5, 1))
        .unwrap();
    let trace = SurfaceTrace::Line {
        surface: refined,
        plane: plane(0.),
        start: [0., 0.],
        end: [1., 1.],
    };
    let curve = trace.to_curve().unwrap();
    assert_eq!(curve.control_points.len(), 9);
    for i in 0..=100 {
        let t = i as f64 / 100.;
        let point = curve.evaluate(t).unwrap().point;
        for (a, b) in point.iter().zip([t, t, t * t]) {
            assert!((a - b).abs() < 1e-12);
        }
    }
}
#[test]
fn ruled_conversion_rejects_an_interior_excursion_from_the_source_domain() {
    let source = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [-1., 3., -1.]
            .into_iter()
            .enumerate()
            .map(|(i, z)| vec![vec![i as f64 / 2., 0., z], vec![i as f64 / 2., 1., z + 2.]])
            .collect(),
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    for surface in [source.clone(), transpose_surface(&source)] {
        for reverse in [false, true] {
            let interval = if reverse { [1., 0.] } else { [0., 1.] };
            let trace = if surface.degree_v == 1 {
                SurfaceTrace::Ruled {
                    surface: surface.clone(),
                    plane: plane(0.),
                    u_interval: interval,
                }
            } else {
                SurfaceTrace::RuledU {
                    surface: surface.clone(),
                    plane: plane(0.),
                    v_interval: interval,
                }
            };
            assert!(trace.evaluate(0.).is_ok());
            assert!(trace.evaluate(1.).is_ok());
            assert!(trace.evaluate(0.5).is_err());
            assert!(
                trace
                    .to_curve()
                    .unwrap_err()
                    .to_string()
                    .contains("leaves the source parameter domain")
            );
            assert!(
                trace
                    .to_curve_segments()
                    .unwrap_err()
                    .to_string()
                    .contains("leaves the source parameter domain")
            );
        }
    }
}
#[test]
fn trace_conversion_refuses_an_ambiguous_ruling_family() {
    let source = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [-1., 2., -1.]
            .into_iter()
            .enumerate()
            .map(|(i, z)| vec![vec![i as f64, 0., z], vec![i as f64, 1., -z]])
            .collect(),
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let trace = SurfaceTrace::Ruled {
        surface: source,
        plane: plane(0.),
        u_interval: [0., 1.],
    };
    assert!(trace.evaluate(0.).is_ok());
    assert!(trace.evaluate(1.).is_ok());
    assert!(trace.to_curve().is_err());
    assert!(trace.to_curve_segments().is_err());
    let late_ambiguous = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [-1., -1., -1., 2., -1.]
            .into_iter()
            .enumerate()
            .map(|(i, z)| vec![vec![i as f64, 0., z], vec![i as f64, 1., -z]])
            .collect(),
        weights: vec![vec![1.; 2]; 5],
        periodic_u: false,
        periodic_v: false,
    };
    let first = SurfaceTrace::Ruled {
        surface: late_ambiguous.clone(),
        plane: plane(0.),
        u_interval: [0., 0.5],
    };
    assert_eq!(first.to_curve_segments().unwrap().len(), 1);
    let whole = SurfaceTrace::Ruled {
        surface: late_ambiguous,
        plane: plane(0.),
        u_interval: [0., 1.],
    };
    assert!(whole.to_curve_segments().is_err()); // no partial success after the valid first span
    let mut dense = flat();
    for row in &mut dense.control_points {
        row[1][2] = 4.;
    }
    let dense = dense
        .edit_axis(nurbs_core::surface::Axis::U, |c| {
            let mut curve = c.elevate(12)?;
            for i in 1..13 {
                curve = curve.insert(i as f64 / 16., 1)?;
            }
            Ok(curve)
        })
        .unwrap();
    let oversized = SurfaceTrace::Ruled {
        surface: dense,
        plane: plane(2.),
        u_interval: [0., 1.],
    };
    assert!(
        oversized
            .to_curve_segments()
            .unwrap_err()
            .to_string()
            .contains("256 control points")
    );
}
fn flat() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1., 1.]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn affine_patch_sections_and_curve_surface_clipping() {
    let surface = flat();
    let report = surface_plane(
        &surface,
        Plane {
            normal: [1., 1., 0.],
            offset: 1.,
        },
        Options::default(),
    )
    .unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
    assert_eq!(report.components.len(), 1);
    let curve = Curve::from_polyline(vec![vec![0.5, 0.5, -1.], vec![0.5, 0.5, 1.]]).unwrap();
    let report = curve_surface(&curve, &surface, Options::default()).unwrap();
    assert_eq!(report.components.len(), 1);
    let CurveSurfaceComponent::Point {
        uv,
        surface_residual,
        ..
    } = report.components[0]
    else {
        panic!()
    };
    assert_eq!(uv, [0.5, 0.5]);
    assert!(surface_residual < 1e-12);
    let outside = Curve::from_polyline(vec![vec![2., 0.5, -1.], vec![2., 0.5, 1.]]).unwrap();
    assert!(
        curve_surface(&outside, &surface, Options::default())
            .unwrap()
            .components
            .is_empty()
    );
    let crossing = Curve::from_polyline(vec![vec![0.5, 0.5, 0.], vec![2., 0.5, 0.]]).unwrap();
    assert_eq!(
        curve_surface(&crossing, &surface, Options::default())
            .unwrap()
            .coverage,
        Coverage::Incomplete
    );
}

#[test]
fn nearby_roots_are_not_merged_and_curve_edits_preserve_intersections() {
    let separation = 1e-5_f64;
    let c = 0.25 - separation * separation;
    let curve = bezier(&[c, c - 0.5, c]);
    for source in [
        curve.clone(),
        curve.reverse().unwrap(),
        curve.insert(0.4, 1).unwrap(),
        curve.elevate(4).unwrap(),
    ] {
        let report = curve_plane(&source, plane(0.), Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        let roots = points(&report);
        assert_eq!(roots.len(), 2, "{report:?}");
        assert!((roots[0].parameter - (0.5 - separation)).abs() < 1e-9);
        assert!((roots[1].parameter - (0.5 + separation)).abs() < 1e-9);
    }
}

#[test]
fn invalid_inputs_and_unsupported_surfaces_remain_explicit() {
    assert!(
        curve_plane(
            &bezier(&[-1., 1.]),
            Plane {
                normal: [0.; 3],
                offset: 0.
            },
            Options::default()
        )
        .is_err()
    );
    assert!(
        curve_plane(
            &bezier(&[-1., 1.]),
            plane(0.),
            Options {
                max_boxes: 0,
                ..Options::default()
            }
        )
        .is_err()
    );
    let mut curved = flat();
    curved.control_points[1][1][2] = 1.;
    let curve = bezier(&[-1., 1.]);
    let report = curve_surface(&curve, &curved, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::UnsupportedSurface
    );
    assert!(!report.permits_topology_change());
}

fn line3(a: [f64; 3], b: [f64; 3]) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights: vec![1., 1.],
        periodic: false,
    }
}
fn quarter_circle() -> Curve {
    let w = std::f64::consts::FRAC_1_SQRT_2;
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]],
        weights: vec![1., w, 1.],
        periodic: false,
    }
}
fn cc_points(report: &Report<CurveCurveComponent>) -> Vec<(f64, f64)> {
    report
        .components
        .iter()
        .filter_map(|c| match c {
            CurveCurveComponent::Point { first, second, .. } => Some((*first, *second)),
            _ => None,
        })
        .collect()
}
#[test]
fn crossing_lines_resolve_one_transverse_point() {
    let a = line3([0., 0., 0.], [2., 0., 0.]);
    let b = line3([1., -1., 0.], [1., 1., 0.]);
    let report = curve_curve(&a, &b, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert!(!report.permits_topology_change());
    let CurveCurveComponent::Point {
        first,
        second,
        point,
        residual,
        contact,
        ..
    } = &report.components[0]
    else {
        panic!("Expected one point: {report:?}")
    };
    assert_eq!(report.components.len(), 1, "{report:?}");
    // Independent Bernstein oracle: A(t)=(2t,0,0), B(u)=(1,2u-1,0).
    assert!((*first - 0.5).abs() <= 1e-10, "{report:?}");
    assert!((*second - 0.5).abs() <= 1e-10, "{report:?}");
    assert!(distance(*point, [1., 0., 0.]) <= 1e-9);
    assert!(*residual <= 1e-9);
    assert_eq!(*contact, Contact::Transverse);
}
#[test]
fn rational_quarter_circle_crosses_diagonal_line() {
    let arc = quarter_circle();
    let diagonal = line3([0., 0., 0.], [1., 1., 0.]);
    let report = curve_curve(&diagonal, &arc, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert_eq!(report.components.len(), 1, "{report:?}");
    let CurveCurveComponent::Point {
        first,
        second,
        point,
        residual,
        ..
    } = &report.components[0]
    else {
        panic!("Expected point")
    };
    // Line point (t,t,0) on x^2+y^2=1 gives t=1/sqrt(2); the symmetric
    // rational quarter circle reaches 45 degrees at u=1/2.
    assert!(
        (*first - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-9,
        "{report:?}"
    );
    assert!((*second - 0.5).abs() < 1e-9, "{report:?}");
    assert!((point[0] * point[0] + point[1] * point[1] - 1.).abs() < 1e-10);
    assert!(*residual <= 1e-9);
}
#[test]
fn two_beziers_resolve_two_crossings_in_parameter_order() {
    // A: x=2t, y=2t(1-t); B: y=3/8. 2t(1-t)=3/8 gives t=1/4 and 3/4.
    let parabola = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0., 0.], vec![1., 1., 0.], vec![2., 0., 0.]],
        weights: vec![1., 1., 1.],
        periodic: false,
    };
    let line = line3([0., 0.375, 0.], [2., 0.375, 0.]);
    let report = curve_curve(&parabola, &line, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    let roots = cc_points(&report);
    assert_eq!(roots.len(), 2, "{report:?}");
    for ((t, u), expected) in roots.iter().zip([0.25, 0.75]) {
        assert!((t - expected).abs() < 1e-9, "{report:?}");
        assert!((u - expected).abs() < 1e-9, "{report:?}");
    }
}
#[test]
fn parallel_disjoint_and_skew_pairs_are_empty_and_resolved() {
    let a = line3([0., 0., 0.], [1., 0., 0.]);
    let parallel = line3([0., 1., 0.], [1., 1., 0.]);
    let report = curve_curve(&a, &parallel, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert!(report.components.is_empty());
    let skew = line3([0., 0., 1.], [1., 1., 1.]);
    let report = curve_curve(&a, &skew, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert!(report.components.is_empty());
}
#[test]
fn coincident_collinear_segments_clip_both_trim_intervals() {
    let a = line3([0., 0., 0.], [1., 0., 0.]);
    for reversed in [false, true] {
        let b = if reversed {
            line3([1.5, 0., 0.], [0.5, 0., 0.])
        } else {
            line3([0.5, 0., 0.], [1.5, 0., 0.])
        };
        let report = curve_curve(&a, &b, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        assert_eq!(report.components.len(), 1, "{report:?}");
        let CurveCurveComponent::Overlap {
            first_interval,
            second_interval,
            reversed: run_reversed,
            max_control_residual,
        } = &report.components[0]
        else {
            panic!("Expected overlap: {report:?}")
        };
        assert_eq!(*first_interval, [0.5, 1.]);
        assert_eq!(
            *second_interval,
            if reversed { [0.5, 1.] } else { [0., 0.5] }
        );
        assert_eq!(*run_reversed, reversed);
        assert_eq!(*max_control_residual, 0.);
    }
    // Weighted rational line x(t)=3t/(1+2t): geometric [1/2,1] maps to
    // [1/4,1] in source parameters (independent Möbius inversion oracle).
    let mut weighted = line3([0., 0., 0.], [1., 0., 0.]);
    weighted.weights = vec![1., 3.];
    let b = line3([0.5, 0., 0.], [1.5, 0., 0.]);
    let report = curve_curve(&weighted, &b, Options::default()).unwrap();
    let CurveCurveComponent::Overlap { first_interval, .. } = &report.components[0] else {
        panic!("Expected weighted overlap: {report:?}")
    };
    assert!((first_interval[0] - 0.25).abs() < 1e-12, "{report:?}");
    assert!((first_interval[1] - 1.).abs() < 1e-12, "{report:?}");
}
#[test]
fn shared_endpoint_is_one_boundary_event_with_exact_parameters() {
    let a = line3([0., 0., 0.], [1., 0., 0.]);
    let b = line3([1., 0., 0.], [2., 1., 0.]);
    for (first, second, t, u) in [(&a, &b, 1., 0.), (&b, &a, 0., 1.)] {
        let report = curve_curve(first, second, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        assert_eq!(report.components.len(), 1, "{report:?}");
        let CurveCurveComponent::Point {
            first: p,
            second: q,
            contact,
            residual,
            ..
        } = &report.components[0]
        else {
            panic!("Expected endpoint event")
        };
        assert_eq!(*p, t);
        assert_eq!(*q, u);
        assert_eq!(*contact, Contact::Boundary);
        assert_eq!(*residual, 0.);
    }
}
#[test]
fn tangent_curves_stay_unresolved_without_a_guessed_point() {
    let parabola = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0., 0.], vec![1., 1., 0.], vec![2., 0., 0.]],
        weights: vec![1., 1., 1.],
        periodic: false,
    };
    let tangent_line = line3([0., 0.5, 0.], [2., 0.5, 0.]);
    let report = curve_curve(&parabola, &tangent_line, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::Incomplete, "{report:?}");
    assert!(
        report.components.is_empty()
            || report
                .components
                .iter()
                .all(|c| matches!(c, CurveCurveComponent::Overlap { .. })),
        "{report:?}"
    );
    assert!(
        report
            .unresolved
            .iter()
            .any(|u| u.reason == UnresolvedReason::TangencyOrMultipleRoot),
        "{report:?}"
    );
}
#[test]
fn tight_budget_preserves_pending_parameter_regions() {
    // Two-span curves: four span pairs; two boxes process, the remaining
    // pairs stay explicit pending regions in source knot coordinates.
    let two_span = |x: f64| Curve {
        degree: 1,
        knots: vec![0., 0., 0.5, 1., 1.],
        control_points: vec![vec![x - 1., 0., 0.], vec![x, 0., 0.], vec![x + 1., 0., 0.]],
        weights: vec![1., 1., 1.],
        periodic: false,
    };
    let mut b = two_span(3.);
    for point in &mut b.control_points {
        point[1] = point[0] - 3.;
        point[0] = 3.;
    }
    let report = curve_curve(
        &two_span(0.),
        &b,
        Options {
            max_boxes: 2,
            ..Options::default()
        },
    )
    .unwrap();
    assert_eq!(report.coverage, Coverage::Incomplete, "{report:?}");
    assert!(report.boxes_visited <= 2);
    assert!(
        report
            .unresolved
            .iter()
            .all(|u| u.reason == UnresolvedReason::BudgetExceeded && u.parameter_box.len() == 4),
        "{report:?}"
    );
    assert!(report.unresolved.len() >= 2, "{report:?}");
}
#[test]
fn operand_swap_maps_event_parameters() {
    let a = line3([0., 0., 0.], [2., 0., 0.]);
    let b = line3([1., -1., 0.], [1., 1., 0.]);
    let forward = curve_curve(&a, &b, Options::default()).unwrap();
    let swapped = curve_curve(&b, &a, Options::default()).unwrap();
    let forward_points = cc_points(&forward);
    let swapped_points = cc_points(&swapped);
    assert_eq!(forward_points.len(), swapped_points.len());
    for ((t, u), (v, s)) in forward_points.iter().zip(&swapped_points) {
        assert_eq!(t, s);
        assert_eq!(u, v);
    }
}
#[test]
fn knot_shift_scale_and_weight_scaling_leave_geometry_invariant() {
    let mut a = line3([0., 0., 0.], [2., 0., 0.]);
    a.knots = vec![3., 3., 7., 7.];
    a.weights = vec![7., 7.];
    let mut b = line3([1., -1., 0.], [1., 1., 0.]);
    b.knots = vec![-2., -2., 0., 0.];
    b.weights = vec![0.5, 0.5];
    let report = curve_curve(&a, &b, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    let roots = cc_points(&report);
    assert_eq!(roots.len(), 1, "{report:?}");
    // t: 0.5 -> 3+4*0.5 = 5; u: 0.5 -> -2+2*0.5 = -1.
    assert!((roots[0].0 - 5.).abs() <= 4e-10, "{report:?}");
    assert!((roots[0].1 + 1.).abs() <= 2e-10, "{report:?}");
}
#[test]
fn curved_full_coincidence_reports_both_domains_under_knot_shift() {
    let arc = quarter_circle();
    let mut shifted = quarter_circle();
    shifted.knots = shifted.knots.iter().map(|k| 3. + 2. * k).collect();
    for (first, second, expected) in [
        (&arc, &shifted, ([0., 1.], [3., 5.])),
        (&shifted, &arc, ([3., 5.], [0., 1.])),
    ] {
        let report = curve_curve(first, second, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        assert_eq!(report.components.len(), 1, "{report:?}");
        let CurveCurveComponent::Overlap {
            first_interval,
            second_interval,
            reversed,
            ..
        } = &report.components[0]
        else {
            panic!("Expected overlap: {report:?}")
        };
        assert_eq!(*first_interval, expected.0);
        assert_eq!(*second_interval, expected.1);
        assert!(!reversed);
    }
    let reversed_arc = arc.reverse().unwrap();
    let report = curve_curve(&arc, &reversed_arc, Options::default()).unwrap();
    let CurveCurveComponent::Overlap { reversed, .. } = &report.components[0] else {
        panic!("Expected reversed overlap: {report:?}")
    };
    assert!(reversed);
}
#[test]
fn internal_knot_crossing_is_reported_once() {
    // Two-span line crossed exactly at its internal knot t=0.5.
    let a = Curve {
        degree: 1,
        knots: vec![0., 0., 0.5, 1., 1.],
        control_points: vec![vec![0., 0., 0.], vec![1., 0., 0.], vec![2., 0., 0.]],
        weights: vec![1., 1., 1.],
        periodic: false,
    };
    let b = line3([1., -1., 0.], [1., 1., 0.]);
    let report = curve_curve(&a, &b, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    let roots = cc_points(&report);
    assert_eq!(roots.len(), 1, "{report:?}");
    assert_eq!(roots[0].0, 0.5);
    assert!((roots[0].1 - 0.5).abs() <= 1e-10, "{report:?}");
}
#[test]
fn curve_curve_rejects_non_3d_and_invalid_options() {
    let flat = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.], vec![1., 0.]],
        weights: vec![1., 1.],
        periodic: false,
    };
    let line = line3([0., 0., 0.], [1., 0., 0.]);
    assert!(curve_curve(&flat, &line, Options::default()).is_err());
    assert!(
        curve_curve(
            &line,
            &line,
            Options {
                max_depth: 0,
                ..Options::default()
            }
        )
        .is_err()
    );
    assert!(
        !curve_curve(&line, &line, Options::default())
            .unwrap()
            .permits_topology_change()
    );
}

/// Full-circle ruled cylinder: radius 2 around the Z axis, z in [0,4].
/// The top boundary weights are `top` times the bottom weights, so
/// unequal endpoint weights give rational (Möbius) rulings in V.
fn ruled_cylinder(top: f64) -> Surface {
    let w = std::f64::consts::FRAC_1_SQRT_2;
    let ring = [
        [2., 0.],
        [2., 2.],
        [0., 2.],
        [-2., 2.],
        [-2., 0.],
        [-2., -2.],
        [0., -2.],
        [2., -2.],
        [2., 0.],
    ];
    let weights = [1., w, 1., w, 1., w, 1., w, 1.];
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: ring
            .iter()
            .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 4.]])
            .collect(),
        weights: weights.iter().map(|w| vec![*w, *w * top]).collect(),
        periodic_u: false,
        periodic_v: false,
    }
}
fn cs_points(report: &Report<CurveRuledSurfaceComponent>) -> Vec<(f64, [f64; 2])> {
    report
        .components
        .iter()
        .filter_map(|c| match c {
            CurveRuledSurfaceComponent::Point { t, uv, .. } => Some((*t, *uv)),
            _ => None,
        })
        .collect()
}
/// Unique Möbius map through three (t,v) samples, by the cross-ratio
/// identity (v-v0)(v1-v2)/((v-v2)(v1-v0)) = (t-t0)(t1-t2)/((t-t2)(t1-t0)).
fn mobius_through(samples: &[[f64; 3]; 3], t: f64) -> f64 {
    let (t0, v0) = (samples[0][0], samples[0][2]);
    let (t1, v1) = (samples[1][0], samples[1][2]);
    let (t2, v2) = (samples[2][0], samples[2][2]);
    let k = (t - t0) * (t1 - t2) / ((t - t2) * (t1 - t0));
    (v0 * (v1 - v2) - k * v2 * (v1 - v0)) / ((v1 - v2) - k * (v1 - v0))
}
#[test]
fn seam_crossing_on_closed_cylinder_reports_one_event_at_canonical_u() {
    let r3 = 3_f64.sqrt();
    for top in [1., 3.] {
        let surface = ruled_cylinder(top);
        // Chord through the seam point (2,0,2) and the 30-degree point
        // (sqrt3,1,2), offset by non-dyadic thirds/sevenths so both
        // crossings land strictly inside subdivision boxes: seam crossing
        // at t=7/31, second crossing at t=28/31.
        let d = [r3 - 2., 1., 0.];
        let curve = line3(
            [2. - d[0] / 3., -d[1] / 3., 2.],
            [r3 + d[0] / 7., 1. + d[1] / 7., 2.],
        );
        let report = curve_ruled_surface(&curve, &surface, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        let points = cs_points(&report);
        // One event for the seam contact (not one per seam side), one for
        // the 30-degree crossing.
        assert_eq!(points.len(), 2, "{report:?}");
        // The Möbius v of the z=2 slice: s=1/2, v = s/(top-s(top-1)).
        let v2 = 0.5 / (top - 0.5 * (top - 1.));
        let (t0, uv0) = points[0];
        assert!((t0 - 7. / 31.).abs() < 1e-9, "{report:?}");
        assert!(uv0[0] < 1e-6 || uv0[0] > 1. - 1e-6, "{report:?}");
        assert!((uv0[1] - v2).abs() < 1e-9, "{report:?}");
        let (t1, uv1) = points[1];
        assert!((t1 - 28. / 31.).abs() < 1e-9, "{report:?}");
        assert!(uv1[0] > 0.08 && uv1[0] < 0.09, "{report:?}");
        assert!((uv1[1] - v2).abs() < 1e-9, "{report:?}");
        // The seam contact is a single event — never one per seam side.
        // A one-sided Newton resolution keeps its resolving-side
        // parameter; confirmed two-sided contacts fold to u=0.
        let seam_side = points
            .iter()
            .filter(|(_, uv)| uv[0] < 1e-6 || uv[0] > 1. - 1e-6)
            .count();
        assert_eq!(seam_side, 1, "{report:?}");
        let CurveRuledSurfaceComponent::Point { point, .. } = &report.components[1] else {
            panic!("Expected point: {report:?}")
        };
        assert!(distance(*point, [r3, 1., 2.]) < 1e-9, "{report:?}");
    }
}
#[test]
fn seam_endpoint_contact_merges_to_one_canonical_event() {
    for top in [1., 3.] {
        let surface = ruled_cylinder(top);
        // Line in the z=0 plane ending exactly on the seam directrix point
        // (2,0,0): x^2+y^2=4 along the line only at t=1, so the endpoint
        // owns the contact; it is admitted from BOTH seam sides
        // (u=0 and u=1 corners) and must unify to one canonical u=0 event.
        let curve = line3([4., -2., 0.], [2., 0., 0.]);
        let report = curve_ruled_surface(&curve, &surface, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        assert_eq!(report.components.len(), 1, "{report:?}");
        let CurveRuledSurfaceComponent::Point {
            t,
            uv,
            point,
            residual,
            contact,
            ..
        } = &report.components[0]
        else {
            panic!("Expected seam endpoint event: {report:?}")
        };
        assert_eq!(*t, 1., "{report:?}");
        assert_eq!(uv[0], 0., "{report:?}");
        assert_eq!(uv[1], 0., "{report:?}");
        assert!(distance(*point, [2., 0., 0.]) <= 1e-9);
        assert!(*residual <= 1e-9);
        assert_eq!(*contact, Contact::Boundary);
        let _ = top;
    }
}
#[test]
fn seam_ruling_reports_single_wrapped_overlap_with_mobius_correspondence() {
    for top in [1., 3.] {
        let surface = ruled_cylinder(top);
        // The seam ruling itself: x=2, y=0, z from 1 to 3.
        let curve = line3([2., 0., 1.], [2., 0., 3.]);
        let report = curve_ruled_surface(&curve, &surface, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        assert_eq!(report.components.len(), 1, "{report:?}");
        let CurveRuledSurfaceComponent::Overlap {
            curve_interval,
            uv_start,
            uv_end,
            seam_wrap,
            correspondence,
            ..
        } = &report.components[0]
        else {
            panic!("Expected seam ruling overlap: {report:?}")
        };
        assert_eq!(*curve_interval, [0., 1.]);
        assert_eq!(uv_start[0], 0., "{report:?}");
        assert_eq!(uv_end[0], 0., "{report:?}");
        assert!(seam_wrap, "{report:?}");
        let Some(correspondence) = correspondence else {
            panic!("Expected correspondence: {report:?}")
        };
        assert_eq!(correspondence.kind, OverlapCorrespondenceKind::MobiusV);
        let lift = |z: f64| {
            let s = z / 4.;
            s / (top - s * (top - 1.))
        };
        assert!(
            (correspondence.samples[0][2] - lift(1.)).abs() < 1e-9,
            "{report:?}"
        );
        assert!(
            (correspondence.samples[2][2] - lift(3.)).abs() < 1e-9,
            "{report:?}"
        );
        assert!(
            correspondence.samples.iter().all(|s| s[1] == 0.),
            "{report:?}"
        );
        // The reconstructed Möbius t->v map matches direct evaluation at
        // seven interior parameters.
        for i in 1..=7 {
            let t = i as f64 / 8.;
            let v = mobius_through(&correspondence.samples, t);
            let p = surface.evaluate(0., v).unwrap().point;
            let q = curve.evaluate(t).unwrap().point;
            assert!(distance(p, point3(&q)) < 1e-9, "{report:?}");
        }
    }
}
#[test]
fn near_closed_seam_keeps_duplicates_behind_unresolved_bands() {
    let mut surface = ruled_cylinder(1.);
    // Perturb the seam's last control column: the seam rulings no longer
    // coincide exactly, so nothing merges by tolerance.
    surface.control_points[8][0][0] += 1e-7;
    surface.control_points[8][1][0] += 1e-7;
    let curve = line3([3., 0., 2.], [-3., 0., 2.]);
    let report = curve_ruled_surface(&curve, &surface, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::Incomplete, "{report:?}");
    for u in [0., 1.] {
        assert!(
            report.unresolved.iter().any(|band| {
                band.reason == UnresolvedReason::NearCoincidence
                    && band.parameter_box[2] == u
                    && band.parameter_box[3] == u
            }),
            "{report:?}"
        );
    }
    // No unification: the two seam-side contacts survive as separate
    // events instead of one canonical u=0 event.
    let points = cs_points(&report);
    let seam_side: Vec<_> = points
        .iter()
        .filter(|(_, uv)| uv[0] < 0.01 || uv[0] > 0.99)
        .collect();
    assert_eq!(seam_side.len(), 2, "{report:?}");
    assert!(
        points
            .iter()
            .any(|(t, uv)| (*t - 5. / 6.).abs() < 1e-9 && (uv[0] - 0.5).abs() < 1e-9),
        "{report:?}"
    );
}
#[test]
fn iso_v_overlap_carries_affine_u_correspondence() {
    let w = std::f64::consts::FRAC_1_SQRT_2;
    let bottom = [vec![2., 0., 0.], vec![2., 2., 0.], vec![0., 2., 0.]];
    let top = [vec![2., 0., 4.], vec![2., 2., 4.], vec![0., 2., 4.]];
    let surface = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..3)
            .map(|i| vec![bottom[i].clone(), top[i].clone()])
            .collect(),
        weights: [[1., 1.], [w, w], [1., 1.]]
            .iter()
            .map(|r| r.to_vec())
            .collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let curve = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: bottom.to_vec(),
        weights: vec![1., w, 1.],
        periodic: false,
    };
    let report = curve_ruled_surface(&curve, &surface, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert_eq!(report.components.len(), 1, "{report:?}");
    let CurveRuledSurfaceComponent::Overlap {
        seam_wrap,
        correspondence,
        ..
    } = &report.components[0]
    else {
        panic!("Expected iso-v overlap: {report:?}")
    };
    assert!(!seam_wrap, "{report:?}");
    let Some(correspondence) = correspondence else {
        panic!("Expected correspondence: {report:?}")
    };
    assert_eq!(correspondence.kind, OverlapCorrespondenceKind::AffineU);
    let s = &correspondence.samples;
    assert!(s.iter().all(|s| s[2] == 0.), "{report:?}");
    // Affine u(t) through the samples matches direct evaluation at five
    // interior parameters.
    for i in 1..=5 {
        let t = i as f64 / 6.;
        let u = s[0][1] + (s[2][1] - s[0][1]) * (t - s[0][0]) / (s[2][0] - s[0][0]);
        let p = surface.evaluate(u, s[0][2]).unwrap().point;
        let q = curve.evaluate(t).unwrap().point;
        assert!(distance(p, point3(&q)) < 1e-9, "{report:?}");
    }
}

#[test]
fn line_through_ruled_plane_resolves_exact_parameters() {
    let surface = flat();
    let curve = line3([0.25, 0.25, -1.], [0.25, 0.25, 1.]);
    let report = curve_ruled_surface(&curve, &surface, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert!(!report.permits_topology_change());
    assert_eq!(report.components.len(), 1, "{report:?}");
    let CurveRuledSurfaceComponent::Point {
        t,
        uv,
        point,
        residual,
        contact,
        ..
    } = &report.components[0]
    else {
        panic!("Expected point: {report:?}")
    };
    // Independent oracle: C(t)=(1/4,1/4,2t-1), z=0 at t=1/2, S(u,v)=(u,v,0).
    assert!((*t - 0.5).abs() <= 1e-10, "{report:?}");
    assert!((uv[0] - 0.25).abs() <= 1e-10, "{report:?}");
    assert!((uv[1] - 0.25).abs() <= 1e-10, "{report:?}");
    assert!(distance(*point, [0.25, 0.25, 0.]) <= 1e-9);
    assert!(*residual <= 1e-9);
    assert_eq!(*contact, Contact::Transverse);
}
#[test]
fn line_pierces_ruled_cylinder_twice_at_known_parameters() {
    let surface = ruled_cylinder(1.);
    // Line y=x at z=2 crosses the circle at 45 and 225 degrees: those are
    // the midpoints of quadratic spans 0 and 2, so u = 1/8 and 5/8, v = 1/2.
    let curve = line3([-3., -3., 2.], [3., 3., 2.]);
    for surface in [surface.clone(), transpose_surface(&surface)] {
        let report = curve_ruled_surface(&curve, &surface, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        let points = cs_points(&report);
        assert_eq!(points.len(), 2, "{report:?}");
        let s2 = std::f64::consts::SQRT_2;
        // Ascending t: (-sqrt2,-sqrt2) at 225 degrees is u=5/8 (span 2
        // midpoint); (sqrt2,sqrt2) at 45 degrees is u=1/8 (span 0 midpoint).
        let oracle = [((3. - s2) / 6., 0.625), ((3. + s2) / 6., 0.125)];
        for ((t, uv), (et, eu)) in points.iter().zip(oracle) {
            assert!((t - et).abs() < 1e-9, "{report:?}");
            let (uu, vv) = if surface.degree_v == 1 {
                (uv[0], uv[1])
            } else {
                (uv[1], uv[0])
            };
            assert!((uu - eu).abs() < 1e-9, "{report:?}");
            assert!((vv - 0.5).abs() < 1e-9, "{report:?}");
        }
        for component in &report.components {
            let CurveRuledSurfaceComponent::Point {
                point,
                residual,
                contact,
                ..
            } = component
            else {
                panic!("Expected points only: {report:?}")
            };
            assert!((point[0] * point[0] + point[1] * point[1] - 4.).abs() < 1e-9);
            assert!((point[2] - 2.).abs() < 1e-9);
            assert!(*residual <= 1e-9);
            assert_eq!(*contact, Contact::Transverse);
        }
    }
}
#[test]
fn ruling_coincidence_reports_overlap_with_lifted_uv_path() {
    for top in [1., 3.] {
        let surface = ruled_cylinder(top);
        // Ruling at 45 degrees (u=1/8): x=y=sqrt(2), z from 1 to 3.
        // z(v) = 4·s with s = v·top/(1-v+v·top): z=1 -> v=1/(2+2top)... solve
        // 4·top·v/(1-v+top·v) = z  =>  v = z/(4top - z(top-1)).
        let lift = |z: f64| z / (4. * top - z * (top - 1.));
        let curve = line3(
            [std::f64::consts::SQRT_2, std::f64::consts::SQRT_2, 1.],
            [std::f64::consts::SQRT_2, std::f64::consts::SQRT_2, 3.],
        );
        let report = curve_ruled_surface(&curve, &surface, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        assert_eq!(report.components.len(), 1, "{report:?}");
        let CurveRuledSurfaceComponent::Overlap {
            curve_interval,
            uv_start,
            uv_end,
            max_control_residual,
            ..
        } = &report.components[0]
        else {
            panic!("Expected ruling overlap: {report:?}")
        };
        assert_eq!(*curve_interval, [0., 1.]);
        assert!((uv_start[0] - 0.125).abs() < 1e-9, "{report:?}");
        assert!((uv_end[0] - 0.125).abs() < 1e-9, "{report:?}");
        assert!((uv_start[1] - lift(1.)).abs() < 1e-9, "{report:?}");
        assert!((uv_end[1] - lift(3.)).abs() < 1e-9, "{report:?}");
        assert!(*max_control_residual <= 1e-9);
        // The lifted path endpoints re-evaluate to the curve ends on the
        // source surface; along an unequal-weight ruling the parameter
        // correspondence is Möbius: with w1/w0 = top uniformly and
        // Cartesian fraction s along the ruling, v = s/(top-s(top-1)).
        let c = std::f64::consts::SQRT_2;
        for i in 0..=20 {
            let f = i as f64 / 20.;
            let expected = curve.evaluate(f).unwrap().point;
            let s = (1. + 2. * f) / 4.;
            let v = s / (top - s * (top - 1.));
            let p = surface.evaluate(uv_start[0], v).unwrap().point;
            assert!(distance(p, point3(&expected)) < 1e-9, "{report:?}");
            assert!((expected[0] - c).abs() < 1e-12);
            assert!((expected[2] - (1. + 2. * f)).abs() < 1e-12);
        }
    }
}
#[test]
fn iso_v_coincidence_reports_overlap_in_both_orientations() {
    // Open ruled patch between a quarter arc at z=0 and its lift at z=4.
    // (A closed-in-U surface would legitimately report seam-duplicate
    // contacts at u=0/1; that double cover is out of scope.)
    let w = std::f64::consts::FRAC_1_SQRT_2;
    let bottom = [vec![2., 0., 0.], vec![2., 2., 0.], vec![0., 2., 0.]];
    let top = [vec![2., 0., 4.], vec![2., 2., 4.], vec![0., 2., 4.]];
    let surface = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..3)
            .map(|i| vec![bottom[i].clone(), top[i].clone()])
            .collect(),
        weights: [[1., 1.], [w, w], [1., 1.]]
            .iter()
            .map(|r| r.to_vec())
            .collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let arc = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: bottom.to_vec(),
        weights: vec![1., w, 1.],
        periodic: false,
    };
    for reversed in [false, true] {
        let curve = if reversed {
            arc.reverse().unwrap()
        } else {
            arc.clone()
        };
        let report = curve_ruled_surface(&curve, &surface, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        assert_eq!(report.components.len(), 1, "{report:?}");
        let CurveRuledSurfaceComponent::Overlap {
            curve_interval,
            uv_start,
            uv_end,
            max_control_residual,
            ..
        } = &report.components[0]
        else {
            panic!("Expected iso-v overlap: {report:?}")
        };
        assert_eq!(*curve_interval, [0., 1.]);
        let (start_u, end_u) = if reversed { (1., 0.) } else { (0., 1.) };
        assert_eq!(*uv_start, [start_u, 0.]);
        assert_eq!(*uv_end, [end_u, 0.]);
        assert!(*max_control_residual <= 1e-9);
    }
}
#[test]
fn missing_and_grazing_lines_are_empty_or_unresolved_never_guessed() {
    let surface = ruled_cylinder(1.);
    let outside = line3([-3., -3., 6.], [3., 3., 6.]);
    let report = curve_ruled_surface(&outside, &surface, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert!(report.components.is_empty());
    // Tangent to the cylinder along the y direction at x=2, z=2.
    let tangent = line3([2., -1., 2.], [2., 1., 2.]);
    let report = curve_ruled_surface(&tangent, &surface, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::Incomplete, "{report:?}");
    assert!(
        !report
            .components
            .iter()
            .any(|c| matches!(c, CurveRuledSurfaceComponent::Point { .. })),
        "{report:?}"
    );
    assert!(
        report
            .unresolved
            .iter()
            .any(|u| u.reason == UnresolvedReason::TangencyOrMultipleRoot
                && u.parameter_box.len() == 6),
        "{report:?}"
    );
}
#[test]
fn ruled_surface_budget_exhaustion_preserves_pending_boxes() {
    let surface = ruled_cylinder(1.);
    let curve = line3([-3., -3., 2.], [3., 3., 2.]);
    let report = curve_ruled_surface(
        &curve,
        &surface,
        Options {
            max_boxes: 2,
            ..Options::default()
        },
    )
    .unwrap();
    assert_eq!(report.coverage, Coverage::Incomplete, "{report:?}");
    assert!(report.boxes_visited <= 2);
    assert!(
        report
            .unresolved
            .iter()
            .all(|u| u.reason == UnresolvedReason::BudgetExceeded && u.parameter_box.len() == 6),
        "{report:?}"
    );
    assert!(!report.unresolved.is_empty());
}
#[test]
fn curve_endpoints_own_contacts_at_all_four_patch_corners() {
    let surface = flat();
    for (corner, start) in [
        ([0., 0.], [-1., -1., 1.]),
        ([1., 0.], [2., -1., 1.]),
        ([1., 1.], [2., 2., 1.]),
        ([0., 1.], [-1., 2., 1.]),
    ] {
        let end = [corner[0], corner[1], 0.];
        let curve = line3(start, end);
        let report = curve_ruled_surface(&curve, &surface, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        assert_eq!(report.components.len(), 1, "{report:?}");
        let CurveRuledSurfaceComponent::Point {
            t,
            uv,
            point,
            residual,
            contact,
            ..
        } = &report.components[0]
        else {
            panic!("Expected corner contact: {report:?}")
        };
        assert_eq!(*t, 1.);
        assert_eq!(*uv, corner);
        assert!(distance(*point, end) <= 1e-9);
        assert!(*residual <= 1e-9);
        assert_eq!(*contact, Contact::Boundary);
    }
}
#[test]
fn ruled_query_invariant_under_knot_shift_scale_and_weight_scaling() {
    let mut surface = flat();
    surface.knots_u = vec![3., 3., 7., 7.];
    surface.knots_v = vec![-2., -2., 0., 0.];
    surface.weights = vec![vec![5., 5.]; 2];
    let mut curve = line3([0.25, 0.25, -1.], [0.25, 0.25, 1.]);
    curve.knots = vec![10., 10., 14., 14.];
    curve.weights = vec![0.25, 0.25];
    let report = curve_ruled_surface(&curve, &surface, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    let points = cs_points(&report);
    assert_eq!(points.len(), 1, "{report:?}");
    // t: 0.5 -> 10+4*0.5 = 12; u: 0.25 -> 3+4*0.25 = 4; v: 0.25 -> -1.5.
    assert!((points[0].0 - 12.).abs() <= 4e-10, "{report:?}");
    assert!((points[0].1[0] - 4.).abs() <= 4e-10, "{report:?}");
    assert!((points[0].1[1] + 1.5).abs() <= 2e-10, "{report:?}");
}
#[test]
fn ruled_query_reversal_maps_curve_parameter_and_keeps_uv() {
    let surface = ruled_cylinder(1.);
    let curve = line3([-3., -3., 2.], [3., 3., 2.]);
    let forward = curve_ruled_surface(&curve, &surface, Options::default()).unwrap();
    let reversed =
        curve_ruled_surface(&curve.reverse().unwrap(), &surface, Options::default()).unwrap();
    let (a, b) = (cs_points(&forward), cs_points(&reversed));
    assert_eq!(a.len(), 2, "{forward:?}");
    assert_eq!(b.len(), 2, "{reversed:?}");
    // Reversal flips event order: first reversed event is the last forward one.
    for ((t, uv), (rt, ruv)) in a.iter().zip(b.iter().rev()) {
        assert!((1. - t - rt).abs() < 1e-9, "{forward:?} {reversed:?}");
        assert!((uv[0] - ruv[0]).abs() < 1e-12, "{forward:?} {reversed:?}");
        assert!((uv[1] - ruv[1]).abs() < 1e-12, "{forward:?} {reversed:?}");
    }
}
#[test]
fn ruled_query_refuses_unsupported_surfaces_and_invalid_inputs() {
    // A biquadratic tensor patch is not a ruled surface in either direction.
    let curved = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| vec![i as f64, j as f64, (i * j) as f64 * 0.25])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let curve = line3([0.25, 0.25, -1.], [0.25, 0.25, 1.]);
    let report = curve_ruled_surface(&curve, &curved, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::Incomplete, "{report:?}");
    assert_eq!(report.unresolved.len(), 1);
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::UnsupportedSurface
    );
    assert_eq!(report.unresolved[0].parameter_box.len(), 6);
    assert!(!report.permits_topology_change());
    let flat2d = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.], vec![1., 0.]],
        weights: vec![1., 1.],
        periodic: false,
    };
    assert!(curve_ruled_surface(&flat2d, &flat(), Options::default()).is_err());
    assert!(
        curve_ruled_surface(
            &curve,
            &flat(),
            Options {
                max_boxes: 0,
                ..Options::default()
            }
        )
        .is_err()
    );
}

/// Piecewise-linear curve through a C0 kink at (1,1,0) on the plane z=0;
/// both one-sided tangents leave the plane transversally.
fn kink_curve() -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 2., 2.],
        control_points: vec![vec![0., 0., -1.], vec![1., 1., 0.], vec![2., 0., 1.]],
        weights: vec![1.; 3],
        periodic: false,
    }
}
/// Ruled plane z=0 over x in [-1,3], y in [-1,2] (canonical ruled-in-V).
fn wide_ruled_plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![-1., -1., 0.], vec![-1., 2., 0.]],
            vec![vec![3., -1., 0.], vec![3., 2., 0.]],
        ],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    }
}
/// Cubic with a single transverse root of z=0 exactly at t=1/2 whose de
/// Casteljau split value is not a bitwise zero (thirds in the polygon).
fn dyadic_root_cubic() -> Curve {
    bezier(&[-2., -1. / 3., 2. / 3., 1.])
}
#[test]
fn c0_knot_plane_crossing_resolves_with_one_sided_jets() {
    let report = curve_plane(&kink_curve(), plane(0.), Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert!(report.unresolved.is_empty(), "{report:?}");
    let roots = points(&report);
    assert_eq!(roots.len(), 1, "{report:?}");
    assert_eq!(roots[0].parameter, 1.);
    assert_eq!(roots[0].point, [1., 1., 0.]);
    assert_eq!(roots[0].contact, Contact::Boundary);
    assert!(roots[0].plane_residual <= 1e-9);
}
#[test]
fn c0_knot_curve_curve_crossing_resolves_with_one_sided_jets() {
    // The kink at t=1 is crossed at u=1/2 by a line transverse to both
    // one-sided tangents; the knot-aligned root must not collapse into
    // an unresolved band.
    let b = line3([1., 0., -1.], [1., 2., 1.]);
    let report = curve_curve(&kink_curve(), &b, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    let roots = cc_points(&report);
    assert_eq!(roots.len(), 1, "{report:?}");
    assert_eq!(roots[0].0, 1.);
    assert_eq!(roots[0].1, 0.5);
}
#[test]
fn c0_knot_ruled_surface_crossing_resolves_with_one_sided_jets() {
    // The kink at t=1 pierces the ruled plane z=0 at (u,v)=(1/2,2/3).
    let report =
        curve_ruled_surface(&kink_curve(), &wide_ruled_plane(), Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    let roots = cs_points(&report);
    assert_eq!(roots.len(), 1, "{report:?}");
    assert_eq!(roots[0].0, 1.);
    assert!((roots[0].1[0] - 0.5).abs() <= 1e-12, "{report:?}");
    assert!((roots[0].1[1] - 2. / 3.).abs() <= 1e-12, "{report:?}");
}
#[test]
fn dyadic_boundary_root_is_reported_once_at_the_exact_parameter() {
    // Root bitwise on the depth-1 subdivision face t=1/2 of [0,1].
    let report = curve_plane(&dyadic_root_cubic(), plane(0.), Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert!(report.unresolved.is_empty(), "{report:?}");
    let roots = points(&report);
    assert_eq!(roots.len(), 1, "{report:?}");
    assert_eq!(roots[0].parameter, 0.5);
    assert!(roots[0].plane_residual <= 1e-9);
    assert!(
        roots[0].parameter_interval[0] <= 0.5 && 0.5 <= roots[0].parameter_interval[1],
        "{report:?}"
    );
}
#[test]
fn dyadic_boundary_curve_curve_root_is_reported_once() {
    // Both parameters land bitwise on subdivision faces: t=u=1/2.
    let b = line3([0.5, -1., -1.], [0.5, 1., 1.]);
    let report = curve_curve(&dyadic_root_cubic(), &b, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert!(report.unresolved.is_empty(), "{report:?}");
    let roots = cc_points(&report);
    assert_eq!(roots.len(), 1, "{report:?}");
    assert_eq!(roots[0], (0.5, 0.5));
    // Reversed operand order owns the same boundary root exactly once.
    let swapped = curve_curve(&b, &dyadic_root_cubic(), Options::default()).unwrap();
    let swapped_roots = cc_points(&swapped);
    assert_eq!(swapped_roots.len(), 1, "{swapped:?}");
    assert_eq!(swapped_roots[0], (0.5, 0.5));
}
#[test]
fn dyadic_boundary_ruled_surface_root_is_reported_once() {
    // The cubic pierces the ruled plane z=0 at t=1/2, u=3/8, v=1/3.
    let report = curve_ruled_surface(
        &dyadic_root_cubic(),
        &wide_ruled_plane(),
        Options::default(),
    )
    .unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert!(report.unresolved.is_empty(), "{report:?}");
    let roots = cs_points(&report);
    assert_eq!(roots.len(), 1, "{report:?}");
    assert_eq!(roots[0].0, 0.5);
    assert!((roots[0].1[0] - 0.375).abs() <= 1e-12, "{report:?}");
    assert!((roots[0].1[1] - 1. / 3.).abs() <= 1e-12, "{report:?}");
}
#[test]
fn domain_end_root_is_owned_exactly_once_across_queries() {
    // Cubic ending on the plane: C(1)=0 with a transverse derivative and
    // a strictly negative interior (no other roots).
    let ending = bezier(&[-2., -1., -1. / 3., 0.]);
    let report = curve_plane(&ending, plane(0.), Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    let roots = points(&report);
    assert_eq!(roots.len(), 1, "{report:?}");
    assert_eq!(roots[0].parameter, 1.);
    assert_eq!(roots[0].contact, Contact::Boundary);
    // Curve/curve at the shared domain end (1,1).
    let b = line3([1., -1., -1.], [1., 1., 1.]);
    let report = curve_curve(&ending, &b, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert_eq!(cc_points(&report), vec![(1., 0.5)], "{report:?}");
    // Curve/ruled-surface at the curve's domain end.
    let report = curve_ruled_surface(&ending, &wide_ruled_plane(), Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    let roots = cs_points(&report);
    assert_eq!(roots.len(), 1, "{report:?}");
    assert_eq!(roots[0].0, 1.);
}
#[test]
fn full_sweep_over_knot_dyadic_and_domain_end_roots_has_no_dupes_or_misses() {
    // Degree-1 curve with a C0 kink root at the knot t=1/2, a dyadic
    // mid-span root at t=5/4 and a domain-end root at t=2; all crossings
    // lie on the x-axis segment from (0,0,0) to (4,0,0).
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 0.5, 1., 1.5, 2., 2.],
        control_points: vec![
            vec![0., 0., -1.],
            vec![1., 0., 0.],
            vec![2., 0., 2.],
            vec![3., 0., -2.],
            vec![4., 0., 0.],
        ],
        weights: vec![1.; 5],
        periodic: false,
    };
    let report = curve_plane(&curve, plane(0.), Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert!(report.unresolved.is_empty(), "{report:?}");
    let parameters: Vec<f64> = points(&report).iter().map(|p| p.parameter).collect();
    assert_eq!(parameters, vec![0.5, 1.25, 2.], "{report:?}");
    // The same sweep through curve/segment keeps one event per root.
    let report = curve_segment(&curve, [0., 0., 0.], [4., 0., 0.], Options::default()).unwrap();
    let hits = report
        .components
        .iter()
        .filter(|c| matches!(c, CurveSegmentComponent::Point { .. }))
        .count();
    assert_eq!(hits, 3, "{report:?}");
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
}
