//! Close a certified spatial section across the exact periodic surface seam.
use super::{
    PResult, Poly, Q, Refusal, Result, Surface, SurfacePlaneGraphStripCell,
    SurfacePlaneWorldStripOptions, SurfacePlaneWorldStripReport, Work, check, homogeneous_cell,
    stitch_surface_plane_graph_strip,
    world_strip::{approximate, distance_l1},
};
use crate::exact_geometry::exact_projective::UvAxis;
use num_traits::Zero;

/// Trace one complete periodic strip and close it in XYZ. Both original UV
/// seam endpoints remain in the output, with identical spatial endpoints.
/// Exact equality of the four homogeneous seam traces certifies the wrap.
pub fn close_periodic_surface_plane_strip_xyz(
    surface: &Surface,
    normal: [f64; 3],
    offset: f64,
    options: SurfacePlaneWorldStripOptions,
) -> Result<SurfacePlaneWorldStripReport> {
    surface.validate()?;
    let solved = usize::from(options.strip.solve_axis == UvAxis::V);
    let periodic = [surface.periodic_u, surface.periodic_v];
    let span_range = [
        [surface.degree_u, surface.control_points.len() - 1],
        [surface.degree_v, surface.control_points[0].len() - 1],
    ];
    check(
        periodic[1 - solved]
            && options.strip.transverse_spans == span_range[1 - solved]
            && options.max_xyz_error.is_finite()
            && options.max_xyz_error > 0.
            && (1..=1024).contains(&options.max_segments_per_cell),
        "Closure needs one complete transverse surface period, a positive XYZ tolerance and bounded samples",
    )?;
    let stitched = stitch_surface_plane_graph_strip(surface, normal, offset, options.strip)?;
    let mut work = Work {
        used: stitched.work_used,
        limit: options.strip.max_work,
    };
    let outcome = stitched.outcome.and_then(|strip| {
        let first = strip.cells.first().ok_or(Refusal::InvalidOutput)?;
        let last = strip.cells.last().ok_or(Refusal::InvalidOutput)?;
        let before = trace(surface, last, solved, true, &mut work)?;
        let after = trace(surface, first, solved, false, &mut work)?;
        if before != after {
            return Err(Refusal::UnprovedIdentity);
        }
        let tolerance = work.lift(options.max_xyz_error)?;
        let inner = work.op(&tolerance, &Q::from_integer(3.into()), |a, b| a / b)?;
        let mut geometry = approximate(surface, normal, offset, options, strip, &inner, &mut work)?;
        let first = geometry.xyz.first().ok_or(Refusal::InvalidOutput)?.clone();
        let last = geometry.xyz.last_mut().ok_or(Refusal::InvalidOutput)?;
        let error = geometry
            .cell_error_upper
            .last_mut()
            .ok_or(Refusal::InvalidOutput)?;
        let displacement = distance_l1(&first, last, &mut work)?;
        *error = work.op(error, &displacement, |a, b| a + b)?;
        if *error > tolerance {
            return Err(Refusal::UnprovedIdentity);
        }
        *last = first;
        let solved_coordinate = geometry.uv[0][solved].clone();
        geometry.uv.last_mut().ok_or(Refusal::InvalidOutput)?[solved] = solved_coordinate;
        Ok(geometry)
    });
    Ok(SurfacePlaneWorldStripReport {
        outcome,
        work_used: work.used,
    })
}

/// Restrict each original bivariate power polynomial to t=0 or t=1.
fn trace(
    surface: &Surface,
    cell: &SurfacePlaneGraphStripCell,
    solved: usize,
    upper: bool,
    work: &mut Work,
) -> PResult<Vec<Vec<Q>>> {
    let homogeneous = homogeneous_cell(surface, cell.span, cell.domain, work)?;
    let degree = [surface.degree_u, surface.degree_v][solved];
    homogeneous
        .iter()
        .map(|channel: &Poly| {
            let mut result = vec![Q::zero(); degree + 1];
            for (&(u, v), value) in channel {
                let index = [u, v];
                if upper || index[1 - solved] == 0 {
                    let coefficient = &mut result[index[solved]];
                    *coefficient = work.op(coefficient, value, |a, b| a + b)?;
                }
            }
            Ok(result)
        })
        .collect()
}
