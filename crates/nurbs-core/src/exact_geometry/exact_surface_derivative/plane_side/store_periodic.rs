//! Stored XYZ/UV curve pairs around a closed periodic section.
use super::{
    PResult, Q, Refusal, Result, StoredSurfacePlaneSectionPair, Surface, SurfacePlaneWorldStrip,
    SurfacePlaneWorldStripOptions, Work, check, close_periodic_surface_plane_strip_xyz,
    store_curve::store_world_vertices,
    store_pair::{store_absolute_uv, trace_refusal},
};
use crate::exact_geometry::exact_projective::{UvAxis, pcurve_gap::certify_pcurve_edge_gap};

use num_traits::Signed;

#[derive(Clone, Debug)]
pub struct StoredPeriodicSurfacePlaneSection {
    /// Ordered curve pairs in original knot-cell order.
    pub pieces: Vec<StoredSurfacePlaneSectionPair>,
    /// The transverse period, in the original surface chart.
    pub uv_period: Q,
}

#[derive(Clone, Debug)]
pub struct StoredPeriodicSurfacePlaneSectionReport {
    pub outcome: std::result::Result<StoredPeriodicSurfacePlaneSection, Refusal>,
    pub work_used: u64,
}

/// Store a closed periodic section with per-cell degree-one spatial and UV
/// curves. Each pair has an exact shared clock and a continuous correspondence
/// proof. Ordinary XYZ/UV joins and the periodic wrap are checked after rounding.
pub fn store_periodic_surface_plane_section(
    surface: &Surface,
    normal: [f64; 3],
    offset: f64,
    options: SurfacePlaneWorldStripOptions,
    correspondence_tolerance: f64,
) -> Result<StoredPeriodicSurfacePlaneSectionReport> {
    let (outcome, work_used) =
        store_with_orientation(surface, normal, offset, options, correspondence_tolerance)?;
    Ok(StoredPeriodicSurfacePlaneSectionReport {
        outcome: outcome.map(|(section, _)| section),
        work_used,
    })
}

pub(super) fn store_with_orientation(
    surface: &Surface,
    normal: [f64; 3],
    offset: f64,
    options: SurfacePlaneWorldStripOptions,
    correspondence_tolerance: f64,
) -> Result<(PResult<(StoredPeriodicSurfacePlaneSection, bool)>, u64)> {
    check(
        options.max_xyz_error.is_finite()
            && options.max_xyz_error > 0.
            && correspondence_tolerance.is_finite()
            && correspondence_tolerance >= 0.
            && (1..=255).contains(&options.max_segments_per_cell),
        "Periodic section storage needs finite tolerances and 1..255 segments per cell",
    )?;
    let mut budget = options.max_xyz_error;
    if correspondence_tolerance > 0. {
        budget = budget.min(correspondence_tolerance);
    }
    let half = budget * 0.5;
    let traced = close_periodic_surface_plane_strip_xyz(
        surface,
        normal,
        offset,
        SurfacePlaneWorldStripOptions {
            max_xyz_error: if half > 0. { half } else { budget },
            ..options
        },
    )?;
    let mut work = Work {
        used: traced.work_used,
        limit: options.strip.max_work,
    };
    let outcome = traced.outcome.and_then(|geometry| {
        let increasing = geometry
            .strip
            .cells
            .first()
            .ok_or(Refusal::InvalidOutput)?
            .graph
            .derivative[0]
            .is_positive();
        store(
            surface,
            normal,
            offset,
            options,
            correspondence_tolerance,
            geometry,
            &mut work,
        )
        .map(|section| (section, increasing))
    });
    Ok((outcome, work.used))
}

fn store(
    surface: &Surface,
    normal: [f64; 3],
    offset: f64,
    options: SurfacePlaneWorldStripOptions,
    correspondence_tolerance: f64,
    geometry: SurfacePlaneWorldStrip,
    work: &mut Work,
) -> PResult<StoredPeriodicSurfacePlaneSection> {
    let tolerance = work.lift(options.max_xyz_error)?;
    let correspondence_tolerance = work.lift(correspondence_tolerance)?;
    let mut pieces = Vec::with_capacity(geometry.strip.cells.len());
    for ((cell, &[start, end]), error) in geometry
        .strip
        .cells
        .iter()
        .zip(&geometry.cell_vertices)
        .zip(&geometry.cell_error_upper)
    {
        let spatial = store_world_vertices(
            &geometry.xyz[start..=end],
            cell.domain,
            error,
            normal,
            offset,
            &tolerance,
            work,
        )?;
        let pcurve = store_absolute_uv(&geometry.uv[start..=end], cell.domain, work)?;
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
        pieces.push(StoredSurfacePlaneSectionPair {
            spatial,
            pcurve,
            correspondence_tolerance: correspondence_tolerance.clone(),
        });
    }
    for pair in pieces.windows(2) {
        work.charge(1)?;
        if pair[0].spatial.curve.control_points.last()
            != pair[1].spatial.curve.control_points.first()
            || pair[0].pcurve.control_points.last() != pair[1].pcurve.control_points.first()
        {
            return Err(Refusal::UnprovedIdentity);
        }
    }
    let first = pieces.first().ok_or(Refusal::InvalidOutput)?;
    let last = pieces.last().ok_or(Refusal::InvalidOutput)?;
    let solved = usize::from(options.strip.solve_axis == UvAxis::V);
    let [start, end] = [
        first
            .pcurve
            .control_points
            .first()
            .ok_or(Refusal::InvalidOutput)?,
        last.pcurve
            .control_points
            .last()
            .ok_or(Refusal::InvalidOutput)?,
    ];
    if first.spatial.curve.control_points.first() != last.spatial.curve.control_points.last()
        || start[solved] != end[solved]
    {
        return Err(Refusal::UnprovedIdentity);
    }
    let start = work.lift(start[1 - solved])?;
    let end = work.lift(end[1 - solved])?;
    let uv_period = work.op(&end, &start, |a, b| a - b)?;
    Ok(StoredPeriodicSurfacePlaneSection { pieces, uv_period })
}
