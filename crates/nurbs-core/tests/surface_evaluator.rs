use nurbs_core::surface::{Evaluation, Surface, SurfaceEvaluator};

fn fixture() -> Surface {
    let surface = Surface {
        degree_u: 3,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 0., 0.35, 1.1, 2.2, 2.2, 3., 3., 3., 3.],
        knots_v: vec![0., 0., 0., 0.4, 1.7, 2., 2., 2.],
        control_points: (0..8)
            .map(|i| {
                (0..5)
                    .map(|j| {
                        vec![
                            1000. + i as f64 * 0.8 + j as f64 * j as f64 * 0.07,
                            -500. + j as f64 * 0.9 + (i * j) as f64 * 0.03,
                            20. + (i as f64 * 0.7).sin() + (j as f64 * 0.4).cos(),
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: (0..8)
            .map(|i| {
                (0..5)
                    .map(|j| 0.2 + ((i * 7 + j * 3) % 9) as f64 * 0.35)
                    .collect()
            })
            .collect(),
        periodic_u: false,
        periodic_v: false,
    };
    surface.validate().unwrap();
    surface
}

fn assert_vector_close(actual: [f64; 3], expected: [f64; 3], tolerance: f64) {
    for axis in 0..3 {
        assert!(
            (actual[axis] - expected[axis]).abs() <= tolerance,
            "axis {axis}: actual={}, expected={}",
            actual[axis],
            expected[axis]
        );
    }
}

fn assert_jet_close(actual: &Evaluation, expected: &Evaluation) {
    assert_vector_close(actual.point, expected.point, 1e-8);
    let rounding_bound = actual.rounding_bound().unwrap();
    assert!(rounding_bound.is_finite() && rounding_bound >= 0.);
    assert_eq!(expected.rounding_bound(), None);
    match (actual.first_derivatives(), expected.first_derivatives()) {
        (Some(actual), Some(expected)) => {
            assert_vector_close(actual.0, expected.0, 1e-6);
            assert_vector_close(actual.1, expected.1, 1e-6);
        }
        (None, None) => {}
        _ => panic!("first derivative availability differs"),
    }
    match (actual.second_derivatives(), expected.second_derivatives()) {
        (Some(actual), Some(expected)) => {
            assert_vector_close(actual.0, expected.0, 1e-6);
            assert_vector_close(actual.1, expected.1, 1e-6);
            assert_vector_close(actual.2, expected.2, 1e-6);
        }
        (None, None) => {}
        _ => panic!("second derivative availability differs"),
    }
    match (actual.unit_normal(), expected.unit_normal()) {
        (Some(actual), Some(expected)) => assert_vector_close(actual, expected, 1e-8),
        (None, None) => {}
        _ => panic!("normal availability differs"),
    }
    match (actual.curvatures(), expected.curvatures()) {
        (Some(actual), Some(expected)) => {
            assert!((actual.0 - expected.0).abs() <= 1e-6);
            assert!((actual.1 - expected.1).abs() <= 1e-6);
        }
        (None, None) => {}
        _ => panic!("curvature availability differs"),
    }
    assert_eq!(actual.derivative_status(), expected.derivative_status());
    assert_eq!(actual.derivative_sides(), expected.derivative_sides());
    assert_eq!(actual.domains(), expected.domains());
}

#[test]
fn public_evaluator_matches_existing_evaluation() {
    let surface = fixture();
    let mut evaluator = SurfaceEvaluator::new(&surface).unwrap();
    assert!(std::ptr::eq(evaluator.definition(), &surface));

    for u in [0., 0.1, 0.35, 0.7, 1.1, 1.9, 2.8, 3.] {
        for v in [0., 0.2, 0.4, 1., 1.7, 1.999, 2.] {
            let actual = evaluator.evaluate(u, v).unwrap();
            let expected = surface.evaluate(u, v).unwrap();
            assert_jet_close(&actual, &expected);
        }
    }

    for (u, v) in [(-0.1, 0.5), (3.1, 0.5), (0.5, -0.1), (0.5, 2.1)] {
        assert!(evaluator.evaluate(u, v).is_err());
    }
}
