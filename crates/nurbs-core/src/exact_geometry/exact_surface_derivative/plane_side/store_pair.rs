//! Synchronized section curves and trim curves with continuous gap certification.
use super::{
    PResult, Q, Refusal, Result, StoredSurfacePlaneCurve, Surface,
    SurfacePlaneWorldPolylineOptions, Work, check, samples::cell_domain, store_curve::store_world,
    validate, world_polyline::build,
};
use crate::{
    curve::Curve,
    exact_geometry::{
        exact_derivative::Refusal as TraceRefusal,
        exact_projective::pcurve_gap::certify_pcurve_edge_gap,
    },
};
use num_traits::{Signed, ToPrimitive, Zero};

#[derive(Clone, Debug)]
pub struct StoredSurfacePlaneSectionPair {
    pub spatial: StoredSurfacePlaneCurve,
    /// Trace in the original surface's absolute UV coordinates.
    pub pcurve: Curve,
    /// Certified Euclidean gap threshold at the shared absolute parameter.
    pub correspondence_tolerance: Q,
}

#[derive(Clone, Debug)]
pub struct StoredSurfacePlaneSectionPairReport {
    pub outcome: std::result::Result<StoredSurfacePlaneSectionPair, Refusal>,
    pub work_used: u64,
}

/// Store a section with a matching pcurve. Both curves use the same integer
/// knots, and the lifted UV trace is tested against the rounded spatial curve
/// on every knot interval with an exact squared-distance polynomial.
pub fn store_surface_plane_section_pair(
    surface: &Surface,
    normal: [f64; 3],
    offset: f64,
    options: SurfacePlaneWorldPolylineOptions,
    correspondence_tolerance: f64,
) -> Result<StoredSurfacePlaneSectionPairReport> {
    let domain = validate_pair(surface, normal, offset, options, correspondence_tolerance)?;
    let mut work = Work {
        used: 0,
        limit: options.max_work,
    };
    let outcome = store_section(
        surface,
        normal,
        offset,
        options,
        correspondence_tolerance,
        domain,
        &mut work,
    )
    .map(|(pair, _)| pair);
    Ok(StoredSurfacePlaneSectionPairReport {
        outcome,
        work_used: work.used,
    })
}

pub(super) fn validate_pair(
    surface: &Surface,
    normal: [f64; 3],
    offset: f64,
    options: SurfacePlaneWorldPolylineOptions,
    correspondence_tolerance: f64,
) -> Result<[[f64; 2]; 2]> {
    validate(surface, normal, offset, options.max_work)?;
    check(
        options.max_xyz_error.is_finite()
            && options.max_xyz_error > 0.
            && correspondence_tolerance.is_finite()
            && correspondence_tolerance >= 0.
            && (1..=255).contains(&options.max_segments),
        "Section pairing needs finite tolerances and 1..255 segments",
    )?;
    cell_domain(surface, options.span)
}

pub(super) fn store_section(
    surface: &Surface,
    normal: [f64; 3],
    offset: f64,
    options: SurfacePlaneWorldPolylineOptions,
    correspondence_tolerance: f64,
    domain: [[f64; 2]; 2],
    work: &mut Work,
) -> PResult<(StoredSurfacePlaneSectionPair, bool)> {
    let tolerance = work.lift(options.max_xyz_error)?;
    let correspondence_tolerance = work.lift(correspondence_tolerance)?;
    let tracing_budget = if correspondence_tolerance.is_zero() {
        &tolerance
    } else {
        (&tolerance).min(&correspondence_tolerance)
    };
    let half = work.op(tracing_budget, &Q::from_integer(2.into()), |a, b| a / b)?;
    let world = build(surface, normal, offset, options, domain, &half, work)?;
    let spatial = store_world(&world, normal, offset, &tolerance, work)?;
    let pcurve = store_uv(&world.uv, domain, work)?;
    if !certify_pcurve_edge_gap(
        surface,
        &pcurve,
        &spatial.curve,
        &correspondence_tolerance,
        work,
    )
    .map_err(trace_refusal)?
    {
        return Err(Refusal::UnprovedIdentity);
    }
    Ok((
        StoredSurfacePlaneSectionPair {
            spatial,
            pcurve,
            correspondence_tolerance,
        },
        world.graph.derivative[0].is_positive(),
    ))
}

fn store_uv(uv: &[[Q; 2]], domain: [[f64; 2]; 2], work: &mut Work) -> PResult<Curve> {
    let mut origins = [Q::zero(), Q::zero()];
    let mut widths = [Q::zero(), Q::zero()];
    for i in 0..2 {
        origins[i] = work.lift(domain[i][0])?;
        let end = work.lift(domain[i][1])?;
        widths[i] = work.op(&end, &origins[i], |a, b| a - b)?;
    }
    let mut points = Vec::with_capacity(uv.len());
    for vertex in uv {
        let mut absolute = [Q::zero(), Q::zero()];
        for i in 0..2 {
            let delta = work.op(&widths[i], &vertex[i], |a, b| a * b)?;
            absolute[i] = work.op(&origins[i], &delta, |a, b| a + b)?;
        }
        points.push(absolute);
    }
    store_absolute_uv(&points, domain, work)
}

pub(super) fn store_absolute_uv(
    uv: &[[Q; 2]],
    domain: [[f64; 2]; 2],
    work: &mut Work,
) -> PResult<Curve> {
    let mut points = Vec::with_capacity(uv.len());
    for vertex in uv {
        let mut point = Vec::with_capacity(2);
        for (i, absolute) in vertex.iter().enumerate() {
            work.charge(1 + (absolute.numer().bits() + absolute.denom().bits()) / 64)?;
            let stored = absolute
                .to_f64()
                .filter(|p| p.is_finite())
                .ok_or(Refusal::NotRepresentable)?;
            if stored < domain[i][0] || stored > domain[i][1] {
                return Err(Refusal::NotRepresentable);
            }
            point.push(stored);
        }
        if points.last() == Some(&point) {
            return Err(Refusal::NotRepresentable);
        }
        points.push(point);
    }
    Curve::from_polyline(points).map_err(|_| Refusal::InvalidOutput)
}

pub(super) fn trace_refusal(reason: TraceRefusal) -> Refusal {
    match reason {
        TraceRefusal::ResourceLimit => Refusal::ResourceLimit,
        TraceRefusal::NotRepresentable => Refusal::NotRepresentable,
        TraceRefusal::OutputDegreeLimit => Refusal::OutputDegreeLimit,
        TraceRefusal::UnprovedDenominator => Refusal::UnprovedDenominator,
        TraceRefusal::InvalidOutput => Refusal::InvalidOutput,
        TraceRefusal::UnprovedIdentity => Refusal::UnprovedIdentity,
    }
}
