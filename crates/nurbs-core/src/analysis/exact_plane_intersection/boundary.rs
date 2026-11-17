//! Exact common portions of trim contours on a transverse support line.
use super::{
    ExactSupportLine, ProjectedTrimmedFace, Refusal, SupportPlaneIntersection, Work,
    intersect_bands, intersect_planar_supports,
};
use crate::{
    Result, analysis::planar_coplanar_ray::exact_boundary_ranges, check,
    editing::exact_projective::pcurve_seam_isolation::SpanLineCoverage,
};
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
    intersect(left, right, max_work, SpanLineCoverage::Endpoints)
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
    intersect(left, right, max_work, SpanLineCoverage::Contacts)
}

/// Include excursions of coincident rational spans by evaluating exact
/// stationary parameters or rational witnesses in isolated stationary brackets.
/// The width limits each bracket in its local normalized span parameter.
/// Returned intervals are certified subsets of the boundary image.
pub fn refine_planar_face_boundary_extents(
    left: &ProjectedTrimmedFace,
    right: &ProjectedTrimmedFace,
    stationary_width: f64,
    max_work: u64,
) -> Result<PlanarBoundaryIntersectionReport> {
    check(
        stationary_width.is_finite() && stationary_width > 0. && stationary_width <= 1.,
        "Boundary extent refinement needs a normalized stationary width in (0, 1]",
    )?;
    intersect(
        left,
        right,
        max_work,
        SpanLineCoverage::Refined { stationary_width },
    )
}

fn intersect(
    left: &ProjectedTrimmedFace,
    right: &ProjectedTrimmedFace,
    max_work: u64,
    coverage: SpanLineCoverage,
) -> Result<PlanarBoundaryIntersectionReport> {
    let support = intersect_planar_supports(&left.projection, &right.projection, max_work)?;
    let mut work = Work {
        used: support.work_used,
        limit: max_work,
    };
    let outcome = match support.outcome {
        SupportPlaneIntersection::Line(line) => {
            match shared(left, right, &line, coverage, &mut work)? {
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
    coverage: SpanLineCoverage,
    work: &mut Work,
) -> Result<std::result::Result<Vec<[Q; 2]>, Refusal>> {
    let a = match exact_boundary_ranges(left, line, coverage, work)? {
        Ok(ranges) => ranges,
        Err(reason) => return Ok(Err(reason)),
    };
    let b = match exact_boundary_ranges(right, line, coverage, work)? {
        Ok(ranges) => ranges,
        Err(reason) => return Ok(Err(reason)),
    };
    Ok(Ok(intersect_bands(&a, &b, true)))
}
