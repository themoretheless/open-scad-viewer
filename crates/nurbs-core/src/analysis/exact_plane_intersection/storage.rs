//! Certified binary64 storage of exact planar intersection segments.
use super::{
    ExactPlanarIntersectionSegment, PlanarSurfaceProjection, Refusal, Work,
    validate_projection_axes,
};
use crate::{Result, check, curve::Curve};
use num_rational::BigRational as Q;
use num_traits::{ToPrimitive, Zero};
use std::cmp::Ordering;

mod ordered;
pub use ordered::{
    StoredPlanarIntersection, StoredPlanarIntersectionEdge, StoredPlanarIntersectionReport,
    store_trimmed_planar_intersection,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanarSegmentError {
    /// Squared Euclidean error bound between the stored XYZ curve and the exact segment.
    pub spatial: Q,
    /// Squared error bounds between each stored chart curve lifted to XYZ and the exact segment.
    pub lifted_pcurves: [Q; 2],
    /// Squared separation bounds between stored XYZ and each lifted chart curve.
    pub correspondence: [Q; 2],
}

#[derive(Clone, Debug)]
pub enum PlanarSegmentStorage {
    Stored {
        spatial: Curve,
        pcurves: [Curve; 2],
        squared_error: PlanarSegmentError,
    },
    ToleranceExceeded(PlanarSegmentError),
    Refused(Refusal),
}

#[derive(Clone, Debug)]
pub struct PlanarSegmentStorageReport {
    pub outcome: PlanarSegmentStorage,
    pub work_used: u64,
}

/// Store the segment and its corresponding Cartesian chart curves in binary64.
/// The same absolute XYZ tolerance applies to the spatial error, both lifted
/// chart errors and both chart-to-spatial separations. A zero tolerance requires
/// exact storage. For an affine segment the squared norm of each error is
/// convex in the shared parameter, so checking its endpoints suffices.
pub fn store_planar_intersection_segment(
    segment: &ExactPlanarIntersectionSegment,
    projections: [&PlanarSurfaceProjection; 2],
    tolerance: f64,
    max_work: u64,
) -> Result<PlanarSegmentStorageReport> {
    store_with(segment, projections, tolerance, max_work, false)
}

/// Minimize the largest squared error over the adjacent binary64 endpoints
/// bracketing each exact coordinate. Candidate pairs preserve coordinate order
/// and keep all three segments nondegenerate. Ties prefer the original nearest
/// rounding. This is a bounded search over at most 64 endpoint pairs.
pub fn optimize_planar_intersection_segment_storage(
    segment: &ExactPlanarIntersectionSegment,
    projections: [&PlanarSurfaceProjection; 2],
    tolerance: f64,
    max_work: u64,
) -> Result<PlanarSegmentStorageReport> {
    store_with(segment, projections, tolerance, max_work, true)
}

fn store_with(
    segment: &ExactPlanarIntersectionSegment,
    projections: [&PlanarSurfaceProjection; 2],
    tolerance: f64,
    max_work: u64,
    optimize: bool,
) -> Result<PlanarSegmentStorageReport> {
    for projection in projections {
        validate_projection_axes(projection)?;
    }
    check(
        tolerance.is_finite()
            && tolerance >= 0.
            && (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Planar segment storage needs a finite nonnegative tolerance and bounded work",
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let outcome = store(segment, projections, tolerance, optimize, &mut work)
        .unwrap_or_else(PlanarSegmentStorage::Refused);
    Ok(PlanarSegmentStorageReport {
        outcome,
        work_used: work.used,
    })
}

type Checked<T> = std::result::Result<T, Refusal>;

fn store(
    segment: &ExactPlanarIntersectionSegment,
    projections: [&PlanarSurfaceProjection; 2],
    tolerance: f64,
    optimize: bool,
    work: &mut Work,
) -> Checked<PlanarSegmentStorage> {
    let tolerance = work.lift(tolerance)?;
    let limit = work.op(&tolerance, &tolerance, |a, b| a * b)?;
    let first = candidates(segment, 0, projections, optimize, work)?;
    let last = candidates(segment, 1, projections, optimize, work)?;
    let [start, end] = if optimize {
        best_pair(segment, projections, &first, &last, work)?
    } else {
        [&first[0], &last[0]]
    };
    let errors = segment_error(start, end);
    if errors.worst() > &limit {
        return Ok(PlanarSegmentStorage::ToleranceExceeded(errors));
    }
    let (spatial, pcurves) = curves([start.coordinates, end.coordinates], projections)?;
    Ok(PlanarSegmentStorage::Stored {
        spatial,
        pcurves,
        squared_error: errors,
    })
}

fn segment_error(start: &Candidate, end: &Candidate) -> PlanarSegmentError {
    PlanarSegmentError {
        spatial: start.error.spatial.clone().max(end.error.spatial.clone()),
        lifted_pcurves: std::array::from_fn(|i| {
            start.error.lifted_pcurves[i]
                .clone()
                .max(end.error.lifted_pcurves[i].clone())
        }),
        correspondence: std::array::from_fn(|i| {
            start.error.correspondence[i]
                .clone()
                .max(end.error.correspondence[i].clone())
        }),
    }
}

fn curves(
    coordinates: [[f64; 3]; 2],
    projections: [&PlanarSurfaceProjection; 2],
) -> Checked<(Curve, [Curve; 2])> {
    let spatial = curve(coordinates)?;
    let [left, right] = projections
        .map(|chart| curve(coordinates.map(|p| chart.retained_axes.map(|axis| p[axis]))));
    Ok((spatial, [left?, right?]))
}

fn curve<const D: usize>(points: [[f64; D]; 2]) -> Checked<Curve> {
    if points[0] == points[1] {
        return Err(Refusal::InvalidOutput);
    }
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: points.into_iter().map(|p| p.to_vec()).collect(),
        weights: vec![1., 1.],
        periodic: false,
    };
    curve.validate().map_err(|_| Refusal::InvalidOutput)?;
    Ok(curve)
}

fn lift(chart: &PlanarSurfaceProjection, u: &Q, v: &Q, work: &mut Work) -> Checked<Q> {
    let x = work.op(&chart.slopes[0], u, |a, b| a * b)?;
    let y = work.op(&chart.slopes[1], v, |a, b| a * b)?;
    let xy = work.op(&x, &y, |a, b| a + b)?;
    Ok(work.op(&xy, &chart.offset, |a, b| a + b)?)
}

fn squared_distance(a: &[Q; 3], b: &[Q; 3], work: &mut Work) -> Checked<Q> {
    let mut sum = Q::zero();
    for (x, y) in a.iter().zip(b) {
        let d = work.op(x, y, |x, y| x - y)?;
        let squared = work.op(&d, &d, |x, y| x * y)?;
        sum = work.op(&sum, &squared, |x, y| x + y)?;
    }
    Ok(sum)
}

impl PlanarSegmentError {
    fn worst(&self) -> &Q {
        std::iter::once(&self.spatial)
            .chain(&self.lifted_pcurves)
            .chain(&self.correspondence)
            .max()
            .expect("spatial error is present")
    }
}

struct Candidate {
    coordinates: [f64; 3],
    error: PlanarSegmentError,
}

fn candidates(
    segment: &ExactPlanarIntersectionSegment,
    end: usize,
    projections: [&PlanarSurfaceProjection; 2],
    optimize: bool,
    work: &mut Work,
) -> Checked<Vec<Candidate>> {
    let original = &segment.xyz[end];
    for (i, projection) in projections.iter().enumerate() {
        let [u, v] = projection.retained_axes;
        let [a, b] = &segment.pcurves[i][end];
        if a != &original[u]
            || b != &original[v]
            || lift(projection, a, b, work)? != original[projection.omitted_axis]
        {
            return Err(Refusal::UnprovedIdentity);
        }
    }
    let x = rounding(&original[0], optimize, work)?;
    let y = rounding(&original[1], optimize, work)?;
    let z = rounding(&original[2], optimize, work)?;
    let mut out = Vec::new();
    for &x in &x {
        for &y in &y {
            for &z in &z {
                let coordinates = [x, y, z];
                if !optimize || coordinates.iter().all(|x| x.abs() <= 1e9) {
                    out.push(Candidate {
                        coordinates,
                        error: endpoint_error(original, coordinates, projections, work)?,
                    });
                }
            }
        }
    }
    Ok(out)
}

fn rounding(q: &Q, optimize: bool, work: &mut Work) -> Checked<Vec<f64>> {
    work.charge(1 + (q.numer().bits() + q.denom().bits()) / 64)?;
    let f = q
        .to_f64()
        .filter(|x| x.is_finite())
        .ok_or(Refusal::NotRepresentable)?;
    let mut candidates = vec![f];
    if optimize {
        let ordering = work.lift(f)?.cmp(q);
        let neighbor = match ordering {
            Ordering::Less => f.next_up(),
            Ordering::Greater => f.next_down(),
            Ordering::Equal => return Ok(candidates),
        };
        if work.lift(neighbor)?.cmp(q) == ordering {
            return Err(Refusal::NotRepresentable);
        }
        candidates.push(neighbor);
    }
    Ok(candidates)
}

fn endpoint_error(
    original: &[Q; 3],
    [x, y, z]: [f64; 3],
    projections: [&PlanarSurfaceProjection; 2],
    work: &mut Work,
) -> Checked<PlanarSegmentError> {
    let stored = [work.lift(x)?, work.lift(y)?, work.lift(z)?];
    let mut error = PlanarSegmentError {
        spatial: squared_distance(original, &stored, work)?,
        lifted_pcurves: [Q::zero(), Q::zero()],
        correspondence: [Q::zero(), Q::zero()],
    };
    for (i, projection) in projections.iter().enumerate() {
        let [u, v] = projection.retained_axes;
        let mut lifted = stored.clone();
        lifted[projection.omitted_axis] = lift(projection, &stored[u], &stored[v], work)?;
        error.lifted_pcurves[i] = squared_distance(original, &lifted, work)?;
        error.correspondence[i] = squared_distance(&stored, &lifted, work)?;
    }
    Ok(error)
}

fn best_pair<'a>(
    segment: &ExactPlanarIntersectionSegment,
    projections: [&PlanarSurfaceProjection; 2],
    first: &'a [Candidate],
    last: &'a [Candidate],
    work: &mut Work,
) -> Checked<[&'a Candidate; 2]> {
    let ordering: [_; 3] = std::array::from_fn(|i| segment.xyz[0][i].cmp(&segment.xyz[1][i]));
    let mut best = None;
    for start in first {
        for end in last {
            work.charge(1)?;
            if !preserves_order(start, end, ordering, projections, None) {
                continue;
            }
            let score = start.error.worst().max(end.error.worst());
            if best.is_none_or(|(_, bound)| score < bound) {
                best = Some(([start, end], score));
            }
        }
    }
    best.map(|(pair, _)| pair).ok_or(Refusal::InvalidOutput)
}

fn preserves_order(
    start: &Candidate,
    end: &Candidate,
    ordering: [Ordering; 3],
    projections: [&PlanarSurfaceProjection; 2],
    strict_axis: Option<usize>,
) -> bool {
    let direction: [_; 3] = std::array::from_fn(|i| {
        start.coordinates[i]
            .partial_cmp(&end.coordinates[i])
            .expect("finite coordinates")
    });
    !direction
        .iter()
        .zip(ordering)
        .any(|(&actual, expected)| actual != Ordering::Equal && actual != expected)
        && projections.iter().all(|p| {
            p.retained_axes
                .iter()
                .any(|&i| direction[i] != Ordering::Equal)
        })
        && strict_axis.is_none_or(|i| direction[i] == Ordering::Less)
}
