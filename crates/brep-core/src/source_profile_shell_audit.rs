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
    Pending,
    Excluded,
    Planar(OriginalPlanarContact<'x>),
    Fiber(OriginalFiberContact<'x>),
    VertexOrInterior(OriginalAdditionalContact<'x>),
}
impl PairContactProof<'_> {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Excluded => "excluded",
            Self::Planar(_) => "planar",
            Self::Fiber(_) => "fiber",
            Self::VertexOrInterior(_) => "vertex-or-interior",
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
}
impl ShellContactAudit<'_> {
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
        value_codec::json!({"version":1,"kind":"expression-shell-contact-audit","pendingPairs":self.pending_pairs(),
            "cells":self.cells,"spans":self.spans,"domainCells":self.domain_cells,"exactWork":self.exact_work,"driverCells":self.driver_cells,
            "pairs":self.pairs.iter().map(|p|value_codec::json!({"faces":p.support.faces,"proof":p.proof.kind(),"reason":p.reason,
                "sharedEdges":p.support.shared_edges,"pendingProducts":p.support.inspection.pending,
                "outsideProducts":p.support.inspection.outside,"separatedProducts":p.support.inspection.separated})).collect::<Vec<_>>()})
    }
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    /// Generate every proof freshly from this immutable owned shell. Budgets
    /// cover the entire audit; no caller-supplied report can authorize a pair.
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
            out.pairs.push(AuditedContactPair {
                support,
                proof,
                reason,
            });
        }
        Ok(out)
    }
}
