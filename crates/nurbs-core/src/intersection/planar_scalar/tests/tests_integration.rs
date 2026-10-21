    use super::*;
    fn cubic(y: [f64; 4]) -> Curve {
        Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: (0..4).map(|i| vec![i as f64, y[i]]).collect(),
            weights: vec![1.; 4],
            periodic: false,
        }
    }
    #[test]
    fn resolves_interior_second_and_third_order_contacts_without_chords() {
        let zero = cubic([0.; 4]);
        for (y, order) in [([3., -1., -1., 3.], 2), ([-1., 1., -1., 1.], 3)] {
            let other = cubic(y);
            let Some(Relation::Points(events)) = intersect(&zero, &other, 1e-7).unwrap() else {
                panic!("Missing scalar reduction")
            };
            assert_eq!(events.len(), 1);
            assert!((events[0].first - 0.5).abs() < 1e-10);
            assert_eq!(events[0].multiplicity, order);
        }
    }
    #[test]
    fn line_curve_tangency_has_rational_line_parameters_and_finite_segment_ownership() {
        let mut line = Curve::from_polyline(vec![vec![0., 0.], vec![3., 0.]]).unwrap();
        line.weights = vec![1., 2.];
        let curve = cubic([3., -1., -1., 3.]);
        let Some(Relation::Points(events)) = intersect(&line, &curve, 1e-7).unwrap() else {
            panic!("Missing line reduction")
        };
        assert_eq!(events.len(), 1);
        assert!((events[0].first - 1. / 3.).abs() < 1e-10);
        assert_eq!(events[0].multiplicity, 2);
        let short = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
        assert!(
            matches!(intersect(&short,&curve,1e-7).unwrap(),Some(Relation::Points(p)) if p.is_empty())
        );
    }
    #[test]
    fn coincident_reversed_and_scaled_weight_definitions_keep_their_identity() {
        let a = cubic([0., 1., 2., 1.]);
        let mut b = a.reverse().unwrap();
        b.weights = vec![8.; 4];
        assert!(matches!(
            intersect(&a, &b, 1e-7).unwrap(),
            Some(Relation::Coincident { reversed: true, .. })
        ));
    }
