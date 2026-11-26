//! Location of certified transverse contacts relative to both trim domains.
use super::{
    CoplanarRayPartitionOptions, ExactSupportLine, ProjectedTrimmedFace, Refusal,
    SupportPlaneIntersection, Work, intersect_bands, intersect_planar_supports, trim_both_faces,
    validate_partition_options,
};
use crate::{
    Result, analysis::planar_coplanar_ray::exact_boundary_ranges,
    exact_geometry::exact_projective::pcurve_seam_isolation::SpanLineCoverage,
};
use num_rational::BigRational as Q;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundaryInInterior {
    pub parameter: [Q; 2],
    /// Whether each limit is strictly inside the other face. Singletons have
    /// both flags set.
    pub closed: [bool; 2],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransverseFaceContacts {
    pub line: ExactSupportLine,
    /// Open intervals inside both faces.
    pub interior: Vec<[Q; 2]>,
    /// Closed intervals or points on both boundaries.
    pub common_boundary: Vec<[Q; 2]>,
    pub left_boundary_in_right_interior: Vec<BoundaryInInterior>,
    pub right_boundary_in_left_interior: Vec<BoundaryInInterior>,
    /// Closed upper cover of all contacts, including unresolved contacts.
    pub possible: Vec<[Q; 2]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanarFaceContactPartition {
    Transverse(Box<TransverseFaceContacts>),
    Parallel,
    Coincident,
    Refused(Refusal),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanarFaceContactReport {
    pub outcome: PlanarFaceContactPartition,
    pub work_used: u64,
    pub winding_cells: usize,
}

/// Classify certified contacts by whether each face contributes material
/// interior or trim boundary. Contacts outside these proofs remain within
/// the conservative possible cover.
pub fn classify_planar_face_contacts(
    left: &ProjectedTrimmedFace,
    right: &ProjectedTrimmedFace,
    options: CoplanarRayPartitionOptions,
) -> Result<PlanarFaceContactReport> {
    validate_partition_options(options)?;
    let support = intersect_planar_supports(&left.projection, &right.projection, options.max_work)?;
    let mut work = Work {
        used: support.work_used,
        limit: options.max_work,
    };
    let mut winding_cells = 0;
    let outcome = match support.outcome {
        SupportPlaneIntersection::Line(line) => {
            classify(left, right, line, options, &mut work, &mut winding_cells)?
        }
        SupportPlaneIntersection::Parallel => PlanarFaceContactPartition::Parallel,
        SupportPlaneIntersection::Coincident => PlanarFaceContactPartition::Coincident,
        SupportPlaneIntersection::ResourceLimit => {
            PlanarFaceContactPartition::Refused(Refusal::ResourceLimit)
        }
    };
    Ok(PlanarFaceContactReport {
        outcome,
        work_used: work.used,
        winding_cells,
    })
}

fn classify(
    left: &ProjectedTrimmedFace,
    right: &ProjectedTrimmedFace,
    line: ExactSupportLine,
    options: CoplanarRayPartitionOptions,
    work: &mut Work,
    winding_cells: &mut usize,
) -> Result<PlanarFaceContactPartition> {
    let [a, b] = match trim_both_faces(left, right, &line, options, work, winding_cells)? {
        Ok(ranges) => ranges,
        Err(reason) => return Ok(PlanarFaceContactPartition::Refused(reason)),
    };
    let left_boundary = match exact_boundary_ranges(left, &line, SpanLineCoverage::Contacts, work)?
    {
        Ok(boundary) => boundary,
        Err(reason) => return Ok(PlanarFaceContactPartition::Refused(reason)),
    };
    let right_boundary =
        match exact_boundary_ranges(right, &line, SpanLineCoverage::Contacts, work)? {
            Ok(boundary) => boundary,
            Err(reason) => return Ok(PlanarFaceContactPartition::Refused(reason)),
        };
    Ok(PlanarFaceContactPartition::Transverse(Box::new(
        TransverseFaceContacts {
            line,
            interior: intersect_bands(&a.interior, &b.interior, false),
            common_boundary: intersect_bands(&left_boundary, &right_boundary, true),
            left_boundary_in_right_interior: mixed_ranges(&left_boundary, &b.interior),
            right_boundary_in_left_interior: mixed_ranges(&right_boundary, &a.interior),
            possible: intersect_bands(&a.possible, &b.possible, true),
        },
    )))
}

fn mixed_ranges(boundary: &[[Q; 2]], interior: &[[Q; 2]]) -> Vec<BoundaryInInterior> {
    let mut output = Vec::new();
    let (mut i, mut j) = (0, 0);
    while let (Some(a), Some(b)) = (boundary.get(i), interior.get(j)) {
        let lo = (&a[0]).max(&b[0]);
        let hi = (&a[1]).min(&b[1]);
        let closed = [lo > &b[0], hi < &b[1]];
        if lo < hi || (lo == hi && closed == [true, true]) {
            output.push(BoundaryInInterior {
                parameter: [lo.clone(), hi.clone()],
                closed,
            });
        }
        if a[1] <= b[1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    output
}
