//! Finite thread candidates bound to certified original surfaces and clipping.
use super::*;
impl Default for FiniteThreadLimits {
    fn default() -> Self {
        Self {
            uv_error: 1e-12,
            tolerance_mm: 1e-3,
            max_spans: 128,
            max_curve_change: 1e-8,
            agreement_cells: 4_000_000,
            trim_pairs_per_cap: 32768,
            trim_cells_per_cap: 100000,
            domain_cells_per_cap: 1000000,
            mass_relative_tolerance: 1e-4,
            mass_evaluations: 2_000_000,
        }
    }
}
#[derive(Debug)]
pub struct CertifiedFiniteThread {
    candidate: FiniteThreadCandidate,
    patch_certificate: nurbs_core::thread::ThreadCertificate,
    trim_certificate: nurbs_core::thread::TrimCoverageCertificate,
    boundary: crate::boundary_agreement::Report,
    original_wall_indices: Vec<usize>,
    cap_profiles: Vec<internal::CapProfileProof>,
}
impl CertifiedFiniteThread {
    pub fn cap_profiles(&self) -> &[internal::CapProfileProof] {
        &self.cap_profiles
    }
    pub fn candidate(&self) -> &FiniteThreadCandidate {
        &self.candidate
    }
    pub fn patch_certificate(&self) -> &nurbs_core::thread::ThreadCertificate {
        &self.patch_certificate
    }
    pub fn trim_certificate(&self) -> &nurbs_core::thread::TrimCoverageCertificate {
        &self.trim_certificate
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
    /// Original-wall accuracy and axial clipping bands are separate. No full
    /// ideal finite-body Hausdorff/rounding certificate is currently issued.
    pub fn ideal_finite_body_error_upper(&self) -> Option<f64> {
        None
    }
    pub fn into_model(self) -> Model {
        self.candidate.body.model
    }
}
/// Construct a finite external body candidate with original nominal patch
/// accuracy, complete affine clipping/UV isotopy and current saved-boundary
/// agreement. Requested planes must admit complete connected end contours.
/// Whole-solid geometry and a combined ideal finite-body bound remain separate.
pub fn finite_external_with_certificates(
    spec: nurbs_core::thread::Spec,
    z_limits: [f64; 2],
    limits: FiniteThreadLimits,
) -> Result<CertifiedFiniteThread> {
    let fail = |message: &str| Error::new("BREP_THREAD_PATCH_UNRESOLVED", message);
    if spec.kind != nurbs_core::thread::Kind::External {
        return Err(fail(
            "External certified constructor needs external thread kind",
        ));
    }
    let (_, patch_certificate) = nurbs_core::thread::patches_certified(spec, 2560)?;
    let trims = nurbs_core::thread::trim_candidates(spec, z_limits, limits.tolerance_mm)?;
    let trim_certificate = nurbs_core::thread::certify_trim_candidates(
        spec,
        &trims,
        limits.uv_error,
        limits.tolerance_mm,
        2560,
    )?;
    let (candidate, cap_profiles) = internal::external_candidate(spec, z_limits, limits)?;
    admit(
        candidate,
        spec,
        trims,
        patch_certificate,
        trim_certificate,
        cap_profiles,
        limits,
    )
}
pub(super) fn admit(
    mut candidate: FiniteThreadCandidate,
    spec: nurbs_core::thread::Spec,
    trims: nurbs_core::thread::TrimCandidates,
    patch_certificate: nurbs_core::thread::ThreadCertificate,
    trim_certificate: nurbs_core::thread::TrimCoverageCertificate,
    cap_profiles: Vec<internal::CapProfileProof>,
    limits: FiniteThreadLimits,
) -> Result<CertifiedFiniteThread> {
    let fail = |message: &str| Error::new("BREP_THREAD_PATCH_UNRESOLVED", message);
    let thread_faces = candidate
        .side_faces
        .checked_sub(candidate.outer_wall_faces)
        .ok_or_else(|| fail("Invalid native wall face accounting"))?;
    if thread_faces != trims.candidates.len()
        || candidate.body.model.faces.len() != candidate.side_faces + 2
    {
        return Err(fail(
            "Finite body does not cover every retained original thread surface",
        ));
    }
    let mut used = vec![false; trims.candidates.len()];
    let mut original_wall_indices = Vec::new();
    for face in &candidate.body.model.faces[..thread_faces] {
        let index = trims
            .candidates
            .iter()
            .enumerate()
            .position(|(i, c)| !used[i] && c.patch.surface == face.surface)
            .ok_or_else(|| fail("Saved thread wall is not an admitted original surface"))?;
        used[index] = true;
        let p = &trims.candidates[index].patch;
        original_wall_indices
            .push(((p.start * spec.turns + p.turn) * 4 + p.quarter) * 5 + p.profile_segment);
    }
    if !used.iter().all(|v| *v) {
        return Err(fail("Finite body omitted an admitted retained patch"));
    }
    let remaining = limits
        .agreement_cells
        .checked_sub(candidate.agreement_cells)
        .ok_or_else(|| fail("Thread agreement accounting exceeded the shared budget"))?
        .min(1_000_000);
    if remaining == 0 {
        return Err(fail(
            "No shared budget remains for final boundary verification",
        ));
    }
    let boundary = crate::boundary_agreement::verify(&candidate.body.model, remaining)?;
    if !boundary.complete {
        return Err(fail("Final saved thread boundaries remain unresolved"));
    }
    candidate.agreement_cells += boundary.cells;
    Ok(CertifiedFiniteThread {
        candidate,
        patch_certificate,
        trim_certificate,
        boundary,
        original_wall_indices,
        cap_profiles,
    })
}
