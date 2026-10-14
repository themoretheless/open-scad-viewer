    #[test]
    fn fixed_shear_projection_proves_the_whole_rectangle_without_symmetric_jacobian() {
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![10., 1., 0.]],
                vec![vec![1., 0., 0.], vec![11., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let before = s.clone();
        let report = inspect(&s, [[1, 0, 0], [0, 1, 0]], 1).unwrap();
        assert!(report.certified, "{report:?}");
        assert_eq!(report.cells, 1);
        assert!(!inspect(&s, [[1, 0, 0], [0, 1, 0]], 0).unwrap().certified);
        assert_eq!(s, before);
    }
    #[test]
    fn station_split_refuses_variable_station_weights_and_rounded_differences() {
        let mut s = quarter();
        let before = s.clone();
        assert!(exact_station_split(&s).is_some());
        assert_eq!(s, before);
        s.weights[1][1] = s.weights[1][1] * 2.;
        assert!(exact_station_split(&s).is_none());
        s = before;
        s.control_points[0][0][0] = 1.;
        s.control_points[1][0][0] = 2_f64.powi(-54);
        assert!(exact_station_split(&s).is_none());
    }
    #[test]
    fn small_translated_spatial_wall_uses_original_span_jets_under_finite_work() {
        let mut s = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![
                    vec![-0.7062272226232672, 0.5675679493147961, 0.5968080444325607],
                    vec![-0.7474669927920602, 0.49724679499522595, 0.5426947020020663],
                ],
                vec![
                    vec![-0.699565435445212, 0.5608768152774212, 0.5998523983908421],
                    vec![-0.7405573704272115, 0.4909357350407962, 0.546020243648579],
                ],
                vec![
                    vec![-0.6943107816690395, 0.5634326861227221, 0.5920719792065087],
                    vec![-0.7351112988881411, 0.4937291108059471, 0.5384203254299099],
                ],
            ],
            weights: vec![vec![1.; 2], vec![0.7071067811865476; 2], vec![1.; 2]],
            periodic_u: false,
            periodic_v: false,
        };
        let before = s.clone();
        let proof = inspect_candidate(&s, 100).unwrap();
        assert!(proof.certified, "{proof:?}");
        assert!(proof.cells > 0 && proof.cells <= 100);
        assert!(!inspect(&s, proof.projection, 0).unwrap().certified);
        assert_eq!(s, before);
        // Same authored image on shifted non-unit knot domains: stable jets
        // differentiate in the original span, never in the query-cell width.
        s.knots_u = vec![23., 23., 23., 25., 25., 25.];
        s.knots_v = vec![-7., -7., -5., -5.];
        assert!(inspect_candidate(&s, 100).unwrap().certified);
        eprintln!(
            "thin spatial wall injectivity cells={} projection={:?}",
            proof.cells, proof.projection
        );
    }
    #[test]
    fn pole_projection_requires_exact_arithmetic_and_keeps_rational_basis() {
        let s = quarter();
        let p = exact_projected_surface(&s, [[1, 1, 0], [0, 0, 1]]).unwrap();
        assert_eq!(s.weights, p.weights);
        assert_eq!(s.knots_u, p.knots_u);
        assert_eq!(s.knots_v, p.knots_v);
        assert_eq!(s.degree_u, p.degree_u);
        assert_eq!(s.degree_v, p.degree_v);
        for (a, b) in s
            .control_points
            .iter()
            .flatten()
            .zip(p.control_points.iter().flatten())
        {
            assert_eq!(b[0], a[0] + a[1]);
            assert_eq!(b[1], a[2]);
            assert_eq!(b[2], 0.);
        }
        let mut rounded = s.clone();
        rounded.control_points[0][0][0] = 1.;
        rounded.control_points[0][0][1] = 2_f64.powi(-54);
        let before = rounded.clone();
        assert!(exact_projected_surface(&rounded, [[1, 1, 0], [0, 0, 1]]).is_none());
        assert_eq!(rounded, before);
    }
    #[test]
    fn sheared_wall_has_a_fixed_p_matrix_and_periodic_domain_still_refuses() {
        let s = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![
                    vec![0.6530574994261776, -0.4177342973963124, 2.8505263778629866],
                    vec![0.6902224237062069, -0.39135860255547433, 3.564086526338608],
                ],
                vec![
                    vec![1.2456075346925495, 0.11617155954812869, 3.9102440379557444],
                    vec![1.3054479503736172, 0.15174399496075583, 4.637554869660337],
                ],
                vec![
                    vec![0.5905981815837974, 0.5726061673805435, 3.6546149667003553],
                    vec![0.6121768723162396, 0.5936397910373246, 4.317133369542564],
                ],
            ],
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        let proof = inspect_candidate(&s, 10000).unwrap();
        assert!(proof.certified, "{proof:?}");
        assert!(proof.cells <= 10000);
        let mut periodic = s.clone();
        periodic.periodic_u = true;
        assert!(inspect_candidate(&periodic, 10000).is_err());
        let transported = inspect(&s, [[0, 17, -1], [-15, -14, 16]], 10000);
        assert!(transported.unwrap().certified);
    }

    use super::*;
    #[test]
    fn fixed_rational_profile_bijection_preserves_the_source_and_budget() {
        let s = Surface {
            degree_u: 1,
            degree_v: 5,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 0., 0., 0., 0., 1., 1., 1., 1., 1., 1.],
            control_points: (0..2)
                .map(|u| (0..6).map(|v| vec![u as f64, v as f64 / 5., 0.]).collect())
                .collect(),
            weights: vec![vec![1.; 6], vec![2.; 6]],
            periodic_u: false,
            periodic_v: false,
        };
        let before = s.clone();
        let proof = inspect_candidate(&s, 1000).unwrap();
        assert!(proof.certified);
        let source = proof.profile_parameterization.unwrap();
        assert_eq!(source.weights, [1., 2.]);
        assert!(source.derivative_lower > 0. && source.derivative_lower <= 0.5);
        assert_eq!(s, before);
        let denied = inspect_candidate(&s, 0).unwrap();
        assert!(!denied.certified && denied.cells == 0);
        let mut changed = s.clone();
        changed.weights[1][2] = 3.;
        assert!(inspect_candidate(&changed, 1000)
            .unwrap()
            .profile_parameterization
            .is_none());
    }
    #[test]
    fn fixed_weighted_metric_is_proposed_then_proved_on_the_whole_chart() {
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![-16., 16., 0.]],
                vec![vec![0.125, 0.125, 0.], vec![-15.875, 16.125, 0.]],
            ],
            weights: vec![vec![1., 1.]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let map = [[1, 0, 0], [0, 1, 0]];
        assert!(!inspect_metric(&s, map, [1., 1.], 64, 1).unwrap().certified);
        let r = inspect(&s, map, 64).unwrap();
        assert!(r.certified);
        assert_eq!(r.row_weights, [1. / 128., 1.]);
        assert!(r.cells <= 64);
        for budget in [0, 1, 2] {
            let refused = inspect(&s, map, budget).unwrap();
            assert!(!refused.certified);
            assert!(refused.cells <= budget);
        }
        let mut singular = s.clone();
        singular.control_points[1][0] = vec![0.125, -0.125, 0.];
        singular.control_points[1][1] = vec![-15.875, 15.875, 0.];
        assert!(!inspect(&singular, map, 64).unwrap().certified);
        assert!(inspect_metric(&s, map, [-1., 1.], 64, 1).is_err());
    }
    #[test]
    fn transverse_proposal_does_not_mix_long_station_motion_into_profile_row() {
        let mut s = quarter();
        for row in &mut s.control_points {
            for (v, p) in row.iter_mut().enumerate() {
                p[2] = 100. * v as f64 + p[0];
            }
        }
        assert!(
            inspect(&s, [[-1, 1, -1], [0, 0, 1]], 1000)
                .unwrap()
                .certified
        );
        let r = inspect_candidate(&s, 1000).unwrap();
        assert!(r.certified, "{r:?}");
        assert!(
            inspect(&s, [[-1, 1, 0], [0, 0, 1]], 1000)
                .unwrap()
                .certified
        );
        assert!(inspect(&s, r.projection, 1000).unwrap().certified);
        assert!(!inspect_candidate(&s, 0).unwrap().certified);
    }
    #[test]
    fn spatial_station_uses_a_fixed_transverse_integer_projection() {
        let mut s = quarter();
        for row in &mut s.control_points {
            for (v, p) in row.iter_mut().enumerate() {
                let y = p[1];
                p[1] = y + 10. * v as f64;
                p[2] = -2. * y + 5. * v as f64;
            }
        }
        let r = inspect_candidate(&s, 10000).unwrap();
        assert!(r.certified, "{r:?}");
        assert!(r.cells <= 10000);
        assert!(!inspect_candidate(&s, 0).unwrap().certified);
    }
    fn quarter() -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![1., 0., 0.], vec![1., 0., 5.]],
                vec![vec![1., 1., 0.], vec![1., 1., 5.]],
                vec![vec![0., 1., 0.], vec![0., 1., 5.]],
            ],
            weights: vec![vec![1.; 2], vec![0.5_f64.sqrt(); 2], vec![1.; 2]],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn rational_quarter_wall_has_one_global_oblique_projection() {
        let s = quarter();
        let p = [[-1, 1, 0], [0, 0, 1]];
        let r = inspect(&s, p, 1000).unwrap();
        assert!(inspect_candidate(&s, 1000).unwrap().certified);
        assert!(r.certified, "{r:?}");
        assert!(r.cells <= 1000);
        assert!(!inspect(&s, p, 0).unwrap().certified);
        assert!(!inspect(&s, [[1, 0, 0], [0, 0, 1]], 100).unwrap().certified);
        assert!(inspect(&s, [[1, 0, 0], [1, 0, 0]], 100).is_err());
    }
    #[test]
    fn folded_chart_cannot_change_projection_between_spans() {
        let mut s = quarter();
        s.degree_u = 1;
        s.knots_u = vec![0., 0., 0.5, 1., 1.];
        s.control_points[2] = s.control_points[0].clone();
        s.weights = vec![vec![1.; 2]; 3];
        assert!(!inspect_candidate(&s, 1000).unwrap().certified);
        assert!(
            !inspect(&s, [[-1, 1, 0], [0, 0, 1]], 1000)
                .unwrap()
                .certified
        );
    }
