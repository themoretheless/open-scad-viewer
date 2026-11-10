//! Native semantic replay binds to actual persistent parent edge identities.
//! Literal parameter clocks remain literal; missing descendants are not guessed.
use super::*;
#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub parent: TopoId,
    /// Physical parameters of the selected edge in its actual input model.
    pub cuts: Vec<f64>,
    pub key: String,
    pub uv_tolerance: f64,
}
impl History {
    pub fn requests(&self) -> Vec<Request> {
        self.steps
            .iter()
            .map(|s| Request {
                parent: s.source.1.edges[s.edge],
                cuts: s.cuts.clone(),
                key: s.key.clone(),
                uv_tolerance: s.uv_tolerance,
            })
            .collect()
    }
    /// New native construction, not approval of any prior saved output. Every
    /// actual source, model, relation and lineage is qualified afresh under b.
    pub fn from_requests(root: &Model, requests: &[Request], b: Budget) -> Result<Self> {
        b.validate()?;
        if requests.len() > b.max_steps {
            return Err(error("Requested history exceeds step allowance"));
        }
        let mut next = Self::new(root, b)?;
        for r in requests {
            let edge = next
                .model()
                .1
                .edges
                .iter()
                .position(|id| *id == r.parent)
                .ok_or_else(|| {
                    error("History parent edge identity is missing after regeneration")
                })?;
            next.split(edge, &r.cuts, &r.key, r.uv_tolerance, b)?;
        }
        Ok(next)
    }
    /// Re-evaluate authored requests on an explicitly supplied canonical root.
    /// Persistent IDs bind selections even if storage order has changed.
    pub fn rebased(&self, root: &Model, b: Budget) -> Result<Self> {
        Self::from_requests(root, &self.requests(), b)
    }
    /// Edit one operation and reconstruct all dependent operations. Original
    /// keys/parent IDs persist; invalid new domains or removed parents refuse.
    pub fn with_cuts(&self, step: usize, cuts: &[f64], b: Budget) -> Result<Self> {
        let mut requests = self.requests();
        let r = requests
            .get_mut(step)
            .ok_or_else(|| error("Choose an existing history step"))?;
        r.cuts = cuts.to_vec();
        Self::from_requests(&self.root, &requests, b)
    }
    pub fn replace_cuts(&mut self, step: usize, cuts: &[f64], b: Budget) -> Result<()> {
        let next = self.with_cuts(step, cuts, b)?;
        *self = next;
        Ok(())
    }
    /// Independent branch from an exact prefix; no source history is consumed.
    pub fn prefix(&self, count: usize, b: Budget) -> Result<Self> {
        if count > self.steps.len() {
            return Err(error("History prefix exceeds available steps"));
        }
        Self::from_requests(&self.root, &self.requests()[..count], b)
    }
    pub fn truncate(&mut self, count: usize, b: Budget) -> Result<()> {
        let next = self.prefix(count, b)?;
        *self = next;
        Ok(())
    }
}
