//! Complementary full-period trim loops in the unwrapped surface chart.
use super::{
    PResult, Refusal, Result, StoredPeriodicSurfacePlaneSection, Surface,
    SurfacePlaneWorldStripOptions, Work, split_trim::trim_perimeter,
    store_periodic::store_with_orientation,
};
use crate::{curve::Curve, exact_geometry::exact_projective::UvAxis};

#[derive(Clone, Debug)]
pub struct PeriodicSurfacePlaneTrimSplit {
    pub section: StoredPeriodicSurfacePlaneSection,
    /// Counterclockwise loop in the original unwrapped UV chart.
    pub negative: Vec<Curve>,
    /// Counterclockwise loop in the original unwrapped UV chart.
    pub positive: Vec<Curve>,
}

#[derive(Clone, Debug)]
pub struct PeriodicSurfacePlaneTrimSplitReport {
    pub outcome: std::result::Result<PeriodicSurfacePlaneTrimSplit, Refusal>,
    pub work_used: u64,
}

/// Split a full periodic knot strip into two complementary trim regions.
/// Each loop uses the same certified section trace and its own rectangle
/// perimeter in the unwrapped chart. The transverse ends identify the
/// periodic seam, with the original UV period carried by the section.
pub fn split_periodic_surface_strip_by_plane(
    surface: &Surface,
    normal: [f64; 3],
    offset: f64,
    options: SurfacePlaneWorldStripOptions,
    correspondence_tolerance: f64,
) -> Result<PeriodicSurfacePlaneTrimSplitReport> {
    let (stored, work_used) =
        store_with_orientation(surface, normal, offset, options, correspondence_tolerance)?;
    let mut work = Work {
        used: work_used,
        limit: options.strip.max_work,
    };
    let outcome = stored.and_then(|(section, increasing)| {
        let solved = usize::from(options.strip.solve_axis == UvAxis::V);
        let [lower, upper] = trim_chains(&section, solved, &mut work)?;
        let (negative, positive) = if increasing {
            (lower, upper)
        } else {
            (upper, lower)
        };
        Ok(PeriodicSurfacePlaneTrimSplit {
            section,
            negative,
            positive,
        })
    });
    Ok(PeriodicSurfacePlaneTrimSplitReport {
        outcome,
        work_used: work.used,
    })
}

fn trim_chains(
    section: &StoredPeriodicSurfacePlaneSection,
    solved: usize,
    work: &mut Work,
) -> PResult<[Vec<Curve>; 2]> {
    let transverse = 1 - solved;
    let first = section.pieces.first().ok_or(Refusal::InvalidOutput)?;
    let last = section.pieces.last().ok_or(Refusal::InvalidOutput)?;
    let [lo, hi] = first.spatial.domain[solved];
    let mut previous = None;
    for (i, piece) in section.pieces.iter().enumerate() {
        let controls = &piece.pcurve.control_points;
        for (j, point) in controls.iter().enumerate() {
            work.charge(1)?;
            if i > 0 && j == 0 {
                if previous != Some(point) {
                    return Err(Refusal::UnprovedIdentity);
                }
                continue;
            }
            if previous.is_some_and(|prior: &Vec<f64>| prior[transverse] >= point[transverse]) {
                return Err(Refusal::UnprovedIdentity);
            }
            if point[solved] <= lo || point[solved] >= hi {
                return Err(Refusal::UnprovedIdentity);
            }
            previous = Some(point);
        }
    }
    if first.pcurve.control_points[0][transverse] != first.spatial.domain[transverse][0]
        || last
            .pcurve
            .control_points
            .last()
            .ok_or(Refusal::InvalidOutput)?[transverse]
            != last.spatial.domain[transverse][1]
    {
        return Err(Refusal::UnprovedIdentity);
    }
    Ok([
        make_chain(section, solved, lo, solved == 1)?,
        make_chain(section, solved, hi, solved == 0)?,
    ])
}

fn make_chain(
    section: &StoredPeriodicSurfacePlaneSection,
    solved: usize,
    side: f64,
    reverse: bool,
) -> PResult<Vec<Curve>> {
    let mut curves: Vec<_> = section
        .pieces
        .iter()
        .map(|piece| piece.pcurve.clone())
        .collect();
    if reverse {
        curves.reverse();
        for curve in &mut curves {
            curve.control_points.reverse();
        }
    }
    let first = curves
        .first()
        .and_then(|c| c.control_points.first())
        .ok_or(Refusal::InvalidOutput)?;
    let last = curves
        .last()
        .and_then(|c| c.control_points.last())
        .ok_or(Refusal::InvalidOutput)?;
    curves.push(trim_perimeter(first.clone(), last.clone(), solved, side)?);
    Ok(curves)
}
