//! Exact common portions of trim contours on a transverse support line.
use super::{
    ExactSupportLine, ProjectedTrimmedFace, Refusal, SupportPlaneIntersection, Work,
    intersect_bands, intersect_planar_supports,
};
use crate::{Result, analysis::planar_coplanar_ray::exact_boundary_ranges};
use num_rational::BigRational as Q;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanarBoundaryIntersection {
    Transverse {
        line: ExactSupportLine,
        /// Closed intervals proved to belong to both boundaries. Zero-width
        /// intervals represent a shared boundary point.
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
    intersect(left, right, max_work, false)
}

/// Certify shared boundary intervals and isolated rational contacts of
/// transverse faces. Contacts at span limits and interior rational roots
/// of linear and quadratic homogeneous line equations are included.
/// Higher-degree or irrational interior roots require a separate proof.
pub fn intersect_planar_face_boundary_contacts(
    left: &ProjectedTrimmedFace,
    right: &ProjectedTrimmedFace,
    max_work: u64,
) -> Result<PlanarBoundaryIntersectionReport> {
    intersect(left, right, max_work, true)
}

fn intersect(
    left: &ProjectedTrimmedFace,
    right: &ProjectedTrimmedFace,
    max_work: u64,
    include_isolated: bool,
) -> Result<PlanarBoundaryIntersectionReport> {
    let support = intersect_planar_supports(&left.projection, &right.projection, max_work)?;
    let mut work = Work {
        used: support.work_used,
        limit: max_work,
    };
    let outcome = match support.outcome {
        SupportPlaneIntersection::Line(line) => {
            match shared(left, right, &line, include_isolated, &mut work)? {
                Ok(shared_boundary) => PlanarBoundaryIntersection::Transverse {
                    line,
                    shared_boundary,
                },
                Err(reason) => PlanarBoundaryIntersection::Refused(reason),
            }
        }
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
    include_isolated: bool,
    work: &mut Work,
) -> Result<std::result::Result<Vec<[Q; 2]>, Refusal>> {
    let a = match exact_boundary_ranges(left, line, include_isolated, work)? {
        Ok(ranges) => ranges,
        Err(reason) => return Ok(Err(reason)),
    };
    let b = match exact_boundary_ranges(right, line, include_isolated, work)? {
        Ok(ranges) => ranges,
        Err(reason) => return Ok(Err(reason)),
    };
    Ok(Ok(intersect_bands(&a, &b, true)))
}
