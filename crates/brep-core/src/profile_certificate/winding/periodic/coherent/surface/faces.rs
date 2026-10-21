//! Owned faces on the original support; artificial chart cuts are sewn only
//! through their original connector provenance and a fresh full lift proof.
use super::super::super::face_build::{self, FaceContext};
use super::*;
use crate::Model;
use nurbs_core::surface_injectivity;
#[derive(Clone, Copy, Debug)]
pub struct PeriodicFaceLimits {
    pub tolerance_world: f64,
    pub injectivity_spans: usize,
    /// Complete-cover refinements, 0..6; all levels share injectivity_spans.
    pub injectivity_depth: usize,
    pub lift_cells: usize,
    pub agreement_cells: usize,
}
impl Default for PeriodicFaceLimits {
    fn default() -> Self {
        Self {
            tolerance_world: 1e-7,
            injectivity_spans: 100000,
            injectivity_depth: 4,
            lift_cells: 100000,
            agreement_cells: 1000000,
        }
    }
}
impl PeriodicFaceLimits {
    pub(in crate::profile_certificate::winding::periodic) fn validate(self) -> Result<()> {
        if !(1e-10..=1e-2).contains(&self.tolerance_world)
            || !(1..=100000).contains(&self.injectivity_spans)
            || self.injectivity_depth > 6
            || !(1..=100000).contains(&self.lift_cells)
            || !(1..=1000000).contains(&self.agreement_cells)
        {
            return Err(refused(
                "Invalid periodic face tolerance or shared proof allowance",
            ));
        }
        Ok(())
    }
}
pub struct OwnedPeriodicFaces {
    source: SurfacePeriodicRegions,
    limits: PeriodicFaceLimits,
    model: Model,
    domains: Vec<[[f64; 2]; 2]>,
    injectivity: Vec<surface_injectivity::Report>,
    seam_edges: Vec<usize>,
    agreement: crate::boundary_agreement::Report,
    boundary_error_upper: f64,
}
impl OwnedPeriodicFaces {
    pub fn source(&self) -> &SurfacePeriodicRegions {
        &self.source
    }
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn domains(&self) -> &[[[f64; 2]; 2]] {
        &self.domains
    }
    pub fn injectivity(&self) -> &[surface_injectivity::Report] {
        &self.injectivity
    }
    pub fn seam_edges(&self) -> &[usize] {
        &self.seam_edges
    }
    pub fn agreement(&self) -> &crate::boundary_agreement::Report {
        &self.agreement
    }
    /// Original retained boundary to actual owned world edge. Surface-region
    /// isotopy, face-local injectivity and cross-face spatial embedding differ.
    pub fn boundary_error_upper(&self) -> f64 {
        self.boundary_error_upper
    }
    pub fn global_spatial_embedding_certified(&self) -> bool {
        false
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = assemble_periodic_surface_faces(&self.source, self.limits)?;
        if fresh.model != self.model
            || fresh.domains != self.domains
            || fresh.seam_edges != self.seam_edges
            || !self.agreement.complete
            || self.boundary_error_upper != fresh.boundary_error_upper
        {
            return Err(refused(
                "Owned periodic model or its source-bound receipt changed",
            ));
        }
        Ok(())
    }
    /// Independently rebuild the source-owned topology before accepting an
    /// actual saved model. Edits cannot inherit a previous source certificate.
    pub fn verify_model(&self, model: &Model) -> Result<()> {
        let fresh = assemble_periodic_surface_faces(&self.source, self.limits)?;
        if fresh.model != *model {
            return Err(refused(
                "Saved periodic model differs from its complete original source ownership",
            ));
        }
        Ok(())
    }
}
/// Construct every retained region on an unchanged supporting surface. Every
/// original generated connector needs exactly one opposite chart mate. Source
/// boundary curves are never paired merely because their endpoints coincide.
/// The returned model owns open/closed sheet shells, never a manufactured Body.
pub fn assemble_periodic_surface_faces(
    source: &SurfacePeriodicRegions,
    limits: PeriodicFaceLimits,
) -> Result<OwnedPeriodicFaces> {
    limits.validate()?;
    source.reverify_source()?;
    let built = face_build::build(
        FaceContext {
            surface: &source.surface,
            wrapped: source.wrapped,
            chart: source.regions.chart,
            components: source.regions.candidate_regions().components(),
            cycle_edges: &source.regions.cycle_edges,
            graph: admitted(&source.regions.assembly)?,
            error_upper: source.correspondence.error_upper,
        },
        limits,
    )?;
    Ok(OwnedPeriodicFaces {
        source: source.clone(),
        limits,
        model: built.model,
        domains: built.domains,
        injectivity: built.injectivity,
        seam_edges: built.seam_edges,
        agreement: built.agreement,
        boundary_error_upper: built.boundary_error_upper,
    })
}
