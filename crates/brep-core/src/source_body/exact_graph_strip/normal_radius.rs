//! Declared-direction normal-radius diagnostic, not circular-fillet admission.
use super::*;
use nurbs_core::original_graph_transition::{NormalRadiusDecision, NormalRadiusReport};
pub struct TransitionNormalRadiusAudit {
    reports: Vec<(usize, NormalRadiusReport)>,
}
impl TransitionNormalRadiusAudit {
    pub fn reports(&self) -> &[(usize, NormalRadiusReport)] {
        &self.reports
    }
    /// Passes the declared Sv radius constraint only, not a rolling-ball proof.
    pub fn declared_direction_within(&self) -> bool {
        !self.reports.is_empty()
            && self
                .reports
                .iter()
                .all(|(_, r)| r.decision == NormalRadiusDecision::Within)
    }
}
impl NativeVolume {
    /// Explicit normalized domain on EVERY transition, with no silent exclusion
    /// of source rims/runouts. Original retained/imported faces are not fillets.
    pub fn inspect_transition_normal_radius(
        &self,
        radius_mm: f64,
        tolerance_mm: f64,
        domain: [[f64; 2]; 2],
        max_cells: usize,
        b: &mut Budget,
    ) -> Result<TransitionNormalRadiusAudit> {
        inspect_roof_geometry(self.topology(), b)?;
        let mut reports = Vec::new();
        for (face, f) in self.topology().faces.iter().enumerate() {
            if let FaceGeometry::Transition(p) = &f.surface {
                reports.push((
                    face,
                    p.inspect_normal_radius(radius_mm, tolerance_mm, domain, max_cells, b)?,
                ));
            }
        }
        Ok(TransitionNormalRadiusAudit { reports })
    }
}
