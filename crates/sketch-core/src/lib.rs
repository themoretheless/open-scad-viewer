#![doc = include_str!("../README.md")]
//! Shared bounded 2D sketch solver. Independent of polygon and NURBS geometry.
#[cfg(feature = "codec")]
mod serialization;
pub use math_core::{Error, Result};
const INVALID_INPUT: &str = "SKETCH_INVALID_INPUT";
fn error(message: impl Into<String>) -> Error {
    Error::new(INVALID_INPUT, message)
}
#[derive(Clone, Debug)]
pub struct Circle {
    pub center: usize,
    pub radius: f64,
}

#[derive(Clone, Debug)]
pub enum Constraint {
    Fix {
        point: usize,
        at: [f64; 2],
    },
    Horizontal {
        a: usize,
        b: usize,
    },
    Vertical {
        a: usize,
        b: usize,
    },
    Coincident {
        a: usize,
        b: usize,
    },
    Distance {
        a: usize,
        b: usize,
        value: f64,
    },
    Parallel {
        a: usize,
        b: usize,
        c: usize,
        d: usize,
    },
    Perpendicular {
        a: usize,
        b: usize,
        c: usize,
        d: usize,
    },
    EqualLength {
        a: usize,
        b: usize,
        c: usize,
        d: usize,
    },
    Radius {
        circle: usize,
        value: f64,
    },
    PointOnCircle {
        point: usize,
        circle: usize,
    },
    TangentLineCircle {
        a: usize,
        b: usize,
        circle: usize,
    },
    TangentCircles {
        a: usize,
        b: usize,
        internal: bool,
    },
}

#[derive(Clone, Debug)]
pub struct Sketch {
    pub points: Vec<[f64; 2]>,
    pub circles: Vec<Circle>,
    pub constraints: Vec<Constraint>,
}

#[derive(Clone, Debug)]
pub struct Solution {
    pub sketch: Sketch,
    pub status: &'static str,
    pub iterations: usize,
    pub max_residual: f64,
    pub residuals: Vec<Vec<f64>>,
    pub degrees_of_freedom: usize,
    pub convergence_certified: bool,
}

