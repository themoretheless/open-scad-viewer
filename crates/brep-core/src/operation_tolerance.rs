//! Enforced B-rep operation tolerance hand-off.
//!
//! Invariant: the published spatial tolerance is at least the source tolerance
//! and at least the measured/conservatively bounded output deviation. The
//! shared operation-specific allowance is enforced; user tolerances are never
//! silently tightened. Publication requires both a valid contract and a fresh
//! global solid audit.
use crate::Model;
use crate::solid_audit::SolidAuditCertificate;
use cad_predicates::{OperationBoundary, OperationToleranceContract};
use nurbs_core::{Error, Result};

#[derive(Clone, Debug)]
pub struct ModelToleranceHandoff {
    pub contract: OperationToleranceContract,
    /// Measured value or a conservative analytic upper bound, in millimetres.
    pub measured_output_mm: f64,
    pub audit: SolidAuditCertificate,
}

fn refused(stage: &str, detail: impl std::fmt::Display) -> Error {
    Error::new(
        "BREP_TOLERANCE_CONTRACT_REFUSED",
        format!("{stage} tolerance hand-off failed: {detail}"),
    )
}

pub fn admit_operation_input(operation: OperationBoundary, input_mm: f64) -> Result<()> {
    if !input_mm.is_finite() || input_mm <= 0. {
        return Err(refused(
            operation.invariant(),
            "input tolerance must be finite and positive",
        ));
    }
    Ok(())
}

/// Apply the declared output policy and audit the exact model being published.
pub fn enforce_model_tolerance_handoff(
    operation: OperationBoundary,
    input_mm: f64,
    measured_output_mm: f64,
    model: &mut Model,
) -> Result<ModelToleranceHandoff> {
    admit_operation_input(operation, input_mm)?;
    if !measured_output_mm.is_finite() || measured_output_mm < 0. {
        return Err(refused(
            operation.invariant(),
            "output measure must be finite and non-negative",
        ));
    }
    admit_operation_input(operation, model.tolerance_mm)?;
    // max, never min: a smaller authored/user tolerance survives unchanged.
    model.tolerance_mm = model.tolerance_mm.max(input_mm).max(measured_output_mm);
    let contract =
        cad_predicates::declare_brep_tolerance_contract(operation, &[input_mm], model.tolerance_mm)
            .map_err(|error| refused(operation.invariant(), error))?;
    model
        .validate()
        .map_err(|error| refused(operation.invariant(), error))?;
    let audit = crate::solid_audit::audit_solid(model)
        .map_err(|error| refused(operation.invariant(), error))?;
    Ok(ModelToleranceHandoff {
        contract,
        measured_output_mm,
        audit,
    })
}
