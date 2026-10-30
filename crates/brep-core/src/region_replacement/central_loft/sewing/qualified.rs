//! Immutable admission bound to the same owned candidate, never a supplied report.
use super::*;
pub struct QualifiedSewnCorner<'a> {
    candidate: &'a SewnCornerCandidate,
    report: CornerBodyAudit,
}
impl SewnCornerCandidate {
    pub fn qualify(
        &self,
        tolerance_uv: f64,
        limits: CornerAuditLimits,
    ) -> Result<QualifiedSewnCorner<'_>> {
        require(
            self.strip_fits.len() == 3 && self.final_jets.len() >= 12 && self.radii.len() == 3,
            "Corner qualification needs full strip displacement and actual sewn G2 evidence",
        )?;
        let report = self.audit(tolerance_uv, limits)?;
        require(
            report.absence_proven
                && report.exact_boundary
                && report.volume_bounds_mm3.is_some()
                && report.material_preservation_proven,
            "Closed sewn corner embedding, material or volume remains unproven",
        )?;
        Ok(QualifiedSewnCorner {
            candidate: self,
            report,
        })
    }
}
impl QualifiedSewnCorner<'_> {
    pub fn model(&self) -> &Model {
        self.candidate.model()
    }
    pub fn report(&self) -> &CornerBodyAudit {
        &self.report
    }
    pub fn volume_mm3(&self) -> [f64; 2] {
        self.report.volume_bounds_mm3.unwrap()
    }
    pub fn removed_volume_mm3(&self) -> [f64; 2] {
        self.report.removed_volume_bounds_mm3.unwrap()
    }
}
