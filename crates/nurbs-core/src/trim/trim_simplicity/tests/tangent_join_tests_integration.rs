    use super::*;
    #[test]
    fn positive_bezier_handles_admit_tangency_but_not_crossings_or_flat_spline_returns() {
        let line = Curve::from_polyline(vec![vec![0., 0.], vec![10., 0.]]).unwrap();
        let arc = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![3., 3.], vec![3., 0.], vec![0., 0.]],
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
            periodic: false,
        };
        assert!(adjacent_separated(&line, &arc, &[0., 0.]));
        assert!(prove_endpoint_contacts_only(&line, &arc).unwrap());
        let mut crossing = arc.clone();
        crossing.control_points[1][1] = -3.;
        assert!(!adjacent_separated(&line, &crossing, &[0., 0.]));
        assert!(!prove_endpoint_contacts_only(&line, &crossing).unwrap());
        let spline = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
            control_points: vec![
                vec![3., 3.],
                vec![3., 0.],
                vec![2., 0.],
                vec![1., 0.],
                vec![0., 0.],
            ],
            weights: vec![1.; 5],
            periodic: false,
        };
        spline.validate().unwrap();
        assert!(!adjacent_separated(&line, &spline, &[0., 0.]));
        assert!(!prove_endpoint_contacts_only(&line, &spline).unwrap());
    }
