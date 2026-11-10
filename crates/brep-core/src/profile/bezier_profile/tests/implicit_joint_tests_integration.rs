    use super::*;
    fn cubic(p: Vec<Vec<f64>>) -> Curve {
        Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: p,
            weights: vec![1.; 4],
            periodic: false,
        }
    }
    #[test]
    fn same_side_curved_cusp_lens_with_nonlinear_speed_keeps_exact_carriers() {
        let a = cubic(vec![vec![0., 0.], vec![2., 0.], vec![4., 2.], vec![6., 6.]]);
        let b = cubic(vec![
            vec![0., 0.],
            vec![2., 0.],
            vec![6., 2.],
            vec![12., 12.],
        ]);
        for transformed in [false, true] {
            let mut wire = vec![
                b.clone(),
                Curve::from_polyline(vec![vec![12., 12.], vec![6., 6.]]).unwrap(),
                a.reverse().unwrap(),
            ];
            if transformed {
                for c in &mut wire {
                    for p in &mut c.control_points {
                        let (x, y) = (p[0], p[1]);
                        p[0] = 17. + x - y;
                        p[1] = -9. + x + y;
                    }
                }
            }
            let before = wire
                .iter()
                .map(|c| c.control_points.clone())
                .collect::<Vec<_>>();
            let expected = if transformed { 25.2 } else { 12.6 };
            assert!((validate_single_wire(&wire, 1e-7).unwrap() - expected).abs() < 1e-8);
            let normalized = normalize(&[wire.clone()], 1e-7).unwrap();
            assert_eq!(normalized.len(), 1);
            assert!((signed_area(&normalized[0], 1e-7).unwrap() - expected).abs() < 1e-8);
            assert!(normalized[0].iter().filter(|c| c.degree == 3).count() >= 2);
            assert_eq!(
                wire.iter()
                    .map(|c| c.control_points.clone())
                    .collect::<Vec<_>>(),
                before
            );
        }
    }
    #[test]
    fn curved_third_order_difference_is_a_valid_cusp_component() {
        let region = |y: [f64; 4]| {
            let c = cubic((0..4).map(|i| vec![10. * i as f64, y[i]]).collect());
            vec![
                Curve::from_polyline(vec![vec![0., -2.], vec![30., -2.]]).unwrap(),
                Curve::from_polyline(vec![vec![30., -2.], vec![30., y[3]]]).unwrap(),
                c.reverse().unwrap(),
                Curve::from_polyline(vec![vec![0., y[0]], vec![0., -2.]]).unwrap(),
            ]
        };
        let a = region([0., 0., 1., 3.]);
        let b = region([0., 0., 1., 6.]);
        let r = boolean(&[b], &[a], "difference", 1e-7).unwrap();
        assert_eq!(r.len(), 1);
        assert!((signed_area(&r[0], 1e-7).unwrap() - 22.5).abs() < 1e-7);
    }
