//! Sew qualified cap face sectors while retaining their full original sources.
use super::{Regions, require};
use crate::shell_thickness::contact_pair::cap::sewing as kernel;
use crate::{Model, Result};
pub struct Sewn {
    regions: Vec<Regions>,
    model: Model,
    joins: Vec<kernel::Join>,
    limits: kernel::Limits,
    pairs: usize,
    cells: usize,
    boundary: crate::boundary_agreement::Report,
}
impl Sewn {
    pub fn regions(&self) -> &[Regions] {
        &self.regions
    }
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn joins(&self) -> &[kernel::Join] {
        &self.joins
    }
    pub fn pairs(&self) -> usize {
        self.pairs
    }
    pub fn correspondence_cells(&self) -> usize {
        self.cells
    }
    pub fn boundary_proof(&self) -> &crate::boundary_agreement::Report {
        &self.boundary
    }
    pub fn reverify(&self) -> Result<()> {
        for r in &self.regions {
            r.reverify()?;
        }
        let models = self.regions.iter().map(Regions::model).collect::<Vec<_>>();
        let (model, joins, _, _, _) = kernel::assemble_models(&models, self.limits)?;
        require(
            model == self.model
                && joins.len() == self.joins.len()
                && joins
                    .iter()
                    .zip(&self.joins)
                    .all(|(a, b)| a.edges == b.edges && a.reversed == b.reversed),
            "Sewn cap sectors differ from complete original reconstruction",
        )
    }
}
/// Internal sector edges retain their two original uses. Only exposed complete
/// boundaries can gain mates; coincident internal edges require material choice.
/// Final whole-boundary lifts and full shell topology are checked. No Body,
/// outward/cavity role or global embedding is inferred from sewn face sectors.
pub fn build(regions: Vec<Regions>, limits: kernel::Limits) -> Result<Sewn> {
    let models = regions.iter().map(Regions::model).collect::<Vec<_>>();
    let (model, joins, pairs, cells, boundary) = kernel::assemble_models(&models, limits)?;
    Ok(Sewn {
        regions,
        model,
        joins,
        limits,
        pairs,
        cells,
        boundary,
    })
}
