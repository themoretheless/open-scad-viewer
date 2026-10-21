//! Separating original-boundary cochains and positively witnessed components.
use super::*;
use crate::profile_certificate::algebraic_intersections::{decode_polynomial, decode_ratio};
use arithmetic::branch_neighbourhood::ParameterValue;
use num_bigint::BigInt;
use std::collections::BTreeMap;
use std::sync::Arc;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicCochainTerm {
    pub walk: usize,
    pub coefficient: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicSeparatingCochain {
    pub terms: Vec<OriginalPeriodicCochainTerm>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicRegionProbe {
    pub walk: usize,
    /// Actual graph edge and side of a selected original material sample.
    pub endpoint: [usize; 2],
    pub signature: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct SignatureData {
    cochains: Vec<OriginalPeriodicSeparatingCochain>,
    probes: Vec<OriginalPeriodicRegionProbe>,
    work: usize,
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicSupportSignatures {
    owner: Arc<OwnedPeriodicSupportLifts>,
    limits: ParameterOrderLimits,
    max_work: usize,
    data: SignatureData,
}
fn integer(s: &str) -> Result<BigInt> {
    if s.is_empty() || s.len() > 5000 {
        return Err(refused(
            "Original boundary homology exceeds exact allowance",
        ));
    }
    s.parse()
        .map_err(|_| refused("Invalid original boundary homology integer"))
}
fn det(a: &[BigInt; 2], b: &[BigInt; 2]) -> BigInt {
    &a[0] * &b[1] - &a[1] * &b[0]
}
/// Sparse integer rows spanning the COMPLETE rational kernel, including zero,
/// rank-one and rank-two columns. Saturation of the integer lattice is not
/// required for zero/equality tests of ALL rationally spanned potentials.
fn kernel(h: &[[BigInt; 2]]) -> Vec<Vec<(usize, BigInt)>> {
    let Some(p) = h.iter().position(|v| v.iter().any(|x| !x.is_zero())) else {
        return (0..h.len()).map(|j| vec![(j, BigInt::from(1))]).collect();
    };
    let q = (0..h.len()).find(|&j| !det(&h[p], &h[j]).is_zero());
    let mut rows = Vec::new();
    for j in 0..h.len() {
        if j == p || q == Some(j) {
            continue;
        }
        let mut row = if let Some(q) = q {
            vec![
                (p, -det(&h[j], &h[q])),
                (q, -det(&h[p], &h[j])),
                (j, det(&h[p], &h[q])),
            ]
        } else {
            let axis = usize::from(h[p][0].is_zero());
            vec![(p, -h[j][axis].clone()), (j, h[p][axis].clone())]
        };
        row.retain(|(_, c)| !c.is_zero());
        row.sort_by_key(|(i, _)| *i);
        rows.push(row);
    }
    rows
}
fn add(work: &mut usize, n: usize, cap: usize) -> Result<()> {
    *work = work
        .checked_add(n)
        .filter(|v| *v <= cap)
        .ok_or_else(|| refused("Complete material component census exceeds allowance"))?;
    Ok(())
}
fn product(a: usize, b: usize) -> Result<usize> {
    a.checked_mul(b)
        .ok_or_else(|| refused("Complete material component work overflow"))
}
struct ArcCell {
    walk: usize,
    direction: i8,
    net: Vec<[Q; 3]>,
    domain: [Q; 2],
    bounds: [ParameterValue; 2],
}
fn derive_signatures(o: &OwnedPeriodicSupportSignatures) -> Result<SignatureData> {
    let _replay = super::periodic_replay::scope();
    if !(1..=1000000).contains(&o.max_work)
        || o.limits.max_refinements > 512
        || !(1..=128).contains(&o.limits.max_resultant_size)
    {
        return Err(refused("Invalid original material signature allowance"));
    }
    super::periodic_replay::verify_arc(&o.owner, |owner| owner.reverify_source())?;
    let selection = o.owner.owner().owner();
    let graph = selection.rule().material().base().owner().owner().owner();
    let cover = graph.owner().owner().pieces();
    let n = o.owner.walks().len();
    let parts = o.owner.parts_count();
    let mut work = 0;
    add(&mut work, n, o.max_work)?;
    if product(n, n.saturating_sub(2))? > o.max_work {
        return Err(refused(
            "Complete material signature storage exceeds allowance",
        ));
    }
    let h = o
        .owner
        .walks()
        .iter()
        .map(|w| Ok([integer(&w.homology[0])?, integer(&w.homology[1])?]))
        .collect::<Result<Vec<_>>>()?;
    for row in &h {
        for axis in 0..2 {
            if !cover.wrapped()[axis] && !row[axis].is_zero() {
                return Err(refused("Boundary homology invents an unwrapped period"));
            }
        }
    }
    for axis in 0..2 {
        if h.iter().map(|row| row[axis].clone()).sum::<BigInt>() != BigInt::from(0) {
            return Err(refused(
                "Complete oriented material boundary has nonzero ambient homology",
            ));
        }
    }
    let basis = kernel(&h);
    // Each free column has a unique nonzero free coefficient, giving independence
    // and dimension n-rank(H). Every row is checked in the exact original H.
    for row in &basis {
        for axis in 0..2 {
            if row.iter().map(|(i, c)| c * &h[*i][axis]).sum::<BigInt>() != BigInt::from(0) {
                return Err(refused(
                    "Original separating cochain has nonzero ambient homology",
                ));
            }
        }
    }
    add(&mut work, product(n, basis.len())?, o.max_work)?;
    add(
        &mut work,
        product(n, basis.iter().map(Vec::len).sum())?,
        o.max_work,
    )?;
    let cochains = basis
        .iter()
        .map(|row| OriginalPeriodicSeparatingCochain {
            terms: row
                .iter()
                .map(|(walk, c)| OriginalPeriodicCochainTerm {
                    walk: *walk,
                    coefficient: c.to_string(),
                })
                .collect(),
        })
        .collect();
    let mut endpoints = Vec::new();
    let mut queries = Vec::new();
    for (walk, w) in o.owner.walks().iter().enumerate() {
        let part = w
            .parts
            .first()
            .ok_or_else(|| refused("Material signature lost a boundary walk"))?;
        let side = &selection.sides()[part.edge];
        let s = match side.selected() {
            [true, false] => 0,
            [false, true] => 1,
            _ => {
                return Err(refused(
                    "Boundary signature has no unique selected material side",
                ));
            }
        };
        let physical_direction = if graph.edges()[part.edge].carriers[0].reversed {
            -part.direction
        } else {
            part.direction
        };
        if side.boundary_direction() != Some(physical_direction) {
            return Err(refused(
                "Boundary signature lost its original directed material side",
            ));
        }
        let sample = side.samples()[s]
            .as_ref()
            .ok_or_else(|| refused("Boundary signature invented an exterior material sample"))?;
        if !sample.selected() || !Arc::ptr_eq(sample.rule(), selection.rule().material()) {
            return Err(refused(
                "Boundary signature belongs to a different material rule",
            ));
        }
        let path = sample
            .query()
            .path()
            .iter()
            .map(|p| Ok([decode_ratio(&p[0])?, decode_ratio(&p[1])?]))
            .collect::<Result<Vec<_>>>()?;
        add(&mut work, product(path.len() - 1, parts)?, o.max_work)?;
        endpoints.push((walk, [part.edge, s]));
        queries.push(path);
    }
    if n == 0 {
        if parts != 0 {
            return Err(refused(
                "Empty signature census omitted original boundary parts",
            ));
        }
        return Ok(SignatureData {
            cochains,
            probes: Vec::new(),
            work,
        });
    }
    // Charge the original-cell extraction bound before allocating its new view.
    for c in cover.sources() {
        add(&mut work, product(c.knots.len(), c.degree + 1)?, o.max_work)?;
    }
    let original = cover
        .sources()
        .iter()
        .map(cells)
        .collect::<Result<Vec<_>>>()?;
    let mut arcs = Vec::new();
    let mut seen = vec![false; graph.edges().len()];
    for (walk, w) in o.owner.walks().iter().enumerate() {
        for p in &w.parts {
            if p.edge >= seen.len() || seen[p.edge] {
                return Err(refused(
                    "Material signature repeats a physical boundary edge",
                ));
            }
            seen[p.edge] = true;
            let c = &original[p.source][p.cell];
            let piece = &cover.pieces()[p.piece];
            let offset = [
                decode_ratio(&piece.translation[0])?,
                decode_ratio(&piece.translation[1])?,
            ];
            let net = c
                .net
                .iter()
                .map(|p| {
                    [
                        &p[0] + &offset[0] * &p[2],
                        &p[1] + &offset[1] * &p[2],
                        p[2].clone(),
                    ]
                })
                .collect();
            let mut bounds = p
                .parameters
                .iter()
                .map(|t| {
                    Ok(ParameterValue {
                        primitive: decode_polynomial(&t.primitive)?,
                        value: decode_polynomial(&t.parameter)?,
                        interval: [decode_ratio(&t.interval[0])?, decode_ratio(&t.interval[1])?],
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            if p.direction < 0 {
                bounds.reverse();
            }
            arcs.push(ArcCell {
                walk,
                direction: p.direction,
                net,
                domain: c.domain.clone(),
                bounds: bounds
                    .try_into()
                    .map_err(|_| refused("Material signature lost original native cuts"))?,
            });
        }
    }
    if arcs.len() != parts || parts != selection.selected().len() {
        return Err(refused(
            "Material signature omitted selected original geometry",
        ));
    }
    let mut probes = Vec::new();
    for ((walk, endpoint), path) in endpoints.into_iter().zip(queries) {
        let mut delta = vec![BigInt::from(0); n];
        for arc in &arcs {
            for segment in path.windows(2) {
                let d = arithmetic::selected_winding::segment(
                    &arc.net,
                    &arc.domain,
                    &arc.bounds,
                    &[segment[0].clone(), segment[1].clone()],
                    o.limits.max_refinements,
                    o.limits.max_resultant_size,
                )?
                .ok_or_else(|| {
                    refused("Material signature probe lies on selected original geometry")
                })?;
                delta[arc.walk] += BigInt::from(d) * BigInt::from(arc.direction);
            }
        }
        let signature = basis
            .iter()
            .map(|row| {
                row.iter()
                    .map(|(i, c)| c * &delta[*i])
                    .sum::<BigInt>()
                    .to_string()
            })
            .collect();
        probes.push(OriginalPeriodicRegionProbe {
            walk,
            endpoint,
            signature,
        });
    }
    Ok(SignatureData {
        cochains,
        probes,
        work,
    })
}
impl OwnedPeriodicSupportSignatures {
    pub fn owner(&self) -> &Arc<OwnedPeriodicSupportLifts> {
        &self.owner
    }
    pub fn cochains(&self) -> &[OriginalPeriodicSeparatingCochain] {
        &self.data.cochains
    }
    pub fn probes(&self) -> &[OriginalPeriodicRegionProbe] {
        &self.data.probes
    }
    pub fn work(&self) -> usize {
        self.data.work
    }
    pub fn reverify_source(&self) -> Result<()> {
        if derive_signatures(self)? != self.data {
            return Err(refused(
                "Original separating signature or source census changed",
            ));
        }
        Ok(())
    }
}
pub fn own_periodic_support_signatures(
    owner: Arc<OwnedPeriodicSupportLifts>,
    limits: ParameterOrderLimits,
    max_work: usize,
) -> Result<OwnedPeriodicSupportSignatures> {
    let mut o = OwnedPeriodicSupportSignatures {
        owner,
        limits,
        max_work,
        data: SignatureData {
            cochains: Vec::new(),
            probes: Vec::new(),
            work: 0,
        },
    };
    o.data = derive_signatures(&o)?;
    Ok(o)
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicMaterialComponent {
    pub boundary_walks: Vec<usize>,
    pub signature: Vec<String>,
    /// Supplied paths connecting this component's actual walk representatives.
    pub witness_paths: Vec<usize>,
    /// A spanning forest subset; auxiliary closed material paths remain retained.
    pub forest_paths: Vec<usize>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct RegionData {
    components: Vec<OriginalPeriodicMaterialComponent>,
    work: usize,
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicSupportRegions {
    owner: Arc<OwnedPeriodicSupportSignatures>,
    paths: Vec<Arc<OwnedPeriodicSupportPath>>,
    base: Arc<OwnedPeriodicMaterialSample>,
    max_work: usize,
    data: RegionData,
}
fn find(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}
fn derive_regions(o: &OwnedPeriodicSupportRegions) -> Result<RegionData> {
    let _replay = super::periodic_replay::scope();
    if !(1..=1000000).contains(&o.max_work) {
        return Err(refused("Invalid complete material region allowance"));
    }
    super::periodic_replay::verify_arc(&o.owner, |owner| owner.reverify_source())?;
    let material = o.owner.owner().owner().owner().rule().material();
    if !Arc::ptr_eq(o.base.rule(), material) || !Arc::ptr_eq(o.base.query(), material.base()) {
        return Err(refused(
            "Complete regions lost their actual source-owned base sample",
        ));
    }
    super::periodic_replay::verify_arc(&o.base, |base| base.reverify_source())?;
    let probes = o.owner.probes();
    let mut work = o.owner.work();
    add(&mut work, probes.len(), o.max_work)?;
    add(&mut work, o.paths.len(), o.max_work)?;
    let mut nodes = BTreeMap::new();
    for (i, p) in probes.iter().enumerate() {
        if p.walk != i || nodes.insert(p.endpoint, i).is_some() {
            return Err(refused(
                "Complete region census repeats or omits a walk representative",
            ));
        }
    }
    let mut parent = (0..probes.len()).collect::<Vec<_>>();
    let mut path_nodes = Vec::new();
    let mut forest = Vec::new();
    for (k, p) in o.paths.iter().enumerate() {
        if !Arc::ptr_eq(p.owner(), o.owner.owner()) {
            return Err(refused(
                "Material connection uses another source lift owner",
            ));
        }
        let mut ids = [0; 2];
        for end in 0..2 {
            ids[end] = *nodes.get(&p.endpoints()[end]).ok_or_else(|| {
                refused("Material connection does not bind actual region representatives")
            })?;
        }
        if probes[ids[0]].signature != probes[ids[1]].signature {
            return Err(refused(
                "Material connection crosses separating original cochains",
            ));
        }
        add(&mut work, p.work(), o.max_work)?;
        super::periodic_replay::verify_arc(p, |p| p.reverify_source())?;
        let a = find(&mut parent, ids[0]);
        let b = find(&mut parent, ids[1]);
        if a != b {
            parent[b] = a;
            forest.push(k);
        }
        path_nodes.push(ids);
    }
    let mut groups = BTreeMap::<Vec<String>, Vec<usize>>::new();
    for p in probes {
        groups.entry(p.signature.clone()).or_default().push(p.walk);
    }
    let mut components = Vec::new();
    for (signature, walks) in groups {
        let root = find(&mut parent, walks[0]);
        for &i in &walks {
            if find(&mut parent, i) != root {
                return Err(refused(
                    "Equal signatures lack a complete positive material connection forest",
                ));
            }
        }
        let witness_paths = path_nodes
            .iter()
            .enumerate()
            .filter_map(|(i, ends)| (probes[ends[0]].signature == signature).then_some(i))
            .collect::<Vec<_>>();
        let forest_paths = forest
            .iter()
            .copied()
            .filter(|i| probes[path_nodes[*i][0]].signature == signature)
            .collect::<Vec<_>>();
        if forest_paths.len() + 1 != walks.len() {
            return Err(refused(
                "Material region forest does not span every original boundary walk",
            ));
        }
        components.push(OriginalPeriodicMaterialComponent {
            boundary_walks: walks,
            signature,
            witness_paths,
            forest_paths,
        });
    }
    // Completeness: every proper nonempty open regularized component has a
    // selected boundary walk. A boundary-free component is clopen in the actual
    // connected chart quotient, hence is the entire support. No periodic outside
    // value is invented; use the actual checked base material sample instead.
    if probes.is_empty() && o.base.selected() {
        components.push(OriginalPeriodicMaterialComponent {
            boundary_walks: Vec::new(),
            signature: Vec::new(),
            witness_paths: Vec::new(),
            forest_paths: Vec::new(),
        });
    }
    Ok(RegionData { components, work })
}
impl OwnedPeriodicSupportRegions {
    pub fn owner(&self) -> &Arc<OwnedPeriodicSupportSignatures> {
        &self.owner
    }
    pub fn paths(&self) -> &[Arc<OwnedPeriodicSupportPath>] {
        &self.paths
    }
    pub fn base_sample(&self) -> &Arc<OwnedPeriodicMaterialSample> {
        &self.base
    }
    pub fn components(&self) -> &[OriginalPeriodicMaterialComponent] {
        &self.data.components
    }
    pub fn work(&self) -> usize {
        self.data.work
    }
    pub fn reverify_source(&self) -> Result<()> {
        if derive_regions(self)? != self.data {
            return Err(refused(
                "Original connected material partition or source changed",
            ));
        }
        Ok(())
    }
}
pub fn own_periodic_support_regions(
    owner: Arc<OwnedPeriodicSupportSignatures>,
    paths: &[Arc<OwnedPeriodicSupportPath>],
    max_work: usize,
) -> Result<OwnedPeriodicSupportRegions> {
    let _replay = super::periodic_replay::scope();
    if !(1..=1000000).contains(&max_work) || paths.len() > max_work {
        return Err(refused("Invalid complete material region allowance"));
    }
    let material = owner.owner().owner().owner().rule().material();
    let base = Arc::new(own_periodic_material_sample(
        material.clone(),
        material.base().clone(),
    )?);
    let mut o = OwnedPeriodicSupportRegions {
        owner,
        paths: paths.to_vec(),
        base,
        max_work,
        data: RegionData {
            components: Vec::new(),
            work: 0,
        },
    };
    o.data = derive_regions(&o)?;
    Ok(o)
}
#[cfg(test)]
pub(in crate::profile_certificate::span_intersections::translated) mod tests;
