//! Declarative tolerance hand-off contracts for topology-changing operations.
//!
//! A contract records both sides of an operation boundary. It never silently
//! tightens an input tolerance and never relabels an achieved error as exact.
use crate::{InputError, ToleranceContext, ToleranceSpecIdentity};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OperationBoundary {
    Boolean,
    Fillet,
    Sewing,
}

impl OperationBoundary {
    pub fn invariant(self) -> &'static str {
        match self {
            Self::Boolean => {
                "regularized material truth table; retained carriers; no tolerance tightening"
            }
            Self::Fillet => {
                "authored radius law and supported incident faces; achieved deviation is reported"
            }
            Self::Sewing => "paired boundary uses and closure evidence; no unowned edge welding",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationToleranceContract {
    pub version: u32,
    pub operation: OperationBoundary,
    pub input: ToleranceSpecIdentity,
    pub output: ToleranceSpecIdentity,
    pub invariant: &'static str,
}

/// Declare a B-rep operation hand-off in millimetres. Output tolerance may
/// grow to the achieved error, never below the input. Boolean and sewing retain
/// the migrated entity-error allowance (`10 * input`). A declared faceted
/// fillet instead grows to its analytic arc-deviation bound: applying the
/// Boolean allowance there would either reject every ordinary coarse fillet or
/// encourage hiding its approximation error. Invalid or shrinking output fails.
pub fn declare_brep_tolerance_contract(
    operation: OperationBoundary,
    inputs_mm: &[f64],
    output_mm: f64,
) -> Result<OperationToleranceContract, InputError> {
    if inputs_mm.is_empty()
        || inputs_mm
            .iter()
            .chain(std::iter::once(&output_mm))
            .any(|v| !v.is_finite() || *v <= 0.)
    {
        return Err(InputError::InvalidInput(
            "Operation tolerance contract needs finite positive input/output bounds",
        ));
    }
    let input_mm = inputs_mm.iter().copied().fold(0., f64::max);
    let exceeds_entity_allowance =
        operation != OperationBoundary::Fillet && output_mm > input_mm * 10.;
    if output_mm < input_mm || exceeds_entity_allowance {
        return Err(InputError::InvalidInput(
            "Operation output tolerance shrinks input or exceeds the operation allowance",
        ));
    }
    let input = ToleranceContext::from_brep_tolerance_mm(input_mm)?;
    let output = ToleranceContext::from_brep_tolerance_mm(output_mm)?;
    Ok(OperationToleranceContract {
        version: 1,
        operation,
        input: input.spec_identity(),
        output: output.spec_identity(),
        invariant: operation.invariant(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_operation_boundaries_declare_their_input_output_and_invariant() {
        for operation in [
            OperationBoundary::Boolean,
            OperationBoundary::Fillet,
            OperationBoundary::Sewing,
        ] {
            let contract = declare_brep_tolerance_contract(operation, &[1e-8, 1e-7], 2e-7).unwrap();
            assert_eq!(contract.version, 1);
            assert_eq!(contract.operation, operation);
            assert_ne!(contract.input, contract.output);
            assert!(!contract.invariant.is_empty());
        }
    }

    #[test]
    fn tolerance_handoff_refuses_shrinking_excessive_or_nonfinite_output() {
        for output in [1e-8, 2e-6, f64::NAN, 0.] {
            assert!(
                declare_brep_tolerance_contract(OperationBoundary::Boolean, &[1e-7], output,)
                    .is_err()
            );
        }
        assert!(declare_brep_tolerance_contract(OperationBoundary::Sewing, &[], 1e-7).is_err());
        assert!(declare_brep_tolerance_contract(OperationBoundary::Fillet, &[1e-7], 1e-8).is_err());
        assert!(declare_brep_tolerance_contract(OperationBoundary::Sewing, &[1e-7], 2e-6).is_err());
        assert!(declare_brep_tolerance_contract(OperationBoundary::Fillet, &[1e-7], 2e-2).is_ok());
    }
}
