use super::*;

fn distance(a: usize, b: usize, value: f64) -> Constraint {
    Constraint::Distance { a, b, value }
}

#[test]
fn dr_plan_finds_rigid_clusters_redundancy_and_local_underconstraint() {
    let mut sketch = Sketch {
        points: vec![[0.0, 0.0], [4.0, 0.0], [0.0, 3.0], [20.0, 20.0]],
        circles: vec![],
        constraints: vec![
            distance(0, 1, 4.0),
            distance(1, 2, 5.0),
            distance(2, 0, 3.0),
        ],
    };
    let plan = plan_dr(&sketch, 1e-6).unwrap();
    let rigid = plan.clusters.iter().find(|cluster| cluster.rigid).unwrap();
    assert_eq!(rigid.points, vec![0, 1, 2]);
    assert_eq!(rigid.degrees_of_freedom, 3);
    assert_eq!(rigid.redundant_equations, 0);
    assert!(rigid.underconstrained_points.is_empty());
    let free = plan
        .clusters
        .iter()
        .find(|cluster| cluster.points == vec![3])
        .unwrap();
    assert_eq!(free.degrees_of_freedom, 2);
    assert_eq!(free.underconstrained_points, vec![3]);

    sketch.constraints.push(distance(0, 1, 4.0));
    let redundant = plan_dr(&sketch, 1e-6).unwrap();
    let cluster = redundant
        .clusters
        .iter()
        .find(|cluster| cluster.rigid)
        .unwrap();
    assert_eq!(cluster.redundant_equations, 1);
    assert_eq!(cluster.overconstrained_constraints, vec![3]);
    assert!(!redundant.steps[3].independent);
}

#[test]
fn solver_pipeline_runs_dr_analytic_closure_then_existing_global_solver() {
    let sketch = Sketch {
        points: vec![[0.0, 0.0], [8.0, 0.0], [1.0, 7.0]],
        circles: vec![],
        constraints: vec![
            Constraint::Fix {
                point: 0,
                at: [0.0, 0.0],
            },
            Constraint::Fix {
                point: 1,
                at: [8.0, 0.0],
            },
            distance(0, 2, 5.0),
            distance(1, 2, 5.0),
        ],
    };
    let (solution, pipeline) = solve_with_pipeline(
        &sketch,
        SolverOptions {
            tolerance: 1e-8,
            max_iterations: 96,
        },
    )
    .unwrap();
    let names: Vec<_> = pipeline.stages.iter().map(|stage| stage.name).collect();
    assert_eq!(
        names,
        vec![
            "dr_reduction",
            "local_analytic_closure",
            "global_damped_least_squares"
        ]
    );
    assert!(pipeline.stages[1].changes >= 1);
    assert!(solution.max_residual <= 1e-8);
    assert!((solution.sketch.points[2][0] - 4.0).abs() < 1e-7);
    assert!((solution.sketch.points[2][1].abs() - 3.0).abs() < 1e-7);
}

#[test]
fn inference_suggests_tangency_without_mutating_or_applying_constraints() {
    let sketch = Sketch {
        points: vec![[0.0, 0.0], [10.0, 0.0], [5.0, 2.0], [12.0, 2.0]],
        circles: vec![
            Circle {
                center: 2,
                radius: 2.0,
            },
            Circle {
                center: 3,
                radius: 5.0,
            },
        ],
        constraints: vec![distance(0, 1, 10.0)],
    };
    let original = sketch.constraints.clone();
    let suggestions = sketch.suggest_constraints(1e-6, 1e-6);
    assert!(suggestions.contains(&Constraint::TangentLineCircle {
        a: 0,
        b: 1,
        circle: 0
    }));
    assert!(suggestions.contains(&Constraint::TangentCircles {
        a: 0,
        b: 1,
        internal: false
    }));
    assert_eq!(sketch.constraints, original);
    assert!(suggestions.len() <= 4096);
}

