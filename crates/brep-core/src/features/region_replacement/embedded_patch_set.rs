//! Admission binds fresh whole-boundary embedding evidence to the saved body.
use super::{ContactQualificationLimits, OwnedPatchSet, RetainedPatchBody};
use crate::{Model, boundary_embedding, face_cut};

#[derive(Clone, Copy)]
pub struct EmbeddedPatchLimits {
    /// Shared trim/sew/G2/curvature ledger.
    pub construction: ContactQualificationLimits,
    /// Independent explicit budgets for the subsequent whole-boundary audit.
    pub global: boundary_embedding::Limits,
    pub linear_cells: usize,
}
/// Source roles allow unresolved final faces to be traced back without
/// publishing an unproven candidate as an admitted result.
pub struct GlobalPatchDiagnostic {
    report: Box<boundary_embedding::Report>,
    source_faces: Vec<Vec<usize>>,
    patch_faces: Vec<usize>,
    limits: EmbeddedPatchLimits,
    tolerance_uv: f64,
}
impl GlobalPatchDiagnostic {
    pub fn limits(&self) -> EmbeddedPatchLimits {
        self.limits
    }
    pub fn tolerance_uv(&self) -> f64 {
        self.tolerance_uv
    }
    pub fn report(&self) -> &boundary_embedding::Report {
        &self.report
    }
    pub fn source_faces(&self) -> &[Vec<usize>] {
        &self.source_faces
    }
    pub fn patch_faces(&self) -> &[usize] {
        &self.patch_faces
    }
}
pub enum EmbeddedPatchFailure {
    Construction(nurbs_core::Error),
    GlobalAudit(nurbs_core::Error),
    Unproven(GlobalPatchDiagnostic),
}
impl std::fmt::Debug for EmbeddedPatchFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Construction(e) => f.debug_tuple("Construction").field(e).finish(),
            Self::GlobalAudit(e) => f.debug_tuple("GlobalAudit").field(e).finish(),
            Self::Unproven(d) => f
                .debug_struct("GlobalUnproven")
                .field("exact_lifts", &d.report.agreement.all_equal)
                .field(
                    "injective_faces",
                    &d.report.intersections.faces.all_faces_injective,
                )
                .field(
                    "pairs_classified",
                    &d.report.intersections.pairs.all_pairs_classified,
                )
                .field("next_pair", &d.report.intersections.pairs.next_pair)
                .finish(),
        }
    }
}
impl std::fmt::Display for EmbeddedPatchFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Construction(e) => write!(f, "Trim/sew construction refused: {e:?}"),
            Self::GlobalAudit(e) => write!(f, "Global boundary audit failed: {e:?}"),
            Self::Unproven(_) => f.write_str("Whole-boundary embedding remains unproven"),
        }
    }
}
impl std::error::Error for EmbeddedPatchFailure {}
/// A closed G2/curvature-qualified patch body AND its actual fresh whole-boundary
/// embedding proof. Neither can be replaced independently. This does not certify
/// material nesting/orientation, volume, wall thickness or the radius law.
pub struct EmbeddedPatchBody {
    retained: RetainedPatchBody,
    global: boundary_embedding::Report,
    limits: EmbeddedPatchLimits,
    tolerance_uv: f64,
}
impl EmbeddedPatchBody {
    pub fn model(&self) -> &Model {
        self.retained.model()
    }
    pub fn retained(&self) -> &RetainedPatchBody {
        &self.retained
    }
    pub fn global(&self) -> &boundary_embedding::Report {
        &self.global
    }
    pub fn limits(&self) -> EmbeddedPatchLimits {
        self.limits
    }
    pub fn tolerance_uv(&self) -> f64 {
        self.tolerance_uv
    }
}
/// One immutable-source transaction. Constructor flags and external reports are
/// never accepted as evidence. No admitted body is returned when ANY global
/// component is incomplete, including an unvisited suffix of face pairs.
pub fn trim_and_sew_embedded_g2_patch_set<F>(
    source: &Model,
    requests: &[face_cut::FaceRetention<'_>],
    key: &str,
    tolerance_uv: f64,
    limits: EmbeddedPatchLimits,
    construct: F,
) -> std::result::Result<EmbeddedPatchBody, EmbeddedPatchFailure>
where
    F: FnOnce(&face_cut::RetentionNetwork) -> nurbs_core::Result<OwnedPatchSet>,
{
    let retained = super::trim_and_sew_g2_patch_set(
        source,
        requests,
        key,
        tolerance_uv,
        limits.construction,
        construct,
    )
    .map_err(EmbeddedPatchFailure::Construction)?;
    let global = boundary_embedding::inspect_with_linear(
        retained.model(),
        tolerance_uv,
        limits.global,
        limits.linear_cells,
    )
    .map_err(EmbeddedPatchFailure::GlobalAudit)?;
    if !global.proven {
        return Err(EmbeddedPatchFailure::Unproven(GlobalPatchDiagnostic {
            report: Box::new(global),
            source_faces: retained.source_faces().to_vec(),
            patch_faces: retained.patch_faces().to_vec(),
            limits,
            tolerance_uv,
        }));
    }
    Ok(EmbeddedPatchBody {
        retained,
        global,
        limits,
        tolerance_uv,
    })
}
