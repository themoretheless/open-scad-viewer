#![doc = include_str!("../README.md")]
//! Shared bounded 2D sketch solver. Independent of polygon and NURBS geometry.
#[cfg(feature = "codec")]
mod serialization;
pub use math_core::{Error, Result};
const INVALID_INPUT: &str = "SKETCH_INVALID_INPUT";
pub(crate) fn error(message: impl Into<String>) -> Error {
    Error::new(INVALID_INPUT, message)
}
#[derive(Clone, Debug)]
pub struct Circle {
    pub center: usize,
    pub radius: f64,
}

#[derive(Clone, Debug, PartialEq)]
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
    Angle {
        a: usize,
        b: usize,
        c: usize,
        d: usize,
        radians: f64,
    },
    PointToLineDistance {
        point: usize,
        a: usize,
        b: usize,
        value: f64,
    },
    Midpoint {
        point: usize,
        a: usize,
        b: usize,
    },
    Symmetric {
        a: usize,
        b: usize,
        axis_a: usize,
        axis_b: usize,
    },
    Collinear {
        a: usize,
        b: usize,
        c: usize,
    },
    Radius {
        circle: usize,
        value: f64,
    },
    Concentric {
        a: usize,
        b: usize,
    },
    EqualRadius {
        a: usize,
        b: usize,
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
    /// Incident point on the infinite line through `a` and `b`.
    PointOnLine {
        point: usize,
        a: usize,
        b: usize,
    },
    /// Line segment `a -> b` at fixed angle `radians` to the +X axis.
    FixAngle {
        a: usize,
        b: usize,
        radians: f64,
    },
    /// Equal cosine of angle between two pairs of lines `(a->b, c->d)` and `(e->f, g->h)`
    /// or equal directed angle between `(a->b, c->d)` (4 points: angle(a->b, c->d) == angle(c->d, a->b) is trivial,
    /// so we store two line pairs or 4 points where `(a->b)` and `(c->d)` have equal angle to +X or two pairs `(a->b, c->d)` and `(e->f, g->h)`).
    EqualAngle {
        a: usize,
        b: usize,
        c: usize,
        d: usize,
        e: usize,
        f: usize,
        g: usize,
        h: usize,
    },
    /// Incident point on circular arc (defined by circle `circle` and start/end angles `start_angle`, `end_angle`).
    PointOnArc {
        point: usize,
        circle: usize,
        start_angle: f64,
        end_angle: f64,
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

pub(crate) fn length(v: [f64; 2]) -> f64 {
    v[0].hypot(v[1])
}
pub(crate) fn subtract(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

mod geometry_ops;
mod residuals;
mod solver;

pub use solver::{
    Diagnostics, DofAnalysis, SolverOptions, analyze_dof, solve, solve_with_diagnostics,
    solve_with_diagnostics_options, solve_with_options,
};

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

    #[test]
    fn suggest_constraints_and_render_polylines() {
        let s = Sketch {
            points: vec![
                [0.0, 0.0],
                [5.0, 0.0001],
                [0.0, 5.0],
                [0.0001, 0.0001],
            ],
            circles: vec![
                Circle {
                    center: 0,
                    radius: 2.0,
                },
                Circle {
                    center: 1,
                    radius: 2.0002,
                },
            ],
            constraints: vec![
                Constraint::Distance {
                    a: 0,
                    b: 1,
                    value: 5.0,
                },
                Constraint::Distance {
                    a: 0,
                    b: 2,
                    value: 5.0,
                },
            ],
        };
        let suggested = s.suggest_constraints(1e-3, 1e-3);
        assert!(suggested.contains(&Constraint::Coincident { a: 0, b: 3 }));
        assert!(suggested.contains(&Constraint::Horizontal { a: 0, b: 1 }));
        assert!(suggested.contains(&Constraint::Vertical { a: 0, b: 2 }));
        assert!(suggested.contains(&Constraint::Perpendicular {
            a: 0,
            b: 1,
            c: 0,
            d: 2,
        }));
        assert!(suggested.contains(&Constraint::EqualLength {
            a: 0,
            b: 1,
            c: 0,
            d: 2,
        }));
        assert!(suggested.contains(&Constraint::EqualRadius { a: 0, b: 1 }));

        let polylines = s.render_polylines(16);
        // 2 distance segments + 2 full circles
        assert_eq!(polylines.len(), 4);
        assert_eq!(polylines[0].len(), 2);
        assert_eq!(polylines[2].len(), 17);
    }

    #[test]
    fn offset_closed_loop_and_fillet_corner() {
        let mut s = Sketch {
            points: vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]],
            circles: vec![],
            constraints: vec![],
        };
        // Outward offset by +1.0 on a 10x10 CCW square produces a 12x12 square [-1, 11]^2.
        let offset_pts = s.offset_closed_loop(&[0, 1, 2, 3], 1.0);
        assert_eq!(offset_pts.len(), 4);
        assert!((offset_pts[0][0] - (-1.0)).abs() < 1e-9 && (offset_pts[0][1] - (-1.0)).abs() < 1e-9);
        assert!((offset_pts[1][0] - 11.0).abs() < 1e-9 && (offset_pts[1][1] - (-1.0)).abs() < 1e-9);
        assert!((offset_pts[2][0] - 11.0).abs() < 1e-9 && (offset_pts[2][1] - 11.0).abs() < 1e-9);
        assert!((offset_pts[3][0] - (-1.0)).abs() < 1e-9 && (offset_pts[3][1] - 11.0).abs() < 1e-9);

        // Fillet the 90-degree corner at point 0 (between point 3=(0,10), point 0=(0,0), point 1=(10,0)) with radius 2.0.
        let (t1_idx, t2_idx, center_idx) = s.fillet_corner(3, 0, 1, 2.0).unwrap();
        assert_eq!((t1_idx, t2_idx, center_idx), (4, 5, 6));
        assert!((s.points[t1_idx][0] - 0.0).abs() < 1e-9 && (s.points[t1_idx][1] - 2.0).abs() < 1e-9);
        assert!((s.points[t2_idx][0] - 2.0).abs() < 1e-9 && (s.points[t2_idx][1] - 0.0).abs() < 1e-9);
        assert!((s.points[center_idx][0] - 2.0).abs() < 1e-9 && (s.points[center_idx][1] - 2.0).abs() < 1e-9);
        assert_eq!(s.circles.len(), 1);
        assert!((s.circles[0].radius - 2.0).abs() < 1e-9);
        // Radius too large for edge length returns None.
        assert!(s.fillet_corner(3, 0, 1, 15.0).is_none());

        // Chamfer the corner at point 1=(10,0) between point 0=(0,0) and point 2=(10,10) with distances (3.0, 4.0).
        let (c1_idx, c2_idx) = s.chamfer_corner(0, 1, 2, 3.0, 4.0).unwrap();
        assert!((s.points[c1_idx][0] - 7.0).abs() < 1e-9 && s.points[c1_idx][1].abs() < 1e-9);
        assert!((s.points[c2_idx][0] - 10.0).abs() < 1e-9 && (s.points[c2_idx][1] - 4.0).abs() < 1e-9);
        assert!(s.chamfer_corner(0, 1, 2, 12.0, 4.0).is_none());

        // Mirror points across the Y-axis segment (point 0=(0,0) -> point 3=(0,10)).
        let mirrored = s.mirror_points(&[1, 2], 0, 3, true).unwrap();
        assert_eq!(mirrored.len(), 2);
        assert!((s.points[mirrored[0]][0] - (-10.0)).abs() < 1e-9 && s.points[mirrored[0]][1].abs() < 1e-9);
        assert!((s.points[mirrored[1]][0] - (-10.0)).abs() < 1e-9 && (s.points[mirrored[1]][1] - 10.0).abs() < 1e-9);
        assert!(s.constraints.contains(&Constraint::Symmetric {
            a: 1,
            b: mirrored[0],
            axis_a: 0,
            axis_b: 3,
        }));
    }

    #[test]
    fn add_regular_polygon_and_degrees_of_freedom() {
        let mut s = Sketch {
            points: vec![],
            circles: vec![],
            constraints: vec![],
        };
        assert!(s.add_regular_polygon((0.0, 0.0), 5.0, 2, 0.0).is_none());
        assert!(s.add_regular_polygon((0.0, 0.0), 0.0, 4, 0.0).is_none());

        let (verts, cons) = s.add_regular_polygon((2.0, -1.0), 5.0, 6, 0.0).unwrap();
        assert_eq!(verts.len(), 6);
        assert_eq!(cons.len(), 5);
        assert_eq!(s.points.len(), 6);
        // 6 vertices -> 12 point coordinates, minus 5 EqualLength scalar equations = 7 DOF.
        assert_eq!(s.degrees_of_freedom(), 7);

        // Verify all 6 sides have equal length and lie on the radius-5 circle around (2, -1).
        let side0 = length(subtract(s.points[verts[1]], s.points[verts[0]]));
        assert!((side0 - 5.0).abs() < 1e-9);
        for i in 0..6 {
            let p = s.points[verts[i]];
            let q = s.points[verts[(i + 1) % 6]];
            assert!((length(subtract(p, [2.0, -1.0])) - 5.0).abs() < 1e-9);
            assert!((length(subtract(q, p)) - side0).abs() < 1e-9);
        }
    }
}