fn length(v: [f64; 2]) -> f64 {
    v[0].hypot(v[1])
}
fn subtract(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}
impl Sketch {
    fn validate(&self) -> Result<()> {
        if self.points.is_empty()
            || self.points.len() > 24
            || self.circles.len() > 12
            || self.constraints.len() > 96
            || self
                .points
                .iter()
                .flatten()
                .any(|v| !v.is_finite() || v.abs() > 1e6)
            || self.circles.iter().any(|c| {
                c.center >= self.points.len()
                    || !c.radius.is_finite()
                    || c.radius <= 1e-9
                    || c.radius > 1e6
            })
        {
            return Err(error("Sketch input or budget invalid"));
        }
        let point = |i: usize| i < self.points.len();
        let circle = |i: usize| i < self.circles.len();
        let positive = |v: f64| v.is_finite() && v > 0. && v <= 1e6;
        for c in &self.constraints {
            let valid = match *c {
                Constraint::Fix { point: p, at } => {
                    point(p) && at.iter().all(|v| v.is_finite() && v.abs() <= 1e6)
                }
                Constraint::Horizontal { a, b }
                | Constraint::Vertical { a, b }
                | Constraint::Coincident { a, b } => point(a) && point(b),
                Constraint::Distance { a, b, value } => {
                    point(a) && point(b) && value.is_finite() && value > 0.
                }
                Constraint::Parallel { a, b, c, d }
                | Constraint::Perpendicular { a, b, c, d }
                | Constraint::EqualLength { a, b, c, d } => [a, b, c, d].iter().all(|&p| point(p)),
                Constraint::Radius { circle: c, value } => circle(c) && positive(value),
                Constraint::PointOnCircle {
                    point: p,
                    circle: c,
                } => point(p) && circle(c),
                Constraint::TangentLineCircle { a, b, circle: c } => {
                    point(a) && point(b) && a != b && circle(c)
                }
                Constraint::TangentCircles { a, b, .. } => circle(a) && circle(b) && a != b,
            };
            if !valid {
                return Err(error("Invalid sketch constraint reference/value"));
            }
        }
        Ok(())
    }
    fn residual(&self, x: &[f64]) -> Vec<Vec<f64>> {
        let p = |i: usize| [x[2 * i], x[2 * i + 1]];
        let r = |i: usize| x[2 * self.points.len() + i];
        let center = |i: usize| p(self.circles[i].center);
        self.constraints
            .iter()
            .map(|constraint| match *constraint {
                Constraint::Fix { point, at } => subtract(p(point), at).to_vec(),
                Constraint::Horizontal { a, b } => vec![p(b)[1] - p(a)[1]],
                Constraint::Vertical { a, b } => vec![p(b)[0] - p(a)[0]],
                Constraint::Coincident { a, b } => subtract(p(a), p(b)).to_vec(),
                Constraint::Distance { a, b, value } => vec![length(subtract(p(a), p(b))) - value],
                Constraint::Parallel { a, b, c, d }
                | Constraint::Perpendicular { a, b, c, d }
                | Constraint::EqualLength { a, b, c, d } => {
                    let u = subtract(p(b), p(a));
                    let v = subtract(p(d), p(c));
                    let lu = length(u);
                    let lv = length(v);
                    let scale = lu.max(lv).max(1e-12);
                    vec![match constraint {
                        Constraint::Parallel { .. } => (u[0] * v[1] - u[1] * v[0]) / scale,
                        Constraint::Perpendicular { .. } => (u[0] * v[0] + u[1] * v[1]) / scale,
                        _ => lu - lv,
                    }]
                }
                Constraint::Radius { circle, value } => vec![r(circle) - value],
                Constraint::PointOnCircle { point, circle } => {
                    vec![length(subtract(p(point), center(circle))) - r(circle)]
                }
                Constraint::TangentLineCircle { a, b, circle } => {
                    let u = subtract(p(b), p(a));
                    let v = subtract(center(circle), p(a));
                    vec![(u[0] * v[1] - u[1] * v[0]).abs() / length(u).max(1e-12) - r(circle)]
                }
                Constraint::TangentCircles { a, b, internal } => vec![
                    length(subtract(center(a), center(b)))
                        - if internal {
                            (r(a) - r(b)).abs()
                        } else {
                            r(a) + r(b)
                        },
                ],
            })
            .collect()
    }
}
fn solve_linear(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    for i in 0..n {
        let pivot = (i..n)
            .max_by(|&j, &k| a[j][i].abs().total_cmp(&a[k][i].abs()))
            .unwrap();
        if a[pivot][i].abs() < 1e-18 {
            return None;
        }
        a.swap(i, pivot);
        b.swap(i, pivot);
        for j in i + 1..n {
            let f = a[j][i] / a[i][i];
            let (before, after) = a.split_at_mut(j);
            for (value, pivot) in after[0][i..n].iter_mut().zip(&before[i][i..n]) {
                *value -= f * pivot;
            }
            b[j] -= f * b[i];
        }
    }
    let mut x = vec![0.; n];
    for i in (0..n).rev() {
        x[i] = (b[i] - (i + 1..n).map(|j| a[i][j] * x[j]).sum::<f64>()) / a[i][i];
    }
    Some(x)
}
fn jacobian(sketch: &Sketch, x: &[f64]) -> Vec<Vec<f64>> {
    let m = sketch.residual(x).iter().map(Vec::len).sum();
    let mut matrix = vec![vec![0.; x.len()]; m];
    for j in 0..x.len() {
        let h = (x[j].abs() * 1e-7).max(1e-6);
        let mut plus = x.to_vec();
        let mut minus = x.to_vec();
        plus[j] += h;
        minus[j] -= h;
        let a: Vec<_> = sketch.residual(&plus).into_iter().flatten().collect();
        let b: Vec<_> = sketch.residual(&minus).into_iter().flatten().collect();
        for i in 0..m {
            matrix[i][j] = (a[i] - b[i]) / (2. * h);
        }
    }
    matrix
}
fn rank(mut a: Vec<Vec<f64>>, n: usize) -> usize {
    let mut row = 0;
    for col in 0..n {
        if row >= a.len() {
            break;
        }
        let pivot = (row..a.len())
            .max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))
            .unwrap();
        if a[pivot][col].abs() < 1e-7 {
            continue;
        }
        a.swap(row, pivot);
        for i in row + 1..a.len() {
            let f = a[i][col] / a[row][col];
            let (before, after) = a.split_at_mut(i);
            for (value, pivot) in after[0][col..n].iter_mut().zip(&before[row][col..n]) {
                *value -= f * pivot;
            }
        }
        row += 1;
    }
    row
}
/// Bounded numerical solver policy, shared by native and named adapters.
#[derive(Clone, Copy, Debug)]
pub struct SolverOptions {
    pub tolerance: f64,
    pub max_iterations: usize,
}
pub fn solve(sketch: &Sketch, tolerance: f64) -> Result<Solution> {
    solve_with_options(
        sketch,
        SolverOptions {
            tolerance,
            max_iterations: 96,
        },
    )
}
pub fn solve_with_options(sketch: &Sketch, options: SolverOptions) -> Result<Solution> {
    let SolverOptions {
        tolerance,
        max_iterations,
    } = options;
    if !(1..=96).contains(&max_iterations) {
        return Err(error("Invalid solver iteration limit"));
    }

    sketch.validate()?;
    if !tolerance.is_finite() || !(1e-8..=0.1).contains(&tolerance) {
        return Err(error("Invalid solver tolerance"));
    }
    let mut x: Vec<_> = sketch
        .points
        .iter()
        .flatten()
        .copied()
        .chain(sketch.circles.iter().map(|c| c.radius))
        .collect();
    let n = x.len();
    let mut damping = 1e-3;
    let mut iterations = 0;
    let objective = |x: &[f64]| {
        sketch
            .residual(x)
            .into_iter()
            .flatten()
            .map(|v| v * v)
            .sum::<f64>()
    };
    for _ in 0..max_iterations {
        let residual: Vec<_> = sketch.residual(&x).into_iter().flatten().collect();
        if residual.iter().all(|v| v.abs() <= tolerance) {
            break;
        }
        iterations += 1;
        let j = jacobian(sketch, &x);
        let mut a = vec![vec![0.; n]; n];
        let mut b = vec![0.; n];
        for i in 0..n {
            for (k, row) in j.iter().enumerate() {
                b[i] -= row[i] * residual[k];
                for l in 0..n {
                    a[i][l] += row[i] * row[l];
                }
            }
            a[i][i] += damping;
        }
        let Some(delta) = solve_linear(a, b) else {
            damping *= 10.;
            continue;
        };
        let candidate: Vec<_> = x.iter().zip(delta).map(|(a, b)| a + b).collect();
        let valid = candidate.iter().all(|v| v.is_finite() && v.abs() <= 1e6)
            && candidate[sketch.points.len() * 2..]
                .iter()
                .all(|r| *r > 1e-9);
        if valid && objective(&candidate) < objective(&x) {
            x = candidate;
            damping = (damping / 3.).max(1e-12);
        } else {
            damping = (damping * 10.).min(1e12);
        }
    }
    let residuals = sketch.residual(&x);
    let max = residuals
        .iter()
        .flatten()
        .map(|v| v.abs())
        .fold(0., f64::max);
    let dof = n - rank(jacobian(sketch, &x), n);
    let mut result = sketch.clone();
    for (i, p) in result.points.iter_mut().enumerate() {
        *p = [x[2 * i], x[2 * i + 1]];
    }
    for (i, c) in result.circles.iter_mut().enumerate() {
        c.radius = x[sketch.points.len() * 2 + i];
    }
    let degenerate = !degenerate_constraint_indices(&result, tolerance).is_empty();
    let status = if degenerate {
        "degenerate"
    } else if max > tolerance {
        "not_converged"
    } else if dof > 0 {
        "underconstrained"
    } else {
        "solved"
    };
    Ok(Solution {
        sketch: result,
        status,
        iterations,
        max_residual: max,
        residuals,
        degrees_of_freedom: dof,
        convergence_certified: false,
    })
}

