//! Local basis proposals from original trace degrees/breaks and residual sites.
//! No sampled residual, basis edit or edit identity is geometric authority.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    U,
    V,
}
impl Direction {
    fn axis(self) -> Axis {
        match self {
            Self::U => Axis::U,
            Self::V => Axis::V,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Step {
    Elevate {
        direction: Direction,
        from: usize,
        to: usize,
    },
    Insert {
        direction: Direction,
        parameter: f64,
    },
}
pub(super) fn apply(s: &Surface, step: Step) -> Result<Surface> {
    match step {
        Step::Elevate {
            direction,
            from,
            to,
        } => {
            if chart(s, direction).0 != from {
                return Err(error(
                    "Adaptive degree edit no longer matches its original basis",
                ));
            }
            crate::surface_edit::elevate(s, direction.axis(), to)
        }
        Step::Insert {
            direction,
            parameter,
        } => crate::surface_edit::insert(s, direction.axis(), parameter, 1),
    }
}
fn chart(s: &Surface, a: Direction) -> (usize, &[f64], usize, [f64; 2]) {
    let (p, k, n) = match a {
        Direction::U => (s.degree_u, s.knots_u.as_slice(), s.control_points.len()),
        Direction::V => (s.degree_v, s.knots_v.as_slice(), s.control_points[0].len()),
    };
    (p, k, n, [k[p], k[n]])
}
pub(super) fn validate_boundary_seed(s: &Surface, sides: &[Boundary]) -> Result<()> {
    if sides.is_empty()
        || sides.len() > 4
        || sides.iter().enumerate().any(|(i, a)| {
            sides[..i]
                .iter()
                .any(|b| std::mem::discriminant(a) == std::mem::discriminant(b))
        })
    {
        return Err(error("Choose distinct original adaptive boundary sides"));
    }
    for a in [Direction::U, Direction::V] {
        let (p, k, n, d) = chart(s, a);
        if k[..=p].iter().any(|v| *v != d[0]) || k[n..].iter().any(|v| *v != d[1]) {
            return Err(error(
                "Adaptive original boundary locks need clamped original bases",
            ));
        }
    }
    // Degree-one seeds legitimately have zero second jets. Construction can
    // elevate them before it needs three coefficient layers; replay still
    // evaluates the ORIGINAL degree-one surface, never its elevated copy.
    Ok(())
}
fn parameter(c: &Curve, t: f64) -> f64 {
    let d = c.domain();
    (d[0] + t * (d[1] - d[0])).clamp(d[0], d[1])
}
fn component(a: Direction) -> usize {
    if matches!(a, Direction::U) { 0 } else { 1 }
}
fn variation(t: &Trace, a: Direction) -> f64 {
    let k = component(a);
    let lo = t
        .parameters
        .control_points
        .iter()
        .map(|p| p[k])
        .fold(f64::INFINITY, f64::min);
    let hi = t
        .parameters
        .control_points
        .iter()
        .map(|p| p[k])
        .fold(f64::NEG_INFINITY, f64::max);
    hi - lo
}
pub(super) fn select(
    s: &Surface,
    traces: &[Trace],
    hard: &[Constraint],
    locks: Option<network::BoundaryLocks<'_>>,
    o: Options,
    iteration: usize,
) -> Result<Step> {
    let axes = [Direction::U, Direction::V];
    for a in axes {
        let (p, _, _, d) = chart(s, a);
        let mut degree = p;
        for t in traces {
            if variation(t, a) > 1e-12 * (d[1] - d[0]) {
                degree = degree.max(t.spatial.degree);
            }
            if t.curvature {
                degree = degree.max(3);
            }
        }
        for h in hard {
            match h {
                Constraint::Jet { equation, .. } => {
                    let required = match a {
                        Direction::U => [2, 1, 0],
                        Direction::V => [0, 1, 2],
                    };
                    for (row, order) in equation.second.iter().zip(required) {
                        if row.iter().any(|v| *v != 0.) {
                            degree = degree.max(order);
                        }
                    }
                }
                Constraint::Derivative { .. } | Constraint::TangentPlane { .. } => {
                    degree = degree.max(1)
                }
                _ => {}
            }
        }
        if let Some(l) = locks {
            let pair = match a {
                Direction::U => [Boundary::UMin, Boundary::UMax],
                Direction::V => [Boundary::VMin, Boundary::VMax],
            };
            let n = pair
                .iter()
                .filter(|side| {
                    l.boundaries
                        .iter()
                        .any(|other| std::mem::discriminant(other) == std::mem::discriminant(*side))
                })
                .count();
            degree = degree.max(if n == 2 {
                6
            } else if n == 1 {
                3
            } else {
                2
            });
        }
        let degree = degree.min(o.max_degree);
        if degree > p {
            return Ok(Step::Elevate {
                direction: a,
                from: p,
                to: degree,
            });
        }
    }
    // Source knot multiplicities identify where an authored guide changes its
    // polynomial pieces. Map that original clock into candidate UV coordinates;
    // these are local insertion proposals, not an assumed inverse identity.
    for t in traces {
        let cd = t.spatial.domain();
        let mut i = t.spatial.degree + 1;
        while i < t.spatial.control_points.len() {
            let mut j = i + 1;
            while j < t.spatial.control_points.len() && t.spatial.knots[j] == t.spatial.knots[i] {
                j += 1;
            }
            let mut q = (t.spatial.knots[i] - cd[0]) / (cd[1] - cd[0]);
            if t.reverse {
                q = 1. - q;
            }
            let uv = t.parameters.evaluate(parameter(&t.parameters, q))?.point;
            for a in axes {
                let (p, k, _, d) = chart(s, a);
                if variation(t, a) <= 1e-12 * (d[1] - d[0]) {
                    continue;
                }
                let mut x = uv[component(a)];
                let epsilon = 64. * f64::EPSILON * (1. + d[0].abs() + d[1].abs());
                if x <= d[0] + epsilon || x >= d[1] - epsilon {
                    continue;
                }
                if let Some(y) = k.iter().find(|y| (**y - x).abs() <= epsilon) {
                    x = *y;
                }
                let existing = k.iter().filter(|y| **y == x).count();
                let required =
                    (p as isize - t.spatial.degree as isize + (j - i) as isize).max(1) as usize;
                let maximum = if t.curvature { p.saturating_sub(2) } else { p };
                if existing < required.min(maximum) {
                    return Ok(Step::Insert {
                        direction: a,
                        parameter: x,
                    });
                }
            }
            i = j;
        }
    }
    // No remaining authored break: split the highest-scored active span.
    // Residuals against the immutable refined anchor only locate construction
    // effort. Every final world curve still needs independent full proof.
    let mut sites = Vec::<([f64; 2], f64)>::new();
    for t in traces {
        for i in 0..=32 {
            let q = i as f64 / 32.;
            let u = t.parameters.evaluate(parameter(&t.parameters, q))?.point;
            let p = t
                .spatial
                .evaluate(parameter(&t.spatial, if t.reverse { 1. - q } else { q }))?
                .point;
            let at = s.evaluate(u[0], u[1])?.point;
            let residual = p
                .iter()
                .zip(at)
                .map(|(a, b)| (a - b) * (a - b))
                .sum::<f64>()
                .sqrt();
            sites.push(([u[0], u[1]], residual.min(1e100)));
        }
    }
    for h in hard {
        let uv = match h {
            Constraint::Point { uv, .. }
            | Constraint::Derivative { uv, .. }
            | Constraint::Jet { uv, .. }
            | Constraint::TangentPlane { uv, .. } => Some(*uv),
            _ => None,
        };
        if let Some(uv) = uv {
            sites.push((uv, 1.));
        }
    }
    let mut candidates = Vec::new();
    for a in if iteration % 2 == 0 {
        axes
    } else {
        [Direction::V, Direction::U]
    } {
        let (p, k, n, d) = chart(s, a);
        let c = component(a);
        let activity = traces
            .iter()
            .map(|t| variation(t, a) / (d[1] - d[0]))
            .fold(0f64, f64::max);
        for span in p..n {
            let lo = k[span];
            let hi = k[span + 1];
            let mid = lo * 0.5 + hi * 0.5;
            if mid <= lo || mid >= hi {
                continue;
            }
            let residual = sites
                .iter()
                .filter(|(uv, _)| lo <= uv[c] && uv[c] <= hi)
                .map(|(_, r)| *r)
                .fold(0f64, f64::max);
            let score = (hi - lo) / (d[1] - d[0]) * (1. + activity) * (1. + residual);
            candidates.push((
                score,
                Step::Insert {
                    direction: a,
                    parameter: mid,
                },
            ));
        }
    }
    candidates
        .into_iter()
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, s)| s)
        .ok_or_else(|| error("No bounded adaptive knot proposal remains"))
}
