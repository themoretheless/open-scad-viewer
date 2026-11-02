//! Native exact feature regeneration. Frozen recipes replay choices, while
//! live edits rebuild the actual feature. No index/geometry guessing of IDs.
use super::*;
use crate::TopoId;
use material::{Limits, Work};
use std::sync::Arc;
#[cfg(feature = "codec")]
pub mod codec;

#[derive(Clone, Copy, Debug)]
pub struct Budget {
    pub max_revisions: usize,
    /// Serialized document allowance; enforced when codec is enabled.
    pub max_bytes: usize,
    /// Aggregate material work; refinement/halving limits are per qualification.
    pub material: Limits,
}
impl Default for Budget {
    fn default() -> Self {
        Self {
            max_revisions: 32,
            max_bytes: 4 << 20,
            material: Limits::default(),
        }
    }
}
impl Budget {
    fn validate(self) -> Result<()> {
        let l = self.material;
        if !(1..=32).contains(&self.max_revisions)
            || !(1..=4 << 20).contains(&self.max_bytes)
            || l.max_pair_projections > 32768
            || l.max_atoms > 32768
            || l.max_ray_cells > 32768
            || l.refinements > 512
            || l.radius_halvings > 512
        {
            return Err(error("Invalid exact Full Round history allowances"));
        }
        Ok(())
    }
    fn remaining(self, work: Work) -> Result<Limits> {
        let mut l = self.material;
        l.max_pair_projections = l
            .max_pair_projections
            .checked_sub(work.pair_projections)
            .ok_or_else(|| error("Exact history pair budget exhausted"))?;
        l.max_atoms = l
            .max_atoms
            .checked_sub(work.atoms)
            .ok_or_else(|| error("Exact history atom budget exhausted"))?;
        l.max_ray_cells = l
            .max_ray_cells
            .checked_sub(work.ray_cells)
            .ok_or_else(|| error("Exact history ray budget exhausted"))?;
        Ok(l)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edit {
    Root,
    RebaseSource,
    Reselect,
}
#[derive(Clone, Debug)]
pub struct Revision {
    edit: Edit,
    body: Arc<AdmittedFullRound>,
}
impl Revision {
    pub fn edit(&self) -> Edit {
        self.edit
    }
    pub fn body(&self) -> &AdmittedFullRound {
        &self.body
    }
}
#[derive(Clone, Debug)]
pub struct History {
    revisions: Vec<Revision>,
    cursor: usize,
    budget: Budget,
    work: Work,
}
fn within(a: Limits, b: Limits) -> bool {
    a.max_pair_projections <= b.max_pair_projections
        && a.max_atoms <= b.max_atoms
        && a.max_ray_cells <= b.max_ray_cells
        && a.refinements <= b.refinements
        && a.radius_halvings <= b.radius_halvings
}
fn used(body: &AdmittedFullRound) -> Work {
    body.material().certificate().relation().work()
}
fn add(a: Work, b: Work) -> Result<Work> {
    let sum = |x: usize, y: usize| {
        x.checked_add(y)
            .ok_or_else(|| error("Exact history work overflow"))
    };
    Ok(Work {
        pair_projections: sum(a.pair_projections, b.pair_projections)?,
        atoms: sum(a.atoms, b.atoms)?,
        ray_cells: sum(a.ray_cells, b.ray_cells)?,
    })
}
fn ids(body: &AdmittedFullRound) -> [TopoId; 3] {
    body.selected_faces().map(|i| body.source().1.faces[i])
}
fn resolve(source: &Model, selected: [TopoId; 3]) -> Result<[usize; 3]> {
    source.validate()?;
    if selected.into_iter().collect::<BTreeSet<_>>().len() != 3 {
        return Err(error(
            "Exact history requires three distinct source face IDs",
        ));
    }
    let mut out = [0; 3];
    for (i, id) in selected.into_iter().enumerate() {
        out[i]=source.1.faces.iter().position(|candidate|*candidate==id)
            .ok_or_else(||error("Exact history source face ID is missing; explicit lineage/reselection required"))?;
    }
    Ok(out)
}
impl History {
    pub fn new(body: AdmittedFullRound, budget: Budget) -> Result<Self> {
        budget.validate()?;
        if !within(
            body.material().certificate().relation().limits(),
            budget.material,
        ) {
            return Err(error("Root proof allowances exceed exact history budget"));
        }
        let history = Self {
            work: used(&body),
            revisions: vec![Revision {
                edit: Edit::Root,
                body: Arc::new(body),
            }],
            cursor: 0,
            budget,
        };
        history.budget.remaining(history.work)?;
        history.check_storage()?;
        Ok(history)
    }
    pub fn current(&self) -> &AdmittedFullRound {
        self.revisions[self.cursor].body()
    }
    pub fn revisions(&self) -> &[Revision] {
        &self.revisions
    }
    pub fn cursor(&self) -> usize {
        self.cursor
    }
    pub fn work(&self) -> Work {
        self.work
    }
    pub fn selected_source_ids(&self) -> [TopoId; 3] {
        ids(self.current())
    }
    pub fn undo(&mut self) -> Result<&AdmittedFullRound> {
        if self.cursor == 0 {
            return Err(error("Exact history has no undo revision"));
        }
        self.cursor -= 1;
        Ok(self.current())
    }
    pub fn redo(&mut self) -> Result<&AdmittedFullRound> {
        if self.cursor + 1 == self.revisions.len() {
            return Err(error("Exact history has no redo revision"));
        }
        self.cursor += 1;
        Ok(self.current())
    }
    /// Retains the original redo branch; returns an independent prefix branch.
    pub fn branch(&self) -> Result<Self> {
        let revisions = self.revisions[..=self.cursor].to_vec();
        let work = revisions
            .iter()
            .try_fold(Work::default(), |w, r| add(w, used(r.body())))?;
        Ok(Self {
            revisions,
            cursor: self.cursor,
            budget: self.budget,
            work,
        })
    }
    /// Rebuild on new source geometry by the SAME authored topology IDs.
    pub fn rebase_source(&mut self, source: &Model) -> Result<&AdmittedFullRound> {
        self.apply(Edit::RebaseSource, source, self.selected_source_ids())
    }
    /// Explicit feature selection edit on the current original source.
    pub fn reselect(&mut self, selected: [TopoId; 3]) -> Result<&AdmittedFullRound> {
        let source = self.current().source().clone();
        self.apply(Edit::Reselect, &source, selected)
    }
    fn apply(
        &mut self,
        edit: Edit,
        source: &Model,
        selected: [TopoId; 3],
    ) -> Result<&AdmittedFullRound> {
        if self.cursor + 1 != self.revisions.len() {
            return Err(error(
                "Branch explicitly before editing an undone exact history",
            ));
        }
        if self.revisions.len() >= self.budget.max_revisions {
            return Err(error("Exact history revision budget exhausted"));
        }
        let faces = resolve(source, selected)?;
        let body = build_admitted(source, faces, self.budget.remaining(self.work)?)?;
        let mut next = self.clone();
        next.work = add(next.work, used(&body))?;
        next.budget.remaining(next.work)?;
        next.revisions.push(Revision {
            edit,
            body: Arc::new(body),
        });
        next.cursor += 1;
        next.check_storage()?;
        *self = next;
        Ok(self.current())
    }
    fn check_storage(&self) -> Result<()> {
        #[cfg(feature = "codec")]
        {
            codec::export(self)?;
        }
        Ok(())
    }
}
