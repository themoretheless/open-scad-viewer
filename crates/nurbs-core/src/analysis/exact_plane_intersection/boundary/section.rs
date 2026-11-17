//! Complete ordered line events for quadratic planar trim boundaries.
use super::{
    AlgebraicBoundaryClock, ExactSupportLine, ProjectedTrimmedFace, Refusal,
    SupportPlaneIntersection, Work, algebraic_order::isolate, intersect_planar_supports,
};
use crate::{Result, analysis::planar_coplanar_ray::algebraic_boundary_clocks};
use num_rational::BigRational as Q;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanarSectionEvent {
    pub clock: AlgebraicBoundaryClock,
    /// Left and right contour incidence.
    pub on_boundary: [bool; 2],
    pub parameter_bounds: [Q; 2],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPlanarSection {
    pub line: ExactSupportLine,
    pub events: Vec<PlanarSectionEvent>,
    /// Rational samples in every bounded gap between events. No curve of
    /// either contour meets the open support interval between those events.
    pub samples: Vec<Q>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanarSectionOutcome {
    Transverse(Box<ExactPlanarSection>),
    Parallel,
    Coincident,
    Refused(Refusal),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanarSectionReport {
    pub outcome: PlanarSectionOutcome,
    pub work_used: u64,
}

/// Build all finite boundary events of a transverse support intersection.
/// Accepts homogeneous contact equations of degree at most two; spans that
/// lie entirely on the line or require higher-degree equations refuse the
/// complete-section proof.
pub fn partition_quadratic_planar_section(
    left: &ProjectedTrimmedFace,
    right: &ProjectedTrimmedFace,
    max_work: u64,
) -> Result<PlanarSectionReport> {
    let support = intersect_planar_supports(&left.projection, &right.projection, max_work)?;
    let mut work = Work {
        used: support.work_used,
        limit: max_work,
    };
    let outcome = match support.outcome {
        SupportPlaneIntersection::Line(line) => section(left, right, line, &mut work)?,
        SupportPlaneIntersection::Parallel => PlanarSectionOutcome::Parallel,
        SupportPlaneIntersection::Coincident => PlanarSectionOutcome::Coincident,
        SupportPlaneIntersection::ResourceLimit => {
            PlanarSectionOutcome::Refused(Refusal::ResourceLimit)
        }
    };
    Ok(PlanarSectionReport {
        outcome,
        work_used: work.used,
    })
}

fn section(
    left: &ProjectedTrimmedFace,
    right: &ProjectedTrimmedFace,
    line: ExactSupportLine,
    work: &mut Work,
) -> Result<PlanarSectionOutcome> {
    let mut points = match algebraic_boundary_clocks(left, &line, true, work)? {
        Ok(points) => points,
        Err(reason) => return Ok(PlanarSectionOutcome::Refused(reason)),
    };
    let left_count = points.len();
    match algebraic_boundary_clocks(right, &line, true, work)? {
        Ok(right) => points.extend(right),
        Err(reason) => return Ok(PlanarSectionOutcome::Refused(reason)),
    }
    if points.len() > 4096 {
        return Ok(PlanarSectionOutcome::Refused(Refusal::ResourceLimit));
    }
    let isolated = match isolate(&points, work) {
        Ok(isolated) => isolated,
        Err(reason) => return Ok(PlanarSectionOutcome::Refused(reason)),
    };
    let events = isolated
        .clocks
        .into_iter()
        .map(|event| PlanarSectionEvent {
            clock: points[event.indices[0]].clone(),
            on_boundary: [
                event.indices.iter().any(|&i| i < left_count),
                event.indices.iter().any(|&i| i >= left_count),
            ],
            parameter_bounds: event.parameter,
        })
        .collect();
    Ok(PlanarSectionOutcome::Transverse(Box::new(
        ExactPlanarSection {
            line,
            events,
            samples: isolated.witnesses,
        },
    )))
}
