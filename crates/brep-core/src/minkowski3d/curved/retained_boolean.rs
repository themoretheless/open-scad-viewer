//! Checked exact rational arrangements on immutable original carriers.
//! Arrangement Models are proposals; their labels and saved topology confer no proof.
use super::*;
use crate::{Coedge, Curve};
use nurbs_core::curve_surface_agreement as agreement;
mod assemble;
mod exact;
mod material;
mod partition;
mod propose;
mod propose_arcs;

#[derive(Clone, Debug, PartialEq)]
pub enum RegionEdgeOrigin {
    Original { edge: usize, range: [f64; 2] },
    Intersection { arc: usize, reversed: bool },
}
#[derive(Clone, Debug, PartialEq)]
pub struct NurbsFacePartition {
    /// Complete original body with every sector retained, never an open result.
    pub model: Model,
    pub face_sources: Vec<usize>,
    pub edge_sources: Vec<RegionEdgeOrigin>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct RetainedRegionMaterial {
    pub operand: usize,
    pub face: usize,
    pub original_face: usize,
    pub seed_uv: [f64; 2],
    pub winding_in_other: i32,
    pub keep: bool,
    pub reverse: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct RetainedNurbsBooleanCertificate {
    pub version: u32,
    pub operation: String,
    pub graph: NurbsIntersectionGraphCertificate,
    pub partitions: [NurbsFacePartition; 2],
    pub regions: Vec<RetainedRegionMaterial>,
    pub result: String,
    pub result_atlas: Option<SourceAtlasCertificate>,
}
pub struct CertifiedRetainedNurbsBoolean {
    model: Model,
    certificate: RetainedNurbsBooleanCertificate,
}
impl CertifiedRetainedNurbsBoolean {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn certificate(&self) -> &RetainedNurbsBooleanCertificate {
        &self.certificate
    }
    pub fn into_model(self) -> Model {
        self.model
    }
}
/// Every original UV domain is covered with multiplicity EXACTLY one by
/// positive connected audited regions. Exact paired intersection curves cover
/// the complete graph. One original-solid winding per connected open region
/// then fixes its material label everywhere; the Boolean truth table chooses
/// oriented faces. Original curves/surfaces survive in the ordinary Model.
pub fn certify_retained_nurbs_boolean(
    a: &Model,
    b: &Model,
    operation: &str,
    partitions: &[NurbsFacePartition; 2],
    limits: NurbsGraphLimits,
) -> Result<CertifiedRetainedNurbsBoolean> {
    if !matches!(operation, "union" | "difference" | "intersection" | "xor") {
        return Err(fail("invalid retained-region Boolean operation"));
    }
    let graph = certify_nurbs_intersection_graph(a, b, limits)?;
    construct([a, b], operation, partitions, graph)
}
fn construct(
    models: [&Model; 2],
    operation: &str,
    partitions: &[NurbsFacePartition; 2],
    graph: QualifiedNurbsIntersectionGraph,
) -> Result<CertifiedRetainedNurbsBoolean> {
    let limits = graph.certificate().limits;
    for side in 0..2 {
        partition::verify(models, &graph, &partitions[side], side)
            .map_err(|e| fail(&format!("partition {side}: {}", e.message)))?;
    }
    partition::paired_arcs(&graph, partitions)?;
    let regions = material::classify(models, partitions, operation, limits.source_depth)
        .map_err(|e| fail(&format!("material: {}", e.message)))?;
    let (model, result_atlas) = assemble::build(models, &graph, partitions, &regions)
        .map_err(|e| fail(&format!("assembly: {}", e.message)))?;
    let certificate = RetainedNurbsBooleanCertificate {
        version: 1,
        operation: operation.into(),
        graph: graph.certificate().clone(),
        partitions: partitions.clone(),
        regions,
        result: source_id(&Source::Brep(model.clone()))?,
        result_atlas,
    };
    Ok(CertifiedRetainedNurbsBoolean { model, certificate })
}
pub fn author_retained_nurbs_boolean(
    a: &Model,
    b: &Model,
    operation: &str,
    limits: NurbsGraphLimits,
) -> Result<CertifiedRetainedNurbsBoolean> {
    a.validate()?;
    b.validate()?;
    if !matches!(operation, "union" | "difference" | "intersection" | "xor") {
        return Err(fail("invalid Boolean operation"));
    }
    let graph = certify_nurbs_intersection_graph(a, b, limits)
        .map_err(|e| fail(&format!("input graph: {}", e.message)))?;
    let p = propose::construct([a, b], &graph).map_err(|e| {
        fail(&format!(
            "exact arrangement proposal refused: {}",
            e.message
        ))
    })?;
    construct([a, b], operation, &p, graph)
}
pub(crate) fn try_retained_nurbs_boolean(
    a: &Model,
    b: &Model,
    operation: &str,
) -> Result<Option<Model>> {
    match author_retained_nurbs_boolean(a, b, operation, NurbsGraphLimits::default()) {
        Ok(r) => Ok(Some(r.into_model())),
        Err(e)
            if e.code == "MINKOWSKI_3D_REFUSED"
                || e.code.ends_with("_UNPROVEN")
                || e.code.ends_with("_REFUSED") =>
        {
            Ok(None)
        }
        Err(e) => Err(e),
    }
}
pub fn verify_retained_nurbs_boolean(
    a: &Model,
    b: &Model,
    result: &Model,
    c: &RetainedNurbsBooleanCertificate,
) -> Result<()> {
    if c.version != 1 || c.result != source_id(&Source::Brep(result.clone()))? {
        return Err(fail("retained Boolean result/version binding mismatch"));
    }
    let fresh = certify_retained_nurbs_boolean(a, b, &c.operation, &c.partitions, c.graph.limits)?;
    if fresh.model != *result || fresh.certificate != *c {
        return Err(fail(
            "retained-region partition, winding, graph or result receipt changed",
        ));
    }
    Ok(())
}
