//! Explicit material reference after source-bound auxiliary UV canonicalization.
//! It is a separate exact rational body, not a mutation/admission of legacy UV.
use super::*;
use crate::profile_certificate::original_restriction::wire::{jordan, nesting, prism};
#[derive(Clone, Debug)]
pub struct CanonicalMaterial {
    boundary: SourceBoundary,
    solid: prism::Solid,
}
fn build(boundary: &SourceBoundary) -> Result<prism::Solid> {
    boundary.reverify()?;
    let model = boundary.source();
    let low = model
        .vertices
        .iter()
        .map(|v| v.point[2])
        .fold(f64::INFINITY, f64::min);
    let high = model
        .vertices
        .iter()
        .map(|v| v.point[2])
        .fold(f64::NEG_INFINITY, f64::max);
    // Native side-image factorization + exact complete horizontal/vertical
    // boundary census identify the material as native profile region × height.
    // Both cap regions use the explicit original-carrier pullbacks, so this
    // statement does not silently identify tolerated legacy UV curves with them.
    prism::extrude_binary64(
        boundary.profiles.region().jordan().sources(),
        [low, high],
        model.tolerance_mm,
    )?
    .qualify_solid(jordan::Limits::default(), nesting::Limits::default())
}
impl SourceBoundary {
    pub fn canonical_material(&self) -> Result<CanonicalMaterial> {
        Ok(CanonicalMaterial {
            boundary: self.clone(),
            solid: build(self)?,
        })
    }
}
impl CanonicalMaterial {
    pub fn boundary(&self) -> &SourceBoundary {
        &self.boundary
    }
    pub fn solid(&self) -> &prism::Solid {
        &self.solid
    }
    pub fn reverify(&self) -> Result<()> {
        self.solid.reverify()?;
        let expected = build(&self.boundary)?;
        if expected.model() != self.solid.model()
            || expected.shell_geometry().source_profiles().len()
                != self.solid.shell_geometry().source_profiles().len()
        {
            return Err(error("Canonical source material reference changed"));
        }
        for (a, b) in expected
            .shell_geometry()
            .source_profiles()
            .iter()
            .zip(self.solid.shell_geometry().source_profiles())
        {
            if a.pieces() != b.pieces() {
                return Err(error("Canonical material native profiles changed"));
            }
        }
        Ok(())
    }
}
