    use super::*;
    #[test]
    fn rotated_curved_tangency_keeps_its_multiplicity_and_reversed_parameter() {
        let curve = |ys: [f64; 4]| Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: (0..4)
                .map(|i| vec![17. + 10. * i as f64 - ys[i], -9. + 10. * i as f64 + ys[i]])
                .collect(),
            weights: vec![1.; 4],
            periodic: false,
        };
        let a = curve([0., 0., 1., 3.]);
        let b = curve([1., -1., 1., 7.]).reverse().unwrap();
        let Some(Relation::Points(points)) = intersect(&a, &b, 1e-7).unwrap() else {
            panic!("Missing rotated contact")
        };
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].multiplicity, 2);
        assert!((points[0].first - 1. / 3.).abs() < 1e-11);
        assert!((points[0].second - 2. / 3.).abs() < 1e-11);
    }
    #[test]
    fn exact_monotonicity_does_not_require_an_ordered_control_polygon() {
        let mut c = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![1., 1.1], vec![0.9, 0.8], vec![1.9, 2.1]],
            weights: vec![1.; 4],
            periodic: false,
        };
        let before = c.clone();
        for axis in 0..2 {
            assert!(c.control_points[1][axis] > c.control_points[2][axis]);
            assert!(stationary(&c, axis).unwrap().unwrap().is_empty());
            assert!(monotone(&c, axis));
        }
        let b = c.trim(0.25, 0.75).unwrap().reverse().unwrap();
        assert!(matches!(
            intersect(&c, &b, 1e-7).unwrap(),
            Some(Relation::Overlap { .. })
        ));
        assert_eq!(c.control_points, before.control_points);
        assert_eq!(c.weights, before.weights);
        // The proof must use the original rational weights: these weights
        // make the same control polygon reverse in both coordinates.
        c.weights = vec![1., 100., 100., 1.];
        assert!(!monotone(&c, 0));
        assert!(!monotone(&c, 1));
    }
    #[test]
    fn stationary_even_multiplicity_preserves_monotonicity_but_reversals_do_not() {
        let mut c = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![1., 1.], vec![-1., 2.], vec![3., 3.]],
            weights: vec![1.; 4],
            periodic: false,
        };
        assert!(monotone(&c, 0));
        assert!(monotone(&c.reverse().unwrap(), 0));
        c.control_points[3][0] = 0.1;
        assert!(!monotone(&c, 0));
    }
    #[test]
    fn stationary_parameters_find_a_non_dyadic_cusp_without_mutating_its_definition() {
        let c = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![vec![1., -1.], vec![-1., 2.], vec![0., -4.], vec![4., 8.]],
            weights: vec![1.; 4],
            periodic: false,
        };
        let before = c.control_points.clone();
        for axis in 0..2 {
            let roots = stationary(&c, axis).unwrap().unwrap();
            assert_eq!(roots.len(), 1);
            assert!((roots[0] - 1. / 3.).abs() < 1e-13);
        }
        assert_eq!(c.control_points, before);
    }
