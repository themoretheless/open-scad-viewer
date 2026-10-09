use super::*;
#[test]
fn periodic_offset_closes_with_a_bounded_seam() {
    let source = Curve {
        degree: 2,
        knots: (0..9).map(|i| i as f64).collect(),
        control_points: vec![
            vec![1., 0.],
            vec![0., 1.],
            vec![-1., 0.],
            vec![0., -1.],
            vec![1., 0.],
            vec![0., 1.],
        ],
        weights: vec![1., 0.7, 1.1, 0.9, 1., 0.7],
        periodic: true,
    };
    let before = format!("{source:?}");
    for distance in [-0.2, 0.2] {
        let result = approximate_curve(&source, distance, 1e-3, 65536).unwrap();
        assert!(result.closed);
        assert_eq!(
            result.curves.first().unwrap().control_points.first(),
            result.curves.last().unwrap().control_points.last()
        );
        for cell in &result.segments {
            for station in 0..=12 {
                let t =
                    cell.domain[0] + (cell.domain[1] - cell.domain[0]) * station as f64 / 12.;
                let e = source.evaluate(t).unwrap();
                let d = e.d1.unwrap();
                let speed = d[0].hypot(d[1]);
                let expected = [
                    e.point[0] - distance * d[1] / speed,
                    e.point[1] + distance * d[0] / speed,
                ];
                let retained = result
                    .curves
                    .iter()
                    .find(|c| t >= c.domain()[0] && t <= c.domain()[1])
                    .unwrap();
                let point = retained.evaluate(t).unwrap().point;
                assert!(
                    (expected[0] - point[0]).hypot(expected[1] - point[1])
                        <= result.error_upper_mm
                );
            }
        }
    }
    let zero = approximate_curve(&source, 0., 1e-3, 1).unwrap();
    assert!(zero.closed && zero.curves[0].periodic);
    assert_eq!(format!("{:?}", zero.curves[0]), before);
    assert_eq!(before, format!("{source:?}"));
}
#[test]
fn periodic_corners_require_the_same_explicit_join_as_open_corners() {
    let source = Curve {
        degree: 1,
        knots: vec![-1., 0., 1., 2., 3., 4., 5.],
        control_points: vec![
            vec![0., 0.],
            vec![1., 0.],
            vec![1., 1.],
            vec![0., 1.],
            vec![0., 0.],
        ],
        weights: vec![1.; 5],
        periodic: true,
    };
    source.validate().unwrap();
    let error = approximate_curve(&source, 0.1, 1e-3, 65536).unwrap_err();
    assert!(
        error.message.contains("parameter 0")
            && error.message.contains("explicit profile join")
    );
}
#[test]
fn connected_result_keeps_source_parameterization_and_bound() {
    let source = Curve {
        degree: 3,
        knots: vec![-3., -3., -3., -3., -1., 2., 7., 7., 7., 7.],
        control_points: vec![
            vec![0.1, 0.3],
            vec![1., 2.],
            vec![3., -1.],
            vec![5., 2.],
            vec![7., 1.],
            vec![9., 3.],
        ],
        weights: vec![0.1, 0.7, 0.3, 0.9, 0.5, 1.2],
        periodic: false,
    };
    let before = format!("{source:?}");
    for distance in [-2., 2.] {
        let result = approximate_curve(&source, distance, 1e-3, 65536).unwrap();
        assert_eq!(
            result.curves.first().unwrap().domain()[0],
            source.domain()[0]
        );
        assert_eq!(
            result.curves.last().unwrap().domain()[1],
            source.domain()[1]
        );
        assert!(
            result
                .curves
                .iter()
                .all(|c| c.degree == 1 && c.control_points.len() <= 256)
        );
        for pair in result.curves.windows(2) {
            assert_eq!(pair[0].domain()[1], pair[1].domain()[0]);
            assert_eq!(
                pair[0].control_points.last(),
                pair[1].control_points.first()
            );
        }
        assert!(result.error_upper_mm <= 1e-3);
        for pair in result.segments.windows(2) {
            assert_eq!(pair[0].points[1], pair[1].points[0]);
        }
        for cell in &result.segments {
            for station in 0..=12 {
                let t =
                    cell.domain[0] + (cell.domain[1] - cell.domain[0]) * station as f64 / 12.;
                let e = source.evaluate(t).unwrap();
                let d = e.d1.unwrap();
                let speed = d[0].hypot(d[1]);
                let exact = [
                    e.point[0] - distance * d[1] / speed,
                    e.point[1] + distance * d[0] / speed,
                ];
                let retained = result
                    .curves
                    .iter()
                    .find(|c| t >= c.domain()[0] && t <= c.domain()[1])
                    .unwrap();
                let actual = retained.evaluate(t).unwrap().point;
                assert!(
                    (exact[0] - actual[0]).hypot(exact[1] - actual[1]) <= cell.error_upper_mm
                );
            }
        }
    }
    assert_eq!(before, format!("{source:?}"));
}
#[test]
fn zero_is_identity_and_corners_require_a_join_policy() {
    let source = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 2., 2.],
        control_points: vec![vec![0., 0.], vec![2., 0.], vec![2., 3.]],
        weights: vec![1., 0.7, 1.],
        periodic: false,
    };
    let before = format!("{source:?}");
    let identity = approximate_curve(&source, 0., 1e-3, 1).unwrap();
    assert_eq!(format!("{:?}", identity.curves[0]), before);
    assert_eq!(identity.error_upper_mm, 0.);
    assert!(
        approximate_curve(&source, 1., 1e-3, 65536)
            .unwrap_err()
            .message
            .contains("parameter 1")
    );
    assert_eq!(before, format!("{source:?}"));
}
#[cfg(feature = "transport")]
#[test]
fn bounded_offset_transport_reports_curve_scope() {
    use value_codec::json;
    let source = json!({"degree":2,"knots":[0.,0.,0.,1.,1.,1.],
        "controlPoints":[[0.,0.],[1.,1.],[2.,0.]],"weights":[1.,0.7,1.],"periodic":false});
    let request = json!({"op":"curve_offset_bounded","curve":source.clone(),"distance":0.2,"toleranceMm":0.001,"maxCells":4096});
    let result = crate::dispatch(request.clone()).unwrap();
    assert_eq!(result["report"]["wholeCurve"], true);
    assert_eq!(result["report"]["regionTopologyCertified"], false);
    assert_eq!(result["report"]["offsetRegularityCertified"], false);
    assert!(result["report"]["errorUpperMm"].as_f64().unwrap() <= 0.001);
    assert_eq!(request["curve"], source);
    let mut short = request;
    short["maxPairs"] = json!(1);
    let limited = crate::dispatch(short.clone()).unwrap();
    assert_eq!(limited["report"]["chainDiagnostics"]["complete"], false);
    assert_eq!(limited["report"]["chainDiagnostics"]["checks"], 1);
    short["maxCells"] = json!(1);
    assert!(crate::dispatch(short).is_err());
}
#[test]
fn rational_offset_error_bound_covers_dense_oracle() {
    let curve = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]],
        weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
        periodic: false,
    };
    for distance in [-0.25, 0.25, 1.25] {
        let pieces = approximate(&curve, distance, 1e-4, 65536).unwrap();
        assert!(pieces.len() < 2000);
        for piece in pieces {
            for step in 0..=20 {
                let x = step as f64 / 20.;
                let t = piece.domain[0] * (1. - x) + piece.domain[1] * x;
                let e = curve.evaluate(t).unwrap();
                let d = e.d1.unwrap();
                let speed = d[0].hypot(d[1]);
                let exact = [
                    e.point[0] - distance * d[1] / speed,
                    e.point[1] + distance * d[0] / speed,
                ];
                let line = [
                    piece.points[0][0] * (1. - x) + piece.points[1][0] * x,
                    piece.points[0][1] * (1. - x) + piece.points[1][1] * x,
                ];
                assert!((exact[0] - line[0]).hypot(exact[1] - line[1]) <= piece.error_upper_mm);
            }
        }
    }
}
#[test]
fn cusp_and_budget_refuse_without_mutating_source() {
    let curve = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0.], vec![1., 0.], vec![0., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let before = format!("{curve:?}");
    assert!(approximate(&curve, 1., 1e-6, 128).is_err());
    assert_eq!(before, format!("{curve:?}"));
}

