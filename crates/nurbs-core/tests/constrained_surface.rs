use nurbs_core::{
    constrained_surface::{self, Axis, Constraint, Options},
    surface::Surface,
};
fn seed() -> Surface {
    Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: (0..4)
            .map(|i| {
                (0..4)
                    .map(|j| vec![i as f64 / 3., j as f64 / 3., 0.])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 4]; 4],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn hard_points_derivatives_and_tangent_plane_are_simultaneously_satisfied() {
    let source = seed();
    let before = source.clone();
    let mut constraints = vec![];
    for uv in [[0., 0.], [0., 1.], [1., 0.], [1., 1.]] {
        constraints.push(Constraint::Point {
            uv,
            point: [uv[0], uv[1], 0.],
        });
    }
    constraints.extend([
        Constraint::Point {
            uv: [0.5, 0.5],
            point: [0.5, 0.5, 1.],
        },
        Constraint::Derivative {
            uv: [0.5, 0.5],
            axis: Axis::U,
            value: [1., 0., 0.],
        },
        Constraint::Derivative {
            uv: [0.5, 0.5],
            axis: Axis::V,
            value: [0., 1., 0.],
        },
        Constraint::TangentPlane {
            uv: [0.5, 0.5],
            normal: [0., 0., 1.],
        },
    ]);
    let result = constrained_surface::solve(&source, &constraints, Options::default()).unwrap();
    assert_eq!(source, before);
    assert!(result.max_position_residual < 1e-8);
    assert!(result.max_derivative_residual < 1e-8);
    assert!(result.max_normal_misalignment < 1e-8);
    assert!(result.constraint_rank < result.equations);
    assert!(result.sampled_regularity_only);
    let point = result.surface.evaluate(0.5, 0.5).unwrap().point;
    assert!((point[2] - 1.).abs() < 1e-8);
}
#[test]
fn incompatible_constraints_refuse_without_mutating_seed() {
    let source = seed();
    let before = source.clone();
    let constraints = [
        Constraint::Point {
            uv: [0.5, 0.5],
            point: [0., 0., 0.],
        },
        Constraint::Point {
            uv: [0.5, 0.5],
            point: [0., 0., 1.],
        },
    ];
    assert!(constrained_surface::solve(&source, &constraints, Options::default()).is_err());
    assert_eq!(source, before);
}
#[test]
fn rational_weights_and_basis_remain_authored() {
    let mut source = seed();
    source.weights[1][1] = 2.;
    source.weights[2][1] = 0.5;
    let target = [0.4, 0.7, 1.2];
    let result = constrained_surface::solve(
        &source,
        &[Constraint::Point {
            uv: [0.3, 0.8],
            point: target,
        }],
        Options::default(),
    )
    .unwrap();
    assert_eq!(result.surface.weights, source.weights);
    assert_eq!(result.surface.knots_u, source.knots_u);
    let p = result.surface.evaluate(0.3, 0.8).unwrap().point;
    assert!(p.iter().zip(target).all(|(p, t)| (p - t).abs() < 1e-8));
}
#[test]
fn invalid_constraints_and_objective_are_refused() {
    let s = seed();
    assert!(
        constrained_surface::solve(
            &s,
            &[Constraint::Point {
                uv: [2., 0.],
                point: [0.; 3]
            }],
            Options::default()
        )
        .is_err()
    );
    assert!(
        constrained_surface::solve(
            &s,
            &[Constraint::TangentPlane {
                uv: [0.5, 0.5],
                normal: [0.; 3]
            }],
            Options::default()
        )
        .is_err()
    );
    assert!(
        constrained_surface::solve(
            &s,
            &[],
            Options {
                anchor_weight: 0.,
                ..Options::default()
            }
        )
        .is_err()
    );
}

#[test]
fn whitened_equality_solve_matches_the_analytic_thin_plate_minimum() {
    let seed = nurbs_core::patches::bezier(
        (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| vec![i as f64 / 2., j as f64 / 2., 0.])
                    .collect()
            })
            .collect(),
        None,
    )
    .unwrap();
    let mut options = Options::default();
    options.fairness_weight = 0.25;
    let out = constrained_surface::solve(
        &seed,
        &[Constraint::Point {
            uv: [0.5, 0.5],
            point: [0.5, 0.5, 1.],
        }],
        options,
    )
    .unwrap();
    // D^T D has eigenvalue 6 along (1,-2,1), zero along (1,1,1).
    // The tensor fairness operator therefore has eigenvalues 0,6,6,12.
    let v = [1., -2., 1.];
    let b = [0.25, 0.5, 0.25];
    let inverse: Vec<Vec<f64>> = (0..3)
        .map(|i| {
            (0..3)
                .map(|j| {
                    1. / 9. - (v[i] + v[j]) / (36. * (1. + 6. * options.fairness_weight))
                        + v[i] * v[j] / (144. * (1. + 12. * options.fairness_weight))
                })
                .collect()
        })
        .collect();
    let denominator = (0..3)
        .map(|i| (0..3).map(|j| b[i] * b[j] * inverse[i][j]).sum::<f64>())
        .sum::<f64>();
    for i in 0..3 {
        for j in 0..3 {
            assert!(
                (out.surface.control_points[i][j][2] - inverse[i][j] / denominator).abs() < 1e-10
            );
            assert!(
                (out.surface.control_points[i][j][0] - seed.control_points[i][j][0]).abs() < 1e-10
            );
            assert!(
                (out.surface.control_points[i][j][1] - seed.control_points[i][j][1]).abs() < 1e-10
            );
        }
    }
}

#[test]
fn scalar_second_jet_equations_use_original_nonuniform_rational_weights() {
    let mut source = seed();
    source.weights[1][1] = 2.;
    source.weights[2][1] = 0.5;
    let before = source.clone();
    let equation = constrained_surface::JetEquation {
        first: [[0., 0., 0.25], [0., 0., -0.125]],
        second: [[0., 0., 1.], [0., 0., -0.5], [0., 0., 0.75]],
        value: 0.125,
    };
    let out = constrained_surface::solve(
        &source,
        &[Constraint::Jet {
            uv: [0.3, 0.7],
            equation,
        }],
        Options::default(),
    )
    .unwrap();
    let at = out.surface.evaluate(0.3, 0.7).unwrap();
    let (u, v) = at.first_derivatives().unwrap();
    let (uu, uv, vv) = at.second_derivatives().unwrap();
    assert!((0.25 * u[2] - 0.125 * v[2] + uu[2] - 0.5 * uv[2] + 0.75 * vv[2] - 0.125).abs() < 1e-8);
    assert_eq!(out.surface.weights, source.weights);
    assert_eq!(source, before);
}
