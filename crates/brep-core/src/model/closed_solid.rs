//! Immutable material ownership after continuous boundary, nesting and material
//! orientation proofs. A topology/sampled solid audit cannot construct this type.
use crate::{Error, Model, Result, ray_parity, volume_validity};
use std::sync::Arc;
#[derive(Clone, Copy)]
pub struct AdmissionPolicy {
    pub tolerance_uv: f64,
    pub max_linear_cells: usize,
    pub geometry: volume_validity::Limits,
}
#[derive(Clone)]
pub struct QualifiedClosedSolid {
    model: Arc<Model>,
    geometry: Arc<volume_validity::Report>,
    policy: AdmissionPolicy,
}
pub struct ClosedSolidAdmission {
    model: Arc<Model>,
    geometry: Arc<volume_validity::Report>,
    solid: Option<QualifiedClosedSolid>,
}
impl ClosedSolidAdmission {
    pub fn geometry(&self) -> &volume_validity::Report {
        &self.geometry
    }
    pub fn candidate(&self) -> &Model {
        &self.model
    }
    pub fn solid(&self) -> Option<&QualifiedClosedSolid> {
        self.solid.as_ref()
    }
    pub fn into_solid(self) -> Option<QualifiedClosedSolid> {
        self.solid
    }
}
impl QualifiedClosedSolid {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn geometry(&self) -> &volume_validity::Report {
        &self.geometry
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = qualify((*self.model).clone(), self.policy)?;
        if fresh.solid.is_none() {
            return Err(Error::new(
                "BREP_CLOSED_SOLID_UNPROVED",
                "Closed material geometry could not be re-established",
            ));
        }
        Ok(())
    }
    /// The owned closed embedded material boundary establishes the precondition
    /// which a standalone parity report deliberately leaves to its caller.
    pub fn classify_point(
        &self,
        point: [f64; 3],
        directions: &[[f64; 3]],
        max_cells: usize,
        max_domain_cells: usize,
    ) -> Result<ray_parity::PointReport> {
        ray_parity::classify_point(
            &self.model,
            point,
            directions,
            self.policy.tolerance_uv,
            max_cells,
            max_domain_cells,
        )
    }
}
pub fn qualify(model: Model, policy: AdmissionPolicy) -> Result<ClosedSolidAdmission> {
    if model.bodies.is_empty() || model.shells.is_empty() || model.shells.iter().any(|s| !s.closed)
    {
        return Err(Error::new(
            "BREP_CLOSED_SOLID_UNPROVED",
            "Material ownership requires nonempty closed body shells",
        ));
    }
    let model = Arc::new(model);
    let geometry = Arc::new(volume_validity::inspect_with_linear(
        &model,
        policy.tolerance_uv,
        policy.geometry,
        policy.max_linear_cells,
    )?);
    let solid = geometry.proven.then(|| QualifiedClosedSolid {
        model: model.clone(),
        geometry: geometry.clone(),
        policy,
    });
    Ok(ClosedSolidAdmission {
        model,
        geometry,
        solid,
    })
}
#[cfg(feature = "codec")]
mod healing;
#[cfg(feature = "codec")]
pub use healing::{GeometricHealing, GeometricHealingStatus, heal_and_qualify};
mod shell_repair;
pub use shell_repair::{ShellRepair, ShellRepairPolicy, rebuild_even_odd_shells};
mod material_seed;
pub use material_seed::{
    QualifiedSurfaceMaterialSeed, SurfaceMaterialLimits, qualify_surface_material_seed,
};
mod boolean_material;
pub use boolean_material::{
    BooleanFace, BooleanFaceSeedLimits, BooleanMaterialOp, BooleanSourceSide,
    QualifiedBooleanFaceSeed, qualify_boolean_face_seed,
};
mod volume;
pub use volume::{VolumeLimits, VolumeReport};
mod material_box;
mod separation;
pub use material_box::{
    QualifiedSurfaceMaterialBox, SurfaceMaterialBoxLimits, qualify_surface_material_box,
};
mod boolean_box;
pub use boolean_box::{BooleanFaceBoxLimits, QualifiedBooleanFaceBox, qualify_boolean_face_box};
