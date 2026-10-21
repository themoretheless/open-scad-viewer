    use super::*;
    #[test]
    fn weighted_norm_qualifies_triangular_coupling_but_not_expansion() {
        let triangular = [[0.1, 3., 0.], [0., 0.1, 0.], [0., 0., 0.1]];
        let (scales, q) = rebalance_norm(triangular, [1.; 3], 3.1).unwrap();
        assert!(q < 1. && scales[0] > scales[1]);
        for i in 0..3 {
            let actual: f64 = (0..3)
                .map(|j| triangular[i][j] * scales[j] / scales[i])
                .sum();
            assert!(actual <= q);
        }
        let expanding = [[1.01, 3., 0.], [0., 0.1, 0.], [0., 0., 0.1]];
        let (_, q) = rebalance_norm(expanding, [1.; 3], 4.01).unwrap();
        assert!(q >= 1.01);
    }
    #[test]
    fn retained_original_operator_scales_and_jacobian_are_fresh_authority() {
        let a = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0.25, 0., 0.], vec![0.25, 1., 0.]],
                vec![vec![0.5, 0., 0.], vec![0.5, 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let b = Surface {
            control_points: vec![
                vec![vec![0., 0.5, -1.], vec![0., 0.5, 1.]],
                vec![vec![1., 0.5, -1.], vec![1., 0.5, 1.]],
            ],
            ..a.clone()
        };
        let source = FiberSources::new(&a, &b).unwrap();
        let f = source
            .certify(
                [[0., 1.], [0.25, 0.75], [0.125, 0.625], [0.25, 0.75]],
                FiberAxis::FirstU,
                FiberLimits::default(),
            )
            .unwrap()
            .into_fiber()
            .unwrap();
        f.reverify().unwrap();
        let mut bad = f.clone();
        bad.report.operator.as_mut().unwrap().preconditioner[0][1] += 1.;
        assert!(bad.reverify().is_err());
        let mut bad = f.clone();
        bad.report.operator.as_mut().unwrap().scales[0] *= 2.;
        assert!(bad.reverify().is_err());
        let mut bad = f.clone();
        bad.report.operator.as_mut().unwrap().first_jacobian[1][1][1] += 0.1;
        assert!(bad.reverify().is_err());
        let mut bad = f.clone();
        bad.report.contraction_upper = Some(0.75);
        assert!(bad.reverify().is_err());
        let mut bad = f;
        bad.report.operator = None;
        assert!(bad.reverify().is_err());
    }
