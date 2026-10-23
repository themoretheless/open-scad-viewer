    use super::*;

    fn bezier(points: &[[f64; 3]]) -> Curve {
        let n = points.len();
        Curve {
            degree: n - 1,
            knots: [vec![0.; n], vec![1.; n]].concat(),
            control_points: points.iter().map(|p| p.to_vec()).collect(),
            weights: vec![1.; n],
            periodic: false,
        }
    }

    #[test]
    fn nan_control_point_is_rejected_at_the_boundary() {
        let a = bezier(&[[0., 0., 0.], [1., 1., 0.]]);
        let mut b = bezier(&[[0., 1., 0.], [1., 0., 0.]]);
        b.control_points[1][0] = f64::NAN;
        let err = intersect_curve_curve_report(&a, &b, None).err().expect("invalid intersection input must fail");
        assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
    }

    #[test]
    fn crossing_pair_resolves_with_guards_active() {
        // The unified guards must not perturb the certified result.
        let a = bezier(&[[0., 0., 0.], [2., 2., 0.]]);
        let b = bezier(&[[0., 2., 0.], [2., 0., 0.]]);
        let r = intersect_curve_curve_report(&a, &b, None).unwrap();
        assert_eq!(r.report.components.len(), 1);
        assert!(r.report.unresolved.is_empty());
        let c = &r.report.components[0];
        assert!((c.first - 0.5).abs() < 1e-9 && (c.second - 0.5).abs() < 1e-9);
        assert!(c.point.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn krawczyk_newton_guard_matches_iteration_cap() {
        // Guard sized exactly at the Newton cap: 12 ticks pass, the 13th fails.
        let mut guard = Budget::with_iterations(12).unwrap().guard("cc_krawczyk_newton");
        for _ in 0..12 {
            guard.tick().unwrap();
        }
        let err = guard.tick().unwrap_err();
        assert_eq!(err.code, crate::RESOURCE_LIMIT, "{err:?}");
        assert!(err.contains("cc_krawczyk_newton"));
    }
