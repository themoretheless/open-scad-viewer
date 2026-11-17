//! Length bounds for the common material of a finite transverse section.
use super::{
    ExactPlanarSection, PlanarSectionMaterial, PlanarSectionWindingOptions, ProjectedTrimmedFace,
    Refusal, Work,
    algebraic_enclosure::{algebraic_parameter_range, support_scale_squared},
    assemble_quadratic_planar_section_material, check,
};
use crate::Result;
use num_rational::BigRational as Q;
use num_traits::Zero;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanarSectionLength {
    Transverse {
        /// Bounds for the sum of lengths in the canonical support coordinate.
        clock_length: [Q; 2],
        /// Exact rational bounds on the square of the total Euclidean length.
        xyz_length_squared: [Q; 2],
    },
    Parallel,
    Coincident,
    Refused(Refusal),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanarSectionLengthReport {
    pub outcome: PlanarSectionLength,
    pub work_used: u64,
    pub winding_cells: usize,
}

/// Bound the length of the intersection of the two closed trim domains
/// along their transverse support line. Each algebraic endpoint is refined
/// to the given spatial tolerance; unresolved winding contributes only to
/// the upper total. Singleton material contacts have zero length.
pub fn measure_quadratic_planar_section_material(
    left: &ProjectedTrimmedFace,
    right: &ProjectedTrimmedFace,
    options: PlanarSectionWindingOptions,
    endpoint_tolerance: f64,
) -> Result<PlanarSectionLengthReport> {
    check(
        endpoint_tolerance.is_finite() && endpoint_tolerance > 0.,
        "Section measurement needs a positive finite endpoint tolerance",
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
        } => measure(&section, &proven, &possible, endpoint_tolerance, &mut work)
            .unwrap_or_else(PlanarSectionLength::Refused),
        PlanarSectionMaterial::Parallel => PlanarSectionLength::Parallel,
        PlanarSectionMaterial::Coincident => PlanarSectionLength::Coincident,
        PlanarSectionMaterial::Refused(reason) => PlanarSectionLength::Refused(reason),
    };
    Ok(PlanarSectionLengthReport {
        outcome,
        work_used: work.used,
        winding_cells: material.winding_cells,
    })
}

type Checked<T> = std::result::Result<T, Refusal>;

fn measure(
    section: &ExactPlanarSection,
    proven: &[[usize; 2]],
    possible: &[[usize; 2]],
    endpoint_tolerance: f64,
    work: &mut Work,
) -> Checked<PlanarSectionLength> {
    let scale_squared = support_scale_squared(&section.line, work)?;
    let tolerance = work.lift(endpoint_tolerance)?;
    let tolerance_squared = work.op(&tolerance, &tolerance, |a, b| a * b)?;
    let mut ranges = vec![None; section.events.len()];
    let mut clock_length = [Q::zero(), Q::zero()];
    for (side, components) in [proven, possible].into_iter().enumerate() {
        for &[start, end] in components {
            if start == end {
                continue;
            }
            for i in [start, end] {
                if ranges[i].is_none() {
                    let event = &section.events[i];
                    let [lo, hi] = algebraic_parameter_range(
                        &event.clock,
                        &scale_squared,
                        &tolerance_squared,
                        work,
                    )?;
                    ranges[i] = Some([
                        lo.max(event.parameter_bounds[0].clone()),
                        hi.min(event.parameter_bounds[1].clone()),
                    ]);
                }
            }
            let a = ranges[start].as_ref().ok_or(Refusal::InvalidOutput)?;
            let b = ranges[end].as_ref().ok_or(Refusal::InvalidOutput)?;
            let length = work
                .op(&b[side], &a[1 - side], |b, a| b - a)?
                .max(Q::zero());
            clock_length[side] = work.op(&clock_length[side], &length, |a, b| a + b)?;
        }
    }
    let square = |length: &Q, work: &mut Work| -> Checked<Q> {
        let length_squared = work.op(length, length, |a, b| a * b)?;
        Ok(work.op(&length_squared, &scale_squared, |a, b| a * b)?)
    };
    let xyz_length_squared = [
        square(&clock_length[0], work)?,
        square(&clock_length[1], work)?,
    ];
    Ok(PlanarSectionLength::Transverse {
        clock_length,
        xyz_length_squared,
    })
}