#[test]
fn non_finite_offset_inputs_are_typed_rejections() {
    let curve = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
    let err = approximate_curve(&curve, f64::NAN, 1e-3, 64).unwrap_err();
    assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
    assert!(err.contains("distance"), "{err}");
    let err = approximate(&curve, 0.1, f64::INFINITY, 64).unwrap_err();
    assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
    assert!(err.contains("tolerance_mm"), "{err}");
    let radius = Curve::from_polyline(vec![vec![0.1, 0.], vec![0.2, 0.]]).unwrap();
    let err = approximate_variable(&curve, &radius, f64::NAN, 64).unwrap_err();
    assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
}
#[test]
fn unified_guard_never_preempts_existing_cell_budget() {
    // The "offset_fit" BudgetGuard sits strictly above the legacy cell
    // budget: the legacy resource error must keep firing first.
    let curve = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0.], vec![1., 0.], vec![0., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let err = approximate(&curve, 1., 1e-12, 8).unwrap_err();
    assert_eq!(err.code, crate::RESOURCE_LIMIT, "{err:?}");
}
#[test]
fn noncircular_multispan_offset_preserves_domain_and_bounds() {
    for shift in [-3., 1e6] {
        let curve = Curve {
            degree: 3,
            knots: vec![
                shift,
                shift,
                shift,
                shift,
                shift + 2.,
                shift + 5.,
                shift + 10.,
                shift + 10.,
                shift + 10.,
                shift + 10.,
            ],
            control_points: vec![
                vec![0.1, 0.3],
                vec![1., 2.],
                vec![3., -1.],
                vec![5., 2.],
                vec![7., 1.],
                vec![9., 3.],
            ],
            weights: vec![0.1, 0.7, 0.3, 0.9, 0.5, 1.2],
            periodic: false,
        };
        for distance in [-2., 0., 2.] {
            let pieces = approximate(&curve, distance, 1e-3, 65536).unwrap();
            assert_eq!(pieces.first().unwrap().domain[0], shift);
            assert_eq!(pieces.last().unwrap().domain[1], shift + 10.);
            for pair in pieces.windows(2) {
                assert_eq!(pair[0].domain[1], pair[1].domain[0]);
            }
            for piece in pieces {
                assert!(piece.error_upper_mm <= 1e-3);
                for step in 0..=8 {
                    let x = step as f64 / 8.;
                    let t = piece.domain[0] + (piece.domain[1] - piece.domain[0]) * x;
                    let e = curve.evaluate(t).unwrap();
                    let d = e.d1.unwrap();
                    let speed = d[0].hypot(d[1]);
                    let exact = [
                        e.point[0] - distance * d[1] / speed,
                        e.point[1] + distance * d[0] / speed,
                    ];
                    let line = [
                        piece.points[0][0] * (1. - x) + piece.points[1][0] * x,
                        piece.points[0][1] * (1. - x) + piece.points[1][1] * x,
                    ];
                    assert!(
                        (exact[0] - line[0]).hypot(exact[1] - line[1]) <= piece.error_upper_mm
                    );
                }
            }
        }
    }
}

