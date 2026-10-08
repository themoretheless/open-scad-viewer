//! Boundary-position bounds under an explicit affine parameter correspondence.
use crate::distance_bounds::{Interval as I, box_distance};
use crate::{Result, check, surface::Surface, surface_distance::rectangle_bounds};
#[derive(Clone, Copy, Debug)]
pub enum Boundary {
    UMin,
    UMax,
    VMin,
    VMax,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    WithinTolerance,
    CorrespondenceExceedsTolerance,
    Unresolved,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub bounds: [f64; 2],
    pub decision: Decision,
    pub cells: usize,
}
fn domain(s: &Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}
pub(crate) fn boundary_box(s: &Surface, edge: Boundary, t: [f64; 2], reverse: bool) -> Result<Vec<I>> {
    let d = domain(s);
    let (fixed, max) = match edge {
        Boundary::UMin => (0, false),
        Boundary::UMax => (0, true),
        Boundary::VMin => (1, false),
        Boundary::VMax => (1, true),
    };
    let free = 1 - fixed;
    let t = I::new(t[0], t[1])?;
    let t = if reverse {
        I::point(1.).sub(t)?.intersect(0., 1.)?
    } else {
        t
    };
    let x = I::point(d[free][0])
        .add(I::point(d[free][1]).sub(I::point(d[free][0]))?.mul(t)?)?
        .intersect(d[free][0], d[free][1])?;
    let mut rectangle = d;
    rectangle[free] = [x.lo, x.hi];
    let endpoint = d[fixed][usize::from(max)];
    rectangle[fixed] = [endpoint, endpoint];
    rectangle_bounds(s, rectangle)?
        .into_iter()
        .map(|b| I::new(b[0], b[1]))
        .collect()
}
struct Cell {
    t: [f64; 2],
    bounds: [f64; 2],
}
fn cell(
    a: &Surface,
    ea: Boundary,
    b: &Surface,
    eb: Boundary,
    reverse: bool,
    t: [f64; 2],
) -> Result<Cell> {
    let upper = box_distance(
        &boundary_box(a, ea, t, false)?,
        &boundary_box(b, eb, t, reverse)?,
    )?
    .1;
    let mut lower = 0_f64;
    for u in [t[0], t[0] * 0.5 + t[1] * 0.5, t[1]] {
        lower = lower.max(
            box_distance(
                &boundary_box(a, ea, [u, u], false)?,
                &boundary_box(b, eb, [u, u], reverse)?,
            )?
            .0,
        );
    }
    Ok(Cell {
        t,
        bounds: [lower, upper],
    })
}
/// A sufficient G0 test for two chosen boundaries with normalized affine
/// parameter correspondence. Rejection concerns that correspondence, not every
/// possible reparameterization of geometrically coincident boundary images.
pub fn inspect_g0(
    a: &Surface,
    ea: Boundary,
    b: &Surface,
    eb: Boundary,
    reverse: bool,
    tolerance: f64,
    max_cells: usize,
) -> Result<Report> {
    a.validate()?;
    b.validate()?;
    check(
        tolerance.is_finite() && tolerance >= 0.,
        "G0 tolerance must be finite and nonnegative",
    )?;
    check(
        (1..=100000).contains(&max_cells),
        "G0 needs 1..100000 cells",
    )?;
    let mut cells = vec![cell(a, ea, b, eb, reverse, [0., 1.])?];
    let mut work = 1;
    let mut lower = cells[0].bounds[0];
    let decision;
    loop {
        let upper = cells.iter().map(|x| x.bounds[1]).fold(lower, f64::max);
        if upper <= tolerance {
            decision = Decision::WithinTolerance;
            break;
        }
        if lower > tolerance {
            decision = Decision::CorrespondenceExceedsTolerance;
            break;
        }
        if work + 2 > max_cells {
            decision = Decision::Unresolved;
            break;
        }
        let index = cells
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.bounds[1].total_cmp(&b.1.bounds[1]))
            .unwrap()
            .0;
        let t = cells[index].t;
        let mid = t[0] * 0.5 + t[1] * 0.5;
        if !(t[0] < mid && mid < t[1]) {
            decision = Decision::Unresolved;
            break;
        }
        cells.swap_remove(index);
        for next in [[t[0], mid], [mid, t[1]]] {
            let x = cell(a, ea, b, eb, reverse, next)?;
            lower = lower.max(x.bounds[0]);
            cells.push(x);
            work += 1;
        }
    }
    Ok(Report {
        bounds: [
            lower,
            cells.iter().map(|x| x.bounds[1]).fold(lower, f64::max),
        ],
        decision,
        cells: work,
    })
}
