//! Exact line of intersection of two affine support planes.
use super::{
    exact_affine_hull::{PlanarSurfaceProjection, validate_projection_axes},
    planar_coplanar_ray::{
        CoplanarRayPartition, CoplanarRayPartitionOptions, CoplanarRayPartitionOutcome,
        merge_bands, partition_support_line, validate_partition_options,
    },
    planar_trim_projection::ProjectedTrimmedFace,
};
use crate::{
    Result, check,
    core::bounded_rational::{Error, Work},
    exact_geometry::exact_derivative::Refusal,
    trim::trim_domain::Location,
};
use num_rational::BigRational as Q;
use num_traits::{One, Signed, Zero};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactSupportLine {
    pub origin: [Q; 3],
    pub direction: [Q; 3],
    /// The line parameter equals the XYZ coordinate on this axis.
    pub clock_axis: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SupportPlaneIntersection {
    Line(ExactSupportLine),
    Parallel,
    Coincident,
    ResourceLimit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SupportPlaneIntersectionReport {
    pub outcome: SupportPlaneIntersection,
    pub work_used: u64,
}

/// Intersect two exact affine plane charts. The dominant line component
/// defines the canonical parameter axis, so interchanging the inputs gives
/// the same line clock. Coefficients stay rational through the entire solve.
pub fn intersect_planar_supports(
    left: &PlanarSurfaceProjection,
    right: &PlanarSurfaceProjection,
    max_work: u64,
) -> Result<SupportPlaneIntersectionReport> {
    validate_projection_axes(left)?;
    validate_projection_axes(right)?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Plane intersection needs a valid arithmetic budget",
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let outcome = solve(left, right, &mut work).unwrap_or(SupportPlaneIntersection::ResourceLimit);
    Ok(SupportPlaneIntersectionReport {
        outcome,
        work_used: work.used,
    })
}

fn normal(projection: &PlanarSurfaceProjection) -> [Q; 3] {
    let mut n = std::array::from_fn(|_| Q::zero());
    for (axis, slope) in projection.retained_axes.into_iter().zip(&projection.slopes) {
        n[axis] = -slope;
    }
    n[projection.omitted_axis] = Q::one();
    n
}

type Exact<T> = std::result::Result<T, Error>;
const COMPLEMENT: [(usize, usize); 3] = [(1, 2), (2, 0), (0, 1)];

fn det(a: &Q, b: &Q, c: &Q, d: &Q, work: &mut Work) -> Exact<Q> {
    let ab = work.op(a, b, |a, b| a * b)?;
    let cd = work.op(c, d, |a, b| a * b)?;
    work.op(&ab, &cd, |a, b| a - b)
}

fn solve(
    left: &PlanarSurfaceProjection,
    right: &PlanarSurfaceProjection,
    work: &mut Work,
) -> Exact<SupportPlaneIntersection> {
    let a = normal(left);
    let b = normal(right);
    let mut direction = std::array::from_fn(|_| Q::zero());
    let mut clock_axis = 0;
    for (axis, (i, j)) in COMPLEMENT.into_iter().enumerate() {
        direction[axis] = det(&a[i], &b[j], &a[j], &b[i], work)?;
        if direction[axis].abs() > direction[clock_axis].abs() {
            clock_axis = axis;
        }
    }
    let pivot = direction[clock_axis].clone();
    if pivot.is_zero() {
        let k = left.omitted_axis;
        let residual = det(&left.offset, &b[k], &right.offset, &a[k], work)?;
        return Ok(if residual.is_zero() {
            SupportPlaneIntersection::Coincident
        } else {
            SupportPlaneIntersection::Parallel
        });
    }
    let (i, j) = COMPLEMENT[clock_axis];
    let u = det(&left.offset, &b[j], &a[j], &right.offset, work)?;
    let v = det(&a[i], &right.offset, &left.offset, &b[i], work)?;
    let mut origin = std::array::from_fn(|_| Q::zero());
    origin[i] = work.op(&u, &pivot, |a, b| a / b)?;
    origin[j] = work.op(&v, &pivot, |a, b| a / b)?;
    for value in &mut direction {
        *value = work.op(value, &pivot, |a, b| a / b)?;
    }
    Ok(SupportPlaneIntersection::Line(ExactSupportLine {
        origin,
        direction,
        clock_axis,
    }))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrimmedPlanarIntersection {
    Transverse {
        line: ExactSupportLine,
        /// Open intervals proved to be strictly interior to both faces.
        shared_interior: Vec<[Q; 2]>,
        /// Closed enclosures covering all common material and boundary contacts.
        possible: Vec<[Q; 2]>,
    },
    Parallel,
    Coincident,
    Refused(Refusal),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrimmedPlanarIntersectionReport {
    pub outcome: TrimmedPlanarIntersection,
    pub work_used: u64,
    pub winding_cells: usize,
}

/// Clip the exact support intersection to two bounded nonzero-winding faces.
/// The arithmetic and winding budgets are shared by both trims.
pub fn intersect_trimmed_planar_faces(
    left: &ProjectedTrimmedFace,
    right: &ProjectedTrimmedFace,
    options: CoplanarRayPartitionOptions,
) -> Result<TrimmedPlanarIntersectionReport> {
    validate_partition_options(options)?;
    let supports =
        intersect_planar_supports(&left.projection, &right.projection, options.max_work)?;
    let mut work = Work {
        used: supports.work_used,
        limit: options.max_work,
    };
    let mut winding_cells = 0;
    let outcome = match supports.outcome {
        SupportPlaneIntersection::Line(line) => {
            clip_support_line(left, right, line, options, &mut work, &mut winding_cells)?
        }
        SupportPlaneIntersection::Parallel => TrimmedPlanarIntersection::Parallel,
        SupportPlaneIntersection::Coincident => TrimmedPlanarIntersection::Coincident,
        SupportPlaneIntersection::ResourceLimit => {
            TrimmedPlanarIntersection::Refused(Refusal::ResourceLimit)
        }
    };
    Ok(TrimmedPlanarIntersectionReport {
        outcome,
        work_used: work.used,
        winding_cells,
    })
}

fn clip_support_line(
    left: &ProjectedTrimmedFace,
    right: &ProjectedTrimmedFace,
    line: ExactSupportLine,
    options: CoplanarRayPartitionOptions,
    work: &mut Work,
    winding_cells: &mut usize,
) -> Result<TrimmedPlanarIntersection> {
    let [left, right] = match trim_both_faces(left, right, &line, options, work, winding_cells)? {
        Ok(ranges) => ranges,
        Err(reason) => return Ok(TrimmedPlanarIntersection::Refused(reason)),
    };
    Ok(TrimmedPlanarIntersection::Transverse {
        line,
        shared_interior: intersect_bands(&left.interior, &right.interior, false),
        possible: intersect_bands(&left.possible, &right.possible, true),
    })
}

fn trim_both_faces(
    left: &ProjectedTrimmedFace,
    right: &ProjectedTrimmedFace,
    line: &ExactSupportLine,
    options: CoplanarRayPartitionOptions,
    work: &mut Work,
    winding_cells: &mut usize,
) -> Result<std::result::Result<[TrimRanges; 2], Refusal>> {
    let mut ranges = |face| -> Result<std::result::Result<TrimRanges, Refusal>> {
        Ok(
            match partition_support_line(face, line, options, work, winding_cells)? {
                CoplanarRayPartitionOutcome::Partition(partition) => Ok(trim_ranges(partition)),
                CoplanarRayPartitionOutcome::Refused(reason) => Err(reason),
                CoplanarRayPartitionOutcome::NotCoplanar => Err(Refusal::UnprovedIdentity),
            },
        )
    };
    let left = match ranges(left)? {
        Ok(left) => left,
        Err(reason) => return Ok(Err(reason)),
    };
    Ok(ranges(right)?.map(|right| [left, right]))
}

struct TrimRanges {
    interior: Vec<[Q; 2]>,
    possible: Vec<[Q; 2]>,
}

fn trim_ranges(partition: CoplanarRayPartition) -> TrimRanges {
    let mut possible = partition.contact_bands;
    let mut interior = Vec::new();
    for gap in partition.gaps {
        if gap.location == Location::Inside {
            interior.push(gap.parameter.clone());
        }
        if gap.location != Location::Outside {
            possible.push(gap.parameter);
        }
    }
    TrimRanges {
        interior,
        possible: merge_bands(possible),
    }
}

fn intersect_bands(left: &[[Q; 2]], right: &[[Q; 2]], include_points: bool) -> Vec<[Q; 2]> {
    let mut ranges = Vec::new();
    let (mut i, mut j) = (0, 0);
    while let (Some(a), Some(b)) = (left.get(i), right.get(j)) {
        let lo = (&a[0]).max(&b[0]);
        let hi = (&a[1]).min(&b[1]);
        if lo < hi || (include_points && lo == hi) {
            ranges.push([lo.clone(), hi.clone()]);
        }
        if a[1] <= b[1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    ranges
}

mod segments;
pub use segments::{
    ExactPlanarIntersectionSegment, PlanarIntersectionSegmentReport, PlanarIntersectionSegments,
    build_planar_intersection_segments,
};

mod storage;
pub use storage::{
    PlanarSegmentError, PlanarSegmentStorage, PlanarSegmentStorageReport, StoredPlanarIntersection,
    StoredPlanarIntersectionEdge, StoredPlanarIntersectionReport,
    optimize_planar_intersection_segment_storage, store_planar_intersection_segment,
    store_trimmed_planar_intersection,
};

mod boundary;
pub use boundary::{
    AdaptiveBoundaryOptions, AdaptiveBoundaryReport, AlgebraicBoundaryClock,
    AlgebraicBoundaryEnclosure, AlgebraicBoundaryEnclosureReport, AlgebraicBoundaryIsolation,
    AlgebraicBoundaryIsolationReport, AlgebraicBoundaryOrderReport, AlgebraicPlanarContactReport,
    AlgebraicPlanarContacts, BoundaryResolution, BoundedPlanarBoundaryIntersection,
    BoundedPlanarBoundaryReport, ClassifiedPlanarSection, ClassifiedPlanarSectionReport,
    ExactPlanarSection, IsolatedAlgebraicBoundaryClock, PlanarBoundaryIntersection,
    PlanarBoundaryIntersectionReport, PlanarSectionEvent, PlanarSectionGeometryOutcome,
    PlanarSectionGeometryReport, PlanarSectionLength, PlanarSectionLengthReport,
    PlanarSectionMaterial, PlanarSectionMaterialReport, PlanarSectionOutcome,
    PlanarSectionRationalEdge, PlanarSectionRationalGeometry, PlanarSectionReport,
    PlanarSectionWindingOptions, assemble_quadratic_planar_section_material,
    bound_planar_face_boundary_intersection, build_quadratic_planar_section_geometry,
    classify_quadratic_planar_section, enclose_algebraic_planar_boundary_point,
    intersect_planar_face_boundaries, intersect_planar_face_boundary_contacts,
    isolate_ordered_algebraic_boundary_clocks, match_quadratic_planar_boundary_contacts,
    measure_quadratic_planar_section_material, order_algebraic_boundary_clocks,
    partition_quadratic_planar_section, refine_planar_face_boundary_extents,
    resolve_planar_face_boundary_intersection,
};

mod contact_partition;
pub use contact_partition::{
    BoundaryInInterior, PlanarFaceContactPartition, PlanarFaceContactReport,
    TransverseFaceContacts, classify_planar_face_contacts,
};
