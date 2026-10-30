//! Closed incidence of the actual central patch, three strips and retained stock.
//! Closed metadata never grants embedding, material-volume or Solid authority.
use super::*;
use crate::Model;
#[path = "sewing/build.rs"]
mod build;
#[path = "sewing/planar.rs"]
mod planar;
#[path = "sewing/topology.rs"]
mod topology;
#[derive(Clone, Copy)]
pub struct CornerSewLimits {
    pub cells: usize,
    pub exact_work: u64,
    pub trim_pairs: usize,
    pub domain_cells: usize,
}
pub struct SewnCornerCandidate {
    model: Model,
    stock_bounds: [[f64; 2]; 3],
    stock_volume: [f64; 2],
    corner_point: [f64; 3],
    surface_fit_upper_mm: f64,
    strip_fits: Vec<StripFitReport>,
    final_jets: Vec<nurbs_core::surface_g2_pcurves::ExtrusionBoundaryReport>,
    radii: Vec<crate::fillet_radius::Report>,
    closure: crate::watertight::QualifiedReport,
    trims: crate::face_domain::TrimAudit,
    central_face: usize,
    strip_faces: [usize; 3],
    cap_faces: [usize; 3],
    source_vertices: Vec<Option<usize>>,
    pub cells: usize,
    pub exact_work: u64,
}
impl SewnCornerCandidate {
    /// Structurally closed B-rep candidate, not a qualified embedded Solid.
    pub fn surface_fit_upper_mm(&self) -> f64 {
        self.surface_fit_upper_mm
    }
    pub fn radii(&self) -> &[crate::fillet_radius::Report] {
        &self.radii
    }
    pub fn final_jets(&self) -> &[nurbs_core::surface_g2_pcurves::ExtrusionBoundaryReport] {
        &self.final_jets
    }
    pub fn strip_fits(&self) -> &[StripFitReport] {
        &self.strip_fits
    }
    pub fn closure(&self) -> &crate::watertight::QualifiedReport {
        &self.closure
    }
    pub fn trims(&self) -> &crate::face_domain::TrimAudit {
        &self.trims
    }
    pub fn central_face(&self) -> usize {
        self.central_face
    }
    pub fn strip_faces(&self) -> [usize; 3] {
        self.strip_faces
    }
    pub fn cap_faces(&self) -> [usize; 3] {
        self.cap_faces
    }
    pub fn source_vertices(&self) -> &[Option<usize>] {
        &self.source_vertices
    }
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn closed_boundary_proven(&self) -> bool {
        self.closure.closure.status == crate::watertight::Closure::WithinTolerance
            && self.trims.all_valid
    }
}
impl CentralLoftCandidate<'_, '_, '_> {
    pub fn sew_corner_rounded(&self, limits: CornerSewLimits) -> Result<SewnCornerCandidate> {
        raw_build::assemble(self, limits)
    }
    pub fn sew_corner(&self, limits: CornerSewLimits) -> Result<SewnCornerCandidate> {
        build::assemble(self, limits)
    }
}

#[cfg(test)]
#[path = "sewing/tests.rs"]
mod tests;

#[path = "sewing/audit.rs"]
mod audit;
#[path = "sewing/flux.rs"]
mod flux;
#[path = "sewing/material.rs"]
mod material;
pub use audit::{CornerAuditLimits, CornerBodyAudit};
pub use flux::{FluxLimits, FluxReport};
pub use material::MaterialReport;

#[path = "sewing/fitted.rs"]
mod fitted;
#[path = "sewing/raw_build.rs"]
mod raw_build;
#[path = "sewing/reclock.rs"]
mod reclock;

#[path = "sewing/projection.rs"]
mod projection;
pub use projection::ProjectionDiskReport;

#[path = "sewing/control_fit.rs"]
mod control_fit;

#[path = "sewing/contacts.rs"]
mod contacts;
pub use contacts::ContactReport;

#[path = "sewing/strip_fit.rs"]
mod strip_fit;
pub use strip_fit::StripFitReport;

#[path = "sewing/final_jets.rs"]
mod final_jets;

#[path = "sewing/qualified.rs"]
mod qualified;
pub use qualified::QualifiedSewnCorner;

#[path = "sewing/strip_jets.rs"]
mod strip_jets;
