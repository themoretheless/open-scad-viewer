//! Continuous boundary evidence for a numerical NURBS Boolean candidate.
//! This does not certify intersection coverage or Boolean material membership.
use super::*;
use nurbs_core::curve_surface_agreement::Status;
use sha2::{Digest, Sha256};
use value_codec::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceScope {
    /// Every authored coedge use agrees with its carrier over its full use
    /// interval. No intersection/region/topology-change authority is granted.
    BoundaryAgreementOnly,
}
#[derive(Clone, Debug, PartialEq)]
pub struct NurbsBooleanBoundaryEvidence {
    pub version: u32,
    pub sources: [String; 2],
    pub result: String,
    pub operation: String,
    pub tolerance_mm: f64,
    /// Shared among all uses of each Model, independently for each of the
    /// two operands and result. Exhaustion refuses rather than trusting samples.
    pub max_cells_per_model: usize,
    pub cells: [usize; 3],
    pub uses: [usize; 3],
    pub scope: EvidenceScope,
}
#[derive(Clone, Debug)]
pub struct ContinuousNurbsBooleanCandidate {
    model: Model,
    evidence: NurbsBooleanBoundaryEvidence,
}
impl ContinuousNurbsBooleanCandidate {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn evidence(&self) -> &NurbsBooleanBoundaryEvidence {
        &self.evidence
    }
    pub fn into_model(self) -> Model {
        self.model
    }
}
fn refused(code: &'static str, message: impl Into<String>) -> Error {
    Error::new(code, message)
}
fn identity(m: &Model) -> Result<String> {
    let bytes = value_codec::encode_binary(&m.to_value())
        .map_err(|e| refused("BREP_NURBS_BOUNDARY_BINDING", e.to_string()))?;
    let mut hash = Sha256::new();
    hash.update(b"nurbs-boolean-boundary-candidate-v1");
    hash.update(bytes);
    Ok(hash.finalize().iter().map(|b| format!("{b:02x}")).collect())
}
fn options(a: &Model, b: &Model, operation: &str, tolerance: f64, cells: usize) -> Result<()> {
    if !matches!(operation, "union" | "difference" | "intersection") {
        return Err(refused(
            "BREP_UNSUPPORTED_OPERATION",
            "Continuous numerical candidate admits union, difference or intersection",
        ));
    }
    if !tolerance.is_finite()
        || tolerance <= 0.
        || tolerance > CEILING
        || tolerance < a.tolerance_mm.max(b.tolerance_mm)
    {
        return Err(refused(
            "BREP_NURBS_BOUNDARY_TOLERANCE",
            "Requested tolerance must cover operand tolerances and lie in (0, 1e-2] mm",
        ));
    }
    if !(1..=1_000_000).contains(&cells) {
        return Err(refused(
            "BREP_NURBS_BOUNDARY_BUDGET",
            "Continuous boundary budget must be 1..1000000 cells per Model",
        ));
    }
    Ok(())
}
fn boundary(m: &Model, budget: usize) -> Result<(usize, usize)> {
    // Exact identities discharge supported uses without approximation.
    // Every remaining use still requires a whole-interval conservative bound.
    let proof = crate::boundary_agreement::verify_with_exact(m, budget, 1_000_000)?;
    for u in &proof.agreement.uses {
        match u.status {
            Status::WithinTolerance => {}
            Status::Mismatch => {
                return Err(refused(
                    "BREP_NURBS_BOUNDARY_MISMATCH",
                    format!(
                        "Continuous mismatch at face {}, coedge {}, edge {}",
                        u.face, u.coedge, u.edge
                    ),
                ));
            }
            Status::Unresolved => {
                return Err(refused(
                    "BREP_NURBS_BOUNDARY_UNRESOLVED",
                    format!(
                        "Continuous agreement unproved at face {}, coedge {}, edge {}",
                        u.face, u.coedge, u.edge
                    ),
                ));
            }
        }
    }
    if !proof.agreement.complete {
        return Err(refused(
            "BREP_NURBS_BOUNDARY_UNRESOLVED",
            "Incomplete continuous boundary evidence",
        ));
    }
    Ok((proof.agreement.cells, proof.agreement.uses.len()))
}
/// Generate a numerical Boolean candidate on independent rational carriers
/// with an explicit tolerance. No shared deformation is required. Each input
/// retains its original tolerance; the result uses exactly the requested one.
/// All actual coedge uses, including post-assembly endpoint adjustments and
/// refitted closed branches, must pass continuous carrier agreement.
///
/// IMPORTANT: this evidence does not prove all SS branches were found, correct
/// UV region classification, ambient embedding, or Boolean set equality.
/// Callers must not treat this handle as a certified general Boolean result.
pub fn nurbs_boolean_continuous_candidate(
    a: &Model,
    b: &Model,
    operation: &str,
    tolerance_mm: f64,
    max_cells_per_model: usize,
) -> Result<ContinuousNurbsBooleanCandidate> {
    options(a, b, operation, tolerance_mm, max_cells_per_model)?;
    let pa = boundary(a, max_cells_per_model)?;
    let pb = boundary(b, max_cells_per_model)?;
    let mut model = boolean(a, b, operation)?;
    model.tolerance_mm = tolerance_mm;
    let pr = boundary(&model, max_cells_per_model)?;
    let evidence = NurbsBooleanBoundaryEvidence {
        version: 1,
        sources: [identity(a)?, identity(b)?],
        result: identity(&model)?,
        operation: operation.into(),
        tolerance_mm,
        max_cells_per_model,
        cells: [pa.0, pb.0, pr.0],
        uses: [pa.1, pb.1, pr.1],
        scope: EvidenceScope::BoundaryAgreementOnly,
    };
    Ok(ContinuousNurbsBooleanCandidate { model, evidence })
}
/// Recompute continuous boundary evidence on the bound immutable operands and
/// candidate. The operation is only a proposal label, not proved set semantics.
pub fn verify_nurbs_boolean_boundary_candidate(
    a: &Model,
    b: &Model,
    result: &Model,
    operation: &str,
    e: &NurbsBooleanBoundaryEvidence,
) -> Result<()> {
    options(a, b, operation, e.tolerance_mm, e.max_cells_per_model)?;
    if e.version != 1
        || e.operation != operation
        || result.tolerance_mm != e.tolerance_mm
        || e.sources != [identity(a)?, identity(b)?]
        || e.result != identity(result)?
    {
        return Err(refused(
            "BREP_NURBS_BOUNDARY_BINDING",
            "Boundary candidate source/result/operation binding mismatch",
        ));
    }
    let pa = boundary(a, e.max_cells_per_model)?;
    let pb = boundary(b, e.max_cells_per_model)?;
    let pr = boundary(result, e.max_cells_per_model)?;
    if e.cells != [pa.0, pb.0, pr.0] || e.uses != [pa.1, pb.1, pr.1] {
        return Err(refused(
            "BREP_NURBS_BOUNDARY_BINDING",
            "Boundary proof ledger mismatch",
        ));
    }
    Ok(())
}
