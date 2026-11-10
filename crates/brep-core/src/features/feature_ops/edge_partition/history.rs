//! Owned replayable edge-refinement documents. Stored models and relation maps
//! are expectations, never certificates: loading reconstructs every actual step.
use super::*;
use value_codec::{Deserialize, Serialize, Value};
mod regenerate;
pub mod sequence;
mod wire;
pub use regenerate::Request;
#[derive(Clone, Copy)]
pub struct Budget {
    pub max_steps: usize,
    pub max_bytes: usize,
    /// Aggregate cells/pairs/domain work; numerical mass allowance is per
    /// qualification, and closed sewing retains its separate native cap.
    pub proof: Limits,
}
impl Default for Budget {
    fn default() -> Self {
        Self {
            max_steps: 32,
            max_bytes: 32 * 1024 * 1024,
            proof: Limits::default(),
        }
    }
}
impl Budget {
    fn validate(self) -> Result<()> {
        if !(1..=32).contains(&self.max_steps)
            || !(1..=32 * 1024 * 1024).contains(&self.max_bytes)
            || !(1..=100000).contains(&self.proof.max_cells)
            || !(1..=100000).contains(&self.proof.max_pairs)
            || !(1..=1000000).contains(&self.proof.max_domain_cells)
            || !(100..=2000000).contains(&self.proof.mass_evaluations)
        {
            return Err(error("Choose bounded edge-history replay allowances"));
        }
        Ok(())
    }
}
pub struct History {
    root: Model,
    steps: Vec<Split>,
    work: Work,
}
impl History {
    pub fn new(root: &Model, budget: Budget) -> Result<Self> {
        budget.validate()?;
        root.validate()?;
        let authored = root.to_value();
        wire::bounded(&authored, budget.max_bytes)?;
        let decoded = Model::from_value(authored.clone())
            .map_err(|e| error(format!("Canonical history root: {e}")))?;
        if !wire::same(&decoded.to_value(), &authored) {
            return Err(error(
                "History root requires decoder repair; retain complete canonical identity data first",
            ));
        }
        let h = Self {
            root: root.clone(),
            steps: Vec::new(),
            work: Work::default(),
        };
        wire::bounded(&wire::document(&h, None), budget.max_bytes)?;
        Ok(h)
    }
    pub fn root(&self) -> &Model {
        &self.root
    }
    pub fn model(&self) -> &Model {
        self.steps.last().map_or(&self.root, |s| &s.model)
    }
    pub fn steps(&self) -> &[Split] {
        &self.steps
    }
    fn check_budget(&self, b: Budget) -> Result<()> {
        b.validate()?;
        if self.steps.len() > b.max_steps {
            return Err(error("Stored history exceeds requested step allowance"));
        }
        let mut work = Work::default();
        for s in &self.steps {
            let left = work.remaining(b.proof)?;
            if !wire::within(s.limits, left) {
                return Err(error(
                    "Stored step limits exceed requested history allowances",
                ));
            }
            work.cells += s.evidence.cells;
            work.pairs += s.evidence.pairs;
            work.domain_cells += s.evidence.domain_cells;
            if work.cells > b.proof.max_cells
                || work.pairs > b.proof.max_pairs
                || work.domain_cells > b.proof.max_domain_cells
            {
                return Err(error("Stored history exceeds aggregate proof allowance"));
            }
        }
        Ok(())
    }
    fn allowance(&self, b: Budget) -> Result<Limits> {
        self.check_budget(b)?;
        if self.steps.len() >= b.max_steps {
            return Err(error("Edge-history step allowance exhausted"));
        }
        self.work.remaining(b.proof)
    }
    /// Atomic append: geometry, source binding, total work and packet size must
    /// all qualify before the previous tip is replaced.
    pub fn split(
        &mut self,
        edge: usize,
        cuts: &[f64],
        key: &str,
        uv_tolerance: f64,
        budget: Budget,
    ) -> Result<&Split> {
        let caps = self.allowance(budget)?;
        let next = split_with_uv_tolerance(self.model(), edge, cuts, key, uv_tolerance, caps)?;
        self.accept(next, budget)
    }
    /// Attach an existing native result without discarding the caller's object.
    /// Reconstruct under the remaining caller budget and require the complete
    /// original source, saved model and ownership maps to match literally.
    pub fn append(&mut self, result: &Split, budget: Budget) -> Result<&Split> {
        let caps = self.allowance(budget)?;
        if !wire::same(&self.model().to_value(), &result.source.to_value()) {
            return Err(error(
                "Existing split source does not match the actual history tip",
            ));
        }
        let next = split_with_uv_tolerance(
            self.model(),
            result.edge,
            &result.cuts,
            &result.key,
            result.uv_tolerance,
            caps,
        )?;
        if !wire::same(&next.model.to_value(), &result.model.to_value())
            || !wire::same(&wire::maps(&next), &wire::maps(result))
        {
            return Err(error(
                "Existing split differs from fresh history-budget reconstruction",
            ));
        }
        self.accept(next, budget)
    }
    fn accept(&mut self, next: Split, b: Budget) -> Result<&Split> {
        wire::bounded(&wire::document(self, Some(&next)), b.max_bytes)?;
        self.work.cells += next.evidence.cells;
        self.work.pairs += next.evidence.pairs;
        self.work.domain_cells += next.evidence.domain_cells;
        self.steps.push(next);
        Ok(self.steps.last().unwrap())
    }
    pub fn to_json(&self, budget: Budget) -> Result<String> {
        self.check_budget(budget)?;
        let v = wire::document(self, None);
        wire::bounded(&v, budget.max_bytes)?;
        Ok(v.to_string())
    }
    pub fn from_json(text: &str, budget: Budget) -> Result<Self> {
        budget.validate()?;
        if text.len() > budget.max_bytes {
            return Err(error("Edge-history document exceeds byte allowance"));
        }
        let v: Value = value_codec::from_str_strict(text)
            .map_err(|e| error(format!("Edge-history JSON: {e}")))?;
        wire::bounded(&v, budget.max_bytes)?;
        let object = wire::object(&v, &["schema", "root", "steps"])?;
        if object["schema"].as_str() != Some("brep-edge-history/1") {
            return Err(error("Unknown edge-history schema"));
        }
        let steps = object["steps"]
            .as_array()
            .ok_or_else(|| error("History steps must be an array"))?;
        if steps.len() > budget.max_steps {
            return Err(error("Edge-history step allowance exhausted"));
        }
        let root = Model::from_value(object["root"].clone())
            .map_err(|e| error(format!("Original history model: {e}")))?;
        root.validate()?;
        if !wire::same(&root.to_value(), &object["root"]) {
            return Err(error(
                "History root must retain canonical complete geometry and identity tables",
            ));
        }
        let mut h = Self::new(&root, budget)?;
        for entry in steps {
            let o = wire::object(
                entry,
                &[
                    "edge",
                    "parent",
                    "cuts",
                    "key",
                    "uvTolerance",
                    "limits",
                    "model",
                    "maps",
                ],
            )?;
            let caps = wire::limits(&o["limits"])?;
            let left = h.allowance(budget)?;
            if !wire::within(caps, left) {
                return Err(error(
                    "Stored step limits cannot increase caller replay allowances",
                ));
            }
            let edge = wire::usize_value(&o["edge"])?;
            if h.model()
                .1
                .edges
                .get(edge)
                .map(ToString::to_string)
                .as_deref()
                != o["parent"].as_str()
            {
                return Err(error(
                    "History parent edge identity does not match the actual previous tip",
                ));
            }
            let raw_cuts = o["cuts"]
                .as_array()
                .filter(|c| (1..=16).contains(&c.len()))
                .ok_or_else(|| error("History cuts need 1..16 scalar parameters"))?;
            let cuts = raw_cuts
                .iter()
                .map(|v| {
                    v.as_f64()
                        .ok_or_else(|| error("History cut must be numeric"))
                })
                .collect::<Result<Vec<_>>>()?;
            let key = o["key"]
                .as_str()
                .ok_or_else(|| error("History key must be a string"))?;
            let uv = o["uvTolerance"]
                .as_f64()
                .ok_or_else(|| error("History UV allowance must be numeric"))?;
            let next = split_with_uv_tolerance(h.model(), edge, &cuts, key, uv, caps)?;
            if !wire::same(&next.model.to_value(), &o["model"])
                || !wire::same(&wire::maps(&next), &o["maps"])
            {
                return Err(error(
                    "Saved history model or original relation ownership differs from fresh reconstruction",
                ));
            }
            h.accept(next, budget)?;
        }
        Ok(h)
    }
    pub fn reverify(&self, budget: Budget) -> Result<()> {
        Self::from_json(&self.to_json(budget)?, budget).map(|_| ())
    }
}
