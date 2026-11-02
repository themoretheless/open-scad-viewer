//! Material-admitted exact native Full Round result. The output is the actual
//! rational topology owned by the certificates, never the old rounded Model.
use super::*;
#[cfg(feature = "codec")]
mod geometry_value;
pub mod lineage;
pub mod recipe;
#[cfg(feature = "codec")]
pub mod transport;
use crate::profile_certificate::original_restriction::wire::{
    material, prism, radius::Radius, SmoothEdits,
};
use material_relation::exact::NoAddedExactMaterial;
#[derive(Clone, Debug)]
pub struct AdmittedFullRound {
    material: NoAddedExactMaterial,
    radius: Radius,
    tangency: SmoothEdits,
    selected_faces: [usize; 3],
    recipe: recipe::Recipe,
}
pub fn build_admitted(
    source: &Model,
    faces: [usize; 3],
    limits: material::Limits,
) -> Result<AdmittedFullRound> {
    let proposed = build(source, faces)?;
    let material = proposed
        .inspect_exact_source_material(limits)?
        .require_no_addition()?;
    let admitted = AdmittedFullRound {
        material,
        radius: proposed.exact_radius().clone(),
        tangency: proposed.exact_smoothness().clone(),
        selected_faces: faces,
        recipe: recipe::Recipe::capture(&proposed),
    };
    admitted.verify_ownership()?;
    Ok(admitted)
}
impl AdmittedFullRound {
    pub fn lineage(&self) -> Result<lineage::Lineage> {
        lineage::derive(self)
    }
    pub fn recipe(&self) -> &recipe::Recipe {
        &self.recipe
    }
    pub fn source(&self) -> &Model {
        self.material.source().boundary().source()
    }
    pub fn model(&self) -> &prism::Topology {
        self.material.result().model()
    }
    pub fn material(&self) -> &NoAddedExactMaterial {
        &self.material
    }
    pub fn radius(&self) -> &Radius {
        &self.radius
    }
    pub fn tangency(&self) -> &SmoothEdits {
        &self.tangency
    }
    pub fn selected_faces(&self) -> [usize; 3] {
        self.selected_faces
    }
    fn verify_ownership(&self) -> Result<()> {
        let outer = self
            .material
            .result()
            .shell_geometry()
            .source_profiles()
            .first()
            .ok_or_else(|| error("Admitted Full Round has no exact outer profile"))?;
        if outer.pieces() != self.radius.wire().pieces()
            || outer.pieces() != self.tangency.wire().pieces()
            || self
                .tangency
                .junctions()
                .iter()
                .any(|j| j.sine_squared != ["0".to_string(), "1".to_string()])
            || self.model().bodies.len() != 1
        {
            return Err(error(
                "Admitted Full Round certificates do not own the output geometry",
            ));
        }
        Ok(())
    }
    pub fn reverify(&self) -> Result<()> {
        self.material.reverify()?;
        self.radius.reverify()?;
        self.tangency.reverify()?;
        self.verify_ownership()
    }
}
