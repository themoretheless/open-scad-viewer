//! Match exactly identified algebraic boundary points in the support clock.
use super::{
    ExactSupportLine, ProjectedTrimmedFace, Refusal, SupportPlaneIntersection, Work,
    intersect_planar_supports,
};
pub use crate::exact_geometry::exact_projective::pcurve_seam_isolation::AlgebraicBoundaryClock;
use crate::{Result, analysis::planar_coplanar_ray::algebraic_boundary_clocks};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AlgebraicPlanarContacts {
    Transverse {
        line: ExactSupportLine,
        /// Exact common points identified from irreducible quadratic contact
        /// equations on both contours. Values are ordered by representation.
        points: Vec<AlgebraicBoundaryClock>,
    },
    Parallel,
    Coincident,
    Refused(Refusal),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlgebraicPlanarContactReport {
    pub outcome: AlgebraicPlanarContacts,
    pub work_used: u64,
}

/// Match the exact image of an irrational quadratic curve parameter across
/// two transverse face contours. An empty list leaves contacts of other
/// algebraic degrees and contacts with coincident spans open.
pub fn match_quadratic_planar_boundary_contacts(
    left: &ProjectedTrimmedFace,
    right: &ProjectedTrimmedFace,
    max_work: u64,
) -> Result<AlgebraicPlanarContactReport> {
    let support = intersect_planar_supports(&left.projection, &right.projection, max_work)?;
    let mut work = Work {
        used: support.work_used,
        limit: max_work,
    };
    let outcome = match support.outcome {
        SupportPlaneIntersection::Line(line) => shared(left, right, line, &mut work)?,
        SupportPlaneIntersection::Parallel => AlgebraicPlanarContacts::Parallel,
        SupportPlaneIntersection::Coincident => AlgebraicPlanarContacts::Coincident,
        SupportPlaneIntersection::ResourceLimit => {
            AlgebraicPlanarContacts::Refused(Refusal::ResourceLimit)
        }
    };
    Ok(AlgebraicPlanarContactReport {
        outcome,
        work_used: work.used,
    })
}

fn shared(
    left: &ProjectedTrimmedFace,
    right: &ProjectedTrimmedFace,
    line: ExactSupportLine,
    work: &mut Work,
) -> Result<AlgebraicPlanarContacts> {
    let a = match algebraic_boundary_clocks(left, &line, false, work)? {
        Ok(points) => points,
        Err(reason) => return Ok(AlgebraicPlanarContacts::Refused(reason)),
    };
    let b = match algebraic_boundary_clocks(right, &line, false, work)? {
        Ok(points) => points,
        Err(reason) => return Ok(AlgebraicPlanarContacts::Refused(reason)),
    };
    let mut points = Vec::new();
    let mut right = b.iter().peekable();
    for point in a {
        while let Some(candidate) = right.peek() {
            if let Err(reason) = work.charge(1) {
                return Ok(AlgebraicPlanarContacts::Refused(reason.into()));
            }
            if *candidate >= &point {
                if *candidate == &point {
                    points.push(point);
                    right.next();
                }
                break;
            }
            right.next();
        }
    }
    Ok(AlgebraicPlanarContacts::Transverse { line, points })
}
