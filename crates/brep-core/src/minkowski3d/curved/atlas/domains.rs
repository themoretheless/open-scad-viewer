//! A checked 3D atlas with exact implicit parameter domains. The parent source
//! charts and their full seam relations are retained, without algebraic rounding.
use super::*;
use crate::profile_certificate::{atlas_roots as roots, ExactRatio, ParameterBranchDecomposition};
mod algebra;
mod complex;
use algebra::Bi;
pub type DomainPolynomial = Vec<Vec<ExactRatio>>;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomainVertex {
    pub cut: usize,
    pub rank: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DomainEdgeKind {
    Section { slab: usize, rank: usize },
    Fiber { cut: usize, rank: usize },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomainEdge {
    pub kind: DomainEdgeKind,
    pub endpoints: [u32; 2],
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomainFace {
    pub slab: usize,
    pub rank: usize,
    /// Signed one-based edge IDs in exact oriented boundary order.
    pub boundary: Vec<i32>,
    /// Polynomial signs on the complete connected open cell, not sampled geometry.
    pub signs: Vec<i8>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct DomainPartition {
    pub cuts: Vec<DomainPolynomial>,
    pub arrangement: ParameterBranchDecomposition,
    pub vertices: Vec<DomainVertex>,
    pub edges: Vec<DomainEdge>,
    pub faces: Vec<DomainFace>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomainSeam {
    pub parents: [u32; 2],
    pub local_edges: [u8; 2],
    /// Both parameters are oriented from the smaller to the larger base ID.
    /// Otherwise the transition is exact equality of the original 3D edge maps.
    pub pointwise: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct AlgebraicDomainAtlas {
    pub version: u32,
    pub source: String,
    pub base: Certificate,
    pub maps: Vec<pullback::AtlasExactTriangle>,
    pub partitions: Vec<DomainPartition>,
    pub seams: Vec<DomainSeam>,
}
fn partition(cuts: &[DomainPolynomial]) -> Result<DomainPartition> {
    if cuts.len() > 4 {
        return Err(fail("algebraic atlas per-chart cut budget exceeded"));
    }
    let decoded = cuts
        .iter()
        .map(algebra::decode)
        .collect::<Result<Vec<_>>>()?;
    let joint = algebra::joint(&decoded)?;
    let arrangement = roots::arrangement_branches(&joint)?;
    complex::assemble(cuts, &decoded, arrangement)
}
fn source(m: &Model, depth: u8, cuts: &[Vec<DomainPolynomial>]) -> Result<String> {
    if cuts.is_empty() || cuts.len() > 1024 {
        return Err(fail("algebraic atlas cut request shape budget exceeded"));
    }
    let mut h = Sha256::new();
    h.update(b"source-algebraic-domain-atlas-v1");
    h.update(source_id(&Source::Brep(m.clone()))?.as_bytes());
    h.update([depth]);
    h.update((cuts.len() as u64).to_le_bytes());
    for chart in cuts {
        if chart.len() > 4 {
            return Err(fail("algebraic atlas cut count budget exceeded"));
        }
        h.update((chart.len() as u64).to_le_bytes());
        for p in chart {
            let p = algebra::decode(p)?;
            h.update((p.len() as u64).to_le_bytes());
            for row in p {
                h.update((row.len() as u64).to_le_bytes());
                for q in row {
                    for s in roots::ratio(&q) {
                        h.update((s.len() as u64).to_le_bytes());
                        h.update(s.as_bytes());
                    }
                }
            }
        }
    }
    Ok(h.finalize().iter().map(|x| format!("{x:02x}")).collect())
}
fn assemble(m: &Model, depth: u8, cuts: &[Vec<DomainPolynomial>]) -> Result<AlgebraicDomainAtlas> {
    let source = source(m, depth, cuts)?;
    let (base, patches) = checked_charts(m, depth)?;
    if cuts.len() != 1 && cuts.len() != patches.len() {
        return Err(fail(
            "algebraic atlas needs one broadcast cut set or one per source chart",
        ));
    }
    let common = if cuts.len() == 1 {
        Some(partition(&cuts[0])?)
    } else {
        None
    };
    let mut partitions = Vec::new();
    let mut cells = 0usize;
    for i in 0..patches.len() {
        let p = if let Some(p) = &common {
            p.clone()
        } else {
            partition(&cuts[i])?
        };
        cells = cells.saturating_add(p.vertices.len() + p.edges.len() + p.faces.len());
        if cells > 16384 {
            return Err(fail("algebraic atlas cell budget exceeded"));
        }
        partitions.push(p);
    }
    let mut pairings = BTreeMap::<(u32, u32), Vec<(usize, usize, Vec<sb::H>)>>::new();
    let mut maps = Vec::new();
    for (i, p) in patches.iter().enumerate() {
        maps.push(
            p.h.iter()
                .map(|r| r.iter().map(|h| h.each_ref().map(roots::ratio)).collect())
                .collect(),
        );
        for (k, mut h) in bernstein::edges(&p.h).into_iter().enumerate() {
            let (a, b) = (p.ids[k], p.ids[(k + 1) % 3]);
            if a > b {
                h.reverse();
            }
            pairings
                .entry((a.min(b), a.max(b)))
                .or_default()
                .push((i, k, h));
        }
    }
    let mut seams = Vec::new();
    let mut work = Work::default();
    for pair in pairings.values() {
        if pair.len() != 2 {
            return Err(fail("algebraic atlas parent seam is not paired"));
        }
        let pointwise = sb::same(&pair[0].2, &pair[1].2);
        if !pointwise && !segment_seam(&pair[0].2, &pair[1].2, &mut work)? {
            return Err(fail("algebraic atlas seam transition is not proved"));
        }
        seams.push(DomainSeam {
            parents: [pair[0].0 as u32, pair[1].0 as u32],
            local_edges: [pair[0].1 as u8, pair[1].1 as u8],
            pointwise,
        });
    }
    Ok(AlgebraicDomainAtlas {
        version: 1,
        source,
        base,
        maps,
        partitions,
        seams,
    })
}
/// Exact domain refinement of a source body whose global embedding is proved
/// by the base source atlas. Child boundaries need not be a conforming PL mesh.
pub fn certify_algebraic_domain_atlas(
    m: &Model,
    depth: u8,
    cuts: &[Vec<DomainPolynomial>],
) -> Result<AlgebraicDomainAtlas> {
    assemble(m, depth, cuts)
}
pub fn verify_algebraic_domain_atlas(
    m: &Model,
    depth: u8,
    cuts: &[Vec<DomainPolynomial>],
    cert: &AlgebraicDomainAtlas,
) -> Result<()> {
    if cert.version != 1 || cert.source != source(m, depth, cuts)? {
        return Err(fail(
            "algebraic domain atlas source/schema binding is false",
        ));
    }
    // Reconstruct both the global source embedding and every implicit local
    // cell/attachment from exact source data, never trust the supplied topology.
    if *cert != assemble(m, depth, cuts)? {
        return Err(fail(
            "algebraic domain atlas maps, cells or seam relations are false",
        ));
    }
    Ok(())
}
#[cfg(test)]
mod tests;
