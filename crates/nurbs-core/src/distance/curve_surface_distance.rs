//! Global distance bounds between a full 3D curve and an untrimmed surface.
use crate::distance::subdivision::{Queue, ScanQueue, Search, Split};
use crate::distance_bounds::{Interval, box_distance};
use crate::{DistanceStopReason, Result, check, resource};
use crate::{curve::Curve, surface::Surface};
use crate::{
    curve_distance::enclosure,
    surface_distance::{Patch, at, patches},
};

#[derive(Clone, Debug)]
pub struct Report {
    pub distance_interval: [f64; 2],
    pub curve_parameter: f64,
    pub surface_parameters: [f64; 2],
    pub points: [Vec<f64>; 2],
    pub point_enclosures: [Vec<[f64; 2]>; 2],
    pub converged: bool,
    pub reason: DistanceStopReason,
    pub cells: usize,
}
struct Cell {
    span: usize,
    domain: [f64; 2],
    patch: Patch,
}
struct Witness {
    t: f64,
    uv: [f64; 2],
    points: [Vec<f64>; 2],
    bounds: [Vec<Interval>; 2],
    upper: f64,
}
fn mid(d: [f64; 2]) -> f64 {
    d[0] * 0.5 + d[1] * 0.5
}
fn cell(c: &Curve, span: usize, domain: [f64; 2], patch: Patch) -> Result<(Cell, f64)> {
    let b = enclosure(c, span, Interval::new(domain[0], domain[1])?)?;
    let lower = box_distance(&b, &patch.bounds)?.0;
    Ok((Cell { span, domain, patch }, lower))
}
fn consider(c: &Curve, s: &Surface, x: &Cell, best: &mut Option<Witness>) -> Result<()> {
    for t in [x.domain[0], mid(x.domain), x.domain[1]] {
        let b = enclosure(c, x.span, Interval::point(t))?;
        for u in [
            x.patch.domain[0][0],
            mid(x.patch.domain[0]),
            x.patch.domain[0][1],
        ] {
            for v in [
                x.patch.domain[1][0],
                mid(x.patch.domain[1]),
                x.patch.domain[1][1],
            ] {
                let (uv, p, pb) = at(s, [u, v])?;
                let upper = box_distance(&b, &pb)?.1;
                if best.as_ref().is_none_or(|w| upper < w.upper) {
                    let parameter = if c.periodic && t == c.domain()[1] {
                        c.domain()[0]
                    } else {
                        t
                    };
                    *best = Some(Witness {
                        t: parameter,
                        uv,
                        points: [c.evaluate(parameter)?.point, p.to_vec()],
                        bounds: [b.clone(), pb],
                        upper,
                    });
                }
            }
        }
    }
    Ok(())
}
struct CurveSurfaceSearch<'a> {
    c: &'a Curve,
    s: &'a Surface,
    best: Option<Witness>,
}
impl Search for CurveSurfaceSearch<'_> {
    type Cell = Cell;
    fn upper(&self) -> Option<f64> {
        self.best.as_ref().map(|w| w.upper)
    }
    fn split(&mut self, x: Cell) -> Result<Split<Cell>> {
        let ds = [x.domain, x.patch.domain[0], x.patch.domain[1]];
        let original = [
            self.c.knots[x.span + 1] - self.c.knots[x.span],
            self.s.knots_u[x.patch.span[0] + 1] - self.s.knots_u[x.patch.span[0]],
            self.s.knots_v[x.patch.span[1] + 1] - self.s.knots_v[x.patch.span[1]],
        ];
        let axis = (0..3)
            .filter(|&i| ds[i][0] < mid(ds[i]) && mid(ds[i]) < ds[i][1])
            .max_by(|&i, &j| {
                ((ds[i][1] - ds[i][0]) / original[i])
                    .total_cmp(&((ds[j][1] - ds[j][0]) / original[j]))
            });
        let Some(axis) = axis else {
            return Ok(Split::Precision(x));
        };
        let middle = mid(ds[axis]);
        let mut children = [None, None];
        for (i, d) in [[ds[axis][0], middle], [middle, ds[axis][1]]]
            .into_iter()
            .enumerate()
        {
            let (next, lower) = if axis == 0 {
                cell(self.c, x.span, d, x.patch.clone())?
            } else {
                let mut uv = x.patch.domain;
                uv[axis - 1] = d;
                cell(self.c, x.span, x.domain, Patch::new(self.s, x.patch.span, uv)?)?
            };
            consider(self.c, self.s, &next, &mut self.best)?;
            children[i] = Some((next, lower));
        }
        Ok(Split::Children(children))
    }
}
/// Bounds the global minimum, including endpoints and surface boundaries.
/// A witness need not be unique or have certified parameter accuracy.
/// Work/precision stops retain bounds; initial span pairs must fit the budget.
pub fn distance(c: &Curve, s: &Surface, tolerance: f64, max_cells: usize) -> Result<Report> {
    c.validate()?;
    s.validate()?;
    check(
        c.control_points[0].len() == 3,
        "Curve/surface distance requires a 3D curve",
    )?;
    check(
        tolerance.is_finite() && tolerance > 0.,
        "Distance tolerance must be positive and finite",
    )?;
    check(
        (1..=100000).contains(&max_cells),
        "Distance needs 1..100000 cells",
    )?;
    let spans: Vec<_> = (c.degree..c.control_points.len())
        .filter(|&i| c.knots[i] < c.knots[i + 1])
        .collect();
    let ps = patches(s)?;
    if spans.len().saturating_mul(ps.len()) > max_cells {
        return Err(resource(
            "Initial curve/surface span pairs exceed the cell budget",
        ));
    }
    let mut queue = ScanQueue::default();
    let mut best = None;
    for span in spans {
        for p in &ps {
            let (x, lower) = cell(c, span, [c.knots[span], c.knots[span + 1]], p.clone())?;
            consider(c, s, &x, &mut best)?;
            queue.push(x, lower);
        }
    }
    let mut search = CurveSurfaceSearch { c, s, best };
    let work = queue.len();
    let outcome =
        crate::distance::subdivision::run(&mut search, &mut queue, work, max_cells, tolerance)?;
    let best = search.best.unwrap();
    Ok(Report {
        distance_interval: [outcome.lower, best.upper],
        curve_parameter: best.t,
        surface_parameters: best.uv,
        points: best.points,
        point_enclosures: best
            .bounds
            .map(|b| b.into_iter().map(|i| [i.lo, i.hi]).collect()),
        converged: outcome.reason == DistanceStopReason::Tolerance,
        reason: outcome.reason,
        cells: outcome.cells,
    })
}
