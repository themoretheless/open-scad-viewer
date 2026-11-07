//! Complete exact real branch graphs on the closed parameter square. Slab
//! sections stop at critical fibers; singular ports are never numerically paired.
use super::*;
use arithmetic::{
    Poly, algebraic_events::fibers, bivariate as bi, derivative, gcd, product, quotient,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlgebraicSetDimension {
    Empty,
    Points,
    Curves,
    Area,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AlgebraicBranchKind {
    Section { slab: usize, rank: usize },
    Vertical { cut: usize, rank: usize },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlgebraicBranchEdge {
    pub kind: AlgebraicBranchKind,
    pub endpoints: [u32; 2],
    pub multiplicity: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlgebraicBranchVertex {
    pub cut: usize,
    pub rank: usize,
    pub singular: bool,
    pub ports: Vec<i32>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlgebraicBranchComponent {
    pub vertices: Vec<u32>,
    pub edges: Vec<u32>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct AlgebraicBranchCertificate {
    pub version: u32,
    pub source: String,
    pub polynomial: Vec<Vec<ExactRatio>>,
    pub dimension: AlgebraicSetDimension,
    pub decomposition: ParameterBranchDecomposition,
    pub vertices: Vec<AlgebraicBranchVertex>,
    pub edges: Vec<AlgebraicBranchEdge>,
    pub components: Vec<AlgebraicBranchComponent>,
}
#[derive(Clone, Debug)]
pub struct VerifiedAlgebraicBranches {
    certificate: AlgebraicBranchCertificate,
}
impl VerifiedAlgebraicBranches {
    pub fn certificate(&self) -> &AlgebraicBranchCertificate {
        &self.certificate
    }
    /// Enclosures identify algebraic selectors, not approximate authored vertices.
    pub fn vertex_enclosure(&self, id: usize) -> Result<[[ExactRatio; 2]; 2]> {
        let v = self
            .certificate
            .vertices
            .get(id)
            .ok_or_else(|| refused("Missing algebraic branch vertex"))?;
        let cut = &self.certificate.decomposition.cuts[v.cut];
        Ok([cut.parameter.clone(), cut.fibers[v.rank].clone()])
    }
    pub fn reverify(&self) -> Result<()> {
        verify_algebraic_branches(&self.certificate.polynomial, &self.certificate).map(|_| ())
    }
}
fn decode(p: &[Vec<ExactRatio>]) -> Result<Vec<Poly>> {
    if p.len() > 17 || p.iter().any(|r| r.len() > 17) {
        return Err(refused("Algebraic branch tensor degree exceeds16"));
    }
    let mut p = p
        .iter()
        .map(|r| r.iter().map(atlas_roots::decode).collect::<Result<Poly>>())
        .collect::<Result<Vec<_>>>()?;
    for r in &mut p {
        while r.last().is_some_and(Zero::is_zero) {
            r.pop();
        }
    }
    while p.last().is_some_and(Vec::is_empty) {
        p.pop();
    }
    Ok(p)
}
fn dy(p: &[Poly]) -> Vec<Poly> {
    p.iter()
        .enumerate()
        .skip(1)
        .map(|(j, r)| r.iter().map(|q| q * Q::from_integer(j.into())).collect())
        .collect()
}
fn original_radical(p: &Vec<Poly>) -> Result<(Vec<Poly>, Poly)> {
    if p.is_empty() {
        return Ok((vec![], vec![]));
    }
    let content = p.iter().try_fold(vec![], |a, r| gcd(&a, r))?;
    let c = quotient(&content, &gcd(&content, &derivative(&content))?)?;
    let primitive = bi::primitive(p.clone())?;
    let reduced =
        bi::exact_bivariate_quotient(&primitive, &bi::common(primitive.clone(), dy(&primitive))?)?;
    Ok((reduced.iter().map(|r| product(r, &c)).collect(), content))
}
fn multiplicity(mut p: Poly, selector: &Poly, range: &[Q; 2]) -> Result<usize> {
    for order in 0..=16 {
        if atlas_roots::arrangement_sign(&p, selector, range)? != 0 {
            return Ok(order);
        }
        p = derivative(&p);
        if p.is_empty() {
            break;
        }
    }
    Err(refused("Algebraic branch multiplicity is not finite"))
}
pub fn certify_algebraic_branches(input: &[Vec<ExactRatio>]) -> Result<VerifiedAlgebraicBranches> {
    let h = decode(input)?;
    let encoded = h
        .iter()
        .map(|r| r.iter().map(atlas_roots::ratio).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let mut hash = Sha256::new();
    hash.update(b"exact-algebraic-branch-square-v1");
    for r in &h {
        hash.update(b"[");
        for q in r {
            hash.update(q.numer().to_string());
            hash.update(b"/");
            hash.update(q.denom().to_string());
            hash.update(b";");
        }
        hash.update(b"]");
    }
    let source = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    let decomposition = branches::decompose(&h, None, &Q::new(1.into(), 65536.into()))?;
    let (radical, content) = original_radical(&h)?;
    let projection = decomposition
        .projection
        .iter()
        .map(atlas_roots::decode)
        .collect::<Result<Poly>>()?;
    let relation = decomposition
        .fiber_relation
        .iter()
        .map(|r| r.iter().map(atlas_roots::decode).collect())
        .collect::<Result<Vec<Poly>>>()?;
    let dx = radical.iter().map(|r| derivative(r)).collect::<Vec<_>>();
    let dy = dy(&radical);
    let mut vertices = vec![];
    let mut ids = BTreeMap::new();
    for (cut, c) in decomposition.cuts.iter().enumerate() {
        let x = c
            .parameter
            .each_ref()
            .map(atlas_roots::decode)
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        let x = [x[0].clone(), x[1].clone()];
        let mut node_relation = relation.clone();
        if c.vertical {
            // Only these artificial boundary nodes are added by decompose(None).
            let mut out = vec![vec![]; relation.len() + 2];
            for (j, r) in relation.iter().enumerate() {
                out[j + 1] = arithmetic::sub(&out[j + 1], r);
                out[j + 2] = arithmetic::sub(&out[j + 2], &r.iter().map(|v| -v).collect::<Poly>());
            }
            node_relation = out;
        }
        for (rank, range) in c.fibers.iter().enumerate() {
            let y = [
                atlas_roots::decode(&range[0])?,
                atlas_roots::decode(&range[1])?,
            ];
            let singular = fibers::zero_at_node(&dx, &node_relation, &projection, &x, &y)?
                && fibers::zero_at_node(&dy, &node_relation, &projection, &x, &y)?;
            ids.insert((cut, rank), vertices.len() as u32);
            vertices.push(AlgebraicBranchVertex {
                cut,
                rank,
                singular,
                ports: vec![],
            });
        }
    }
    let mut edges = vec![];
    for (slab, s) in decomposition.slabs.iter().enumerate() {
        let u = atlas_roots::decode(&s.sample)?;
        let f = bi::fiber(&h, &u);
        let selector = bi::fiber(&relation, &u);
        for (rank, r) in s.roots.iter().enumerate() {
            let range = [atlas_roots::decode(&r[0])?, atlas_roots::decode(&r[1])?];
            let a = *ids
                .get(&(slab, decomposition.cuts[slab].right[rank]))
                .ok_or_else(|| refused("Missing left algebraic incidence"))?;
            let b = *ids
                .get(&(slab + 1, decomposition.cuts[slab + 1].left[rank]))
                .ok_or_else(|| refused("Missing right algebraic incidence"))?;
            edges.push(AlgebraicBranchEdge {
                kind: AlgebraicBranchKind::Section { slab, rank },
                endpoints: [a, b],
                multiplicity: multiplicity(f.clone(), &selector, &range)?,
            });
        }
    }
    for (cut, c) in decomposition
        .cuts
        .iter()
        .enumerate()
        .filter(|(_, c)| c.vertical)
    {
        let x = [
            atlas_roots::decode(&c.parameter[0])?,
            atlas_roots::decode(&c.parameter[1])?,
        ];
        let m = multiplicity(content.clone(), &projection, &x)?;
        for [a, b] in &c.vertical_arcs {
            edges.push(AlgebraicBranchEdge {
                kind: AlgebraicBranchKind::Vertical { cut, rank: *a },
                endpoints: [ids[&(cut, *a)], ids[&(cut, *b)]],
                multiplicity: m,
            });
        }
    }
    if vertices.len() + edges.len() > 16384 {
        return Err(refused("Algebraic branch graph entity budget exceeded"));
    }
    for (id, v) in vertices.iter_mut().enumerate() {
        let mut ports = vec![];
        for (k, e) in edges.iter().enumerate() {
            for side in 0..2 {
                if e.endpoints[side] as usize == id {
                    let signed = (k + 1) as i32 * if side == 0 { 1 } else { -1 };
                    let order = match e.kind {
                        AlgebraicBranchKind::Section { slab, rank } => {
                            if slab == v.cut {
                                (0, rank)
                            } else {
                                (2, usize::MAX - rank)
                            }
                        }
                        AlgebraicBranchKind::Vertical { .. } => {
                            if side == 0 {
                                (1, 0)
                            } else {
                                (3, 0)
                            }
                        }
                    };
                    ports.push((order, signed));
                }
            }
        }
        ports.sort_by_key(|p| p.0);
        v.ports = ports.into_iter().map(|p| p.1).collect();
    }
    let mut components = vec![];
    let mut seen = BTreeSet::new();
    for start in 0..vertices.len() as u32 {
        if !seen.contains(&start) {
            let mut stack = vec![start];
            let mut vs = BTreeSet::new();
            let mut es = BTreeSet::new();
            while let Some(v) = stack.pop() {
                if seen.insert(v) {
                    vs.insert(v);
                    for port in &vertices[v as usize].ports {
                        let e = port.unsigned_abs() - 1;
                        es.insert(e);
                        stack.extend(edges[e as usize].endpoints);
                    }
                }
            }
            components.push(AlgebraicBranchComponent {
                vertices: vs.into_iter().collect(),
                edges: es.into_iter().collect(),
            });
        }
    }
    let dimension = if h.is_empty() {
        AlgebraicSetDimension::Area
    } else if !edges.is_empty() {
        AlgebraicSetDimension::Curves
    } else if !vertices.is_empty() {
        AlgebraicSetDimension::Points
    } else {
        AlgebraicSetDimension::Empty
    };
    Ok(VerifiedAlgebraicBranches {
        certificate: AlgebraicBranchCertificate {
            version: 1,
            source,
            polynomial: encoded,
            dimension,
            decomposition,
            vertices,
            edges,
            components,
        },
    })
}
pub fn verify_algebraic_branches(
    input: &[Vec<ExactRatio>],
    c: &AlgebraicBranchCertificate,
) -> Result<VerifiedAlgebraicBranches> {
    if c.version != 1 {
        return Err(refused("Algebraic branch certificate version mismatch"));
    }
    let fresh = certify_algebraic_branches(input)?;
    if fresh.certificate != *c {
        return Err(refused(
            "Algebraic source, singularity, multiplicity or incidence changed",
        ));
    }
    Ok(fresh)
}
