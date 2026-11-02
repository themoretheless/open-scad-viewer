use super::*;

// Independent five-point residual differences check the analytic derivatives,
// including shared point/radius indices. This is finite regression evidence.
#[test]
fn analytic_jacobian_matches_residual_derivatives_and_restores_scratch() {
    let constraints = vec![
        Constraint::Fix {
            point: 0,
            at: [0., 0.],
        },
        Constraint::Horizontal { a: 0, b: 1 },
        Constraint::Vertical { a: 0, b: 1 },
        Constraint::Coincident { a: 0, b: 1 },
        Constraint::FixAngle {
            a: 0,
            b: 1,
            radians: 0.37,
        },
        Constraint::PointOnLine {
            point: 3,
            a: 0,
            b: 1,
        },
        Constraint::PointOnLine {
            point: 0,
            a: 0,
            b: 1,
        },
        Constraint::Radius {
            circle: 0,
            value: 1.,
        },
        Constraint::Concentric { a: 0, b: 1 },
        Constraint::EqualRadius { a: 0, b: 1 },
        Constraint::Distance {
            a: 0,
            b: 1,
            value: 2.,
        },
        Constraint::EqualLength {
            a: 0,
            b: 1,
            c: 0,
            d: 2,
        },
        Constraint::PointOnCircle {
            point: 1,
            circle: 0,
        },
        Constraint::TangentCircles {
            a: 0,
            b: 1,
            internal: false,
        },
        Constraint::TangentCircles {
            a: 0,
            b: 1,
            internal: true,
        },
    ];
    let sketch = Sketch {
        points: vec![[0.4, -0.7], [3.1, 1.2], [-2., 2.4], [1.7, 4.2]],
        circles: vec![
            Circle {
                center: 0,
                radius: 1.3,
            },
            Circle {
                center: 2,
                radius: 2.1,
            },
        ],
        constraints,
    };
    sketch.validate().unwrap();
    let x: Vec<f64> = sketch
        .points
        .iter()
        .flatten()
        .copied()
        .chain(sketch.circles.iter().map(|circle| circle.radius))
        .collect();
    let n = x.len();
    let mut offsets = Vec::new();
    let mut rows = 0;
    for constraint in &sketch.constraints {
        offsets.push(rows);
        rows += Sketch::constraint_residual_len(constraint);
    }
    let mut scratch = x.clone();
    let mut jacobian = vec![f64::NAN; rows * n];
    residuals::jacobian_into(&sketch, &offsets, &mut scratch, &mut jacobian, n);
    assert_eq!(scratch, x);
    for (ci, constraint) in sketch.constraints.iter().enumerate() {
        for column in 0..n {
            let h = 1e-4 * x[column].abs().max(1.);
            let values: Vec<_> = [-2., -1., 1., 2.]
                .into_iter()
                .map(|multiple| {
                    let mut probe = x.clone();
                    probe[column] += multiple * h;
                    sketch.eval_constraint_residual(constraint, &probe).0
                })
                .collect();
            for row in 0..Sketch::constraint_residual_len(constraint) {
                let reference = (values[0][row] - 8. * values[1][row] + 8. * values[2][row]
                    - values[3][row])
                    / (12. * h);
                let actual = jacobian[(offsets[ci] + row) * n + column];
                assert!(
                    actual.is_finite()
                        && (actual - reference).abs() <= 1e-7 * reference.abs().max(1.),
                    "{constraint:?}, row {row}, column {column}: {actual} vs {reference}"
                );
            }
        }
    }
}
