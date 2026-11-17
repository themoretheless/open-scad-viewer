//! Corresponding exact 3D edges and planar trim curves for transverse faces.
use super::{
    CoplanarRayPartitionOptions, ExactSupportLine, ProjectedTrimmedFace, Refusal,
    TrimmedPlanarIntersection, Work, intersect_trimmed_planar_faces,
};
use crate::Result;
use num_rational::BigRational as Q;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPlanarIntersectionSegment {
    /// Endpoints in the canonical support clock. Its open interval is
    /// certified inside both trims. The endpoints are limits of that interval.
    pub parameter: [Q; 2],
    /// Degree-one unit-weight control net for the spatial segment.
    pub xyz: [[Q; 3]; 2],
    /// Corresponding degree-one unit-weight control nets in the left and
    /// right Cartesian trim charts, respectively. All nets use [0, 1].
    pub pcurves: [[[Q; 2]; 2]; 2],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanarIntersectionSegments {
    Transverse {
        line: ExactSupportLine,
        segments: Vec<ExactPlanarIntersectionSegment>,
        /// Closed enclosures of possible material and boundary contacts,
        /// including any portion from which no segment could be certified.
        possible: Vec<[Q; 2]>,
    },
    Parallel,
    Coincident,
    Refused(Refusal),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanarIntersectionSegmentReport {
    pub outcome: PlanarIntersectionSegments,
    pub work_used: u64,
    pub winding_cells: usize,
}

/// Build exact spatial segments and matching trim curves on both faces.
/// Coordinates, including nonbinary ones, stay rational. The shared work
/// budget includes endpoint evaluation, and exhaustion returns no partial net.
pub fn build_planar_intersection_segments(
    left: &ProjectedTrimmedFace,
    right: &ProjectedTrimmedFace,
    options: CoplanarRayPartitionOptions,
) -> Result<PlanarIntersectionSegmentReport> {
    let clipped = intersect_trimmed_planar_faces(left, right, options)?;
    let mut work = Work {
        used: clipped.work_used,
        limit: options.max_work,
    };
    let outcome = match clipped.outcome {
        TrimmedPlanarIntersection::Transverse {
            line,
            shared_interior,
            possible,
        } => {
            let axes = [
                left.projection.retained_axes,
                right.projection.retained_axes,
            ];
            let segments = shared_interior
                .into_iter()
                .map(|parameter| segment(&line, axes, parameter, &mut work))
                .collect::<std::result::Result<Vec<_>, _>>();
            match segments {
                Ok(segments) => PlanarIntersectionSegments::Transverse {
                    line,
                    segments,
                    possible,
                },
                Err(reason) => PlanarIntersectionSegments::Refused(reason),
            }
        }
        TrimmedPlanarIntersection::Parallel => PlanarIntersectionSegments::Parallel,
        TrimmedPlanarIntersection::Coincident => PlanarIntersectionSegments::Coincident,
        TrimmedPlanarIntersection::Refused(reason) => PlanarIntersectionSegments::Refused(reason),
    };
    Ok(PlanarIntersectionSegmentReport {
        outcome,
        work_used: work.used,
        winding_cells: clipped.winding_cells,
    })
}

pub(super) fn segment(
    line: &ExactSupportLine,
    axes: [[usize; 2]; 2],
    parameter: [Q; 2],
    work: &mut Work,
) -> std::result::Result<ExactPlanarIntersectionSegment, Refusal> {
    let xyz = [
        point(line, &parameter[0], work)?,
        point(line, &parameter[1], work)?,
    ];
    let pcurves = axes.map(|chart| [&xyz[0], &xyz[1]].map(|p| chart.map(|i| p[i].clone())));
    Ok(ExactPlanarIntersectionSegment {
        parameter,
        xyz,
        pcurves,
    })
}

fn point(line: &ExactSupportLine, t: &Q, work: &mut Work) -> std::result::Result<[Q; 3], Refusal> {
    let coordinate = |i: usize, work: &mut Work| {
        let delta = work.op(&line.direction[i], t, |a, b| a * b)?;
        work.op(&line.origin[i], &delta, |a, b| a + b)
    };
    Ok([
        coordinate(0, work)?,
        coordinate(1, work)?,
        coordinate(2, work)?,
    ])
}