fn degenerate_constraint_indices(sketch: &Sketch, tolerance: f64) -> Vec<usize> {
    sketch
        .constraints
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            let degenerate = match *c {
                Constraint::Parallel { a, b, c, d } | Constraint::Perpendicular { a, b, c, d } => {
                    length(subtract(sketch.points[a], sketch.points[b])) <= tolerance
                        || length(subtract(sketch.points[c], sketch.points[d])) <= tolerance
                }
                Constraint::TangentLineCircle { a, b, .. } => {
                    length(subtract(sketch.points[a], sketch.points[b])) <= tolerance
                }
                Constraint::TangentCircles {
                    a,
                    b,
                    internal: true,
                } => (sketch.circles[a].radius - sketch.circles[b].radius).abs() <= tolerance,
                _ => false,
            };
            degenerate.then_some(i)
        })
        .collect()
}

/// Constraint diagnostics for named sketch adapters, independent of transport.
#[derive(Clone, Debug)]
pub struct Diagnostics {
    pub redundant_equations: usize,
    pub degenerate_constraints: Vec<usize>,
    pub constraint_residuals: Vec<f64>,
    pub inconsistent: bool,
}
pub fn solve_with_diagnostics(sketch: &Sketch, tolerance: f64) -> Result<(Solution, Diagnostics)> {
    solve_with_diagnostics_options(
        sketch,
        SolverOptions {
            tolerance,
            max_iterations: 96,
        },
    )
}
pub fn solve_with_diagnostics_options(
    sketch: &Sketch,
    options: SolverOptions,
) -> Result<(Solution, Diagnostics)> {
    let tolerance = options.tolerance;
    let solution = solve_with_options(sketch, options)?;
    let solved = &solution.sketch;
    let x: Vec<_> = solved
        .points
        .iter()
        .flatten()
        .copied()
        .chain(solved.circles.iter().map(|c| c.radius))
        .collect();
    let j = jacobian(solved, &x);
    let jacobian_rank = rank(j.clone(), x.len());
    let residual: Vec<_> = solution.residuals.iter().flatten().copied().collect();
    let linear = solved.constraints.iter().all(|c| {
        matches!(
            c,
            Constraint::Fix { .. }
                | Constraint::Horizontal { .. }
                | Constraint::Vertical { .. }
                | Constraint::Coincident { .. }
        )
    });
    let inconsistent = solution.max_residual > tolerance
        && linear
        && rank(
            j.into_iter()
                .zip(&residual)
                .map(|(mut row, r)| {
                    row.push(*r);
                    row
                })
                .collect(),
            x.len() + 1,
        ) > jacobian_rank;
    let degenerate_constraints = degenerate_constraint_indices(solved, tolerance);
    let diagnostics = Diagnostics {
        redundant_equations: residual.len().saturating_sub(jacobian_rank),
        degenerate_constraints,
        constraint_residuals: solution
            .residuals
            .iter()
            .map(|r| r.iter().map(|x| x.abs()).fold(0., f64::max))
            .collect(),
        inconsistent,
    };
    Ok((solution, diagnostics))
}

#[cfg(test)]
mod tests {
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
}
