//! Exact new Wire material versus explicit canonical original-source reference.
use super::super::*;
use crate::profile_certificate::original_restriction::wire::{
    jordan, material as m, nesting, prism,
};
use source_boundary::material::CanonicalMaterial;
#[derive(Clone, Debug)]
pub struct ExactMaterialRelation {
    source: CanonicalMaterial,
    result: prism::Solid,
    relation: m::Relation,
}
#[derive(Clone, Debug)]
pub struct NoAddedExactMaterial {
    source: CanonicalMaterial,
    result: prism::Solid,
    certificate: m::NoAddition,
}
fn same(a: &nesting::Nesting, b: &nesting::Nesting) -> bool {
    let a = a.jordan().sources();
    let b = b.jordan().sources();
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.pieces() == b.pieces())
}
impl ExactMaterialRelation {
    pub fn source(&self) -> &CanonicalMaterial {
        &self.source
    }
    pub fn result(&self) -> &prism::Solid {
        &self.result
    }
    pub fn relation(&self) -> &m::Relation {
        &self.relation
    }
    pub fn reverify(&self) -> Result<()> {
        self.source.reverify()?;
        self.result.reverify()?;
        self.relation.reverify()?;
        if self.source.solid().shell_geometry().height() != self.result.shell_geometry().height()
            || !same(self.source.solid().nesting(), self.relation.source())
            || !same(self.result.nesting(), self.relation.result())
        {
            return Err(error(
                "Exact material source/result ownership or height changed",
            ));
        }
        Ok(())
    }
    pub fn require_no_addition(self) -> Result<NoAddedExactMaterial> {
        Ok(NoAddedExactMaterial {
            source: self.source,
            result: self.result,
            certificate: self.relation.require_no_addition()?,
        })
    }
}
impl NoAddedExactMaterial {
    pub fn source(&self) -> &CanonicalMaterial {
        &self.source
    }
    pub fn result(&self) -> &prism::Solid {
        &self.result
    }
    pub fn certificate(&self) -> &m::NoAddition {
        &self.certificate
    }
    pub fn reverify(&self) -> Result<()> {
        self.source.reverify()?;
        self.result.reverify()?;
        let relation = self.certificate.relation();
        relation.reverify()?;
        if self.source.solid().shell_geometry().height() != self.result.shell_geometry().height()
            || !same(self.source.solid().nesting(), relation.source())
            || !same(self.result.nesting(), relation.result())
            || !relation.result_is_subset()
        {
            return Err(error(
                "No-added exact material source/result replay changed",
            ));
        }
        Ok(())
    }
}
impl CurvedFullRound {
    pub fn inspect_exact_source_material(
        &self,
        limits: m::Limits,
    ) -> Result<ExactMaterialRelation> {
        let source = self.qualify_source_boundary()?.canonical_material()?;
        let result =
            self.qualify_exact_solid(jordan::Limits::default(), nesting::Limits::default())?;
        inspect_owned(source, result, limits)
    }
}
pub(crate) fn inspect_owned(
    source: CanonicalMaterial,
    result: prism::Solid,
    limits: m::Limits,
) -> Result<ExactMaterialRelation> {
    if source.solid().shell_geometry().height() != result.shell_geometry().height() {
        return Err(error(
            "Exact material relation requires equal original/result heights",
        ));
    }
    let relation = m::inspect(source.solid().nesting(), result.nesting(), limits)?;
    Ok(ExactMaterialRelation {
        source,
        result,
        relation,
    })
}
