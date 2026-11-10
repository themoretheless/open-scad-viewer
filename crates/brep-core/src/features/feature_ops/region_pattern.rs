//! Original-face feature volumes and named affine feature instances. Rim groups
//! reuse original support charts or qualified planes; distinct groups require
//! positive actual trimmed-cap separation. Unsupported pairs refuse atomically.
use super::{PatternMode, error};
use crate::{Model, Result, TopoId, TopoKind};
use std::collections::{BTreeMap, BTreeSet};
mod cap_carrier;
mod cap_components;
mod cap_separation;
mod closure;
mod extraction;
mod naming;
pub(crate) mod qualification;
mod trim_clocks;
#[derive(Clone, Copy, Debug)]
pub enum FeatureRole {
    Additive,
    Subtractive,
}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_cells: usize,
    pub max_pairs: usize,
    pub max_domain_cells: usize,
    pub mass_evaluations: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_cells: 100000,
            max_pairs: 100000,
            max_domain_cells: 1000000,
            mass_evaluations: 200000,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Verification {
    pub boundary_complete: bool,
    pub trim_regions_valid: bool,
    /// Complete admitted face charts: injective enclosures or proved periodic/
    /// collapsed quotients with actual owned seam/pole incidence. This is not
    /// a whole-solid embedding certificate.
    pub charts_injective: bool,
    pub cells: usize,
    pub pairs: usize,
    pub domain_cells: usize,
    /// Numerical orientation diagnostic, NOT a certified volume enclosure.
    pub numerical_volume_mm3: f64,
    pub audit: crate::solid_audit::SolidAuditCertificate,
}
#[derive(Clone, Copy, Default)]
struct ProofWork {
    cells: usize,
    pairs: usize,
    domain_cells: usize,
}
impl ProofWork {
    fn remaining(self, limits: Limits) -> Result<Limits> {
        let remaining = Limits {
            max_cells: limits.max_cells.saturating_sub(self.cells),
            max_pairs: limits.max_pairs.saturating_sub(self.pairs),
            max_domain_cells: limits.max_domain_cells.saturating_sub(self.domain_cells),
            ..limits
        };
        qualification::limits(remaining)?;
        Ok(remaining)
    }
    fn charge_cells(&mut self, n: usize, limits: Limits) -> Result<()> {
        if n > self.remaining(limits)?.max_cells {
            return Err(error("Original closure proof work exhausted"));
        }
        self.cells += n;
        Ok(())
    }
    fn include(self, proof: &mut Verification) {
        proof.cells += self.cells;
        proof.pairs += self.pairs;
        proof.domain_cells += self.domain_cells;
    }
}
#[derive(Clone)]
pub struct FeatureRegion {
    source: Model,
    tool: Model,
    selected: Vec<usize>,
    support: Vec<usize>,
    support_loop: Option<usize>,
    role: FeatureRole,
    caps: Vec<usize>,
    source_map: BTreeMap<TopoId, TopoId>,
    limits: Limits,
    evidence: Verification,
}
impl FeatureRegion {
    pub fn limits(&self) -> Limits {
        self.limits
    }
    pub fn source(&self) -> &Model {
        &self.source
    }
    pub fn tool(&self) -> &Model {
        &self.tool
    }
    pub fn selected_faces(&self) -> &[usize] {
        &self.selected
    }
    pub fn support_faces(&self) -> &[usize] {
        &self.support
    }
    /// One original complete support wire, when the entire closure uses it.
    pub fn support_loop(&self) -> Option<usize> {
        self.support_loop
    }
    /// First closure face in stable original-rim ID order; see `cap_faces`.
    pub fn cap_face(&self) -> usize {
        self.caps[0]
    }
    /// All newly constructed closure faces, including nested material islands.
    pub fn cap_faces(&self) -> &[usize] {
        &self.caps
    }
    pub fn source_ids(&self) -> &BTreeMap<TopoId, TopoId> {
        &self.source_map
    }
    pub fn evidence(&self) -> &Verification {
        &self.evidence
    }
    pub fn reverify(&self, limits: Limits) -> Result<Verification> {
        qualification::limits(limits)?;
        let built = extraction::build(&self.source, &self.selected, self.role, limits)?;
        if built.model != self.tool
            || built.supports != self.support
            || built.support_loop != self.support_loop
            || built.caps != self.caps
            || built.map != self.source_map
        {
            return Err(error(
                "Saved original feature region/closure binding changed",
            ));
        }
        let mut proof = qualification::verify(&self.tool, built.work.remaining(limits)?)?;
        built.work.include(&mut proof);
        Ok(proof)
    }
}
pub fn extract(
    source: &Model,
    faces: &[usize],
    role: FeatureRole,
    limits: Limits,
) -> Result<FeatureRegion> {
    qualification::limits(limits)?;
    let built = extraction::build(source, faces, role, limits)?;
    let mut evidence = qualification::verify(&built.model, built.work.remaining(limits)?)?;
    built.work.include(&mut evidence);
    let mut selected = faces.to_vec();
    selected.sort();
    Ok(FeatureRegion {
        source: source.clone(),
        tool: built.model,
        selected,
        support: built.supports,
        support_loop: built.support_loop,
        role,
        caps: built.caps,
        source_map: built.map,
        limits,
        evidence,
    })
}
#[derive(Clone, Debug)]
pub struct Placement {
    pub key: String,
    pub matrix: [[f64; 4]; 4],
}
pub struct Instance {
    placement: Placement,
    model: Model,
    tool_ids: BTreeMap<TopoId, TopoId>,
    source_ids: BTreeMap<TopoId, TopoId>,
}
impl Instance {
    pub fn placement(&self) -> &Placement {
        &self.placement
    }
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn tool_ids(&self) -> &BTreeMap<TopoId, TopoId> {
        &self.tool_ids
    }
    pub fn source_ids(&self) -> &BTreeMap<TopoId, TopoId> {
        &self.source_ids
    }
}
pub struct Pattern {
    source: Model,
    feature: FeatureRegion,
    model: Model,
    instances: Vec<Instance>,
    mode: PatternMode,
    key: String,
    limits: Limits,
    evidence: Verification,
}
impl Pattern {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn source(&self) -> &Model {
        &self.source
    }
    pub fn feature(&self) -> &FeatureRegion {
        &self.feature
    }
    pub fn instances(&self) -> &[Instance] {
        &self.instances
    }
    pub fn evidence(&self) -> &Verification {
        &self.evidence
    }
    /// Current survivors of an occurrence ID through actual retained Boolean lineage.
    pub fn descendants(&self, id: TopoId) -> Vec<TopoId> {
        naming::descendants(&self.model, id)
    }
    /// Replay original extraction, placements and native Boolean operations. No
    /// stored caller receipt substitutes for the actual saved geometry.
    pub fn reverify(
        &self,
        budget: &nurbs_core::foundation::guards::Budget,
    ) -> Result<Verification> {
        let placements = self
            .instances
            .iter()
            .map(|i| i.placement.clone())
            .collect::<Vec<_>>();
        let replay = pattern(
            &self.source,
            &self.feature,
            &placements,
            self.mode,
            &self.key,
            self.limits,
            budget,
        )?;
        if replay.model != self.model
            || replay.instances.iter().zip(&self.instances).any(|(a, b)| {
                a.model != b.model || a.tool_ids != b.tool_ids || a.source_ids != b.source_ids
            })
        {
            return Err(error(
                "Actual saved feature pattern differs from original Boolean replay",
            ));
        }
        Ok(replay.evidence)
    }
}
pub fn pattern(
    destination: &Model,
    feature: &FeatureRegion,
    placements: &[Placement],
    mode: PatternMode,
    key: &str,
    limits: Limits,
    budget: &nurbs_core::foundation::guards::Budget,
) -> Result<Pattern> {
    qualification::limits(limits)?;
    destination.validate()?;
    if key.is_empty()
        || key.len() > 128
        || placements.is_empty()
        || placements.len() > 8
        || placements
            .iter()
            .any(|p| p.key.is_empty() || p.key.len() > 128)
        || placements
            .iter()
            .map(|p| &p.key)
            .collect::<BTreeSet<_>>()
            .len()
            != placements.len()
    {
        return Err(error(
            "Choose 1..8 unique named feature placements and a bounded pattern key",
        ));
    }
    let first = feature.reverify(limits)?;
    let mut remaining = Limits {
        max_cells: limits.max_cells.saturating_sub(first.cells),
        max_pairs: limits.max_pairs.saturating_sub(first.pairs),
        max_domain_cells: limits.max_domain_cells.saturating_sub(first.domain_cells),
        ..limits
    };
    let mut guard = budget.guard("original-feature-region-pattern");
    let mut result = destination.clone();
    let mut instances = Vec::new();
    for placement in placements {
        guard.tick()?;
        let moved = crate::transform::affine(feature.tool(), placement.matrix)?;
        let clock_work = trim_clocks::verify_occurrence(feature, &moved, remaining)?;
        remaining = clock_work.remaining(remaining)?;
        let (moved, tool_ids) = naming::instance(moved, feature.tool(), key, &placement.key)?;
        let active = naming::ids(&result);
        if tool_ids.values().any(|id| active.contains(id)) {
            return Err(error(
                "Feature occurrence key already exists in the destination",
            ));
        }
        let checked = qualification::verify(&moved, remaining)?;
        remaining.max_cells = remaining.max_cells.saturating_sub(checked.cells);
        remaining.max_pairs = remaining.max_pairs.saturating_sub(checked.pairs);
        remaining.max_domain_cells = remaining
            .max_domain_cells
            .saturating_sub(checked.domain_cells);
        let mut next = crate::operations::boolean(
            &result,
            &moved,
            match mode {
                PatternMode::Join => "union",
                PatternMode::Cut => "difference",
            },
        )?;
        next.inherit_topology_ids(&[&result, &moved]);
        next.validate()?;
        let source_ids = feature
            .source_map
            .iter()
            .map(|(a, b)| (*a, tool_ids[b]))
            .collect();
        instances.push(Instance {
            placement: placement.clone(),
            model: moved,
            tool_ids,
            source_ids,
        });
        result = next;
    }
    let mut evidence = qualification::verify(&result, remaining)?;
    evidence.cells += limits.max_cells - remaining.max_cells;
    evidence.pairs += limits.max_pairs - remaining.max_pairs;
    evidence.domain_cells += limits.max_domain_cells - remaining.max_domain_cells;
    Ok(Pattern {
        source: destination.clone(),
        feature: feature.clone(),
        model: result,
        instances,
        mode,
        key: key.into(),
        limits,
        evidence,
    })
}
