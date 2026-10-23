    use super::*;
    #[test]
    fn solver_iteration_policy_is_bounded() {
        let sketch = Sketch {
            points: vec![[1., 1.]],
            circles: vec![],
            constraints: vec![Constraint::Fix {
                point: 0,
                at: [0., 0.],
            }],
        };
        for limit in [0, 97] {
            assert!(
                solve_with_options(
                    &sketch,
                    SolverOptions {
                        tolerance: 1e-6,
                        max_iterations: limit
                    }
                )
                .is_err()
            );
        }
        let result = solve_with_options(
            &sketch,
            SolverOptions {
                tolerance: 1e-6,
                max_iterations: 1,
            },
        )
        .unwrap();
        assert_eq!(result.iterations, 1);
        assert!(result.max_residual > 1e-6);
    }
    #[test]
    fn diagnostics_distinguish_linear_conflicts_and_redundancy() {
        let sketch = Sketch {
            points: vec![[0.2, -0.1], [1., 0.], [0., 1.]],
            circles: vec![],
            constraints: vec![
                Constraint::Fix {
                    point: 0,
                    at: [0., 0.],
                },
                Constraint::Fix {
                    point: 0,
                    at: [10., 0.],
                },
            ],
        };
        let (solution, report) = solve_with_diagnostics(&sketch, 1e-6).unwrap();
        assert!(report.inconsistent);
        assert_eq!(report.redundant_equations, 2);
        assert_eq!(report.constraint_residuals.len(), 2);
        assert!(solution.max_residual > 4.9);
        assert!(report.degenerate_constraints.is_empty());
        let sketch = Sketch {
            constraints: vec![],
            ..sketch
        };
        let (solution, report) = solve_with_diagnostics(&sketch, 0.1).unwrap();
        assert_eq!(solution.degrees_of_freedom, 6);
        assert!(!report.inconsistent);
    }
    #[test]
    fn line_circle_tangency() {
        let s = Sketch {
            points: vec![[0., 0.], [10., 0.], [3., 3.]],
            circles: vec![Circle {
                center: 2,
                radius: 1.,
            }],
            constraints: vec![
                Constraint::Fix {
                    point: 0,
                    at: [0., 0.],
                },
                Constraint::Fix {
                    point: 1,
                    at: [10., 0.],
                },
                Constraint::Vertical { a: 0, b: 2 },
                Constraint::Radius {
                    circle: 0,
                    value: 2.,
                },
                Constraint::TangentLineCircle {
                    a: 0,
                    b: 1,
                    circle: 0,
                },
            ],
        };
        let r = solve(&s, 1e-6).unwrap();
        assert_eq!(r.status, "solved");
        assert!((r.sketch.points[2][1] - 2.).abs() < 1e-5);
    }
    #[test]
    fn circle_circle_tangency_and_conflicts() {
        let s = Sketch {
            points: vec![[0., 0.], [4., 0.]],
            circles: vec![
                Circle {
                    center: 0,
                    radius: 1.,
                },
                Circle {
                    center: 1,
                    radius: 1.,
                },
            ],
            constraints: vec![
                Constraint::Fix {
                    point: 0,
                    at: [0., 0.],
                },
                Constraint::Horizontal { a: 0, b: 1 },
                Constraint::Radius {
                    circle: 0,
                    value: 2.,
                },
                Constraint::Radius {
                    circle: 1,
                    value: 1.,
                },
                Constraint::TangentCircles {
                    a: 0,
                    b: 1,
                    internal: false,
                },
            ],
        };
        let r = solve(&s, 1e-6).unwrap();
        assert_eq!(r.status, "solved");
        assert!((r.sketch.points[1][0] - 3.).abs() < 1e-5);
        let mut bad = s.clone();
        bad.constraints.push(Constraint::Radius {
            circle: 0,
            value: 7.,
        });
        assert_eq!(solve(&bad, 1e-6).unwrap().status, "not_converged");
    }
    #[test]
    fn reports_underconstraint() {
        let s = Sketch {
            points: vec![[0., 0.], [2., 0.]],
            circles: vec![],
            constraints: vec![Constraint::Distance {
                a: 0,
                b: 1,
                value: 1.,
            }],
        };
        assert_eq!(solve(&s, 1e-6).unwrap().status, "underconstrained");
    }
    #[test]
    fn angle_midpoint_symmetric_collinear_and_concentric_constraints() {
        let s = Sketch {
            points: vec![
                [0., 0.],
                [10., 0.],
                [4., 6.],
                [1., 1.],
                [-2., 4.],
                [0., 10.],
            ],
            circles: vec![
                Circle {
                    center: 0,
                    radius: 5.,
                },
                Circle {
                    center: 3,
                    radius: 2.,
                },
            ],
            constraints: vec![
                Constraint::Fix {
                    point: 0,
                    at: [0., 0.],
                },
                Constraint::Fix {
                    point: 1,
                    at: [10., 0.],
                },
                Constraint::Fix {
                    point: 5,
                    at: [0., 10.],
                },
                Constraint::Angle {
                    a: 0,
                    b: 1,
                    c: 0,
                    d: 2,
                    radians: std::f64::consts::FRAC_PI_4,
                },
                Constraint::Distance {
                    a: 0,
                    b: 2,
                    value: 10.0 * std::f64::consts::SQRT_2,
                },
                Constraint::Midpoint {
                    point: 3,
                    a: 0,
                    b: 1,
                },
                Constraint::Symmetric {
                    a: 2,
                    b: 4,
                    axis_a: 0,
                    axis_b: 5,
                },
                Constraint::Radius {
                    circle: 0,
                    value: 6.,
                },
                Constraint::EqualRadius { a: 0, b: 1 },
            ],
        };
        let r = solve(&s, 1e-6).unwrap();
        assert_eq!(r.status, "solved");
        assert!((r.sketch.points[2][0] - 10.0).abs() < 1e-5);
        assert!((r.sketch.points[2][1] - 10.0).abs() < 1e-5);
        assert!((r.sketch.points[3][0] - 5.0).abs() < 1e-6);
        assert!(r.sketch.points[3][1].abs() < 1e-6);
        assert!((r.sketch.points[4][0] + 10.0).abs() < 1e-5);
        assert!((r.sketch.points[4][1] - 10.0).abs() < 1e-5);
        assert!((r.sketch.circles[1].radius - 6.0).abs() < 1e-6);
    }
    #[test]
    fn solves_large_chain_exceeding_legacy_24_point_limit() {
        let count = 40;
        let mut points = Vec::with_capacity(count);
        let mut constraints = Vec::new();
        for i in 0..count {
            points.push([i as f64 * 0.9, (i % 3) as f64 * 0.2]);
        }
        constraints.push(Constraint::Fix {
            point: 0,
            at: [0., 0.],
        });
        for i in 0..count - 1 {
            constraints.push(Constraint::Horizontal { a: i, b: i + 1 });
            constraints.push(Constraint::Distance {
                a: i,
                b: i + 1,
                value: 1.0,
            });
        }
        let s = Sketch {
            points,
            circles: vec![],
            constraints,
        };
        let r = solve(&s, 1e-6).unwrap();
        assert_eq!(r.status, "solved");
        assert!((r.sketch.points[count - 1][0] - (count - 1) as f64).abs() < 1e-5);
    }
    #[test]
    fn analyze_dof_identifies_underconstrained_points_and_conflicts() {
        let s = Sketch {
            points: vec![[0., 0.], [5., 0.], [2., 3.]],
            circles: vec![],
            constraints: vec![
                Constraint::Fix {
                    point: 0,
                    at: [0., 0.],
                },
                Constraint::Fix {
                    point: 1,
                    at: [5., 0.],
                },
                Constraint::Horizontal { a: 0, b: 2 },
            ],
        };
        let dof = analyze_dof(&s, 1e-6).unwrap();
        assert_eq!(dof.degrees_of_freedom, 1);
        assert_eq!(dof.underconstrained_points, vec![2]);
        assert!(dof.conflicting_constraints.is_empty());

        let conflict = Sketch {
            points: vec![[0., 0.], [5., 0.]],
            circles: vec![],
            constraints: vec![
                Constraint::Fix {
                    point: 0,
                    at: [0., 0.],
                },
                Constraint::Fix {
                    point: 1,
                    at: [5., 0.],
                },
                Constraint::Distance {
                    a: 0,
                    b: 1,
                    value: 10.,
                },
            ],
        };
        let conflict_dof = analyze_dof(&conflict, 1e-6).unwrap();
        assert!(!conflict_dof.conflicting_constraints.is_empty());
    }

    #[test]
    fn point_on_line_fix_angle_equal_angle_and_point_on_arc() {
        let s = Sketch {
            points: vec![
                [0., 0.],
                [4., 3.],
                [2., 1.],
                [0., 5.],
                [4., 6.],
                [2., 3.],
            ],
            circles: vec![Circle {
                center: 0,
                radius: 5.,
            }],
            constraints: vec![
                Constraint::Fix {
                    point: 0,
                    at: [0., 0.],
                },
                Constraint::FixAngle {
                    a: 0,
                    b: 1,
                    radians: std::f64::consts::FRAC_PI_4,
                },
                Constraint::Distance {
                    a: 0,
                    b: 1,
                    value: 10.0 * std::f64::consts::SQRT_2,
                },
                Constraint::PointOnLine {
                    point: 2,
                    a: 0,
                    b: 1,
                },
                Constraint::Vertical { a: 0, b: 3 },
                Constraint::Fix {
                    point: 3,
                    at: [0., 5.],
                },
                Constraint::EqualAngle {
                    a: 0,
                    b: 1,
                    c: 0,
                    d: 3,
                    e: 3,
                    f: 4,
                    g: 0,
                    h: 3,
                },
                Constraint::Distance {
                    a: 3,
                    b: 4,
                    value: 5.0 * std::f64::consts::SQRT_2,
                },
                Constraint::Radius {
                    circle: 0,
                    value: 5.,
                },
                Constraint::PointOnArc {
                    point: 5,
                    circle: 0,
                    start_angle: 0.0,
                    end_angle: std::f64::consts::FRAC_PI_2,
                },
            ],
        };
        let r = solve(&s, 1e-6).unwrap();
        assert!(r.max_residual <= 1e-6);
        assert!((r.sketch.points[1][0] - 10.0).abs() < 1e-5);
        assert!((r.sketch.points[1][1] - 10.0).abs() < 1e-5);
        assert!((r.sketch.points[2][0] - r.sketch.points[2][1]).abs() < 1e-5);
        let p5 = r.sketch.points[5];
        assert!((p5[0].hypot(p5[1]) - 5.0).abs() < 1e-5);
        assert!(p5[0] >= -1e-6 && p5[1] >= -1e-6);
    }

    #[test]
    fn sketch_bounding_box_and_overconstrained_constraints() {
        let s = Sketch {
            points: vec![[0., 0.], [10., 5.]],
            circles: vec![Circle {
                center: 0,
                radius: 3.,
            }],
            constraints: vec![
                Constraint::Fix {
                    point: 0,
                    at: [0., 0.],
                },
                Constraint::Fix {
                    point: 1,
                    at: [10., 5.],
                },
                Constraint::Horizontal { a: 0, b: 1 },
            ],
        };
        let (min, max) = s.bounding_box().unwrap();
        assert_eq!(min, [-3., -3.]);
        assert_eq!(max, [10., 5.]);
        let over = s.overconstrained_constraints(1e-6);
        assert!(over.contains(&2));
    }
