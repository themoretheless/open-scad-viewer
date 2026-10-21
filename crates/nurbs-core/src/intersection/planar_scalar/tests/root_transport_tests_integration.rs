    use super::*;
    #[test]
    fn unrepresentable_interior_contact_is_refused_instead_of_becoming_an_endpoint() {
        let a = 1e8_f64;
        let b = a.next_up();
        let c = Curve {
            degree: 3,
            knots: vec![a, a, a, a, b, b, b, b],
            control_points: vec![vec![0., 0.], vec![1., 1.], vec![2., -1.], vec![3., 0.]],
            weights: vec![1.; 4],
            periodic: false,
        };
        let line = Curve::from_polyline(vec![vec![0., 0.], vec![3., 0.]]).unwrap();
        assert!(intersect(&c, &line, 1e-7).is_err());
        assert!(stationary(&c, 1).is_err());
        assert!(invert(&c, 0, 1.5).is_err());
        assert_eq!(root_parameter(&c, [0., 0.], None).unwrap(), a);
        assert_eq!(root_parameter(&c, [1., 1.], None).unwrap(), b);
        let unit = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
        assert_eq!(
            root_parameter(&unit, [1_f64.next_down(), 1.], None).unwrap(),
            1.
        );
        assert_eq!(
            root_parameter(&unit, [1_f64.next_down(), 1.], Some(1e-7)).unwrap(),
            1.
        );
        let mut signed = c.clone();
        signed.knots = vec![-1e8, -1e8, -1e8, -1e8, 1e-8, 1e-8, 1e-8, 1e-8];
        assert!(map_back(&signed, 1e-8_f64.next_down(), true).is_err());
    }
