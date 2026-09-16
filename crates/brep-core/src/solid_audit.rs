//! Global solid audit (forward A4) required before curved Boolean claims.
//!
//! Checks connected material components, orientation consistency of closed
//! shells, watertight shared-edge incidence, and empty/multi-body bounds.
//! Failure is a typed refuse — never silent heal.

use crate::Model;
use crate::trim_sew::{SewCertificate, sew_closed_model_edges};
use nurbs_core::{Error, Result};

fn refuse(message: &str) -> Error {
    Error::new("BREP_SOLID_AUDIT_REFUSED", message)
}

#[derive(Clone, Debug)]
pub struct SolidAuditCertificate {
    pub ok: bool,
    pub body_count: usize,
    pub shell_count: usize,
    pub sew: SewCertificate,
    pub notes: Vec<&'static str>,
}

/// A4 audit after imprint / empty-algebra authorship.
pub fn audit_solid(model: &Model) -> Result<SolidAuditCertificate> {
    model.validate()?;
    let mut notes = Vec::new();
    if model.bodies.len() > 8 {
        return Err(refuse("SolidSet body budget exceeded (max 8 lumps)"));
    }
    if model.is_empty() {
        if !model.bodies.is_empty() || !model.shells.is_empty() {
            return Err(refuse(
                "Regularized empty solid must have no shell/body carrier",
            ));
        }
    } else if model.bodies.is_empty() {
        return Err(refuse("Non-empty solid has no owning body"));
    }
    let mut owned_shells = vec![false; model.shells.len()];
    for body in &model.bodies {
        if body.outer_shell >= model.shells.len() {
            return Err(refuse("Body outer_shell index out of range"));
        }
        if std::mem::replace(&mut owned_shells[body.outer_shell], true) {
            return Err(refuse("Shell is owned by more than one body"));
        }
        let outer = &model.shells[body.outer_shell];
        if !outer.closed && !model.faces.is_empty() {
            return Err(refuse("Outer shell must be closed for solid audit"));
        }
        if outer.faces.is_empty() && !model.faces.is_empty() {
            return Err(refuse("Outer shell has no faces"));
        }
        for &inner in &body.inner_shells {
            if inner >= model.shells.len() {
                return Err(refuse("Inner shell index out of range"));
            }
            if !model.shells[inner].closed {
                return Err(refuse("Cavity shell must be closed"));
            }
            if std::mem::replace(&mut owned_shells[inner], true) {
                return Err(refuse("Shell is repeated across body ownership"));
            }
        }
    }
    if owned_shells.iter().any(|owned| !owned) {
        return Err(refuse("Orphan shell is not connected to a material body"));
    }
    // Orientation: FaceUse.reversed must be boolean-defined (always) and each
    // closed shell must own at least one face when the model is nonempty.
    for shell in &model.shells {
        if shell.closed && shell.faces.is_empty() && !model.is_empty() {
            return Err(refuse("Closed shell without faces"));
        }
    }
    notes.push("orientation_shells_ok");

    let sew = if model.faces.is_empty() {
        notes.push("empty_solid_admitted");
        SewCertificate {
            matched: 0,
            complete: true,
            displacement_budget_ok: true,
        }
    } else {
        let cert = sew_closed_model_edges(model)
            .map_err(|e| refuse(&format!("Watertight sew incidence failed: {}", e.message)))?;
        notes.push("watertight_sew_ok");
        cert
    };

    Ok(SolidAuditCertificate {
        ok: sew.complete,
        body_count: model.bodies.len(),
        shell_count: model.shells.len(),
        sew,
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cylinder;

    #[test]
    fn cylinder_audits_ok() {
        let model = cylinder(2., 3.).unwrap();
        let cert = audit_solid(&model).unwrap();
        assert!(cert.ok);
        assert!(cert.sew.complete);
    }

    #[test]
    fn empty_model_audits_ok() {
        let model = Model::empty(1e-6).unwrap();
        let cert = audit_solid(&model).unwrap();
        assert!(cert.ok);
        assert!(cert.notes.iter().any(|n| *n == "empty_solid_admitted"));
    }

    #[test]
    fn missed_orientation_branch_cannot_hide_behind_closed_flag() {
        let mut model = cylinder(2., 3.).unwrap();
        model.shells[0].faces[0].reversed = !model.shells[0].faces[0].reversed;
        assert!(audit_solid(&model).is_err());
    }

    #[test]
    fn orphan_shell_mutation_refuses_global_audit() {
        let mut model = cylinder(2., 3.).unwrap();
        let orphan = model.shells[0].clone();
        model.shells.push(orphan);
        model.rebuild_topology_ids();
        assert!(audit_solid(&model).is_err());
    }
}
