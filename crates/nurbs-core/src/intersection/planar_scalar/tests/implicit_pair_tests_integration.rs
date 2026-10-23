    use super::*;
    #[test]
    fn curved_third_order_contact_with_nonlinear_parameter_map_is_complete() {
        let c = |points: Vec<Vec<f64>>| Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: points,
            weights: vec![1.; 4],
            periodic: false,
        };
        let a = c(vec![vec![0., 0.], vec![2., 0.], vec![4., 2.], vec![6., 6.]]);
        let b = c(vec![
            vec![0., 0.],
            vec![2., 0.],
            vec![6., 2.],
            vec![12., 12.],
        ]);
        let before = (a.clone(), b.clone());
        for reverse in [false, true] {
            let b = if reverse {
                b.reverse().unwrap()
            } else {
                b.clone()
            };
            let Some(Relation::Points(points)) = intersect(&a, &b, 1e-7).unwrap() else {
                panic!("missing paired census")
            };
            assert_eq!(points.len(), 1);
            assert_eq!(points[0].first, 0.);
            assert_eq!(points[0].second, if reverse { 1. } else { 0. });
            assert_eq!(points[0].multiplicity, 3);
        }
        assert_eq!(a.control_points, before.0.control_points);
        assert_eq!(b.control_points, before.1.control_points);
    }
    #[test]
    fn endpoint_parameter_mapping_preserves_extreme_signed_domains() {
        let c = Curve {
            degree: 1,
            knots: vec![-1e8, -1e8, 1e-8, 1e-8],
            control_points: vec![vec![0., 0.], vec![1., 1.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        assert_eq!(parameter(&c, 0.), c.domain()[0]);
        assert_eq!(parameter(&c, 1.), c.domain()[1]);
        assert_eq!(map_back(&c, c.domain()[0], true).unwrap(), c.domain()[1]);
        assert_eq!(map_back(&c, c.domain()[1], true).unwrap(), c.domain()[0]);
    }
