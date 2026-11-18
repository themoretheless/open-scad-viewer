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
    /// Return the entry and exit parameter enclosures of each material
    /// interval on a forward ray. An interior origin starts at [0, 0].
    /// A ray with unresolved, coincident or boundary-origin contacts is refused.
    pub fn ray_material_intervals(
        &self,
        origin: [f64; 3],
        direction: [f64; 3],
        max_cells: usize,
        max_domain_cells: usize,
    ) -> Result<Vec<[[f64; 2]; 2]>> {
        let report = ray_parity::classify_ray(
            &self.model,
            origin,
            direction,
            self.policy.tolerance_uv,
            max_cells,
            max_domain_cells,
        )?;
        let starts_inside = report.parity.ok_or_else(|| {
            Error::new("BREP_RAY_UNRESOLVED", "Material ray contains unresolved crossings")
        })?;
        let mut crossings = report.crossings.into_iter().map(|root| root.parameter);
        let mut intervals = Vec::new();
        if starts_inside {
            let exit = crossings.next().ok_or_else(|| {
                Error::new("BREP_RAY_UNRESOLVED", "Interior ray has no exit crossing")
            })?;
            intervals.push([[0., 0.], exit]);
        }
        while let Some(entry) = crossings.next() {
            let exit = crossings.next().ok_or_else(|| {
                Error::new("BREP_RAY_UNRESOLVED", "Material ray has an unmatched entry crossing")
            })?;
            intervals.push([entry, exit]);
        }
        Ok(intervals)
    }

    /// Enclose the length in millimetres of material on the ray segment
    /// `origin + t * direction`, for `0 <= t <= max_parameter`.
    pub fn ray_material_length_mm(
        &self,
        origin: [f64; 3],
        direction: [f64; 3],
        max_parameter: f64,
        max_cells: usize,
        max_domain_cells: usize,
    ) -> Result<[f64; 2]> {
        if !max_parameter.is_finite() || max_parameter < 0. {
            return Err(Error::new("BREP_INVALID_INPUT", "Ray segment requires a finite nonnegative end parameter"));
        }
        let intervals = self.ray_material_intervals(origin, direction, max_cells, max_domain_cells)?;
        let (mut lower, mut upper) = (0_f64, 0_f64);
        for [entry, exit] in intervals {
            let latest_entry = entry[1].min(max_parameter);
            let earliest_entry = entry[0].min(max_parameter);
            let earliest_exit = exit[0].min(max_parameter);
            let latest_exit = exit[1].min(max_parameter);
            if latest_exit > earliest_entry {
                lower = (lower + (earliest_exit - latest_entry).next_down().max(0.)).next_down().max(0.);
                upper = (upper + (latest_exit - earliest_entry).next_up()).next_up();
            }
        }
        if upper == 0. {
            return Ok([0.; 2]);
        }
        let scale = direction.iter().map(|value| value.abs()).fold(0., f64::max);
        let mut norm2 = [0_f64; 2];
        for value in direction.map(f64::abs) {
            if value == 0. {
                continue;
            }
            let ratio = value / scale;
            let ratio_lo = ratio.next_down().max(0.);
            let ratio_hi = ratio.next_up();
            norm2[0] = (norm2[0] + (ratio_lo * ratio_lo).next_down().max(0.)).next_down().max(0.);
            norm2[1] = (norm2[1] + (ratio_hi * ratio_hi).next_up()).next_up();
        }
        let lo = ((lower * scale).next_down().max(0.) * norm2[0].sqrt().next_down()).next_down().max(0.);
        let hi = ((upper * scale).next_up() * norm2[1].sqrt().next_up()).next_up();
        if !hi.is_finite() {
            return Err(Error::new("BREP_RAY_UNRESOLVED", "Material ray length exceeds finite millimetres"));
        }
        Ok([lo, hi])
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
pub use healing::{
    GeometricHealing, GeometricHealingStatus, heal_and_qualify, heal_and_rebuild_shells,
    sew_gaps_and_rebuild_shells,
};
mod shell_repair;
pub use shell_repair::{ShellRepair, ShellRepairPolicy, rebuild_even_odd_shells};
mod material_seed;
pub use material_seed::{
    QualifiedSurfaceMaterialSeed, SurfaceMaterialLimits, qualify_surface_material_seed,
};
mod boolean_material;
pub use boolean_material::{
    BooleanFace, BooleanFaceDisposition, BooleanFaceSeedLimits, BooleanMaterialOp, BooleanSourceSide,
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
