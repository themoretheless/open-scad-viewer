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

use super::planar_face_area::plane_metric;
use crate::trim::trim_domain::{Classification, Location, TrimDomain};

#[derive(Clone, Copy, Debug)]
pub struct PlanarFaceDistanceOptions {
    /// Desired distance interval width in world units.
    pub tolerance: f64,
    /// Classification exclusion band in the retained plane coordinates.
    pub winding_tolerance: f64,
    pub max_winding_cells: usize,
    pub max_boundary_cells: usize,
}

#[derive(Clone, Debug)]
pub struct PlanarFaceDistanceReport {
    pub distance: [f64; 2],
    pub converged: bool,
    pub orthogonal_projection: [[f64; 2]; 3],
    pub projection_classification: Classification,
    /// Present when the boundary search supplied the distance upper bound.
    pub boundary: Option<PlanarBoundaryDistanceReport>,
}

/// Enclose distance to the region with nonzero winding together with its
/// authored contour traces, including the contours of holes.
/// A proved interior projection gives the orthogonal plane distance; a
/// proved exterior projection uses the nearest boundary, including holes.
/// When winding is unresolved, plane distance and a boundary witness still
/// give a valid lower and upper bound respectively.
pub fn distance_to_projected_face(
    face: &ProjectedTrimmedFace,
    point: [f64; 3],
    options: PlanarFaceDistanceOptions,
) -> Result<PlanarFaceDistanceReport> {
    validate_projection_axes(&face.projection)?;
    check(
        point.iter().all(|x| x.is_finite())
            && options.tolerance.is_finite()
            && options.tolerance > 0.
            && (1..=100_000).contains(&options.max_boundary_cells),
        "Face distance needs a finite point, positive tolerance and bounded work",
    )?;
    for contour in &face.loops {
        check(
            exact_loop_joins(contour)? == Some(true),
            "Face distance needs exact closed contours",
        )?;
    }
    let domain = TrimDomain::new(&face.loops, options.winding_tolerance)?;
    let (projected, plane_distance) = orthogonal_projection(face, point)?;
    let [u, v] = face.projection.retained_axes;
    let rectangle = [projected[u], projected[v]].map(|r| [r.lo, r.hi]);
    let classification = domain.classify(rectangle, options.max_winding_cells)?;
    let boundary = if classification.location == Location::Inside {
        None
    } else {
        Some(distance_to_projected_face_boundary(
            face,
            point,
            options.tolerance,
            options.max_boundary_cells,
        )?)
    };
    let distance = match &boundary {
        None => [plane_distance.lo, plane_distance.hi],
        Some(boundary) if classification.location == Location::Outside => boundary.distance,
        Some(boundary) => [
            plane_distance.lo.min(boundary.distance[1]),
            boundary.distance[1],
        ],
    };
    Ok(PlanarFaceDistanceReport {
        distance,
        converged: (distance[1] - distance[0]).next_up() <= options.tolerance,
        orthogonal_projection: projected.map(|i| [i.lo, i.hi]),
        projection_classification: classification,
        boundary,
    })
}

fn orthogonal_projection(
    face: &ProjectedTrimmedFace,
    point: [f64; 3],
) -> Result<([Interval; 3], Interval)> {
    let [u, v] = face.projection.retained_axes;
    let z = face.projection.omitted_axis;
    let (scale, [a, b]) = plane_metric(&face.projection.slopes)?;
    let [px, py, pz] = [point[u], point[v], point[z]].map(Interval::point);
    let residual = pz
        .sub(rational_interval(&face.projection.offset)?)?
        .sub(a.mul(px)?)?
        .sub(b.mul(py)?)?;
    let multiple = residual.div(scale.mul(scale)?)?;
    let mut projected = [Interval::point(0.); 3];
    projected[u] = px.add(a.mul(multiple)?)?;
    projected[v] = py.add(b.mul(multiple)?)?;
    projected[z] = pz.sub(multiple)?;
    let abs_lower = if residual.lo <= 0. && residual.hi >= 0. {
        0.
    } else {
        residual.lo.abs().min(residual.hi.abs())
    };
    let distance =
        Interval::new(abs_lower, residual.lo.abs().max(residual.hi.abs()))?.div(scale)?;
    Ok((projected, distance.intersect(0., f64::MAX)?))
}
