//! Continuous normalized UV error bounds for a surface-plane graph polyline.
use super::{
    PResult, PlaneNet, Q, Refusal, Result, Surface, SurfacePlaneGraph, Work, check,
    samples::{cell_domain, graph_cell, isolate, slice},
    validate,
};
use crate::exact_geometry::exact_projective::UvAxis;
use num_traits::Signed;

#[derive(Clone, Copy, Debug)]
pub struct SurfacePlanePolylineOptions {
    pub span: [usize; 2],
    pub solve_axis: UvAxis,
    /// Absolute error in the normalized solved coordinate, at equal transverse parameter.
    pub max_normalized_error: f64,
    pub max_segments: usize,
    pub max_work: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePlanePolyline {
    pub domain: [[f64; 2]; 2],
    pub graph: SurfacePlaneGraph,
    /// Exact polyline vertices in normalized [U, V] coordinates.
    pub uv: Vec<[Q; 2]>,
    /// Continuous error in the solved coordinate at equal transverse parameter.
    pub normalized_error_upper: Q,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePlanePolylineReport {
    pub outcome: std::result::Result<SurfacePlanePolyline, Refusal>,
    pub work_used: u64,
}

/// Build a rational UV polyline for a certified plane intersection graph.
/// Implicit first and second derivative bounds control the interpolation error
/// between vertices. Each rational root bracket adds a bounded vertex error.
pub fn approximate_surface_plane_graph(
    surface: &Surface,
    normal: [f64; 3],
    offset: f64,
    options: SurfacePlanePolylineOptions,
) -> Result<SurfacePlanePolylineReport> {
    validate(surface, normal, offset, options.max_work)?;
    check(
        options.max_normalized_error.is_finite()
            && options.max_normalized_error > 0.
            && options.max_normalized_error <= 1.
            && (1..=1024).contains(&options.max_segments),
        "Plane graph approximation needs a normalized error in (0, 1] and 1..1024 segments",
    )?;
    let domain = cell_domain(surface, options.span)?;
    let mut work = Work {
        used: 0,
        limit: options.max_work,
    };
    let outcome = (|| {
        let (net, graph) = graph_cell(
            surface,
            normal,
            offset,
            options.span,
            domain,
            options.solve_axis,
            &mut work,
        )?;
        let tolerance = work.lift(options.max_normalized_error)?;
        let (uv, normalized_error_upper) =
            approximate(&net, &graph, &tolerance, options.max_segments, &mut work)?;
        Ok(SurfacePlanePolyline {
            domain,
            graph,
            uv,
            normalized_error_upper,
        })
    })();
    Ok(SurfacePlanePolylineReport {
        outcome,
        work_used: work.used,
    })
}

fn approximate(
    net: &PlaneNet,
    graph: &SurfacePlaneGraph,
    tolerance: &Q,
    max_segments: usize,
    work: &mut Work,
) -> PResult<(Vec<[Q; 2]>, Q)> {
    let solved = usize::from(graph.solve_axis == UvAxis::V);
    let (_, curvature) = graph_derivative_bounds(net, graph, solved, work)?;
    let two = Q::from_integer(2.into());
    let half_tolerance = work.op(tolerance, &two, |a, b| a / b)?;
    let (count, interpolation) = segment_count(&curvature, &half_tolerance, max_segments, work)?;
    let (vertices, radius) = sample_vertices(net, graph, tolerance, count, work)?;
    let total = work.op(&interpolation, &radius, |a, b| a + b)?;
    Ok((vertices, total))
}

pub(super) fn segment_count(
    curvature: &Q,
    limit: &Q,
    max_segments: usize,
    work: &mut Work,
) -> PResult<(usize, Q)> {
    let mut count = 1;
    loop {
        work.charge(1)?;
        let divisor = Q::from_integer((8 * count * count).into());
        let interpolation = work.op(curvature, &divisor, |a, b| a / b)?;
        if interpolation <= *limit {
            return Ok((count, interpolation));
        }
        if count == max_segments {
            return Err(Refusal::ResourceLimit);
        }
        count = (2 * count).min(max_segments);
    }
}

pub(super) fn sample_vertices(
    net: &PlaneNet,
    graph: &SurfacePlaneGraph,
    tolerance: &Q,
    count: usize,
    work: &mut Work,
) -> PResult<(Vec<[Q; 2]>, Q)> {
    let solved = usize::from(graph.solve_axis == UvAxis::V);
    let two = Q::from_integer(2.into());
    let denominator = Q::from_integer(count.into());
    let mut vertices = Vec::with_capacity(count + 1);
    let mut max_radius = Q::default();
    for i in 0..=count {
        let fraction = Q::from_integer(i.into());
        let transverse = work.op(&fraction, &denominator, |a, b| a / b)?;
        let coefficients = slice(net, graph.solve_axis, &transverse, work)?;
        let [lo, hi] = isolate(
            &coefficients,
            tolerance,
            graph.derivative[0].is_positive(),
            work,
        )?;
        let root = work.op(&lo, &hi, |a, b| (a + b) / &two)?;
        let radius = work.op(&hi, &root, |a, b| a - b)?;
        max_radius = max_radius.max(radius);
        vertices.push(if solved == 0 {
            [root, transverse]
        } else {
            [transverse, root]
        });
    }
    Ok((vertices, max_radius))
}

pub(super) fn graph_derivative_bounds(
    net: &PlaneNet,
    graph: &SurfacePlaneGraph,
    s: usize,
    work: &mut Work,
) -> PResult<(Q, Q)> {
    let t = 1 - s;
    let minimum = graph.derivative[0].abs().min(graph.derivative[1].abs());
    let (dt, dt_size) = derivative(&net.numerator, net.size, t, work)?;
    let (ds, ds_size) = derivative(&net.numerator, net.size, s, work)?;
    let dtt = magnitude(&derivative(&dt, dt_size, t, work)?.0);
    let dst = magnitude(&derivative(&ds, ds_size, t, work)?.0);
    let dss = magnitude(&derivative(&ds, ds_size, s, work)?.0);
    let slope = work.op(&magnitude(&dt), &minimum, |a, b| a / b)?;
    let twice_mixed = work.op(&dst, &slope, |a, b| (a * b) * Q::from_integer(2.into()))?;
    let slope_squared = work.op(&slope, &slope, |a, b| a * b)?;
    let solved_term = work.op(&dss, &slope_squared, |a, b| a * b)?;
    let sum = work.op(&dtt, &twice_mixed, |a, b| a + b)?;
    let sum = work.op(&sum, &solved_term, |a, b| a + b)?;
    Ok((slope, work.op(&sum, &minimum, |a, b| a / b)?))
}

pub(super) fn magnitude(net: &[Q]) -> Q {
    net.iter().map(Signed::abs).max().unwrap_or_default()
}

pub(super) fn derivative(
    net: &[Q],
    [nu, nv]: [usize; 2],
    axis: usize,
    work: &mut Work,
) -> PResult<(Vec<Q>, [usize; 2])> {
    let mut size = [nu, nv];
    if size[axis] == 1 {
        return Ok((vec![Q::default()], [1, 1]));
    }
    size[axis] -= 1;
    let degree = Q::from_integer(size[axis].into());
    let stride = if axis == 0 { nv } else { 1 };
    let mut result = Vec::with_capacity(size[0] * size[1]);
    for u in 0..size[0] {
        for v in 0..size[1] {
            let i = u * nv + v;
            let difference = work.op(&net[i + stride], &net[i], |a, b| a - b)?;
            result.push(work.op(&difference, &degree, |a, b| a * b)?);
        }
    }
    Ok((result, size))
}
