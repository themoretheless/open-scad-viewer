//! Independent source-input inverse-radius law for immutable admitted volumes.
use super::*;
use nurbs_core::original_graph_transition::{
    CurvatureLawDecision, SourceContactCurvatureLaw, SourceCurvatureLawReport,
};
use std::collections::BTreeSet;
pub struct TransitionSourceLawAudit {
    reports: Vec<(usize, SourceCurvatureLawReport)>,
}
impl TransitionSourceLawAudit {
    pub fn reports(&self) -> &[(usize, SourceCurvatureLawReport)] {
        &self.reports
    }
    pub fn all_within(&self) -> bool {
        !self.reports.is_empty()
            && self
                .reports
                .iter()
                .all(|(_, r)| r.decision == CurvatureLawDecision::Within)
    }
}
impl NativeVolume {
    /// Every transition must have exactly one matching reference. Geometry
    /// evidence is reused from this immutable admitted owner, not reissued
    /// against a rounded or differently revised model.
    pub fn inspect_transition_source_laws(
        &self,
        laws: &[(usize, SourceContactCurvatureLaw)],
        curvature_tolerance_inverse_mm: f64,
        radial_distance_tolerance_mm: f64,
        normal_chord_tolerance: f64,
        domain: [[f64; 2]; 2],
        b: &mut Budget,
    ) -> Result<TransitionSourceLawAudit> {
        let expected = self
            .topology()
            .faces
            .iter()
            .enumerate()
            .filter_map(|(i, f)| matches!(f.surface, FaceGeometry::Transition(_)).then_some(i))
            .collect::<BTreeSet<_>>();
        let provided = laws.iter().map(|(i, _)| *i).collect::<BTreeSet<_>>();
        require(
            expected == provided && provided.len() == laws.len(),
            "Source curvature laws must cover every transition exactly once",
        )?;
        for (face, law) in laws {
            let FaceGeometry::Transition(p) = &self.topology().faces[*face].surface else {
                unreachable!()
            };
            require(
                law.matches(p),
                "Source curvature law is stale or belongs to another transition",
            )?;
        }
        let mut reports = Vec::new();
        for (face, law) in laws {
            let FaceGeometry::Transition(p) = &self.topology().faces[*face].surface else {
                unreachable!()
            };
            reports.push((
                *face,
                p.inspect_source_curvature_law(
                    law,
                    curvature_tolerance_inverse_mm,
                    radial_distance_tolerance_mm,
                    normal_chord_tolerance,
                    domain,
                    b,
                )?,
            ));
        }
        Ok(TransitionSourceLawAudit { reports })
    }
}
