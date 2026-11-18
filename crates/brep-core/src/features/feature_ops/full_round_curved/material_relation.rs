//! Exact relation of actual stored source/result profile regions. This stage
//! does not replace independent boundary embedding and material-volume proofs.
use super::{CurvedFullRound, source_cover::original_profile};
pub mod exact;
use crate::{Model, Result, feature_ops::error, profile_certificate::material_relation as m};

pub struct SourceProfileRelation {
    source: Model,
    result: Model,
    relation: m::Relation,
}
pub struct NoAddedProfile {
    source: Model,
    result: Model,
    certificate: m::NoAddition,
}
impl SourceProfileRelation {
    pub fn source(&self) -> &Model {
        &self.source
    }
    pub fn result(&self) -> &Model {
        &self.result
    }
    pub fn relation(&self) -> &m::Relation {
        &self.relation
    }
    pub fn require_no_addition(self) -> Result<NoAddedProfile> {
        Ok(NoAddedProfile {
            source: self.source,
            result: self.result,
            certificate: self.relation.require_no_addition()?,
        })
    }
}
impl NoAddedProfile {
    pub fn source(&self) -> &Model {
        &self.source
    }
    pub fn result(&self) -> &Model {
        &self.result
    }
    pub fn certificate(&self) -> &m::NoAddition {
        &self.certificate
    }
}
impl CurvedFullRound {
    /// Exact regularized nonzero-winding profiles from raw original BRep edges.
    /// No tolerance band or rounded Boolean output authorizes NoAddedProfile.
    pub fn inspect_source_profile_relation(
        &self,
        width: f64,
        limits: m::Limits,
    ) -> Result<SourceProfileRelation> {
        let p = crate::prism::recognize(self.source())?
            .ok_or_else(|| error("Source material relation requires the original XY prism"))?;
        let source = original_profile(self.source(), p.z_max)?;
        let result = original_profile(self.model(), p.z_max)?;
        let relation = m::inspect(&source, &result, width, limits)?;
        Ok(SourceProfileRelation {
            source: self.source().clone(),
            result: self.model().clone(),
            relation,
        })
    }
}
