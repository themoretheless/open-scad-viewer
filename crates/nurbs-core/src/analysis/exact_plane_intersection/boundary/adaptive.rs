//! Adaptive common-boundary coverage with a geometric resolution target.
use super::{
    BoundedPlanarBoundaryIntersection, ExactSupportLine, ProjectedTrimmedFace, Refusal,
    SupportPlaneIntersection, Work, bound_shared, check, intersect_bands,
    intersect_planar_supports,
};
use crate::{Result, analysis::planar_coplanar_ray::merge_bands};
use num_rational::BigRational as Q;

#[derive(Clone, Copy, Debug)]
pub struct AdaptiveBoundaryOptions {
    pub xyz_tolerance: f64,
    pub max_passes: usize,
    pub max_work: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundaryResolution {
    WithinTolerance,
    PassLimit,
    Refused(Refusal),
    NoTransverseLine,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdaptiveBoundaryReport {
    pub bounds: BoundedPlanarBoundaryIntersection,
    pub resolution: BoundaryResolution,
    /// Squared XYZ length of the longest connected part of possible coverage
    /// outside verified coverage. This is a localization bound, including when
    /// the existence of an enclosed contact is unresolved.
    pub max_unresolved_length_squared: Option<Q>,
    pub passes: usize,
    pub work_used: u64,
}

/// Refine boundary coverage to a geometric length target. Successful passes
/// accumulate proven coverage and intersect possible coverage. A later work
/// or precision stop retains the last certified bounds.
pub fn resolve_planar_face_boundary_intersection(
    left: &ProjectedTrimmedFace,
    right: &ProjectedTrimmedFace,
    options: AdaptiveBoundaryOptions,
) -> Result<AdaptiveBoundaryReport> {
    check(
        options.xyz_tolerance.is_finite()
            && options.xyz_tolerance > 0.
            && (1..=54).contains(&options.max_passes),
        "Boundary resolution needs a positive finite XYZ tolerance and 1..54 passes",
    )?;
    let support = intersect_planar_supports(&left.projection, &right.projection, options.max_work)?;
    let mut work = Work {
        used: support.work_used,
        limit: options.max_work,
    };
    let (bounds, resolution) = match support.outcome {
        SupportPlaneIntersection::Line(line) => {
            return refine(left, right, line, options, &mut work);
        }
        SupportPlaneIntersection::Parallel => (
            BoundedPlanarBoundaryIntersection::Parallel,
            BoundaryResolution::NoTransverseLine,
        ),
        SupportPlaneIntersection::Coincident => (
            BoundedPlanarBoundaryIntersection::Coincident,
            BoundaryResolution::NoTransverseLine,
        ),
        SupportPlaneIntersection::ResourceLimit => (
            BoundedPlanarBoundaryIntersection::Refused(Refusal::ResourceLimit),
            BoundaryResolution::Refused(Refusal::ResourceLimit),
        ),
    };
    Ok(AdaptiveBoundaryReport {
        bounds,
        resolution,
        max_unresolved_length_squared: None,
        passes: 0,
        work_used: work.used,
    })
}

fn refine(
    left: &ProjectedTrimmedFace,
    right: &ProjectedTrimmedFace,
    line: ExactSupportLine,
    options: AdaptiveBoundaryOptions,
    work: &mut Work,
) -> Result<AdaptiveBoundaryReport> {
    use BoundedPlanarBoundaryIntersection as Bounds;
    let mut report = AdaptiveBoundaryReport {
        bounds: Bounds::Refused(Refusal::ResourceLimit),
        resolution: BoundaryResolution::PassLimit,
        max_unresolved_length_squared: None,
        passes: 0,
        work_used: 0,
    };
    let mut width = 1. / 16.;
    for _ in 0..options.max_passes {
        match bound_shared(left, right, &line, width, work)? {
            Bounds::Transverse {
                proven, possible, ..
            } => {
                let (mut proven, mut possible) = (proven, possible);
                if let Bounds::Transverse {
                    proven: old,
                    possible: outer,
                    ..
                } = &report.bounds
                {
                    proven.extend(old.iter().cloned());
                    proven = merge_bands(proven);
                    possible = intersect_bands(&possible, outer, true);
                }
                let lengths =
                    measure_uncertainty(&line, &proven, &possible, options.xyz_tolerance, work);
                report.bounds = Bounds::Transverse {
                    line: line.clone(),
                    proven,
                    possible,
                };
                report.passes += 1;
                match lengths {
                    Ok((length, within)) => {
                        report.max_unresolved_length_squared = Some(length);
                        if within {
                            report.resolution = BoundaryResolution::WithinTolerance;
                            break;
                        }
                    }
                    Err(reason) => {
                        report.max_unresolved_length_squared = None;
                        report.resolution = BoundaryResolution::Refused(reason);
                        break;
                    }
                }
            }
            Bounds::Refused(reason) => {
                if report.passes == 0 {
                    report.bounds = Bounds::Refused(reason.clone());
                }
                report.resolution = BoundaryResolution::Refused(reason);
                break;
            }
            Bounds::Parallel | Bounds::Coincident => unreachable!("fixed transverse line"),
        }
        width *= 0.25;
    }
    report.work_used = work.used;
    Ok(report)
}

fn measure_uncertainty(
    line: &ExactSupportLine,
    proven: &[[Q; 2]],
    possible: &[[Q; 2]],
    tolerance: f64,
    work: &mut Work,
) -> std::result::Result<(Q, bool), Refusal> {
    let mut greatest = Q::default();
    let mut known = proven.iter().peekable();
    let mut record = |lo: &Q, hi: &Q, work: &mut Work| -> std::result::Result<(), Refusal> {
        if lo < hi {
            greatest = work.op(hi, lo, |a, b| a - b)?.max(greatest.clone());
        }
        Ok(())
    };
    for [low, high] in possible {
        let mut cursor = low;
        while let Some([a, b]) = known.peek().copied() {
            work.charge(1)?;
            if a > high {
                break;
            }
            record(cursor, a, work)?;
            cursor = cursor.max(b);
            known.next();
        }
        record(cursor, high, work)?;
    }
    let mut scale_squared = Q::default();
    for direction in &line.direction {
        let square = work.op(direction, direction, |a, b| a * b)?;
        scale_squared = work.op(&scale_squared, &square, |a, b| a + b)?;
    }
    let parameter_squared = work.op(&greatest, &greatest, |a, b| a * b)?;
    let xyz_squared = work.op(&parameter_squared, &scale_squared, |a, b| a * b)?;
    let tolerance = work.lift(tolerance)?;
    let limit = work.op(&tolerance, &tolerance, |a, b| a * b)?;
    let within = xyz_squared <= limit;
    Ok((xyz_squared, within))
}
