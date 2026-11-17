//! Exact common portions of trim contours on a transverse support line.
use super::{
    ExactSupportLine, ProjectedTrimmedFace, Refusal, SupportPlaneIntersection, Work,
    intersect_bands, intersect_planar_supports,
};
use crate::{Result, analysis::planar_coplanar_ray::coincident_boundary_ranges};
use num_rational::BigRational as Q;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanarBoundaryIntersection {
    Transverse {
        line: ExactSupportLine,
        /// Closed intervals proved to belong to both boundaries. Zero-width
        /// intervals represent a shared endpoint of coincident spans.
        shared_boundary: Vec<[Q; 2]>,
    },
    Parallel,
    Coincident,
    Refused(Refusal),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanarBoundaryIntersectionReport {
    pub outcome: PlanarBoundaryIntersection,
    pub work_used: u64,
}

/// Intersect exact portions of contours that run along the transverse support
/// line. The span endpoints certify their intervening image by continuity.
/// Excursions past the span endpoints and contacts of noncoincident spans
/// are outside this proof, so an empty result gives no separation claim.
pub fn intersect_planar_face_boundaries(
    left: &ProjectedTrimmedFace,
    right: &ProjectedTrimmedFace,
    max_work: u64,
) -> Result<PlanarBoundaryIntersectionReport> {
    let support = intersect_planar_supports(&left.projection, &right.projection, max_work)?;
    let mut work = Work {
        used: support.work_used,
        limit: max_work,
    };
    let outcome = match support.outcome {
        SupportPlaneIntersection::Line(line) => match shared(left, right, &line, &mut work)? {
            Ok(shared_boundary) => PlanarBoundaryIntersection::Transverse {
                line,
                shared_boundary,
            },
            Err(reason) => PlanarBoundaryIntersection::Refused(reason),
        },
        SupportPlaneIntersection::Parallel => PlanarBoundaryIntersection::Parallel,
        SupportPlaneIntersection::Coincident => PlanarBoundaryIntersection::Coincident,
        SupportPlaneIntersection::ResourceLimit => {
            PlanarBoundaryIntersection::Refused(Refusal::ResourceLimit)
        }
    };
    Ok(PlanarBoundaryIntersectionReport {
        outcome,
        work_used: work.used,
    })
}

fn shared(
    left: &ProjectedTrimmedFace,
    right: &ProjectedTrimmedFace,
    line: &ExactSupportLine,
    work: &mut Work,
) -> Result<std::result::Result<Vec<[Q; 2]>, Refusal>> {
    let a = match coincident_boundary_ranges(left, line, work)? {
        Ok(ranges) => ranges,
        Err(reason) => return Ok(Err(reason)),
    };
    let b = match coincident_boundary_ranges(right, line, work)? {
        Ok(ranges) => ranges,
        Err(reason) => return Ok(Err(reason)),
    };
    Ok(Ok(intersect_bands(&a, &b, true)))
}
