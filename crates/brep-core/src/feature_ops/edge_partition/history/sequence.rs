//! Mixed native edge-edit journal. Native operation owners retain geometry and
//! proof authority; saved outputs/maps are expectations for fresh reconstruction.
use super::super::seam;
use super::*;
mod codec;
mod maps;
#[derive(Clone, Debug, PartialEq)]
pub enum Operation {
    Split { cuts: Vec<f64> },
    Relocate { parameter: f64 },
    Compact,
    Reclock { domain: [f64; 2] },
}
#[derive(Clone, Debug, PartialEq)]
pub struct Command {
    pub parent: TopoId,
    pub operation: Operation,
    pub key: String,
    pub uv_tolerance: f64,
}
pub enum Applied {
    Split(Split),
    Relocate(seam::Relocation),
    Compact(seam::compact::Compaction),
    Reclock(super::super::reclock::Reclock),
}
pub struct Step {
    command: Command,
    limits: Limits,
    applied: Applied,
}
impl Step {
    pub fn command(&self) -> &Command {
        &self.command
    }
    pub fn applied(&self) -> &Applied {
        &self.applied
    }
    pub fn model(&self) -> &Model {
        match &self.applied {
            Applied::Split(s) => s.model(),
            Applied::Relocate(s) => s.model(),
            Applied::Compact(s) => s.model(),
            Applied::Reclock(s) => s.model(),
        }
    }
    pub fn evidence(&self) -> &Verification {
        match &self.applied {
            Applied::Split(s) => s.evidence(),
            Applied::Relocate(s) => s.evidence(),
            Applied::Compact(s) => s.evidence(),
            Applied::Reclock(s) => s.evidence(),
        }
    }
}
pub struct Sequence {
    root: Model,
    steps: Vec<Step>,
    work: Work,
}
impl Sequence {
    pub fn new(root: &Model, budget: Budget) -> Result<Self> {
        let canonical = History::new(root, budget)?;
        let next = Self {
            root: canonical.root,
            steps: Vec::new(),
            work: Work::default(),
        };
        wire::bounded(&codec::document(&next, None), budget.max_bytes)?;
        Ok(next)
    }
    pub fn root(&self) -> &Model {
        &self.root
    }
    pub fn model(&self) -> &Model {
        self.steps.last().map_or(&self.root, Step::model)
    }
    pub fn steps(&self) -> &[Step] {
        &self.steps
    }
    pub fn commands(&self) -> Vec<Command> {
        self.steps.iter().map(|s| s.command.clone()).collect()
    }
    fn check_budget(&self, b: Budget) -> Result<()> {
        b.validate()?;
        if self.steps.len() > b.max_steps {
            return Err(error("Mixed history exceeds step allowance"));
        }
        let mut work = Work::default();
        for s in &self.steps {
            if !wire::within(s.limits, work.remaining(b.proof)?) {
                return Err(error("Stored mixed step limits exceed caller allowances"));
            }
            let e = s.evidence();
            work.cells += e.cells;
            work.pairs += e.pairs;
            work.domain_cells += e.domain_cells;
            if work.cells > b.proof.max_cells
                || work.pairs > b.proof.max_pairs
                || work.domain_cells > b.proof.max_domain_cells
            {
                return Err(error("Mixed history exceeds aggregate work allowance"));
            }
        }
        Ok(())
    }
    fn allowance(&self, b: Budget) -> Result<Limits> {
        self.check_budget(b)?;
        if self.steps.len() >= b.max_steps {
            return Err(error("Mixed history step allowance exhausted"));
        }
        self.work.remaining(b.proof)
    }
    fn construct(&self, command: Command, caps: Limits) -> Result<Step> {
        if !command.uv_tolerance.is_finite() || command.uv_tolerance <= 0. {
            return Err(error("Mixed UV tolerance must be finite and positive"));
        }
        let edge = self
            .model()
            .1
            .edges
            .iter()
            .position(|id| *id == command.parent)
            .ok_or_else(|| error("Mixed history parent edge identity is missing"))?;
        let applied = match &command.operation {
            Operation::Reclock { domain } => {
                Applied::Reclock(super::super::reclock::reparameterize(
                    self.model(),
                    edge,
                    *domain,
                    &command.key,
                    caps,
                )?)
            }
            Operation::Split { cuts } => Applied::Split(split_with_uv_tolerance(
                self.model(),
                edge,
                cuts,
                &command.key,
                command.uv_tolerance,
                caps,
            )?),
            Operation::Relocate { parameter } => {
                Applied::Relocate(seam::relocate_with_uv_tolerance(
                    self.model(),
                    edge,
                    *parameter,
                    &command.key,
                    command.uv_tolerance,
                    caps,
                )?)
            }
            Operation::Compact => Applied::Compact(seam::compact::compact_with_uv_tolerance(
                self.model(),
                edge,
                &command.key,
                command.uv_tolerance,
                caps,
            )?),
        };
        Ok(Step {
            command,
            limits: caps,
            applied,
        })
    }
    fn accept(&mut self, next: Step, b: Budget) -> Result<&Step> {
        wire::bounded(&codec::document(self, Some(&next)), b.max_bytes)?;
        let e = next.evidence();
        let work = Work {
            cells: self.work.cells + e.cells,
            pairs: self.work.pairs + e.pairs,
            domain_cells: self.work.domain_cells + e.domain_cells,
        };
        if work.cells > b.proof.max_cells
            || work.pairs > b.proof.max_pairs
            || work.domain_cells > b.proof.max_domain_cells
        {
            return Err(error("Mixed history aggregate work exceeded"));
        }
        self.work = work;
        self.steps.push(next);
        Ok(self.steps.last().unwrap())
    }
    /// No tip mutation occurs until the complete new operation and packet fit.
    pub fn apply(&mut self, command: Command, b: Budget) -> Result<&Step> {
        let caps = self.allowance(b)?;
        let next = self.construct(command, caps)?;
        self.accept(next, b)
    }
    pub fn from_commands(root: &Model, commands: &[Command], b: Budget) -> Result<Self> {
        b.validate()?;
        if commands.len() > b.max_steps {
            return Err(error("Mixed commands exceed step allowance"));
        }
        let mut next = Self::new(root, b)?;
        for c in commands {
            next.apply(c.clone(), b)?;
        }
        Ok(next)
    }
    pub fn rebased(&self, root: &Model, b: Budget) -> Result<Self> {
        Self::from_commands(root, &self.commands(), b)
    }
    pub fn with_command(&self, index: usize, command: Command, b: Budget) -> Result<Self> {
        let mut commands = self.commands();
        *commands
            .get_mut(index)
            .ok_or_else(|| error("Choose an existing mixed step"))? = command;
        Self::from_commands(&self.root, &commands, b)
    }
    pub fn replace_command(&mut self, index: usize, command: Command, b: Budget) -> Result<()> {
        let next = self.with_command(index, command, b)?;
        *self = next;
        Ok(())
    }
    pub fn prefix(&self, count: usize, b: Budget) -> Result<Self> {
        if count > self.steps.len() {
            return Err(error("Mixed prefix exceeds available steps"));
        }
        Self::from_commands(&self.root, &self.commands()[..count], b)
    }
    pub fn truncate(&mut self, count: usize, b: Budget) -> Result<()> {
        let next = self.prefix(count, b)?;
        *self = next;
        Ok(())
    }
    pub fn reverify(&self, b: Budget) -> Result<()> {
        Self::from_json(&self.to_json(b)?, b).map(|_| ())
    }
}
