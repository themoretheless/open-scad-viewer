//! Continuous G2 inspection under an explicit affine boundary correspondence.
use crate::{
    Result, check,
    distance_bounds::{Interval as I, box_distance},
    surface::Surface,
    surface_join::{Boundary, Decision, boundary_box},
    surface_shape_operator,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    Same,
    Opposite,
}
#[derive(Clone, Debug)]
pub struct BranchBounds {
    pub normal_lower: f64,
    pub tensor_lower: f64,
    pub normal_upper: Option<f64>,
    pub tensor_upper: Option<f64>,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub position_bounds: [f64; 2],
    /// Same and opposite orientation respectively. A branch uses one coherent
    /// sign for BOTH the normal and the signed curvature tensor.
    pub branches: [BranchBounds; 2],
    pub orientation: Option<Orientation>,
    pub regularity_proven: bool,
    pub decision: Decision,
    pub cells: usize,
}
fn shape(s: &Surface, e: Boundary, t: [f64; 2], reverse: bool) -> Result<Option<(Vec<I>, Vec<I>)>> {
    let report = surface_shape_operator::boundary_bounds(s, e, t, reverse)?;
    let (Some(normal), Some(tensor)) = (report.normal, report.tensor) else {
        return Ok(None);
    };
    let normal = normal
        .into_iter()
        .map(|x| I::new(x[0], x[1]))
        .collect::<Result<Vec<_>>>()?;
    let tensor = tensor
        .into_iter()
        .flatten()
        .map(|x| I::new(x[0], x[1]))
        .collect::<Result<Vec<_>>>()?;
    Ok(Some((normal, tensor)))
}
fn distances(a: &(Vec<I>, Vec<I>), b: &(Vec<I>, Vec<I>)) -> Result<[[[f64; 2]; 2]; 2]> {
    let mut out = [[[0.; 2]; 2]; 2];
    for sign in 0..2 {
        for component in 0..2 {
            let (x, y) = if component == 0 {
                (&a.0, &b.0)
            } else {
                (&a.1, &b.1)
            };
            let y = if sign == 0 {
                y.clone()
            } else {
                y.iter()
                    .map(|x| I::new(-x.hi, -x.lo))
                    .collect::<Result<Vec<_>>>()?
            };
            let (lo, hi) = box_distance(x, &y)?;
            out[sign][component] = [lo, if component == 0 { hi.min(2.) } else { hi }];
        }
    }
    Ok(out)
}
struct Cell {
    t: [f64; 2],
    position: [f64; 2],
    lower: [[f64; 2]; 2],
    upper: Option<[[f64; 2]; 2]>,
}
fn cell(
    a: &Surface,
    ea: Boundary,
    b: &Surface,
    eb: Boundary,
    reverse: bool,
    t: [f64; 2],
) -> Result<Cell> {
    let upper = match (shape(a, ea, t, false)?, shape(b, eb, t, reverse)?) {
        (Some(x), Some(y)) => Some(distances(&x, &y)?.map(|branch| branch.map(|x| x[1]))),
        _ => None,
    };
    let mut position = [
        0.,
        box_distance(
            &boundary_box(a, ea, t, false)?,
            &boundary_box(b, eb, t, reverse)?,
        )?
        .1,
    ];
    let mut lower = [[0_f64; 2]; 2];
    for p in [t[0], t[0] * 0.5 + t[1] * 0.5, t[1]] {
        position[0] = position[0].max(
            box_distance(
                &boundary_box(a, ea, [p, p], false)?,
                &boundary_box(b, eb, [p, p], reverse)?,
            )?
            .0,
        );
        if let (Some(x), Some(y)) = (shape(a, ea, [p, p], false)?, shape(b, eb, [p, p], reverse)?) {
            let d = distances(&x, &y)?;
            for sign in 0..2 {
                for k in 0..2 {
                    lower[sign][k] = lower[sign][k].max(d[sign][k][0]);
                }
            }
        }
    }
    Ok(Cell {
        t,
        position,
        lower,
        upper,
    })
}
/// Normal chord tolerance is dimensionless; tensor tolerance is a Frobenius
/// bound in inverse-length units. One orientation must pass the ENTIRE seam.
/// No inferred or nonaffine seam correspondence is claimed by this API.
pub fn inspect(
    a: &Surface,
    ea: Boundary,
    b: &Surface,
    eb: Boundary,
    reverse: bool,
    position_tolerance: f64,
    normal_tolerance: f64,
    tensor_tolerance: f64,
    max_cells: usize,
) -> Result<Report> {
    a.validate()?;
    b.validate()?;
    check(
        position_tolerance.is_finite()
            && position_tolerance >= 0.
            && normal_tolerance.is_finite()
            && (0. ..=2.).contains(&normal_tolerance)
            && tensor_tolerance.is_finite()
            && tensor_tolerance >= 0.,
        "G2 needs finite nonnegative position/tensor and 0..2 normal tolerances",
    )?;
    check(
        (1..=100000).contains(&max_cells),
        "G2 needs 1..100000 cells",
    )?;
    let mut cells = vec![cell(a, ea, b, eb, reverse, [0., 1.])?];
    let mut work = 1;
    let mut lower = cells[0].lower;
    let mut position_lower = cells[0].position[0];
    loop {
        let position_upper = cells
            .iter()
            .map(|c| c.position[1])
            .fold(position_lower, f64::max);
        let upper = cells.iter().try_fold([[0_f64; 2]; 2], |mut out, c| {
            let next = c.upper?;
            for sign in 0..2 {
                for k in 0..2 {
                    out[sign][k] = out[sign][k].max(next[sign][k]);
                }
            }
            Some(out)
        });
        let failed = lower.map(|x| x[0] > normal_tolerance || x[1] > tensor_tolerance);
        let orientation = upper
            .and_then(|u| {
                (0..2).find(|&s| u[s][0] <= normal_tolerance && u[s][1] <= tensor_tolerance)
            })
            .map(|s| {
                if s == 0 {
                    Orientation::Same
                } else {
                    Orientation::Opposite
                }
            });
        let decision = if position_lower > position_tolerance || (failed[0] && failed[1]) {
            Some(Decision::CorrespondenceExceedsTolerance)
        } else if position_upper <= position_tolerance && orientation.is_some() {
            Some(Decision::WithinTolerance)
        } else if work + 2 > max_cells {
            Some(Decision::Unresolved)
        } else {
            None
        };
        let report = |decision| Report {
            position_bounds: [position_lower, position_upper],
            branches: std::array::from_fn(|s| BranchBounds {
                normal_lower: lower[s][0],
                tensor_lower: lower[s][1],
                normal_upper: upper.map(|x| x[s][0]),
                tensor_upper: upper.map(|x| x[s][1]),
            }),
            orientation: if decision == Decision::WithinTolerance {
                orientation
            } else {
                None
            },
            regularity_proven: upper.is_some(),
            decision,
            cells: work,
        };
        if let Some(decision) = decision {
            return Ok(report(decision));
        }
        let score = |c: &Cell| {
            c.upper.map_or(f64::INFINITY, |u| {
                let viable = (0..2)
                    .filter(|&s| !failed[s])
                    .map(|s| {
                        (u[s][0] / normal_tolerance.max(1e-12))
                            .max(u[s][1] / tensor_tolerance.max(1e-12))
                    })
                    .fold(f64::INFINITY, f64::min);
                viable.max(c.position[1] / position_tolerance.max(1e-12))
            })
        };
        let index = cells
            .iter()
            .enumerate()
            .max_by(|a, b| {
                score(a.1)
                    .total_cmp(&score(b.1))
                    .then_with(|| (a.1.t[1] - a.1.t[0]).total_cmp(&(b.1.t[1] - b.1.t[0])))
            })
            .unwrap()
            .0;
        let t = cells[index].t;
        let mid = t[0] * 0.5 + t[1] * 0.5;
        if !(t[0] < mid && mid < t[1]) {
            return Ok(report(Decision::Unresolved));
        }
        cells.swap_remove(index);
        for t in [[t[0], mid], [mid, t[1]]] {
            let c = cell(a, ea, b, eb, reverse, t)?;
            position_lower = position_lower.max(c.position[0]);
            for s in 0..2 {
                for k in 0..2 {
                    lower[s][k] = lower[s][k].max(c.lower[s][k]);
                }
            }
            cells.push(c);
            work += 1;
        }
    }
}