#[test]
fn symmetry_and_circular_pattern_use_existing_constraint_system() {
    let symmetric = Sketch {
        points: vec![[0.0, 0.0], [0.0, 10.0], [2.0, 3.0], [4.0, 5.0]],
        circles: vec![],
        constraints: vec![
            Constraint::Fix {
                point: 0,
                at: [0.0, 0.0],
            },
            Constraint::Fix {
                point: 1,
                at: [0.0, 10.0],
            },
            Constraint::Fix {
                point: 2,
                at: [2.0, 3.0],
            },
            Constraint::Symmetric {
                a: 2,
                b: 3,
                axis_a: 0,
                axis_b: 1,
            },
        ],
    };
    let solution = solve(&symmetric, 1e-8).unwrap();
    assert!(solution.max_residual <= 1e-8);
    assert!((solution.sketch.points[3][0] + 2.0).abs() < 1e-7);
    assert!((solution.sketch.points[3][1] - 3.0).abs() < 1e-7);

    let mut pattern = Sketch {
        points: vec![[0.0, 0.0], [10.0, 0.0]],
        circles: vec![],
        constraints: vec![],
    };
    let (points, constraints) = pattern.add_circular_pattern(0, 1, 4).unwrap();
    assert_eq!(points.len(), 4);
    assert_eq!(constraints.len(), 6);
    for &point in &points {
        assert!((length(subtract(pattern.points[point], pattern.points[0])) - 10.0).abs() < 1e-8);
    }
    assert!(
        pattern
            .constraints
            .iter()
            .all(|constraint| matches!(constraint, Constraint::EqualLength { .. }))
    );
    assert!(pattern.add_circular_pattern(0, 1, 129).is_err());
}

#[test]
fn profile_validation_accepts_outer_with_hole_and_rejects_invalid_profiles() {
    let valid = Sketch {
        points: vec![
            [0.0, 0.0],
            [10.0, 0.0],
            [10.0, 10.0],
            [0.0, 10.0],
            [2.0, 2.0],
            [2.0, 4.0],
            [4.0, 4.0],
            [4.0, 2.0],
        ],
        circles: vec![],
        constraints: vec![],
    };
    let profile = validate_profile(&valid, &[vec![0, 1, 2, 3, 0], vec![4, 5, 6, 7]], 1e-6).unwrap();
    assert_eq!(profile.loops[0], vec![0, 1, 2, 3]);
    assert!(profile.signed_outer_area > 0.0);

    let bow_tie = Sketch {
        points: vec![[0.0, 0.0], [2.0, 2.0], [0.0, 2.0], [2.0, 0.0]],
        circles: vec![],
        constraints: vec![],
    };
    let error = validate_profile(&bow_tie, &[vec![0, 1, 2, 3]], 1e-6).unwrap_err();
    assert_eq!(error.code, "SKETCH_INVALID_PROFILE");
    let outside = validate_profile(&valid, &[vec![0, 1, 2, 3], vec![4, 5, 6, 7]], 1e-6);
    assert!(outside.is_ok());
}

#[test]
fn persistent_ids_survive_rebuild_reordering_and_solving() {
    let sketch = Sketch {
        points: vec![[0.0, 0.0], [1.0, 1.0]],
        circles: vec![],
        constraints: vec![Constraint::Fix {
            point: 0,
            at: [0.0, 0.0],
        }],
    };
    let ids = PersistentIds::new(&sketch).unwrap();
    assert_eq!(ids.points, vec![1, 2]);
    assert_eq!(ids.constraints, vec![3]);
    let solved = solve(&sketch, 1e-6).unwrap();
    let solved_ids = ids
        .rebuild(
            &sketch,
            &solved.sketch,
            &RebuildMap::identity(&solved.sketch),
        )
        .unwrap();
    assert_eq!(solved_ids, ids);

    let rebuilt = Sketch {
        points: vec![[2.0, 2.0], [0.0, 0.0]],
        circles: vec![],
        constraints: vec![Constraint::Fix {
            point: 1,
            at: [0.0, 0.0],
        }],
    };
    let map = RebuildMap {
        points: vec![1, 0],
        circles: vec![],
        constraints: vec![0],
    };
    let rebuilt_ids = ids.rebuild(&sketch, &rebuilt, &map).unwrap();
    assert_eq!(rebuilt_ids.points, vec![2, 1]);
    assert_eq!(rebuilt_ids.constraints, ids.constraints);
    let bad = RebuildMap {
        points: vec![0, 9],
        ..map
    };
    assert_eq!(
        ids.rebuild(&sketch, &rebuilt, &bad).unwrap_err().code,
        "SKETCH_INVALID_IDENTITY"
    );
}

