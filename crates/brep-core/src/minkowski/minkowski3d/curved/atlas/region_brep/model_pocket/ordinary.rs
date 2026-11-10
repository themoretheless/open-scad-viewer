//! Exact promotion into the ordinary rational Model when complete boundary
//! arcs have checked rational NURBS witnesses. Nonrepresentable arcs refuse.
use super::*;
mod arcs;
mod build;
mod poly;
#[derive(Clone, Debug, PartialEq)]
pub struct BoundaryArcWitness {
    pub domain_edge: u32,
    pub curve: crate::Curve,
}
#[derive(Clone, Debug, PartialEq)]
pub struct PocketModelCertificate {
    pub version: u32,
    pub operation: String,
    pub pocket: ModelPocketCertificate,
    pub arcs: Vec<BoundaryArcWitness>,
    pub result: String,
    pub maximum_boundary_error_mm: ExactRatio,
}
#[derive(Clone, Debug)]
pub struct CertifiedPocketModel {
    model: Model,
    certificate: PocketModelCertificate,
}
impl CertifiedPocketModel {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn certificate(&self) -> &PocketModelCertificate {
        &self.certificate
    }
    pub fn into_model(self) -> Model {
        self.model
    }
}
/// Ordinary Model Boolean between the original source and its proved interior
/// pocket volume. All four regularized operations have a checked subset rule;
/// this is not a general independent-Model surface/surface Boolean solver.
pub fn certify_pocket_boolean_model(
    pocket: &AlgebraicModelPocket,
    operation: &str,
) -> Result<CertifiedPocketModel> {
    let witnesses = if operation == "union" {
        vec![]
    } else {
        arcs::automatic(pocket)?
    };
    certify_pocket_boolean_model_with_arcs(pocket, operation, &witnesses)
}
/// Supply rational proposals for complete CAD boundary edges. The verifier
/// proves endpoints, monotone coverage, polynomial identity and ordered-root
/// ownership; a fitted curve through a few matching points has no authority.
pub fn certify_pocket_boolean_model_with_arcs(
    pocket: &AlgebraicModelPocket,
    operation: &str,
    witnesses: &[BoundaryArcWitness],
) -> Result<CertifiedPocketModel> {
    if !matches!(operation, "union" | "difference" | "intersection" | "xor") {
        return Err(fail("unknown ordinary pocket Boolean operation"));
    }
    let required = pocket
        .certificate
        .faces
        .iter()
        .flat_map(|f| &f.loops)
        .flatten()
        .filter_map(|e| {
            if let PocketEdge::Top(id) = e.edge {
                Some(id)
            } else {
                None
            }
        })
        .collect::<BTreeSet<_>>();
    let mut data = BTreeMap::new();
    let mut vertices = BTreeMap::new();
    let mut work = Work::default();
    if operation != "union" {
        if witnesses.len() != required.len() {
            return Err(fail(
                "ordinary Model witnesses omit or duplicate complete algebraic edges",
            ));
        }
        for w in witnesses {
            if !required.contains(&w.domain_edge) || data.contains_key(&w.domain_edge) {
                return Err(fail(
                    "ordinary Model witness refers to an unselected or duplicate edge",
                ));
            }
            let arc = arcs::validate(pocket, w, &mut vertices, &mut work)?;
            data.insert(w.domain_edge, arc);
        }
    } else if !witnesses.is_empty() {
        return Err(fail(
            "contained-pocket union needs no boundary conversion witnesses",
        ));
    }
    let model = if operation == "union" {
        pocket.original.clone()
    } else {
        build::build(pocket, &data, &vertices, operation == "intersection")?
    };
    let certificate = PocketModelCertificate {
        version: 1,
        operation: operation.to_string(),
        pocket: pocket.certificate.clone(),
        arcs: witnesses.to_vec(),
        result: source_id(&Source::Brep(model.clone()))?,
        maximum_boundary_error_mm: roots::ratio(&Q::zero()),
    };
    Ok(CertifiedPocketModel { model, certificate })
}
pub fn verify_pocket_boolean_model(
    source: &Model,
    floor: &Surface,
    result: &Model,
    certificate: &PocketModelCertificate,
) -> Result<CertifiedPocketModel> {
    if certificate.version != 1 || certificate.result != source_id(&Source::Brep(result.clone()))? {
        return Err(fail("ordinary pocket Model result binding mismatch"));
    }
    let pocket = verify_model_pocket(source, floor, &certificate.pocket)?;
    let expected =
        certify_pocket_boolean_model_with_arcs(&pocket, &certificate.operation, &certificate.arcs)?;
    if expected.model != *result || expected.certificate != *certificate {
        return Err(fail(
            "ordinary pocket Model exact geometry/topology certificate mismatch",
        ));
    }
    Ok(expected)
}
#[cfg(test)]
mod tests;

mod frame;
pub use frame::{
    certify_framed_pocket_boolean_model, certify_model_pocket_batch,
    certify_oriented_pocket_boolean_model, verify_framed_pocket_boolean_model,
    verify_model_pocket_batch, CertifiedFramedPocketModel, CertifiedPocketModelBatch,
    FramedPocketModelCertificate, ModelPocketRequest, PocketCoordinateFrame,
    PocketModelBatchCertificate,
};
