//! Continuous G2 inspection under affine or monotone piecewise affine correspondence.
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
/// A continuous, strictly increasing correspondence of normalized boundary
/// parameters. Each segment is affine; the whole relation may be nonaffine.
/// Validation always evaluates the original surfaces, not an extracted proxy.
fn mapped(nodes: &[[f64; 2]], t: [f64; 2]) -> Result<[f64; 2]> {
    let segment = nodes
        .windows(2)
        .find(|w| t[0] >= w[0][0] && t[1] <= w[1][0])
        .expect("G2 cells are partitioned at correspondence nodes");
    let a = segment[0];
    let b = segment[1];
    // Exact identity has no arithmetic uncertainty. In particular, do not
    // create artificial tiny overlaps at the source's internal knots.
    if a[0] == a[1] && b[0] == b[1] {
        return Ok(t);
    }

    let y = I::new(t[0], t[1])?
        .sub(I::point(a[0]))?
        .div(I::point(b[0]).sub(I::point(a[0]))?)?
        .mul(I::point(b[1]).sub(I::point(a[1]))?)?
        .add(I::point(a[1]))?
        .intersect(a[1], b[1])?;
    Ok([y.lo, y.hi])
}
fn cell(
    a: &Surface,
    ea: Boundary,
    b: &Surface,
    eb: Boundary,
    reverse: bool,
    t: [f64; 2],
    nodes: &[[f64; 2]],
    position_exact: bool,
) -> Result<Cell> {
    let upper = match (
        shape(a, ea, t, false)?,
        shape(b, eb, mapped(nodes, t)?, reverse)?,
    ) {
        (Some(x), Some(y)) => Some(distances(&x, &y)?.map(|branch| branch.map(|x| x[1]))),
        _ => None,
    };
    let mut position = if position_exact {[0.,0.]}else{[
        0.,
        box_distance(
            &boundary_box(a, ea, t, false)?,
            &boundary_box(b, eb, mapped(nodes, t)?, reverse)?,
        )?
        .1,
    ]};
    let mut lower = [[0_f64; 2]; 2];
    for p in [t[0], t[0] * 0.5 + t[1] * 0.5, t[1]] {
        if !position_exact {position[0] = position[0].max(
            box_distance(
                &boundary_box(a, ea, [p, p], false)?,
                &boundary_box(b, eb, mapped(nodes, [p, p])?, reverse)?,
            )?
            .0,
        );}
        if let (Some(x), Some(y)) = (
            shape(a, ea, [p, p], false)?,
            shape(b, eb, mapped(nodes, [p, p])?, reverse)?,
        ) {
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
    inspect_with_correspondence(
        a,
        ea,
        b,
        eb,
        reverse,
        &[[0., 0.], [1., 1.]],
        position_tolerance,
        normal_tolerance,
        tensor_tolerance,
        max_cells,
    )
}
/// Inspect a supplied monotone bijection. Exceeding tolerance disproves this
/// relation only; it does not disprove the existence of another correspondence.
/// Exhausted work or unproved regularity returns `Unresolved`.
pub fn inspect_with_correspondence(
    a: &Surface,
    ea: Boundary,
    b: &Surface,
    eb: Boundary,
    reverse: bool,
    nodes: &[[f64; 2]],
    position_tolerance: f64,
    normal_tolerance: f64,
    tensor_tolerance: f64,
    max_cells: usize,
) -> Result<Report> {
    check(nodes.first()==Some(&[0.,0.])&&nodes.last()==Some(&[1.,1.]),
        "Full G2 correspondence must cover both complete boundaries")?;
    inspect_boundary_ranges(a,ea,b,eb,reverse,nodes,position_tolerance,normal_tolerance,tensor_tolerance,max_cells)
}
/// Inspect only the explicitly declared normalized boundary ranges. Reports
/// cover these ranges, never the omitted parts. Source surfaces are evaluated
/// directly: no rounded trimming or reconstruction substitutes their geometry.
pub fn inspect_boundary_ranges(
    a:&Surface,ea:Boundary,b:&Surface,eb:Boundary,reverse:bool,
    nodes:&[[f64;2]],position_tolerance:f64,normal_tolerance:f64,
    tensor_tolerance:f64,max_cells:usize,
)->Result<Report> {
    check(
        nodes.len() >= 2
            && nodes.len() <= 1025
            && nodes.iter().flatten().all(|x| (0. ..=1.).contains(x))
            && nodes.iter().flatten().all(|x| x.is_finite())
            && nodes
                .windows(2)
                .all(|w| w[0][0] < w[1][0] && w[0][1] < w[1][1]),
        "G2 ranges need 2..1025 strictly ordered nodes within [0,1]",
    )?;
    check(
        nodes.len() - 1 <= max_cells,
        "G2 budget must cover every map segment",
    )?;
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
    let position_exact=!reverse && nodes.iter().all(|node|node[0]==node[1])
        && crate::surface_join::identical_boundary(a,ea,b,eb);
    let mut cells = nodes
        .windows(2)
        .map(|w| cell(a, ea, b, eb, reverse, [w[0][0], w[1][0]], nodes,position_exact))
        .collect::<Result<Vec<_>>>()?;
    let mut work = cells.len();
    let mut lower = [[0_f64; 2]; 2];
    let mut position_lower = 0_f64;
    for c in &cells {
        position_lower = position_lower.max(c.position[0]);
        for s in 0..2 {
            for k in 0..2 {
                lower[s][k] = lower[s][k].max(c.lower[s][k]);
            }
        }
    }
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
            let c = cell(a, ea, b, eb, reverse, t, nodes,position_exact)?;
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

/// Candidate inference and its independently enclosed verification. Search
/// samples are never evidence of G2. A failed candidate says nothing about
/// other possible relations. This open-seam API fixes both endpoint pairs;
/// cyclic phase inference is not provided.
#[derive(Clone, Debug)]
pub struct InferredReport {
    pub nodes: Vec<[f64; 2]>,
    pub verification: Option<Report>,
    /// False when numerical search did not produce a monotone bijection.
    pub candidate_monotone: bool,
}

pub fn inspect_inferred(
    a: &Surface,
    ea: Boundary,
    b: &Surface,
    eb: Boundary,
    reverse: bool,
    segments: usize,
    position_tolerance: f64,
    normal_tolerance: f64,
    tensor_tolerance: f64,
    max_cells: usize,
) -> Result<InferredReport> {
    a.validate()?;
    b.validate()?;
    check(
        (1..=1024).contains(&segments),
        "G2 inference needs 1..1024 segments",
    )?;
    check(
        (1..=100000).contains(&max_cells) && segments <= max_cells,
        "G2 inference budget must cover all segments, at most 100000 cells",
    )?;
    check(
        position_tolerance.is_finite()
            && position_tolerance >= 0.
            && normal_tolerance.is_finite()
            && (0. ..=2.).contains(&normal_tolerance)
            && tensor_tolerance.is_finite()
            && tensor_tolerance >= 0.,
        "G2 inference needs valid tolerances",
    )?;
    let point = |s: &Surface, e, t, rev| -> Result<Vec<f64>> {
        Ok(boundary_box(s, e, [t, t], rev)?
            .into_iter()
            .map(|x| x.lo * 0.5 + x.hi * 0.5)
            .collect())
    };
    let mut nodes = vec![[0., 0.]];
    for i in 1..segments {
        let t = i as f64 / segments as f64;
        let target = point(a, ea, t, false)?;
        let distance = |u| -> Result<f64> {
            Ok(target
                .iter()
                .zip(point(b, eb, u, reverse)?)
                .map(|(x, y)| (x - y) * (x - y))
                .sum())
        };
        let mut best = 0_usize;
        let mut best_distance = f64::INFINITY;
        for j in 0..=128 {
            let d = distance(j as f64 / 128.)?;
            if d < best_distance {
                best = j;
                best_distance = d;
            }
        }
        let mut lo = best.saturating_sub(1) as f64 / 128.;
        let mut hi = (best + 1).min(128) as f64 / 128.;
        for _ in 0..40 {
            let l = lo + (hi - lo) / 3.;
            let r = hi - (hi - lo) / 3.;
            if distance(l)? <= distance(r)? {
                hi = r;
            } else {
                lo = l;
            }
        }
        nodes.push([t, lo * 0.5 + hi * 0.5]);
    }
    nodes.push([1., 1.]);
    let candidate_monotone = nodes.windows(2).all(|w| w[0][1] < w[1][1]);
    let verification = if candidate_monotone {
        Some(inspect_with_correspondence(
            a,
            ea,
            b,
            eb,
            reverse,
            &nodes,
            position_tolerance,
            normal_tolerance,
            tensor_tolerance,
            max_cells,
        )?)
    } else {
        None
    };
    Ok(InferredReport {
        nodes,
        verification,
        candidate_monotone,
    })
}
