    use super::*;
    #[test]
    fn nodal_parent_survives_stationary_splitting_and_affine_transform() {
        for transformed in [false, true] {
            let c = Curve {
                degree: 3,
                knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
                control_points: vec![
                    vec![9., -18.],
                    vec![-7., 26.],
                    vec![-7., -26.],
                    vec![9., 18.],
                ],
                weights: vec![1.; 4],
                periodic: false,
            };
            let mut wire = vec![
                c,
                Curve::from_polyline(vec![vec![9., 18.], vec![9., -18.]]).unwrap(),
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
            let result = normalize(&[wire], 1e-7).unwrap();
            assert_eq!(result.len(), 2);
            let area = result
                .iter()
                .map(|w| signed_area(w, 1e-7).unwrap())
                .sum::<f64>();
            assert!((area - if transformed { 288. } else { 144. }).abs() < 1e-6);
            assert!(result.iter().flatten().any(|c| c.degree == 3));
        }
    }
