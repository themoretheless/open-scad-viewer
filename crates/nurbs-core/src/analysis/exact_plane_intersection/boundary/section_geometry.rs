//! Exact rational geometry contained in certified section components.
use super::{
    ExactPlanarSection, PlanarSectionMaterial, PlanarSectionWindingOptions, ProjectedTrimmedFace,
    Refusal, Work,
    algebraic_enclosure::{algebraic_parameter_range, scaled_width_squared, support_scale_squared},
    assemble_quadratic_planar_section_material, check,
};
use crate::{
    Result,
    analysis::exact_plane_intersection::{ExactPlanarIntersectionSegment, segments::segment},
};
use num_rational::BigRational as Q;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanarSectionRationalEdge {
    /// The algebraic endpoints of the original component, as section event indices.
    pub events: [usize; 2],
    pub segment: ExactPlanarIntersectionSegment,
    /// Squared XYZ distance bounds from each constructed endpoint to the original one.
    pub endpoint_error_squared: [Q; 2],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanarSectionRationalGeometry {
    pub section: Box<ExactPlanarSection>,
    pub edges: Vec<PlanarSectionRationalEdge>,
    /// Exact singleton contacts, as section event indices.
    pub points: Vec<usize>,
    /// Upper coverage in exact event indices, including unresolved material.
    pub possible: Vec<[usize; 2]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanarSectionGeometryOutcome {
    Transverse(Box<PlanarSectionRationalGeometry>),
    Parallel,
    Coincident,
    Refused(Refusal),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanarSectionGeometryReport {
    pub outcome: PlanarSectionGeometryOutcome,
    pub work_used: u64,
    pub winding_cells: usize,
}

/// Construct exact rational segments in the proven common material. An
/// irrational endpoint is moved inward by at most the given XYZ tolerance.
/// Each segment has corresponding rational control nets on both face charts.
pub fn build_quadratic_planar_section_geometry(
    left: &ProjectedTrimmedFace,
    right: &ProjectedTrimmedFace,
    options: PlanarSectionWindingOptions,
    endpoint_tolerance: f64,
) -> Result<PlanarSectionGeometryReport> {
    check(
        endpoint_tolerance.is_finite() && endpoint_tolerance >= 0.,
        "Section geometry needs a finite nonnegative endpoint tolerance",
    )?;
    let material = assemble_quadratic_planar_section_material(left, right, options)?;
    let mut work = Work {
        used: material.work_used,
        limit: options.max_work,
    };
    let outcome = match material.outcome {
        PlanarSectionMaterial::Transverse {
            section,
            proven,
            possible,
        } => {
            let axes = [
                left.projection.retained_axes,
                right.projection.retained_axes,
            ];
            match build(&section, &proven, axes, endpoint_tolerance, &mut work) {
                Ok((edges, points)) => PlanarSectionGeometryOutcome::Transverse(Box::new(
                    PlanarSectionRationalGeometry {
                        section,
                        edges,
                        points,
                        possible,
                    },
                )),
                Err(reason) => PlanarSectionGeometryOutcome::Refused(reason),
            }
        }
        PlanarSectionMaterial::Parallel => PlanarSectionGeometryOutcome::Parallel,
        PlanarSectionMaterial::Coincident => PlanarSectionGeometryOutcome::Coincident,
        PlanarSectionMaterial::Refused(reason) => PlanarSectionGeometryOutcome::Refused(reason),
    };
    Ok(PlanarSectionGeometryReport {
        outcome,
        work_used: work.used,
        winding_cells: material.winding_cells,
    })
}

fn build(
    section: &ExactPlanarSection,
    proven: &[[usize; 2]],
    axes: [[usize; 2]; 2],
    endpoint_tolerance: f64,
    work: &mut Work,
) -> std::result::Result<(Vec<PlanarSectionRationalEdge>, Vec<usize>), Refusal> {
    let scale = support_scale_squared(&section.line, work)?;
    let tolerance = work.lift(endpoint_tolerance)?;
    let tolerance_squared = work.op(&tolerance, &tolerance, |a, b| a * b)?;
    let mut edges = Vec::new();
    let mut points = Vec::new();
    for &[start, end] in proven {
        work.charge(1)?;
        if start == end {
            points.push(start);
            continue;
        }
        let mut ranges = Vec::with_capacity(2);
        for i in [start, end] {
            let event = &section.events[i];
            let [lo, hi] =
                algebraic_parameter_range(&event.clock, &scale, &tolerance_squared, work)?;
            ranges.push([
                lo.max(event.parameter_bounds[0].clone()),
                hi.min(event.parameter_bounds[1].clone()),
            ]);
        }
        let parameter = [ranges[0][1].clone(), ranges[1][0].clone()];
        if parameter[0] >= parameter[1] {
            return Err(Refusal::InvalidOutput);
        }
        let endpoint_error_squared = [
            scaled_width_squared(&ranges[0], &scale, work)?,
            scaled_width_squared(&ranges[1], &scale, work)?,
        ];
        edges.push(PlanarSectionRationalEdge {
            events: [start, end],
            segment: segment(&section.line, axes, parameter, work)?,
            endpoint_error_squared,
        });
    }
    Ok((edges, points))
}
