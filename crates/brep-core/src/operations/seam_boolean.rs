//! Retry existing Boolean proofs after source-exact artificial seam removal.
//! This never promotes a numerical candidate or a new SS family to authority.
use super::*;
use crate::minkowski3d::curved::{self, SameCarrierMergeCertificate};
#[derive(Clone, Debug, PartialEq)]
pub struct SeamNormalizedBooleanCertificate {
    pub version: u32,
    pub sources: [String; 2],
    pub operation: String,
    pub depth: u8,
    /// Each entry binds its immediate input/output: no source change is hidden.
    pub merges: [Vec<SameCarrierMergeCertificate>; 2],
    pub result: String,
}
#[derive(Clone, Debug)]
pub struct CertifiedSeamNormalizedBoolean {
    model: Model,
    certificate: SeamNormalizedBooleanCertificate,
}
impl CertifiedSeamNormalizedBoolean {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn certificate(&self) -> &SeamNormalizedBooleanCertificate {
        &self.certificate
    }
    pub fn into_model(self) -> Model {
        self.model
    }
}
fn refusal(code: &str) -> bool {
    super::boolean::proof_refusal(code) || code.ends_with("_UNPROVEN")
}
fn normalize(source: &Model, depth: u8) -> Result<(Model, Vec<SameCarrierMergeCertificate>)> {
    if source.faces.len() > 64 {
        return Err(Error::new(
            "BREP_SEAM_BOOLEAN_REFUSED",
            "Bounded seam normalization requires at most 64 source faces",
        ));
    }
    let mut model = source.clone();
    let mut merges = Vec::new();
    loop {
        let mut attempted = BTreeSet::new();
        let mut next = None;
        for face in 0..model.faces.len() {
            if attempted.contains(&face) {
                continue;
            }
            let group = curved::same_carrier_component(&model, face)?;
            attempted.extend(group.iter().copied());
            if group.len() < 2 {
                continue;
            }
            match curved::certify_same_carrier_merge(&model, &group, depth) {
                Ok(merge) => {
                    next = Some(merge);
                    break;
                }
                Err(e) if refusal(e.code) => {}
                Err(e) => return Err(e),
            }
        }
        let Some(next) = next else {
            return Ok((model, merges));
        };
        if merges.len() == 8 {
            return Err(Error::new(
                "BREP_RESOURCE_LIMIT",
                "Same-carrier Boolean exceeds eight merge steps per operand",
            ));
        }
        merges.push(next.certificate().clone());
        model = next.into_model();
    }
}
/// Source equivalence is proved by whole-carrier/UV identities and exact
/// winding-chain cancellation, including holes. The normalized operands still
/// require an independently admitted EXISTING Boolean path. Different carriers,
/// unresolved SS components, or an unproved retained-region result still refuse.
/// Replay reconstructs the entire chain and the downstream standard operation.
pub fn certify_seam_normalized_boolean(
    a: &Model,
    b: &Model,
    operation: &str,
    depth: u8,
) -> Result<CertifiedSeamNormalizedBoolean> {
    a.validate()?;
    b.validate()?;
    if depth > 3 || !matches!(operation, "union" | "intersection" | "difference" | "xor") {
        return Err(Error::new(
            "BREP_SEAM_BOOLEAN_REFUSED",
            "Need a supported operation and bounded source depth",
        ));
    }
    let (na, ma) = normalize(a, depth)?;
    let (nb, mb) = normalize(b, depth)?;
    if ma.is_empty() && mb.is_empty() {
        return Err(Error::new(
            "BREP_SEAM_BOOLEAN_REFUSED",
            "No exact artificial seam was removed",
        ));
    }
    // False prevents recursive seam retries. Every successful path still runs
    // its usual geometry/material gates on the proved-equivalent full Models.
    let mut model = super::boolean::boolean_impl(&na, &nb, operation, false)?;
    model.inherit_topology_ids(&[&na, &nb]);
    model.refresh_change_set(&[a, b]);
    model.validate()?;
    let certificate = SeamNormalizedBooleanCertificate {
        version: 1,
        sources: [curved::seam_model_id(a)?, curved::seam_model_id(b)?],
        operation: operation.into(),
        depth,
        merges: [ma, mb],
        result: curved::seam_model_id(&model)?,
    };
    Ok(CertifiedSeamNormalizedBoolean { model, certificate })
}
pub fn verify_seam_normalized_boolean(
    a: &Model,
    b: &Model,
    result: &Model,
    cert: &SeamNormalizedBooleanCertificate,
) -> Result<()> {
    if cert.version != 1 {
        return Err(Error::new(
            "BREP_SEAM_BOOLEAN_REFUSED",
            "Seam Boolean certificate version mismatch",
        ));
    }
    let fresh = certify_seam_normalized_boolean(a, b, &cert.operation, cert.depth)?;
    if fresh.model != *result || fresh.certificate != *cert {
        return Err(Error::new(
            "BREP_SEAM_BOOLEAN_REFUSED",
            "Seam Boolean source, material operation, proof chain or result changed",
        ));
    }
    Ok(())
}
pub(super) fn try_seam_normalized_boolean(
    a: &Model,
    b: &Model,
    operation: &str,
) -> Result<Option<Model>> {
    match certify_seam_normalized_boolean(a, b, operation, 0) {
        Ok(r) => Ok(Some(r.into_model())),
        Err(e) if refusal(e.code) => Ok(None),
        Err(e) => Err(e),
    }
}
