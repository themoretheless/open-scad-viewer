//! Conforming rectangular refinement with continuous conservative error bounds.
use crate::{
    Result, check,
    distance_bounds::{Interval as I, box_distance},
    resource,
    surface::Surface,
};
use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    Tolerance,
    WorkLimit,
    PrecisionLimit,
}
#[derive(Clone, Debug)]
pub struct Triangle {
    pub vertices: [usize; 3],
    pub error_upper: f64,
    pub within_tolerance: bool,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub points: Vec<[f64; 3]>,
    pub parameters: Vec<[f64; 2]>,
    pub triangles: Vec<Triangle>,
    pub error_upper: f64,
    pub within_tolerance: bool,
    pub cells: usize,
    pub reason: StopReason,
}
struct Cell {
    span: [usize; 2],
    domain: [[f64; 2]; 2],
    points: [[f64; 3]; 4],
    error: f64,
}
fn corners(d: [[f64; 2]; 2]) -> [[f64; 2]; 4] {
    [
        [d[0][0], d[1][0]],
        [d[0][1], d[1][0]],
        [d[0][1], d[1][1]],
        [d[0][0], d[1][1]],
    ]
}
fn measure(s: &Surface, span: [usize; 2], domain: [[f64; 2]; 2]) -> Result<Cell> {
    let mut points = [[0.; 3]; 4];
    for (i, p) in corners(domain).into_iter().enumerate() {
        points[i] = s.evaluate_validated(p[0], p[1])?.point;
    }
    let mut mesh = [I::point(0.); 3];
    for k in 0..3 {
        mesh[k] = I::new(
            points.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min),
            points
                .iter()
                .map(|p| p[k])
                .fold(f64::NEG_INFINITY, f64::max),
        )?;
    }
    let image = crate::surface_distance::enclosure(s, span, domain)?;
    let error = box_distance(&image, &mesh)?.1;
    Ok(Cell {
        span,
        domain,
        points,
        error,
    })
}
/// Covers every active knot rectangle. Refining an entire grid strip preserves
/// shared edges within the surface. max_cells counts all evaluated rectangles,
/// including replaced parents. Limits return the whole mesh with explicit flags.
/// Bounds certify corresponding positions and symmetric Hausdorff deviation;
/// they do not certify mesh topology or regularity. Periodic seams stay cut.
pub fn tessellate(surface: &Surface, tolerance: f64, max_cells: usize) -> Result<Report> {
    surface.validate()?;
    check(
        tolerance.is_finite() && tolerance > 0.,
        "Tessellation tolerance must be positive and finite",
    )?;
    check(
        (1..=100000).contains(&max_cells),
        "Tessellation needs 1..100000 cells",
    )?;
    let mut spans = [Vec::new(), Vec::new()];
    for axis in 0..2 {
        let (p, k, n) = if axis == 0 {
            (
                surface.degree_u,
                &surface.knots_u,
                surface.control_points.len(),
            )
        } else {
            (
                surface.degree_v,
                &surface.knots_v,
                surface.control_points[0].len(),
            )
        };
        for &t in k {
            if k[p] < t && t < k[n] {
                check(
                    k.iter().filter(|&&x| x == t).count() <= p,
                    "Continuous tessellation requires continuous interior knots",
                )?;
            }
        }
        spans[axis] = (p..n).filter(|&i| k[i] < k[i + 1]).collect();
    }
    if spans[0].len() * spans[1].len() > max_cells {
        return Err(resource(
            "Initial surface knot rectangles exceed the cell budget",
        ));
    }
    let mut grid = Vec::new();
    for &u in &spans[0] {
        for &v in &spans[1] {
            grid.push(measure(
                surface,
                [u, v],
                [
                    [surface.knots_u[u], surface.knots_u[u + 1]],
                    [surface.knots_v[v], surface.knots_v[v + 1]],
                ],
            )?);
        }
    }
    let mut work = grid.len();
    let reason = loop {
        let worst = grid
            .iter()
            .max_by(|a, b| a.error.total_cmp(&b.error))
            .unwrap();
        if worst.error <= tolerance {
            break StopReason::Tolerance;
        }
        let d = worst.domain;
        let span = worst.span;
        let ratios = [
            (d[0][1] - d[0][0]) / (surface.knots_u[span[0] + 1] - surface.knots_u[span[0]]),
            (d[1][1] - d[1][0]) / (surface.knots_v[span[1] + 1] - surface.knots_v[span[1]]),
        ];
        let preferred = usize::from(ratios[1] > ratios[0]);
        let mut choice = None;
        let mut representable = false;
        for axis in [preferred, 1 - preferred] {
            let midpoint = d[axis][0] * 0.5 + d[axis][1] * 0.5;
            if !(d[axis][0] < midpoint && midpoint < d[axis][1]) {
                continue;
            }
            representable = true;
            let strip = grid.iter().filter(|c| c.domain[axis] == d[axis]).count();
            if work + 2 * strip <= max_cells {
                choice = Some((axis, midpoint, strip));
                break;
            }
        }
        let Some((axis, midpoint, strip)) = choice else {
            break if representable {
                StopReason::WorkLimit
            } else {
                StopReason::PrecisionLimit
            };
        };
        let mut next = Vec::with_capacity(grid.len() + strip);
        for c in grid {
            if c.domain[axis] == d[axis] {
                for half in 0..2 {
                    let mut domain = c.domain;
                    domain[axis][1 - half] = midpoint;
                    next.push(measure(surface, c.span, domain)?);
                }
            } else {
                next.push(c);
            }
        }
        work += 2 * strip;
        grid = next;
    };
    let mut out = Report {
        points: Vec::new(),
        parameters: Vec::new(),
        triangles: Vec::new(),
        error_upper: 0.,
        within_tolerance: reason == StopReason::Tolerance,
        cells: work,
        reason,
    };
    let mut ids = BTreeMap::new();
    for c in grid {
        let mut vertices = [0; 4];
        for (i, uv) in corners(c.domain).into_iter().enumerate() {
            let key = uv.map(|x| if x == 0. { 0 } else { x.to_bits() });
            vertices[i] = *ids.entry(key).or_insert_with(|| {
                let id = out.points.len();
                out.points.push(c.points[i]);
                out.parameters.push(uv);
                id
            });
        }
        out.error_upper = out.error_upper.max(c.error);
        for order in [[0, 1, 2], [0, 2, 3]] {
            out.triangles.push(Triangle {
                vertices: order.map(|i| vertices[i]),
                error_upper: c.error,
                within_tolerance: c.error <= tolerance,
            });
        }
    }
    Ok(out)
}
