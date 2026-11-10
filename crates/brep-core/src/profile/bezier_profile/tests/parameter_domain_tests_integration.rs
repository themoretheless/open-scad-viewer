    use super::*;
    #[test]
    fn normalization_keeps_cusp_geometry_on_small_and_large_parameter_domains() {
        for end in [1e-15, 1., 1e8] {
            let c = Curve {
                degree: 3,
                knots: vec![0., 0., 0., 0., end, end, end, end],
                control_points: vec![vec![3., -1.], vec![-1., 1.], vec![-1., -1.], vec![3., 1.]],
                weights: vec![1.; 4],
                periodic: false,
            };
            let wire = vec![
                c,
                Curve::from_polyline(vec![vec![3., 1.], vec![3., -1.]]).unwrap(),
            ];
            let result = normalize(&[wire], 1e-7).unwrap();
            assert_eq!(result.len(), 1);
            assert!((signed_area(&result[0], 1e-7).unwrap() - 2.4).abs() < 1e-8);
            assert_eq!(result[0].iter().filter(|c| c.degree == 3).count(), 2);
        }
    }
