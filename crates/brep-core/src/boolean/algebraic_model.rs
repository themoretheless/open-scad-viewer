//! Exact algebraic geometry payloads. This type deliberately has no Deref or
//! implicit conversion to the legacy rational-Curve Model.
use crate::minkowski3d::curved::{
    AlgebraicRegionBrep, AlgebraicRegionCertificate, DomainEdgeKind, DomainPolynomial,
    DomainVertex, GraphSlab,
};
use crate::profile_certificate::{ExactRatio, atlas_roots as roots};
use crate::{Error, Result, Surface};
use std::collections::BTreeMap;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlgebraicVertex {
    pub domain_vertex: u32,
    pub selector: DomainVertex,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlgebraicEdge {
    pub domain_edge: u32,
    pub selector: DomainEdgeKind,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlgebraicPcurve {
    pub domain_edge: u32,
    pub carrier: u8,
    pub reversed: bool,
    pub normalized_uv: [[ExactRatio; 3]; 2],
}
pub type AlgebraicTopology =
    brep_topology::Model<AlgebraicEdge, Surface, AlgebraicPcurve, AlgebraicVertex>;
#[derive(Clone, Debug)]
pub struct AlgebraicModel {
    region: AlgebraicRegionBrep,
    topology: AlgebraicTopology,
}
fn fail(message: &str) -> Error {
    Error::new("BREP_ALGEBRAIC_MODEL_UNPROVED", message)
}
impl AlgebraicModel {
    pub fn from_region(region: AlgebraicRegionBrep, tolerance_mm: f64) -> Result<Self> {
        let region = crate::minkowski3d::curved::verify_graph_region_brep(
            &region.carriers()[0],
            &region.carriers()[1],
            region.certificate(),
        )?;
        let cert = region.certificate();
        let mut topology = AlgebraicTopology {
            vertices: vec![],
            edges: vec![],
            loops: vec![],
            faces: vec![],
            shells: vec![],
            bodies: vec![],
            tolerance_mm,
        };
        for (component, _) in cert.components.iter().enumerate() {
            let mut vs = BTreeMap::new();
            let mut es = BTreeMap::new();
            let mut uses = vec![];
            for face_id in [2 * component, 2 * component + 1] {
                let face = &cert.faces[face_id];
                let mut cycles = face.boundary_cycles.clone();
                // CAD vertex IDs are ordered by critical x and then y. The
                // outer cycle has the leftmost vertex of a connected region;
                // every strictly nested hole starts later. No rounded area.
                cycles.sort_by_key(|cycle| {
                    cycle
                        .iter()
                        .flat_map(|e| cert.partition.edges[e.unsigned_abs() as usize - 1].endpoints)
                        .min()
                });
                let mut wires = vec![];
                for mut cycle in cycles {
                    if face.reversed {
                        cycle = cycle.into_iter().rev().map(|e| -e).collect();
                    }
                    let mut coedges = vec![];
                    for signed in cycle {
                        let id = signed.unsigned_abs() - 1;
                        let e = &cert.partition.edges[id as usize];
                        let index = if let Some(&index) = es.get(&id) {
                            index
                        } else {
                            let mut endpoints = [0; 2];
                            for k in 0..2 {
                                let v = e.endpoints[k];
                                let index = *vs.entry(v).or_insert_with(|| {
                                    let i = topology.vertices.len();
                                    topology.vertices.push(brep_topology::Vertex {
                                        point: AlgebraicVertex {
                                            domain_vertex: v,
                                            selector: cert.partition.vertices[v as usize].clone(),
                                        },
                                    });
                                    i
                                });
                                endpoints[k] = index;
                            }
                            let i = topology.edges.len();
                            topology.edges.push(brep_topology::Edge {
                                degenerate: false,
                                vertices: endpoints,
                                curve: AlgebraicEdge {
                                    domain_edge: id,
                                    selector: e.kind.clone(),
                                },
                            });
                            es.insert(id, i);
                            i
                        };
                        coedges.push(brep_topology::Coedge::new(
                            index,
                            signed < 0,
                            AlgebraicPcurve {
                                domain_edge: id,
                                carrier: face.carrier,
                                reversed: signed < 0,
                                normalized_uv: cert.normalized_parameter_maps
                                    [face.carrier as usize]
                                    .clone(),
                            },
                        ));
                    }
                    wires.push(topology.loops.len());
                    topology.loops.push(brep_topology::Loop { coedges });
                }
                let outer = *wires
                    .first()
                    .ok_or_else(|| fail("Algebraic face has no outer cycle"))?;
                let fi = topology.faces.len();
                topology.faces.push(brep_topology::Face {
                    surface: region.carriers()[face.carrier as usize].clone(),
                    outer,
                    holes: wires[1..].to_vec(),
                });
                uses.push(brep_topology::FaceUse {
                    face: fi,
                    reversed: face.reversed,
                });
            }
            let si = topology.shells.len();
            topology.shells.push(brep_topology::Shell {
                faces: uses,
                closed: true,
            });
            topology.bodies.push(brep_topology::Body {
                outer_shell: si,
                inner_shells: vec![],
            });
        }
        topology.validate_topology_with_vertices(|v| {
            if cert.partition.vertices.get(v.domain_vertex as usize) != Some(&v.selector) {
                return Err(fail("Algebraic vertex selector changed"));
            }
            Ok(())
        })?;
        Ok(Self { region, topology })
    }
    pub fn topology(&self) -> &AlgebraicTopology {
        &self.topology
    }
    pub fn certificate(&self) -> &AlgebraicRegionCertificate {
        self.region.certificate()
    }
    pub fn carriers(&self) -> &[Surface; 2] {
        self.region.carriers()
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = Self::from_region(self.region.clone(), self.topology.tolerance_mm)?;
        if fresh.topology != self.topology {
            return Err(fail(
                "Algebraic model incidence or geometry binding changed",
            ));
        }
        Ok(())
    }
    pub fn from_certificate(
        carriers: [Surface; 2],
        certificate: &AlgebraicRegionCertificate,
        tolerance_mm: f64,
    ) -> Result<Self> {
        let region = crate::minkowski3d::curved::verify_graph_region_brep(
            &carriers[0],
            &carriers[1],
            certificate,
        )?;
        Self::from_region(region, tolerance_mm)
    }
    pub fn evaluate_face_rational(
        &self,
        face: u32,
        u: &ExactRatio,
        v: &ExactRatio,
    ) -> Result<[ExactRatio; 3]> {
        self.region.evaluate_rational(face, u, v)
    }
    /// A source-clock query inside an open algebraic section. Returns a proved
    /// parameter enclosure; it does not round an algebraic point into a pole.
    pub fn edge_parameter_enclosure(
        &self,
        edge: usize,
        parameter: &ExactRatio,
    ) -> Result<[[ExactRatio; 2]; 2]> {
        let e = self
            .topology
            .edges
            .get(edge)
            .ok_or_else(|| fail("Missing algebraic model edge"))?;
        let DomainEdgeKind::Section { slab, rank } = e.curve.selector else {
            return Err(fail(
                "Critical-fiber queries need a dependent algebraic clock",
            ));
        };
        let x = roots::decode(parameter)?;
        let d = &self.certificate().partition.arrangement;
        let projection = roots::arrangement_polynomial(&d.projection)?;
        let linear = vec![
            -x.clone(),
            num_rational::BigRational::from_integer(1.into()),
        ];
        let lo = d.cuts[slab].parameter.each_ref().map(roots::decode);
        let hi = d.cuts[slab + 1].parameter.each_ref().map(roots::decode);
        let lo = [lo[0].clone()?, lo[1].clone()?];
        let hi = [hi[0].clone()?, hi[1].clone()?];
        if roots::arrangement_sign(&linear, &projection, &lo)? != -1
            || roots::arrangement_sign(&linear, &projection, &hi)? != 1
        {
            return Err(fail("Algebraic query is outside the open source slab"));
        }
        let fiber = d
            .fiber_relation
            .iter()
            .map(|r| {
                let p = roots::arrangement_polynomial(r)?;
                Ok(p.iter()
                    .rev()
                    .fold(num_rational::BigRational::from_integer(0.into()), |v, c| {
                        v * &x + c
                    }))
            })
            .collect::<Result<Vec<_>>>()?;
        let y = roots::roots(&fiber)?
            .get(rank)
            .cloned()
            .ok_or_else(|| fail("Source algebraic root rank disappeared"))?;
        Ok([
            [parameter.clone(), parameter.clone()],
            y.each_ref().map(roots::ratio),
        ])
    }
}
#[derive(Clone, Debug)]
pub enum ExactModel {
    Nurbs(crate::Model),
    GraphSlab(GraphSlab),
    Algebraic(AlgebraicModel),
}
#[derive(Clone, Debug)]
pub struct ExactBooleanOptions {
    pub tolerance_mm: f64,
    pub regularity_multipliers: Vec<DomainPolynomial>,
}
impl Default for ExactBooleanOptions {
    fn default() -> Self {
        Self {
            tolerance_mm: 1e-6,
            regularity_multipliers: vec![vec![vec![["4".into(), "1".into()]]]],
        }
    }
}
/// Typed exact dispatch. Legacy rational geometry retains its existing entry;
/// algebraic output cannot be consumed as an unchecked rational Curve Model.
pub fn boolean_exact(
    a: &ExactModel,
    b: &ExactModel,
    operation: &str,
    options: &ExactBooleanOptions,
) -> Result<ExactModel> {
    if !matches!(operation, "intersection" | "union" | "difference" | "xor") {
        return Err(Error::new(
            "BREP_INVALID_OPERATION",
            "Invalid exact Boolean operation",
        ));
    }
    let identical = match (a, b) {
        (ExactModel::GraphSlab(a), ExactModel::GraphSlab(b)) => {
            a.lower() == b.lower() && a.upper() == b.upper()
        }
        (ExactModel::Algebraic(a), ExactModel::Algebraic(b)) => {
            a.carriers() == b.carriers()
                && a.certificate() == b.certificate()
                && a.topology() == b.topology()
        }
        _ => false,
    };
    if identical {
        return if matches!(operation, "intersection" | "union") {
            Ok(a.clone())
        } else {
            crate::Model::empty(options.tolerance_mm).map(ExactModel::Nurbs)
        };
    }
    match (a, b) {
        (ExactModel::Nurbs(a), ExactModel::Nurbs(b)) => {
            crate::operations::boolean(a, b, operation).map(ExactModel::Nurbs)
        }
        (ExactModel::GraphSlab(a), ExactModel::GraphSlab(b)) if operation == "intersection" => {
            if options.regularity_multipliers.is_empty() || options.regularity_multipliers.len() > 8
            {
                return Err(fail("Exact Boolean multiplier budget must be1..8"));
            }
            let mut last = None;
            for multiplier in &options.regularity_multipliers {
                match certify_graph_slab_boolean(a, b, multiplier, options.tolerance_mm) {
                    Ok(result) => return Ok(ExactModel::Algebraic(result.into_model())),
                    Err(e) if e.code.ends_with("REFUSED") => last = Some(e),
                    Err(e) => return Err(e),
                }
            }
            Err(last.unwrap_or_else(|| fail("Algebraic intersection is unproved")))
        }
        _ => Err(fail(
            "General algebraic-solid Boolean requires source-face overlay and material reconstruction",
        )),
    }
}

#[cfg(feature = "codec")]
mod codec {
    use super::*;
    use value_codec::{Deserialize, Serialize, Value, error, json};
    fn selector(s: &DomainEdgeKind) -> Value {
        match s {
            DomainEdgeKind::Section { slab, rank } => {
                json!({"kind":"section","slab":slab,"rank":rank})
            }
            DomainEdgeKind::Fiber { cut, rank } => json!({"kind":"fiber","cut":cut,"rank":rank}),
        }
    }
    impl Serialize for AlgebraicVertex {
        fn to_value(&self) -> Value {
            json!({"domainVertex":self.domain_vertex,"cut":self.selector.cut,"rank":self.selector.rank})
        }
    }
    impl Serialize for AlgebraicEdge {
        fn to_value(&self) -> Value {
            json!({"domainEdge":self.domain_edge,"selector":selector(&self.selector)})
        }
    }
    impl Serialize for AlgebraicPcurve {
        fn to_value(&self) -> Value {
            json!({"domainEdge":self.domain_edge,"carrier":self.carrier,"reversed":self.reversed,"normalizedUv":self.normalized_uv})
        }
    }
    impl Serialize for AlgebraicModel {
        fn to_value(&self) -> Value {
            let c = self.certificate();
            json!({"version":1,"kind":"exact-algebraic-brep","carriers":self.carriers(),"multiplier":c.regularity_multiplier,"factors":c.intersection_factors,"selection":c.selected,"topology":self.topology})
        }
    }
    impl Serialize for ExactModel {
        fn to_value(&self) -> Value {
            match self {
                ExactModel::Nurbs(m) => json!({"version":1,"kind":"nurbs","model":m}),
                ExactModel::GraphSlab(s) => {
                    json!({"version":1,"kind":"graph-slab","lower":s.lower(),"upper":s.upper()})
                }
                ExactModel::Algebraic(m) => json!({"version":1,"kind":"algebraic","model":m}),
            }
        }
    }
    impl<'de> Deserialize<'de> for ExactModel {
        fn from_value(v: Value) -> value_codec::Result<Self> {
            if v["version"].as_u64() != Some(1) {
                return Err(error("Unknown exact model version"));
            }
            match v["kind"].as_str() {
                Some("nurbs") => {
                    let m: crate::Model = Deserialize::from_value(v["model"].clone())?;
                    m.validate().map_err(|e| error(e.to_string()))?;
                    Ok(ExactModel::Nurbs(m))
                }
                Some("graph-slab") => {
                    let lower = Deserialize::from_value(v["lower"].clone())?;
                    let upper = Deserialize::from_value(v["upper"].clone())?;
                    GraphSlab::new(lower, upper)
                        .map(ExactModel::GraphSlab)
                        .map_err(|e| error(e.to_string()))
                }
                Some("algebraic") => {
                    AlgebraicModel::from_value(v["model"].clone()).map(ExactModel::Algebraic)
                }
                _ => Err(error("Unknown exact geometry kind")),
            }
        }
    }
    impl<'de> Deserialize<'de> for AlgebraicModel {
        fn from_value(v: Value) -> value_codec::Result<Self> {
            if v["version"].as_u64() != Some(1)
                || v["kind"].as_str() != Some("exact-algebraic-brep")
                || v["carriers"].as_array().is_none_or(|a| a.len() != 2)
                || v["selection"].as_array().is_none_or(|a| a.len() > 16384)
                || v["factors"].as_array().is_none_or(|a| a.len() > 13)
            {
                return Err(error("Invalid exact algebraic model envelope"));
            }
            let carriers: [Surface; 2] = Deserialize::from_value(v["carriers"].clone())?;
            let multiplier: DomainPolynomial = Deserialize::from_value(v["multiplier"].clone())?;
            let factors: Vec<DomainPolynomial> = Deserialize::from_value(v["factors"].clone())?;
            let selection: Vec<u32> = Deserialize::from_value(v["selection"].clone())?;
            let tolerance: f64 = Deserialize::from_value(v["topology"]["toleranceMm"].clone())?;
            let region = if factors.is_empty() {
                crate::minkowski3d::curved::certify_graph_region_brep(
                    &carriers[0],
                    &carriers[1],
                    &multiplier,
                    Some(&selection),
                )
            } else {
                crate::minkowski3d::curved::certify_graph_region_brep_factored(
                    &carriers[0],
                    &carriers[1],
                    &multiplier,
                    &factors,
                    Some(&selection),
                )
            }
            .map_err(|e| error(e.to_string()))?;
            let model =
                AlgebraicModel::from_region(region, tolerance).map_err(|e| error(e.to_string()))?;
            if model.topology.to_value() != v["topology"] {
                return Err(error(
                    "Algebraic topology or source selectors differ from the independently reconstructed model",
                ));
            }
            Ok(model)
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AlgebraicBooleanCertificate {
    pub version: u32,
    pub operation: String,
    pub input_sources: [[String; 2]; 2],
    pub envelope_reversed: bool,
    pub multiplier: DomainPolynomial,
    pub tolerance_mm: f64,
    pub result: AlgebraicRegionCertificate,
}
pub struct CertifiedAlgebraicBoolean {
    model: AlgebraicModel,
    certificate: AlgebraicBooleanCertificate,
}
impl CertifiedAlgebraicBoolean {
    pub fn model(&self) -> &AlgebraicModel {
        &self.model
    }
    pub fn certificate(&self) -> &AlgebraicBooleanCertificate {
        &self.certificate
    }
    pub fn into_model(self) -> AlgebraicModel {
        self.model
    }
}
pub fn certify_graph_slab_boolean(
    a: &GraphSlab,
    b: &GraphSlab,
    multiplier: &DomainPolynomial,
    tolerance_mm: f64,
) -> Result<CertifiedAlgebraicBoolean> {
    let (region, envelope_reversed) =
        match crate::minkowski3d::curved::intersect_graph_slabs(a, b, multiplier) {
            Ok(region) => (region, false),
            Err(e) if e.code.ends_with("REFUSED") => (
                crate::minkowski3d::curved::intersect_graph_slabs(b, a, multiplier)?,
                true,
            ),
            Err(e) => return Err(e),
        };
    let model = AlgebraicModel::from_region(region, tolerance_mm)?;
    let certificate = AlgebraicBooleanCertificate {
        version: 1,
        operation: "intersection".into(),
        input_sources: [a.source_ids()?, b.source_ids()?],
        envelope_reversed,
        multiplier: multiplier.clone(),
        tolerance_mm,
        result: model.certificate().clone(),
    };
    Ok(CertifiedAlgebraicBoolean { model, certificate })
}
pub fn verify_graph_slab_boolean(
    a: &GraphSlab,
    b: &GraphSlab,
    result: &AlgebraicModel,
    c: &AlgebraicBooleanCertificate,
) -> Result<()> {
    if c.version != 1
        || c.operation != "intersection"
        || c.input_sources != [a.source_ids()?, b.source_ids()?]
    {
        return Err(fail(
            "Algebraic Boolean source or operation binding changed",
        ));
    }
    let fresh = certify_graph_slab_boolean(a, b, &c.multiplier, c.tolerance_mm)?;
    if fresh.certificate != *c
        || fresh.model.topology() != result.topology()
        || fresh.model.certificate() != result.certificate()
        || fresh.model.carriers() != result.carriers()
    {
        return Err(fail(
            "Algebraic Boolean result or exact edge selectors changed",
        ));
    }
    Ok(())
}
