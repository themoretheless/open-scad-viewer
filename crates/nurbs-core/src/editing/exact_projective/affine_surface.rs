//! Independent exact affine clocks on a rational tensor surface.
use super::{
    PResult, Refusal, Report, Surface, Work,
    affine_curve::{map_knots, validate_endpoints},
    finish,
    mobius::validate_clock,
};
use crate::Result;

/// Map the original start and end on each tensor axis to the requested
/// endpoint pair. Descending endpoint pairs reverse their axis. The
/// original control net and weights are permuted to match, preserving
/// the complete rational surface with the affine UV correspondence.
/// Knots outside the active domain follow the same exact map.
pub fn affine_surface_clock(
    source: &Surface,
    endpoints: [[f64; 2]; 2],
    max_work: u64,
) -> Result<Report<Surface>> {
    source.validate()?;
    for axis in endpoints {
        validate_endpoints(axis)?;
    }
    validate_clock(&[], max_work)?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let out = transform(source, endpoints, &mut work);
    Ok(finish(out, work))
}

fn transform(source: &Surface, [u, v]: [[f64; 2]; 2], work: &mut Work) -> PResult<Surface> {
    let nu = source.control_points.len();
    let nv = source.control_points[0].len();
    let knots_u = map_knots(
        &source.knots_u,
        [source.knots_u[source.degree_u], source.knots_u[nu]],
        u,
        work,
    )?;
    let knots_v = map_knots(
        &source.knots_v,
        [source.knots_v[source.degree_v], source.knots_v[nv]],
        v,
        work,
    )?;
    let mut out = source.clone();
    out.knots_u = knots_u;
    out.knots_v = knots_v;
    if u[0] > u[1] {
        out.control_points.reverse();
        out.weights.reverse();
    }
    if v[0] > v[1] {
        for (row, weights) in out.control_points.iter_mut().zip(&mut out.weights) {
            row.reverse();
            weights.reverse();
        }
    }
    out.validate().map_err(|_| Refusal::InvalidOutput)?;
    Ok(out)
}
