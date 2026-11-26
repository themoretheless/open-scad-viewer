//! Spatial chord error certificates for surface-plane graphs.
use super::{
    PResult, Q, Refusal, Result, Surface, SurfacePlaneGraph, Work, check,
    graph::graph,
    homogeneous_cell, plane_net,
    polyline::{derivative, graph_derivative_bounds, magnitude, sample_vertices, segment_count},
    samples::{cell_domain, evaluate},
    storage, validate,
};
use crate::exact_geometry::exact_projective::UvAxis;
use num_traits::{One, Signed, Zero};

#[derive(Clone, Copy, Debug)]
pub struct SurfacePlaneWorldPolylineOptions {
    pub span: [usize; 2],
    pub solve_axis: UvAxis,
    pub max_xyz_error: f64,
    pub max_segments: usize,
    pub max_work: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePlaneWorldPolyline {
    pub domain: [[f64; 2]; 2],
    pub graph: SurfacePlaneGraph,
    pub uv: Vec<[Q; 2]>,
    /// Rational evaluations of the original surface at each UV vertex.
    pub xyz: Vec<[Q; 3]>,
    /// Continuous Euclidean error bound at equal transverse parameter.
    pub xyz_error_upper: Q,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePlaneWorldPolylineReport {
    pub outcome: std::result::Result<SurfacePlaneWorldPolyline, Refusal>,
    pub work_used: u64,
}

/// Trace a certified surface-plane graph as an XYZ polyline. Rational
/// quotient derivative bounds control world chord error and vertex root
/// uncertainty. The Euclidean error is bounded by the sum of coordinate errors.
pub fn approximate_surface_plane_graph_xyz(
    surface: &Surface,
    normal: [f64; 3],
    offset: f64,
    options: SurfacePlaneWorldPolylineOptions,
) -> Result<SurfacePlaneWorldPolylineReport> {
    validate(surface, normal, offset, options.max_work)?;
    check(
        options.max_xyz_error.is_finite()
            && options.max_xyz_error > 0.
            && (1..=1024).contains(&options.max_segments),
        "World graph approximation needs a positive XYZ tolerance and 1..1024 segments",
    )?;
    let domain = cell_domain(surface, options.span)?;
    let mut work = Work {
        used: 0,
        limit: options.max_work,
    };
    let outcome = (|| {
        let tolerance = work.lift(options.max_xyz_error)?;
        build(
            surface, normal, offset, options, domain, &tolerance, &mut work,
        )
    })();
    Ok(SurfacePlaneWorldPolylineReport {
        outcome,
        work_used: work.used,
    })
}

pub(super) fn build(
    surface: &Surface,
    [x, y, z]: [f64; 3],
    offset: f64,
    options: SurfacePlaneWorldPolylineOptions,
    domain: [[f64; 2]; 2],
    tolerance: &Q,
    work: &mut Work,
) -> PResult<SurfacePlaneWorldPolyline> {
    let plane = [
        work.lift(x)?,
        work.lift(y)?,
        work.lift(z)?,
        -work.lift(offset)?,
    ];
    let homogeneous = homogeneous_cell(surface, options.span, domain, work)?;
    let net = plane_net(surface, &homogeneous, &plane, work)?;
    let graph = graph(&net, options.solve_axis, work)?.ok_or(Refusal::UnprovedIdentity)?;
    let s = usize::from(options.solve_axis == UvAxis::V);
    let (slope, bend) = graph_derivative_bounds(&net, &graph, s, work)?;
    let xyz_nets = homogeneous[..3]
        .iter()
        .map(|channel| storage::bernstein(channel, net.size[0] - 1, net.size[1] - 1, work))
        .collect::<PResult<Vec<_>>>()?;
    let weight = jet(&net.denominator, net.size, s, work)?;
    let min_weight = net
        .denominator
        .iter()
        .min()
        .filter(|w| w.is_positive())
        .ok_or(Refusal::UnprovedDenominator)?;
    let mut sensitivity = Q::zero();
    let mut curvature = Q::zero();
    for channel in &xyz_nets {
        let [spatial_slope, spatial_bend] = world_bounds(
            channel,
            &net.denominator,
            net.size,
            &weight,
            min_weight,
            s,
            &slope,
            &bend,
            work,
        )?;
        sensitivity = work.op(&sensitivity, &spatial_slope, |a, b| a + b)?;
        curvature = work.op(&curvature, &spatial_bend, |a, b| a + b)?;
    }
    let half = work.op(tolerance, &Q::from_integer(2.into()), |a, b| a / b)?;
    let (count, chord_error) = segment_count(&curvature, &half, options.max_segments, work)?;
    let mut width = Q::one();
    if !sensitivity.is_zero() {
        width = width.min(work.op(tolerance, &sensitivity, |a, b| a / b)?);
    }
    let (uv, radius) = sample_vertices(&net, &graph, &width, count, work)?;
    let vertex_error = work.op(&sensitivity, &radius, |a, b| a * b)?;
    let xyz_error_upper = work.op(&chord_error, &vertex_error, |a, b| a + b)?;
    let xyz = uv
        .iter()
        .map(|uv| {
            let weight = value(&net.denominator, net.size[1], uv, work)?;
            let mut coordinates = [Q::zero(), Q::zero(), Q::zero()];
            for (coordinate, channel) in coordinates.iter_mut().zip(&xyz_nets) {
                let homogeneous = value(channel, net.size[1], uv, work)?;
                *coordinate = work.op(&homogeneous, &weight, |a, b| a / b)?;
            }
            Ok(coordinates)
        })
        .collect::<PResult<Vec<_>>>()?;
    Ok(SurfacePlaneWorldPolyline {
        domain,
        graph,
        uv,
        xyz,
        xyz_error_upper,
    })
}

/// Partial bounds ordered as s, t, ss, st, tt.
fn jet(net: &[Q], size: [usize; 2], s: usize, work: &mut Work) -> PResult<[Q; 5]> {
    let (ds, ds_size) = derivative(net, size, s, work)?;
    let (dt, dt_size) = derivative(net, size, 1 - s, work)?;
    Ok([
        magnitude(&ds),
        magnitude(&dt),
        magnitude(&derivative(&ds, ds_size, s, work)?.0),
        magnitude(&derivative(&ds, ds_size, 1 - s, work)?.0),
        magnitude(&derivative(&dt, dt_size, 1 - s, work)?.0),
    ])
}

fn world_bounds(
    channel: &[Q],
    denominator: &[Q],
    size: [usize; 2],
    w: &[Q; 5],
    min_weight: &Q,
    s: usize,
    slope: &Q,
    bend: &Q,
    work: &mut Work,
) -> PResult<[Q; 2]> {
    let mut bound = Q::zero();
    for (n, d) in channel.iter().zip(denominator) {
        bound = bound.max(work.op(n, d, |a, b| (a / b).abs())?);
    }
    let p = jet(channel, size, s, work)?;
    let first = |i: usize, work: &mut Work| -> PResult<Q> {
        let term = work.op(&bound, &w[i], |a, b| a * b)?;
        let sum = work.op(&p[i], &term, |a, b| a + b)?;
        Ok(work.op(&sum, min_weight, |a, b| a / b)?)
    };
    let d = [first(0, work)?, first(1, work)?];
    let second = |k: usize, a: usize, b: usize, work: &mut Work| -> PResult<Q> {
        let t0 = work.op(&bound, &w[k], |a, b| a * b)?;
        let t1 = work.op(&d[a], &w[b], |a, b| a * b)?;
        let t2 = work.op(&d[b], &w[a], |a, b| a * b)?;
        let sum = work.op(&p[k], &t0, |a, b| a + b)?;
        let sum = work.op(&sum, &t1, |a, b| a + b)?;
        let sum = work.op(&sum, &t2, |a, b| a + b)?;
        Ok(work.op(&sum, min_weight, |a, b| a / b)?)
    };
    let [ss, st, tt] = [
        second(2, 0, 0, work)?,
        second(3, 0, 1, work)?,
        second(4, 1, 1, work)?,
    ];
    let mixed = work.op(&st, slope, |a, b| a * b * Q::from_integer(2.into()))?;
    let slope2 = work.op(slope, slope, |a, b| a * b)?;
    let ss = work.op(&ss, &slope2, |a, b| a * b)?;
    let solved = work.op(&d[0], bend, |a, b| a * b)?;
    let sum = work.op(&tt, &mixed, |a, b| a + b)?;
    let sum = work.op(&sum, &ss, |a, b| a + b)?;
    Ok([d[0].clone(), work.op(&sum, &solved, |a, b| a + b)?])
}

fn value(net: &[Q], nv: usize, [u, v]: &[Q; 2], work: &mut Work) -> PResult<Q> {
    let column = net
        .chunks_exact(nv)
        .map(|row| evaluate(row, v, work))
        .collect::<PResult<Vec<_>>>()?;
    evaluate(&column, u, work)
}
