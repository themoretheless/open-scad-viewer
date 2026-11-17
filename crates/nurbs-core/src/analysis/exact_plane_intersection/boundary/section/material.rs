//! Closed material components of a transverse planar section.
use super::{
    ClassifiedPlanarSection, ExactPlanarSection, Location, PlanarSectionWindingOptions,
    ProjectedTrimmedFace, Refusal, Result, Work, classify_quadratic_planar_section,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanarSectionMaterial {
    Transverse {
        section: Box<ExactPlanarSection>,
        /// Closed components in exact event indices, including singleton contacts.
        proven: Vec<[usize; 2]>,
        /// Closed upper coverage in exact event indices.
        possible: Vec<[usize; 2]>,
    },
    Parallel,
    Coincident,
    Refused(Refusal),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanarSectionMaterialReport {
    pub outcome: PlanarSectionMaterial,
    pub work_used: u64,
    pub winding_cells: usize,
}

/// Assemble closed common material components of a finite quadratic section.
/// Authored boundaries are included in each face. Unresolved winding gaps
/// remain in the upper coverage unless either face proves them outside.
pub fn assemble_quadratic_planar_section_material(
    left: &ProjectedTrimmedFace,
    right: &ProjectedTrimmedFace,
    options: PlanarSectionWindingOptions,
) -> Result<PlanarSectionMaterialReport> {
    let classified = classify_quadratic_planar_section(left, right, options)?;
    let mut work = Work {
        used: classified.work_used,
        limit: options.max_work,
    };
    let outcome = match classified.outcome {
        ClassifiedPlanarSection::Transverse { section, gaps } => {
            match material(&section, &gaps, &mut work) {
                Ok([proven, possible]) => PlanarSectionMaterial::Transverse {
                    section,
                    proven,
                    possible,
                },
                Err(reason) => PlanarSectionMaterial::Refused(reason),
            }
        }
        ClassifiedPlanarSection::Parallel => PlanarSectionMaterial::Parallel,
        ClassifiedPlanarSection::Coincident => PlanarSectionMaterial::Coincident,
        ClassifiedPlanarSection::Refused(reason) => PlanarSectionMaterial::Refused(reason),
    };
    Ok(PlanarSectionMaterialReport {
        outcome,
        work_used: work.used,
        winding_cells: classified.winding_cells,
    })
}

fn material(
    section: &ExactPlanarSection,
    gaps: &[[Location; 2]],
    work: &mut Work,
) -> std::result::Result<[Vec<[usize; 2]>; 2], Refusal> {
    let mut components = [Vec::new(), Vec::new()];
    let exterior = [Location::Outside; 2];
    let mut before = exterior;
    for (i, event) in section.events.iter().enumerate() {
        work.charge(1)?;
        let after = gaps.get(i).copied().unwrap_or(exterior);
        let location = std::array::from_fn(|side| {
            if event.on_boundary[side] {
                Location::Inside
            } else if before[side] == Location::Unresolved {
                after[side]
            } else {
                before[side]
            }
        });
        add_cell(&mut components, [i, i], location);
        if i < gaps.len() {
            add_cell(&mut components, [i, i + 1], after);
        }
        before = after;
    }
    Ok(components)
}

fn add_cell(components: &mut [Vec<[usize; 2]>; 2], range: [usize; 2], location: [Location; 2]) {
    let [proven, possible] = components;
    if location == [Location::Inside; 2] {
        extend(proven, range);
    }
    if !location.contains(&Location::Outside) {
        extend(possible, range);
    }
}

fn extend(ranges: &mut Vec<[usize; 2]>, range: [usize; 2]) {
    if let Some(last) = ranges.last_mut()
        && last[1] >= range[0]
    {
        last[1] = last[1].max(range[1]);
    } else {
        ranges.push(range);
    }
}
