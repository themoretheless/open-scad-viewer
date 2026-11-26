//! Certified point distance to an original curvature threshold contour.
use super::{
    curvature_gap_graph::CurvatureGapGraph, curvature_gap_range::outward,
    curvature_gap_world::certify_curvature_gap_world_polyline, *,
};
use crate::distance_bounds::Interval as I;
use num_traits::Signed;

#[derive(Clone, Debug)]
pub struct CurvatureGapDistance {
    pub graph: Option<CurvatureGapGraph>,
    /// Encloses the minimum Euclidean distance to the entire contour.
    pub distance_bounds: Option<[f64; 2]>,
    /// A segment whose corresponding contour interval supplies the upper bound.
    pub upper_segment: Option<usize>,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}

/// Enclose the distance to a certified curvature threshold contour. For
/// each stored chord, exact clamped projection gives a distance interval.
/// The paired contour error expands that interval before taking minima.
pub fn distance_to_curvature_gap_contour(
    source: &Surface,
    span: [usize; 2],
    axis: Axis,
    level: f64,
    point: [f64; 3],
    transverse: &[f64],
    max_depth: u8,
    max_work: u64,
) -> Result<CurvatureGapDistance> {
    check(
        point.iter().all(|x| x.is_finite()),
        "Contour query point must be finite",
    )?;
    let world = certify_curvature_gap_world_polyline(
        source, span, axis, level, transverse, max_depth, max_work,
    )?;
    let mut report = CurvatureGapDistance {
        graph: world.uv.graph,
        distance_bounds: None,
        upper_segment: None,
        work_used: world.uv.work_used,
        refusal: world.uv.refusal,
    };
    let (Some(vertices), Some(errors)) = (world.points, world.world_error_upper) else {
        return Ok(report);
    };
    let mut work = Work {
        used: report.work_used,
        limit: max_work,
    };
    let result = bounds(point, &vertices, &errors, &mut work);
    report.work_used = work.used;
    match result {
        Ok(Some((bounds, segment))) => {
            report.distance_bounds = Some(bounds);
            report.upper_segment = Some(segment);
        }
        Ok(None) => {}
        Err(reason) => {
            report.graph = None;
            report.refusal = Some(reason);
        }
    }
    Ok(report)
}

fn bounds(
    point: [f64; 3],
    vertices: &[[f64; 3]],
    errors: &[f64],
    work: &mut Work,
) -> PResult<Option<([f64; 2], usize)>> {
    let point = lift(point, work)?;
    let mut start = lift(vertices[0], work)?;
    let mut best = [f64::INFINITY; 2];
    let mut upper_segment = 0;
    for (i, (&vertex, &error)) in vertices[1..].iter().zip(errors).enumerate() {
        let end = lift(vertex, work)?;
        let squared = segment_distance_squared(&point, &start, &end, work)?;
        start = end;
        let Some([lower, upper]) = squared_distance_range(&squared, error, work)? else {
            return Ok(None);
        };
        best[0] = best[0].min(lower);
        if upper < best[1] {
            best[1] = upper;
            upper_segment = i;
        }
    }
    Ok(Some((best, upper_segment)))
}

pub(super) fn squared_distance_range(
    squared: &Q,
    error: f64,
    work: &mut Work,
) -> PResult<Option<[f64; 2]>> {
    let Some(lo) = outward(squared, false, work)? else {
        return Ok(None);
    };
    let Some(hi) = outward(squared, true, work)? else {
        return Ok(None);
    };
    let radius = I::point(error);
    let lower = I::point(lo.sqrt().next_down().max(0.))
        .sub(radius)
        .map_err(|_| Refusal::InvalidOutput)?
        .lo
        .max(0.);
    let upper = I::point(hi.sqrt().next_up())
        .add(radius)
        .map_err(|_| Refusal::InvalidOutput)?
        .hi;
    Ok(Some([lower, upper]))
}

pub(super) fn lift(values: [f64; 3], work: &mut Work) -> PResult<[Q; 3]> {
    Ok([
        work.lift(values[0])?,
        work.lift(values[1])?,
        work.lift(values[2])?,
    ])
}

pub(super) fn difference(a: &[Q; 3], b: &[Q; 3], work: &mut Work) -> PResult<[Q; 3]> {
    let mut out = [Q::zero(), Q::zero(), Q::zero()];
    for i in 0..3 {
        out[i] = work.op(&a[i], &b[i], |x, y| x - y)?;
    }
    Ok(out)
}

pub(super) fn dot(a: &[Q; 3], b: &[Q; 3], work: &mut Work) -> PResult<Q> {
    let mut sum = Q::zero();
    for i in 0..3 {
        let term = work.op(&a[i], &b[i], |x, y| x * y)?;
        sum = work.op(&sum, &term, |x, y| x + y)?;
    }
    Ok(sum)
}

pub(super) fn segment_distance_squared(
    p: &[Q; 3],
    a: &[Q; 3],
    b: &[Q; 3],
    work: &mut Work,
) -> PResult<Q> {
    let mut offset = difference(p, a, work)?;
    let direction = difference(b, a, work)?;
    let length = dot(&direction, &direction, work)?;
    if length.is_positive() {
        let along = dot(&offset, &direction, work)?;
        let fraction = work
            .op(&along, &length, |a, b| a / b)?
            .clamp(Q::zero(), Q::one());
        for (value, step) in offset.iter_mut().zip(direction) {
            let position = work.op(&fraction, &step, |a, b| a * b)?;
            *value = work.op(value, &position, |a, b| a - b)?;
        }
    }
    dot(&offset, &offset, work)
}
