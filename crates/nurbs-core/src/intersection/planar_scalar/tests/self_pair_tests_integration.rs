    use super::*;
    #[test]
    fn proper_nodal_source_keeps_both_preimages_while_a_cusp_has_only_one() {
        for cusp in [false, true] {
            let points = if cusp {
                vec![vec![3., -3.], vec![-1., 3.], vec![-1., -3.], vec![3., 3.]]
            } else {
                vec![
                    vec![9., -18.],
                    vec![-7., 26.],
                    vec![-7., -26.],
                    vec![9., 18.],
                ]
            };
            let mut c = Curve {
                degree: 3,
                knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
                control_points: points,
                weights: vec![1.; 4],
                periodic: false,
            };
            for transformed in [false, true] {
                if transformed {
                    for p in &mut c.control_points {
                        let (x, y) = (p[0], p[1]);
                        p[0] = 17. + x - y;
                        p[1] = -9. + x + y;
                    }
                }
                let roots = self_intersections(&c, 1e-7).unwrap().unwrap();
                if cusp {
                    assert!(roots.is_empty())
                } else {
                    assert_eq!(roots.len(), 1);
                    assert_eq!(roots[0].first, 0.25);
                    assert_eq!(roots[0].second, 0.75);
                }
            }
        }
    }
