//! Certified world-space point-to-boundary distance for planar trim curves.
use super::{
    exact_affine_hull::validate_projection_axes, planar_face_area::rational_interval,
    planar_trim_projection::ProjectedTrimmedFace,
};
use crate::{
    DistanceStopReason, Result, check,
    curve::Curve,
    curve_distance::enclosure,
    distance::subdivision::{self, HeapQueue, Queue, Search, Split},
    distance_bounds::{Interval, box_distance},
    trim::trim_domain::exact_loop_joins,
};

#[derive(Clone, Debug)]
pub struct PlanarBoundaryNearestWitness {
    pub contour: usize,
    pub curve: usize,
    pub parameter: f64,
    pub point_bounds: [[f64; 2]; 3],
    pub distance_upper: f64,
}

#[derive(Clone, Debug)]
pub struct PlanarBoundaryDistanceReport {
    pub distance: [f64; 2],
    /// A certified upper-bound witness, which may vary for tied nearest points.
    pub witness: PlanarBoundaryNearestWitness,
    pub converged: bool,
    pub stop_reason: DistanceStopReason,
    pub cells: usize,
}

/// Enclose the minimum Euclidean distance to all closed boundary contours,
/// including holes. Coordinates of the query are in XYZ. The stored curves
/// are evaluated in the projected chart and lifted by the interval plane
/// equation, so exact binary64 storage of spatial control poles is unnecessary.
pub fn distance_to_projected_face_boundary(
    face: &ProjectedTrimmedFace,
    point: [f64; 3],
    tolerance: f64,
    max_cells: usize,
) -> Result<PlanarBoundaryDistanceReport> {
    validate_projection_axes(&face.projection)?;
    check(
        (1..=256).contains(&face.loops.len())
            && (1..=100_000).contains(&max_cells)
            && point.iter().all(|q| q.is_finite())
            && tolerance.is_finite()
            && tolerance > 0.,
        "Boundary distance needs finite coordinates, positive tolerance and bounded work",
    )?;
    let mut search = BoundarySearch {
        loops: &face.loops,
        point: point.map(Interval::point),
        axes: face.projection.retained_axes,
        omitted: face.projection.omitted_axis,
        offset: rational_interval(&face.projection.offset)?,
        slopes: [
            rational_interval(&face.projection.slopes[0])?,
            rational_interval(&face.projection.slopes[1])?,
        ],
        witness: None,
    };
    let mut queue = HeapQueue::default();
    let mut cells = 0;
    for (contour, curves) in face.loops.iter().enumerate() {
        check(
            exact_loop_joins(curves)? == Some(true),
            "Boundary distance needs exact closed contours",
        )?;
        for (index, curve) in curves.iter().enumerate() {
            for span in curve.degree..curve.control_points.len() {
                let domain = [curve.knots[span], curve.knots[span + 1]];
                if domain[0] == domain[1] {
                    continue;
                }
                check(
                    cells < max_cells,
                    "Boundary distance budget cannot cover all original spans",
                )?;
                let cell = Cell {
                    contour,
                    index,
                    span,
                    domain,
                };
                for t in [domain[0], domain[0].midpoint(domain[1]), domain[1]] {
                    search.sample(cell, t)?;
                }
                queue.push(cell, search.lower(cell)?);
                cells += 1;
            }
        }
    }
    let core = subdivision::run(
        &mut search,
        &mut queue,
        cells,
        max_cells,
        tolerance.next_down(),
    )?;
    let witness = search
        .witness
        .expect("validated contours have original spans");
    let distance = [core.lower, witness.distance_upper];
    Ok(PlanarBoundaryDistanceReport {
        distance,
        witness,
        converged: (distance[1] - distance[0]).next_up() <= tolerance,
        stop_reason: core.reason,
        cells: core.cells,
    })
}

#[derive(Clone, Copy)]
struct Cell {
    contour: usize,
    index: usize,
    span: usize,
    domain: [f64; 2],
}

struct BoundarySearch<'a> {
    loops: &'a [Vec<Curve>],
    point: [Interval; 3],
    axes: [usize; 2],
    omitted: usize,
    offset: Interval,
    slopes: [Interval; 2],
    witness: Option<PlanarBoundaryNearestWitness>,
}

impl BoundarySearch<'_> {
    fn world(&self, cell: Cell, parameter: Interval) -> Result<[Interval; 3]> {
        let uv = enclosure(&self.loops[cell.contour][cell.index], cell.span, parameter)?;
        let mut out = [Interval::point(0.); 3];
        out[self.axes[0]] = uv[0];
        out[self.axes[1]] = uv[1];
        out[self.omitted] = self
            .offset
            .add(self.slopes[0].mul(uv[0])?)?
            .add(self.slopes[1].mul(uv[1])?)?;
        Ok(out)
    }

    fn lower(&self, cell: Cell) -> Result<f64> {
        let world = self.world(cell, Interval::new(cell.domain[0], cell.domain[1])?)?;
        Ok(box_distance(&self.point, &world)?.0)
    }

    fn sample(&mut self, cell: Cell, parameter: f64) -> Result<()> {
        let world = self.world(cell, Interval::point(parameter))?;
        let upper = box_distance(&self.point, &world)?.1;
        if self.upper().is_none_or(|old| upper < old) {
            self.witness = Some(PlanarBoundaryNearestWitness {
                contour: cell.contour,
                curve: cell.index,
                parameter,
                point_bounds: world.map(|x| [x.lo, x.hi]),
                distance_upper: upper,
            });
        }
        Ok(())
    }
}

impl Search for BoundarySearch<'_> {
    type Cell = Cell;

    fn upper(&self) -> Option<f64> {
        self.witness.as_ref().map(|w| w.distance_upper)
    }

    fn split(&mut self, cell: Cell) -> Result<Split<Cell>> {
        let [a, b] = cell.domain;
        let mid = a.midpoint(b);
        if mid <= a || mid >= b {
            return Ok(Split::Precision(cell));
        }
        let mut halves = [None, None];
        for (slot, domain) in halves.iter_mut().zip([[a, mid], [mid, b]]) {
            let child = Cell { domain, ..cell };
            self.sample(child, domain[0].midpoint(domain[1]))?;
            let lower = self.lower(child)?;
            if self.upper().is_none_or(|upper| lower <= upper) {
                *slot = Some((child, lower));
            }
        }
        Ok(Split::Children(halves))
    }
}
