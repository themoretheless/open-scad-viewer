//! Source-bound finite cylindrical ZA worm body candidates.
pub use crate::thread::FiniteThreadLimits as BuildLimits;
use crate::{Error, Model, Result};
use nurbs_core::worm::{QualifiedTrimmedPatches, Spec};
mod wheel;
pub use wheel::{WheelBuildOptions, finite_wheel_with_certificates};
pub use wheel::{WheelCapLimits, WheelCaps, wheel_caps, WheelBodyCandidate, wheel_body_candidate};
fn refuse(s: &str) -> Error {
    Error::new("BREP_WORM_UNPROVEN", s)
}
pub struct CertifiedFiniteWorm {
    source: QualifiedTrimmedPatches,
    candidate: crate::thread::FiniteThreadCandidate,
    caps: Vec<crate::thread::CapProfileProof>,
    boundary: crate::boundary_agreement::Report,
    original_wall_indices: Vec<usize>,
}
impl CertifiedFiniteWorm {
    pub fn mass(&self) -> &crate::analysis::MassProperties {
        &self.candidate.body.mass
    }
    pub fn model(&self) -> &Model {
        &self.candidate.body.model
    }
    pub fn source(&self) -> &QualifiedTrimmedPatches {
        &self.source
    }
    pub fn caps(&self) -> &[crate::thread::CapProfileProof] {
        &self.caps
    }
    pub fn boundary_agreement(&self) -> &crate::boundary_agreement::Report {
        &self.boundary
    }
    pub fn original_wall_indices(&self) -> &[usize] {
        &self.original_wall_indices
    }
    pub fn solid_geometry_certified(&self) -> bool {
        false
    }
    pub fn ideal_finite_body_error_upper(&self) -> Option<f64> {
        None
    }
    pub fn into_model(self) -> Model {
        self.candidate.body.model
    }
}
/// Original ZA patch accuracy, complete slab/UV coverage, exact represented cap
/// profiles and final continuous shared-edge agreement. End slabs must admit
/// complete connected cap contours inside the original untrimmed coverage.
/// Whole-solid embedding and combined ideal finite-body error remain separate.
pub fn finite_with_certificates(
    spec: Spec,
    z_limits: [f64; 2],
    limits: BuildLimits,
) -> Result<CertifiedFiniteWorm> {
    if !(1..=10000000).contains(&limits.agreement_cells)
        || !(1..=100000).contains(&limits.trim_pairs_per_cap)
        || !(1..=100000).contains(&limits.trim_cells_per_cap)
        || !(1..=1000000).contains(&limits.domain_cells_per_cap)
    {
        return Err(refuse(
            "Finite ZA worm needs bounded positive agreement and cap proof limits",
        ));
    }
    let source = nurbs_core::worm::trimmed_patches_certified(
        spec,
        z_limits,
        limits.uv_error,
        limits.tolerance_mm,
        2560,
    )?;
    let network = crate::thread::clipped_patch_network(
        source.trims(),
        spec.starts,
        z_limits,
        limits.uv_error,
        limits.tolerance_mm,
        limits.max_spans,
        limits.max_curve_change,
        limits.agreement_cells,
    )?;
    let (mut candidate, caps) =
        crate::thread::cap_external_screw_network(network, spec.hand, z_limits, limits)?;
    let trims = source.trims();
    if candidate.side_faces != trims.candidates.len()
        || candidate.body.model.faces.len() != candidate.side_faces + 2
    {
        return Err(refuse(
            "Finite ZA worm omits original retained wall faces or caps",
        ));
    }
    let mut seen = vec![false; trims.candidates.len()];
    let mut indices = Vec::new();
    for face in &candidate.body.model.faces[..candidate.side_faces] {
        let index = trims
            .candidates
            .iter()
            .enumerate()
            .position(|(i, c)| !seen[i] && c.patch.surface == face.surface)
            .ok_or_else(|| refuse("Finite ZA wall is not an admitted original surface"))?;
        seen[index] = true;
        let p = &trims.candidates[index].patch;
        indices.push(((p.start * spec.turns + p.turn) * 4 + p.quarter) * 5 + p.profile_segment);
    }
    if !seen.iter().all(|x| *x) {
        return Err(refuse("Finite ZA retained-source census is incomplete"));
    }
    for cap in &caps {
        crate::profile_certificate::verify(cap.wires(), limits.tolerance_mm, cap.certificate())?;
    }
    let remaining = limits
        .agreement_cells
        .checked_sub(candidate.agreement_cells)
        .ok_or_else(|| refuse("ZA worm agreement accounting exceeded the shared budget"))?
        .min(1000000);
    if remaining == 0 {
        return Err(refuse(
            "No budget remains for complete saved ZA boundary verification",
        ));
    }
    let boundary = crate::boundary_agreement::verify(&candidate.body.model, remaining)?;
    if !boundary.complete {
        return Err(refuse("Finite ZA worm boundary agreement remains unproved"));
    }
    candidate.agreement_cells += boundary.cells;
    Ok(CertifiedFiniteWorm {
        source,
        candidate,
        caps,
        boundary,
        original_wall_indices: indices,
    })
}
