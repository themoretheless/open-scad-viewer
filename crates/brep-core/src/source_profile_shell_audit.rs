//! Fresh combined pair audit; no Body admission follows from a partial report.
use super::*;
#[derive(Clone, Copy)]
pub struct ContactAuditLimits {
    pub cells: usize,
    pub cells_per_pair: usize,
    pub controls_per_enclosure: usize,
    pub spans: usize,
    pub domain_cells: usize,
    pub domain_cells_per_pair: usize,
    pub exact_work: u64,
    pub driver_cells: usize,
    pub tolerance_uv: f64,
}
pub enum PairContactProof<'x> {
    TransitionXyPlane(TransitionXyPlaneContact<'x>),
    ProfilePaired(ProfilePairedContact<'x, 'x>),
    ProfileRuled(ProfileRuledContact<'x, 'x>),
    ProfileBoundary(ProfileBoundaryContact<'x, 'x>),
    TransitionCoordinate(TransitionCoordinateContact<'x>),
    ProfileVertex(ProfileVertexContact<'x>),
    TransitionXyBounds(TransitionXyBoundsContact<'x>),
    TransitionXyHull(TransitionXyHullContact<'x>),
    Boundary(OriginalBoundaryContact<'x>),
    ProfileHull(ProfileHullContact<'x>),
    Pole(OriginalPoleContact<'x>),
    Hull(OriginalHullContact<'x>),
    Pending,
    Excluded,
    Planar(OriginalPlanarContact<'x>),
    Fiber(OriginalFiberContact<'x>),
    VertexOrInterior(OriginalAdditionalContact<'x>),
    TransitionPlane(TransitionPlaneContact<'x>),
    TransitionRuled(TransitionRuledContact<'x>),
    TransitionSection(TransitionSectionContact<'x>),
}
impl PairContactProof<'_> {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::TransitionXyPlane(_) => "transition-xy-material-plane-separated",
            Self::ProfilePaired(_) => "profile-paired-contact",
            Self::ProfileRuled(_) => "profile-ruled-contact",
            Self::ProfileBoundary(_) => "profile-boundary-contact",
            Self::TransitionCoordinate(_) => "transition-coordinate-separated",            Self::ProfileVertex(c) => if c.excluded_domain().is_some() {"profile-vertex-excluded"} else {"profile-vertex-contact"},            Self::TransitionXyBounds(_) => "transition-xy-bounds-separated",
            Self::TransitionXyHull(_) => "transition-xy-hull-separated",
            Self::Boundary(_) => "boundary-contact",
            Self::ProfileHull(_) => "profile-hull-separated",
            Self::Pole(_) => "pole-contact",
            Self::Hull(_) => "hull-separated",
            Self::Pending => "pending",
            Self::Excluded => "excluded",
            Self::Planar(_) => "planar",
            Self::Fiber(_) => "fiber",
            Self::VertexOrInterior(_) => "vertex-or-interior",
            Self::TransitionPlane(_) => "transition-plane",
            Self::TransitionRuled(_) => "transition-ruled",
            Self::TransitionSection(_) => "transition-section",
        }
    }
}
pub struct AuditedContactPair<'x> {
    support: ShellSupportPair,
    proof: PairContactProof<'x>,
    reason: &'static str,
}
impl AuditedContactPair<'_> {
    pub fn support(&self) -> &ShellSupportPair {
        &self.support
    }
    pub fn proof(&self) -> &PairContactProof<'_> {
        &self.proof
    }
    pub fn reason(&self) -> &'static str {
        self.reason
    }
}
pub struct ShellContactAudit<'x> {
    owners: &'x [ShellFace],
    pairs: Vec<AuditedContactPair<'x>>,
    cells: usize,
    spans: usize,
    domain_cells: usize,
    exact_work: u64,
    driver_cells: usize,
    linear_cells: usize,
}
impl ShellContactAudit<'_> {
    /// Complete cross-face coverage only. This does not admit individual face
    /// charts, signed volume, radius, thickness or a surrounding Body.
    pub fn complete_cross_face_contacts(&self) -> bool {
        let count = self.owners.len();
        if count < 2 || self.pending_pairs() != 0 {
            return false;
        }
        let Some(expected) = count.checked_mul(count - 1).map(|n| n / 2) else {
            return false;
        };
        if self.pairs.len() != expected {
            return false;
        }
        let mut seen = std::collections::BTreeSet::new();
        for pair in &self.pairs {
            let [a, b] = pair.support.faces;
            if a >= count || b >= count || a == b
                || !seen.insert([a.min(b), a.max(b)])
            {
                return false;
            }
        }
        seen.len() == expected
    }
    pub fn linear_cells(&self) -> usize { self.linear_cells }
    pub fn owners(&self) -> &[ShellFace] {
        self.owners
    }
    pub fn pairs(&self) -> &[AuditedContactPair<'_>] {
        &self.pairs
    }
    pub fn pending_pairs(&self) -> usize {
        self.pairs
            .iter()
            .filter(|p| matches!(p.proof, PairContactProof::Pending))
            .count()
    }
    pub fn work(&self) -> (usize, usize, usize, u64, usize) {
        (
            self.cells,
            self.spans,
            self.domain_cells,
            self.exact_work,
            self.driver_cells,
        )
    }
    /// Transport diagnostics only; this Value never restores geometric authority.
    pub fn diagnostics(&self) -> value_codec::Value {
        let kind=|face:&ShellFace| match face.kind() {ShellFaceKind::Original(_)=>"original",ShellFaceKind::Retained(_)=>"retained",ShellFaceKind::Transition=>"transition"};
        let mut pending_reasons=std::collections::BTreeMap::<String,usize>::new();
        let mut pending_kinds=std::collections::BTreeMap::<String,usize>::new();
        for pair in &self.pairs {
            if !matches!(pair.proof,PairContactProof::Pending) {continue;}
            *pending_reasons.entry(pair.reason.into()).or_default()+=1;
            let mut kinds=pair.support.faces.map(|i|kind(&self.owners[i]));
            kinds.sort();
            *pending_kinds.entry(format!("{}-{}",kinds[0],kinds[1])).or_default()+=1;
        }
        let faces=self.owners.iter().enumerate().map(|(index,face)| {
            match face.kind() {
                ShellFaceKind::Original(source)=>value_codec::json!({"index":index,"kind":"original","sourceFace":source}),
                ShellFaceKind::Retained(side)=>value_codec::json!({"index":index,"kind":"retained","retainedSide":side}),
                ShellFaceKind::Transition=>value_codec::json!({"index":index,"kind":"transition"}),
            }
        }).collect::<Vec<_>>();
        value_codec::json!({"version":1,"kind":"expression-shell-contact-audit","pendingPairs":self.pending_pairs(),
            "faces":faces,"pendingByReason":pending_reasons,"pendingByFaceKinds":pending_kinds,
            "cells":self.cells,"spans":self.spans,"domainCells":self.domain_cells,"exactWork":self.exact_work,"driverCells":self.driver_cells,"linearCells":self.linear_cells,
            "pairs":self.pairs.iter().map(|p|value_codec::json!({"faces":p.support.faces,"proof":p.proof.kind(),"reason":p.reason,
                "sharedEdges":p.support.shared_edges,"pendingProducts":p.support.inspection.pending,
                "outsideProducts":p.support.inspection.outside,"separatedProducts":p.support.inspection.separated})).collect::<Vec<_>>()})
    }
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    /// Generate every proof freshly from this immutable owned shell. Budgets
    /// cover the entire audit; no caller-supplied report can authorize a pair.
    /// Extend the complete base audit only with fresh contact proofs tied to
    /// this shell's exact transition boundary and remaining shared work.
    pub fn inspect_contacts_with_original_region(
        &self,
        limits: ContactAuditLimits,
        region: &OriginalProjectedRegion<'_, '_, '_, '_, '_>,
    ) -> Result<ShellContactAudit<'_>> {
        let mut out = self.inspect_contacts(limits)?;
        for pair in &mut out.pairs {
            if !matches!(pair.proof, PairContactProof::Pending) {
                continue;
            }
            for &edge in &pair.support.shared_edges {
                let ShellPairKind::Rail(rail) = self.pairs[edge].kind else {
                    continue;
                };
                let ruled = self.qualify_transition_ruled_contact(
                    rail,
                    region,
                    limits.controls_per_enclosure,
                    limits.exact_work - out.exact_work,
                    limits.driver_cells - out.driver_cells,
                )?;
                out.exact_work += ruled.exact_work;
                out.driver_cells += ruled.driver_cells;
                pair.reason = ruled.reason;
                if let Some(certificate) = ruled.certificate {
                    pair.proof = PairContactProof::TransitionRuled(certificate);
                    break;
                }
            }
        }
        Ok(out)
    }
    pub fn inspect_contacts_with_profile_region(
        &self,
        limits: ContactAuditLimits,
        region: &ProfileProjectedRegion<'_, '_, '_, '_, '_>,
    ) -> Result<ShellContactAudit<'_>> {
        let mut out = self.inspect_contacts(limits)?;
        for pair in &mut out.pairs {
            if !matches!(pair.proof, PairContactProof::Pending) {
                continue;
            }
            for &edge in &pair.support.shared_edges {
                let ShellPairKind::Rail(rail) = self.pairs[edge].kind else {
                    continue;
                };
                let ruled = self.qualify_transition_profile_ruled_contact(
                    rail,
                    region,
                    limits.controls_per_enclosure,
                    limits.exact_work - out.exact_work,
                    limits.driver_cells - out.driver_cells,
                )?;
                out.exact_work += ruled.exact_work;
                out.driver_cells += ruled.driver_cells;
                pair.reason = ruled.reason;
                if let Some(certificate) = ruled.certificate {
                    pair.proof = PairContactProof::TransitionRuled(certificate);
                    break;
                }
            }
        }
        for pair in &mut out.pairs {
            if !matches!(pair.proof,PairContactProof::Pending) {continue;}
            for &edge in &pair.support.shared_edges {
                let ShellPairKind::Section(end)=self.pairs[edge].kind else {continue;};
                let section=self.qualify_transition_section_contact(end,region,limits.controls_per_enclosure)?;
                pair.reason=section.reason;
                if let Some(certificate)=section.certificate {pair.proof=PairContactProof::TransitionSection(certificate);break;}
            }
        }
        Ok(out)
    }
    pub fn inspect_contacts(&self, limits: ContactAuditLimits) -> Result<ShellContactAudit<'_>> {
        if limits.exact_work > 100000000 || limits.driver_cells > 100000 {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Bound combined contact exact and driver work",
            ));
        }
        let supports = self.inspect_shell_trimmed(
            limits.cells,
            limits.cells_per_pair,
            limits.controls_per_enclosure,
            limits.spans,
            limits.tolerance_uv,
            limits.domain_cells,
            limits.domain_cells_per_pair,
        )?;
        let mut out = ShellContactAudit {
            owners: &self.faces,
            pairs: Vec::new(),
            cells: supports.cells,
            spans: supports.spans,
            domain_cells: supports.domain_cells,
            exact_work: 0,
            driver_cells: 0,
            linear_cells: 0,
        };
        for support in supports.pairs {
            let mut proof = PairContactProof::Pending;
            let mut reason;
            if support.inspection.complete_excluded() {
                proof = PairContactProof::Excluded;
                reason = "expression-contact-products-excluded";
            } else {
                let planar = self.qualify_original_planar_contact(
                    support.faces,
                    limits.exact_work - out.exact_work,
                    limits.spans - out.spans,
                )?;
                out.exact_work += planar.exact_work;
                out.spans += planar.spans;
                reason = planar.reason;
                if let Some(certificate) = planar.certificate {
                    proof = PairContactProof::Planar(certificate);
                } else {
                    let fiber = self.qualify_original_fiber_contact(
                        support.faces,
                        limits.exact_work - out.exact_work,
                        limits.spans - out.spans,
                        limits.driver_cells - out.driver_cells,
                    )?;
                    out.exact_work += fiber.exact_work;
                    out.spans += fiber.spans;
                    out.driver_cells += fiber.driver_cells;
                    reason = fiber.reason;
                    if let Some(certificate) = fiber.certificate {
                        proof = PairContactProof::Fiber(certificate);
                    } else {
                        let extra = self.qualify_original_additional_contact(
                            support.faces,
                            limits.exact_work - out.exact_work,
                            limits.spans - out.spans,
                            limits.driver_cells - out.driver_cells,
                        )?;
                        out.exact_work += extra.exact_work;
                        out.spans += extra.spans;
                        out.driver_cells += extra.driver_cells;
                        reason = extra.reason;
                        if let Some(certificate) = extra.certificate {
                            proof = PairContactProof::VertexOrInterior(certificate);
                        }
                    }
                }
            }
            if matches!(proof, PairContactProof::Pending) {
                for &edge in &support.shared_edges {
                    let ShellPairKind::Rail(rail) = self.pairs[edge].kind else {
                        continue;
                    };
                    let plane = self.qualify_transition_plane_contact(
                        rail,
                        limits.exact_work - out.exact_work,
                        limits.controls_per_enclosure,
                        limits.cells - out.cells,
                        limits.spans - out.spans,
                    )?;
                    out.exact_work += plane.exact_work;
                    out.cells += plane.cells;
                    out.spans += plane.spans;
                    reason = plane.reason;
                    if let Some(certificate) = plane.certificate {
                        proof = PairContactProof::TransitionPlane(certificate);
                        break;
                    }
                }
            }
            out.pairs.push(AuditedContactPair {
                support,
                proof,
                reason,
            });
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagnostic_groups_count_only_pending_and_preserve_owner_indices() {
        let owners=[ShellFace {kind:ShellFaceKind::Original(5),loops:vec![]},ShellFace {kind:ShellFaceKind::Retained(0),loops:vec![]},ShellFace {kind:ShellFaceKind::Transition,loops:vec![]},ShellFace {kind:ShellFaceKind::Original(7),loops:vec![]}];
        let pair=|faces,reason,proof| AuditedContactPair {support:ShellSupportPair {faces,shared_edges:vec![],inspection:SupportPairReport {separated:vec![],pending:vec![],outside:vec![],domain_cells:0,cells:0,spans:0}},reason,proof};
        let audit=ShellContactAudit {owners:&owners,pairs:vec![pair([0,1],"budget",PairContactProof::Pending),pair([2,0],"budget",PairContactProof::Pending),pair([1,2],"boundary",PairContactProof::Pending),pair([0,3],"excluded",PairContactProof::Excluded)],cells:0,spans:0,domain_cells:0,exact_work:0,driver_cells:0,linear_cells:0};
        let d=audit.diagnostics();
        assert_eq!(d["pendingPairs"].as_u64(),Some(3));
        assert_eq!(d["pendingByReason"]["budget"].as_u64(),Some(2));
        assert_eq!(d["pendingByReason"]["boundary"].as_u64(),Some(1));
        assert!(d["pendingByReason"]["excluded"].is_null());
        for key in ["original-retained","original-transition","retained-transition"] {assert_eq!(d["pendingByFaceKinds"][key].as_u64(),Some(1));}
        assert_eq!(d["faces"][0]["sourceFace"].as_u64(),Some(5));
        assert_eq!(d["faces"][1]["retainedSide"].as_u64(),Some(0));
        assert_eq!(d["faces"][2]["kind"].as_str(),Some("transition"));
        assert_eq!(value_codec::decode_binary(&value_codec::encode_binary(&d).unwrap()).unwrap(),d);
    }
    #[test]
    fn complete_contacts_require_every_distinct_pair_and_no_pending() {
        let owners = (0..3).map(|i| ShellFace { kind: ShellFaceKind::Original(i), loops: vec![] }).collect::<Vec<_>>();
        let pair = |faces| AuditedContactPair {
            support: ShellSupportPair { faces, shared_edges: vec![], inspection: SupportPairReport { separated: vec![], pending: vec![], outside: vec![], domain_cells: 0, cells: 0, spans: 0 } },
            reason: "test-only", proof: PairContactProof::Excluded,
        };
        let mut audit = ShellContactAudit { owners: &owners, pairs: vec![], cells: 0, spans: 0, domain_cells: 0, exact_work: 0, driver_cells: 0, linear_cells: 0 };
        assert!(!audit.complete_cross_face_contacts());
        audit.pairs = vec![pair([0,1]), pair([2,0]), pair([1,2])];
        assert!(audit.complete_cross_face_contacts());
        audit.pairs[2].proof = PairContactProof::Pending;
        assert!(!audit.complete_cross_face_contacts());
        for invalid in [[1,0], [2,2], [1,3]] {
            audit.pairs[2] = pair(invalid);
            assert!(!audit.complete_cross_face_contacts());
        }
        audit.pairs.clear();
        audit.owners = &[];
        assert!(!audit.complete_cross_face_contacts());
    }
}

#[path = "source_profile_shell_refinement.rs"]
mod refinement;
