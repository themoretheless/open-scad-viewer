//! Complete ordered line events for quadratic planar trim boundaries.
use super::{
    AlgebraicBoundaryClock, ExactSupportLine, ProjectedTrimmedFace, Refusal,
    SupportPlaneIntersection, Work, algebraic_order::isolate, intersect_planar_supports,
};
use crate::{
    Result,
    analysis::planar_coplanar_ray::{algebraic_boundary_clocks, section_sample_rectangle},
    check,
    trim::trim_domain::{Location, TrimDomain},
};
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

#[derive(Clone, Copy, Debug)]
pub struct PlanarSectionWindingOptions {
    pub winding_tolerance: f64,
    pub max_winding_cells: usize,
    pub max_work: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClassifiedPlanarSection {
    Transverse {
        section: Box<ExactPlanarSection>,
        /// One classification per open gap, in left/right face order.
        gaps: Vec<[Location; 2]>,
    },
    Parallel,
    Coincident,
    Refused(Refusal),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassifiedPlanarSectionReport {
    pub outcome: ClassifiedPlanarSection,
    pub work_used: u64,
    pub winding_cells: usize,
}

/// Classify the complete open gaps of a finite quadratic section. Each
/// rational sample proves its entire gap because the exact event list
/// accounts for every boundary crossing. Ambiguous winding stays unresolved.
/// The two unbounded tails are outside both bounded contour sets.
pub fn classify_quadratic_planar_section(
    left: &ProjectedTrimmedFace,
    right: &ProjectedTrimmedFace,
    options: PlanarSectionWindingOptions,
) -> Result<ClassifiedPlanarSectionReport> {
    check(
        options.winding_tolerance.is_finite()
            && options.winding_tolerance > 0.
            && (1..=100_000).contains(&options.max_winding_cells),
        "Section classification needs a positive winding tolerance and bounded work",
    )?;
    let partition = partition_quadratic_planar_section(left, right, options.max_work)?;
    let mut work = Work {
        used: partition.work_used,
        limit: options.max_work,
    };
    let mut winding_cells = 0;
    let outcome = match partition.outcome {
        PlanarSectionOutcome::Transverse(section) => {
            match classify_gaps(
                [left, right],
                &section,
                options,
                &mut work,
                &mut winding_cells,
            )? {
                Ok(gaps) => ClassifiedPlanarSection::Transverse { section, gaps },
                Err(reason) => ClassifiedPlanarSection::Refused(reason),
            }
        }
        PlanarSectionOutcome::Parallel => ClassifiedPlanarSection::Parallel,
        PlanarSectionOutcome::Coincident => ClassifiedPlanarSection::Coincident,
        PlanarSectionOutcome::Refused(reason) => ClassifiedPlanarSection::Refused(reason),
    };
    Ok(ClassifiedPlanarSectionReport {
        outcome,
        work_used: work.used,
        winding_cells,
    })
}

fn classify_gaps(
    faces: [&ProjectedTrimmedFace; 2],
    section: &ExactPlanarSection,
    options: PlanarSectionWindingOptions,
    work: &mut Work,
    winding_cells: &mut usize,
) -> Result<std::result::Result<Vec<[Location; 2]>, Refusal>> {
    if section.samples.is_empty() {
        return Ok(Ok(Vec::new()));
    }
    let [left, right] = faces.map(|face| TrimDomain::new(&face.loops, options.winding_tolerance));
    let domains = [left?, right?];
    let projected = faces.map(|face| {
        let axes = face.projection.retained_axes;
        [
            axes.map(|i| section.line.origin[i].clone()),
            axes.map(|i| section.line.direction[i].clone()),
        ]
    });
    let mut gaps = Vec::with_capacity(section.samples.len());
    for sample in &section.samples {
        let mut locations = [Location::Unresolved; 2];
        for i in 0..2 {
            let remaining = options.max_winding_cells - *winding_cells;
            if remaining == 0 {
                break;
            }
            let rectangle = match section_sample_rectangle(&projected[i], sample, work) {
                Ok(rectangle) => rectangle,
                Err(reason) => return Ok(Err(reason)),
            };
            locations[i] = if let Some(rectangle) = rectangle {
                let classification = domains[i].classify(rectangle, remaining)?;
                *winding_cells += classification.cells;
                classification.location
            } else {
                Location::Outside
            };
        }
        gaps.push(locations);
    }
    Ok(Ok(gaps))
}

mod material;
pub use material::{
    PlanarSectionMaterial, PlanarSectionMaterialReport, assemble_quadratic_planar_section_material,
};