fn scalar_curve(values: &[[f64; 2]]) -> Curve {
    let count = values.len();
    let mut knots = vec![0.; 2];
    knots.extend((1..count - 1).map(|i| i as f64));
    knots.extend([count as f64 - 1., count as f64 - 1.]);
    Curve {
        degree: 1,
        knots,
        control_points: values.iter().map(|v| v.to_vec()).collect(),
        weights: vec![1.; count],
        periodic: false,
    }
}

#[test]
fn variable_offset_of_a_line_is_the_analytic_rotated_line() {
    let source = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
    // r(t) linear from 0.1 to 0.3: the offset is the line y = 0.1 + 0.2·x.
    let radius = scalar_curve(&[[0.1, 0.], [0.3, 0.]]);
    let report = approximate_variable(&source, &radius, 1e-6, 64).unwrap();
    assert!(report.within_tolerance);
    assert!(report.min_fold_margin.is_infinite());
    let fitted = &report.curves[0];
    let [a, b] = fitted.domain();
    for step in 0..=10 {
        let u = a + (b - a) * step as f64 / 10.;
        let point = fitted.evaluate(u).unwrap().point;
        let expected = 0.1 + 0.2 * point[0];
        let residual = (point[1] - expected).abs();
        assert!(
            residual <= report.error_upper_mm + 1e-9,
            "u={u} point={point:?} residual={residual}"
        );
    }
}

#[test]
fn variable_offset_refuses_a_folding_radius_on_a_tight_circle() {
    let w = std::f64::consts::FRAC_1_SQRT_2;
    let circle = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.],
        control_points: [
            [1., 0.],
            [1., 1.],
            [0., 1.],
            [-1., 1.],
            [-1., 0.],
            [-1., -1.],
            [0., -1.],
            [1., -1.],
            [1., 0.],
        ]
        .iter()
        .map(|p| p.to_vec())
        .collect(),
        weights: vec![1., w, 1., w, 1., w, 1., w, 1.],
        periodic: false,
    };
    // Inward radius 1.5 exceeds the circle radius 1: r + 1/κ = -0.5 folds.
    let inward = scalar_curve(&[[-1.5, 0.], [-1.5, 0.]]);
    let error = approximate_variable(&circle, &inward, 1e-6, 64).unwrap_err();
    assert!(error.message.contains("folds locally"), "{error:?}");
    // A safe outward radius fits cleanly with a positive margin.
    let outward = scalar_curve(&[[0.5, 0.], [0.5, 0.]]);
    let report = approximate_variable(&circle, &outward, 1e-5, 128).unwrap();
    assert!(report.min_fold_margin > 0.);
    assert!(report.max_deviation <= 1e-5);
}
