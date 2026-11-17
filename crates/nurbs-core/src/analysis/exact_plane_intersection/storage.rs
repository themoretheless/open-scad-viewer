//! Certified binary64 storage of exact planar intersection segments.
use super::{
    ExactPlanarIntersectionSegment, PlanarSurfaceProjection, Refusal, Work,
    validate_projection_axes,
};
use crate::{Result, check, curve::Curve};
use num_rational::BigRational as Q;
use num_traits::{ToPrimitive, Zero};

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
    let outcome = store(segment, projections, tolerance, &mut work)
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
    work: &mut Work,
) -> Checked<PlanarSegmentStorage> {
    let tolerance = work.lift(tolerance)?;
    let limit = work.op(&tolerance, &tolerance, |a, b| a * b)?;
    let mut errors = PlanarSegmentError {
        spatial: Q::zero(),
        lifted_pcurves: [Q::zero(), Q::zero()],
        correspondence: [Q::zero(), Q::zero()],
    };
    let mut coordinates = [[0.; 3]; 2];
    for (end, original) in segment.xyz.iter().enumerate() {
        let mut stored = [Q::zero(), Q::zero(), Q::zero()];
        for (axis, q) in original.iter().enumerate() {
            work.charge(1 + (q.numer().bits() + q.denom().bits()) / 64)?;
            let value = q
                .to_f64()
                .filter(|x| x.is_finite())
                .ok_or(Refusal::NotRepresentable)?;
            coordinates[end][axis] = value;
            stored[axis] = work.lift(value)?;
        }
        raise(
            &mut errors.spatial,
            squared_distance(original, &stored, work)?,
        );
        for (i, projection) in projections.iter().enumerate() {
            let [u, v] = projection.retained_axes;
            let [a, b] = &segment.pcurves[i][end];
            let omitted = projection.omitted_axis;
            if a != &original[u]
                || b != &original[v]
                || lift(projection, a, b, work)? != original[omitted]
            {
                return Err(Refusal::UnprovedIdentity);
            }
            let mut lifted = stored.clone();
            lifted[omitted] = lift(projection, &stored[u], &stored[v], work)?;
            raise(
                &mut errors.lifted_pcurves[i],
                squared_distance(original, &lifted, work)?,
            );
            raise(
                &mut errors.correspondence[i],
                squared_distance(&stored, &lifted, work)?,
            );
        }
    }
    if std::iter::once(&errors.spatial)
        .chain(&errors.lifted_pcurves)
        .chain(&errors.correspondence)
        .any(|error| error > &limit)
    {
        return Ok(PlanarSegmentStorage::ToleranceExceeded(errors));
    }
    let spatial = curve(coordinates)?;
    let [left, right] = projections
        .map(|chart| curve(coordinates.map(|p| chart.retained_axes.map(|axis| p[axis]))));
    Ok(PlanarSegmentStorage::Stored {
        spatial,
        pcurves: [left?, right?],
        squared_error: errors,
    })
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

fn raise(bound: &mut Q, value: Q) {
    if value > *bound {
        *bound = value;
    }
}