#[test]
fn characteristic_scaling_preserves_dof_and_convergence_across_units() {
    let base = Sketch {
        points: vec![[0.0, 0.0], [4.0, 0.0], [0.0, 3.0]],
        circles: vec![],
        constraints: vec![
            Constraint::Fix {
                point: 0,
                at: [0.0, 0.0],
            },
            distance(0, 1, 4.0),
            distance(1, 2, 5.0),
            distance(2, 0, 3.0),
        ],
    };
    let small = Sketch {
        points: base
            .points
            .iter()
            .map(|p| [p[0] * 1e-3, p[1] * 1e-3])
            .collect(),
        constraints: base
            .constraints
            .iter()
            .map(|constraint| match *constraint {
                Constraint::Distance { a, b, value } => distance(a, b, value * 1e-3),
                Constraint::Fix { point, at } => Constraint::Fix {
                    point,
                    at: [at[0] * 1e-3, at[1] * 1e-3],
                },
                _ => unreachable!(),
            })
            .collect(),
        circles: vec![],
    };
    let large = Sketch {
        points: base
            .points
            .iter()
            .map(|p| [p[0] * 1e3, p[1] * 1e3])
            .collect(),
        constraints: base
            .constraints
            .iter()
            .map(|constraint| match *constraint {
                Constraint::Distance { a, b, value } => distance(a, b, value * 1e3),
                Constraint::Fix { point, at } => Constraint::Fix {
                    point,
                    at: [at[0] * 1e3, at[1] * 1e3],
                },
                _ => unreachable!(),
            })
            .collect(),
        circles: vec![],
    };
    let base_plan = plan_dr(&base, 1e-5).unwrap();
    assert!(
        (plan_dr(&small, 1e-8).unwrap().characteristic_scale / base_plan.characteristic_scale
            - 1e-3)
            .abs()
            < 1e-9
    );
    assert!(
        (plan_dr(&large, 1e-2).unwrap().characteristic_scale / base_plan.characteristic_scale
            - 1e3)
            .abs()
            < 1e-6
    );
    let normal = solve(&base, 1e-5).unwrap();
    let tiny = solve(&small, 1e-8).unwrap();
    let huge = solve(&large, 1e-2).unwrap();
    assert_eq!(normal.degrees_of_freedom, tiny.degrees_of_freedom);
    assert_eq!(normal.degrees_of_freedom, huge.degrees_of_freedom);
    assert!(tiny.max_residual <= 1e-8 && huge.max_residual <= 1e-2);
    assert_eq!(normal.status, tiny.status);
    assert_eq!(normal.status, huge.status);
}

#[test]
fn invalid_inputs_and_iteration_budgets_remain_typed_and_bounded() {
    let nan = Sketch {
        points: vec![[f64::NAN, 0.0]],
        circles: vec![],
        constraints: vec![],
    };
    assert_eq!(solve(&nan, 1e-6).unwrap_err().code, "SKETCH_INVALID_INPUT");
    let sketch = Sketch {
        points: vec![[0.0, 0.0]],
        circles: vec![],
        constraints: vec![],
    };
    let error = solve_with_options(
        &sketch,
        SolverOptions {
            tolerance: 1e-6,
            max_iterations: 97,
        },
    )
    .unwrap_err();
    assert_eq!(error.code, "SKETCH_INVALID_INPUT");
}
