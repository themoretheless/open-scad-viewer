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
    pub result_shells: Option<NurbsShellForestCertificate>,
    pub xor_decomposition: Option<RetainedXorDecomposition>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct RetainedXorDecomposition {
    /// Separately embedded A\\B and B\\A. Their material interiors are
    /// disjoint by the checked source-region truth tables. Closure contact is
    /// allowed; no global embedded-boundary atlas is asserted for their join.
    pub parts: [Model; 2],
    pub atlases: [Option<SourceAtlasCertificate>; 2],
    pub shell_forests: [Option<NurbsShellForestCertificate>; 2],
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
    let (model, result_atlas, result_shells, xor_decomposition) = if operation == "xor" {
        let mut parts = Vec::new();
        let mut atlases = Vec::new();
        let mut forests = Vec::new();
        for part in 0..2 {
            let mut selected = regions.clone();
            for r in &mut selected {
                let owner = if r.winding_in_other == 0 {
                    r.operand
                } else {
                    1 - r.operand
                };
                r.keep = owner == part;
            }
            let (m, a, f) = assemble::build(models, &graph, partitions, &selected)?;
            parts.push(m);
            atlases.push(a);
            forests.push(f);
        }
        let parts: [Model; 2] = parts.try_into().unwrap();
        let joined = crate::boolean_support::concatenate_bodies(&parts[0], &parts[1])?;
        let decomposition = RetainedXorDecomposition {
            parts,
            atlases: atlases.try_into().unwrap(),
            shell_forests: forests.try_into().unwrap(),
        };
        (joined, None, None, Some(decomposition))
    } else {
        let (m, a, f) = assemble::build(models, &graph, partitions, &regions)
            .map_err(|e| fail(&format!("assembly: {}", e.message)))?;
        (m, a, f, None)
    };
    let certificate = RetainedNurbsBooleanCertificate {
        version: 3,
        operation: operation.into(),
        graph: graph.certificate().clone(),
        partitions: partitions.clone(),
        regions,
        result: source_id(&Source::Brep(model.clone()))?,
        result_atlas,
        result_shells,
        xor_decomposition,
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
    if c.version != 3 || c.result != source_id(&Source::Brep(result.clone()))? {
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
