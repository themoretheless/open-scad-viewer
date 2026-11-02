//! Filled-volume admission after fresh joint boundary, roles and orientation.
use super::Qualification;
use crate::{Model, Result, volume_validity};
use std::sync::Arc;

/// Immutable native Full Round construction admitted by the complete volume
/// inspector. This does not extend the geometric construction's admitted domain.
pub struct QualifiedVolume {
    qualification: Arc<Qualification>,
    report: volume_validity::Report,
}
impl QualifiedVolume {
    pub fn model(&self) -> &Model {
        self.qualification.model()
    }
    pub fn qualification(&self) -> &Qualification {
        &self.qualification
    }
    pub fn report(&self) -> &volume_validity::Report {
        &self.report
    }
}
impl Qualification {
    /// Recompute boundary embedding, shell nesting/roles and physical outward
    /// orientation on the immutable construction. Stage budgets are explicit;
    /// local G1/radius certificates or a volume estimate cannot authorize it.
    pub fn qualify_volume(
        self: Arc<Self>,
        tolerance_uv: f64,
        limits: volume_validity::Limits,
        max_linear_cells: usize,
    ) -> Result<QualifiedVolume> {
        let report = volume_validity::inspect_with_linear(
            self.model(),
            tolerance_uv,
            limits,
            max_linear_cells,
        )?;
        if !report.proven {
            return Err(crate::feature_ops::error(format!(
                "Full Round material volume remains unproven: boundary={}, roles={:?}, orientations={:?}",
                report.boundary.proven,
                report.nesting.as_ref().and_then(|r| r.roles_consistent),
                report
                    .orientations
                    .iter()
                    .map(|o| (o.expected_outward, o.outward))
                    .collect::<Vec<_>>(),
            )));
        }
        Ok(QualifiedVolume {
            qualification: self,
            report,
        })
    }
}
