//! Global common-edge identity through complete original parameter intervals.
use super::*;
use std::cmp::Ordering;
#[derive(Clone, Debug)]
pub struct OriginalCommonPartitionRef {
    pub pair: usize,
    pub component: usize,
    pub owner: Arc<OwnedOriginalCommonPartition>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OriginalCommonEdgeUse {
    pub partition: usize,
    pub segment: usize,
    pub reversed: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalCommonEdge {
    pub uses: Vec<OriginalCommonEdgeUse>,
}
#[derive(Clone, Debug)]
pub struct OwnedOriginalCommonDictionary {
    sources: Vec<Curve>,
    pairs: Vec<[usize; 2]>,
    owners: Vec<Arc<OwnedOriginalOverlapComponents>>,
    partitions: Vec<OriginalCommonPartitionRef>,
    width: f64,
    limits: ParameterOrderLimits,
    edges: Vec<OriginalCommonEdge>,
}
fn compare(
    a: &AlgebraicEventPosition,
    side: usize,
    b: &AlgebraicEventPosition,
    other: usize,
    l: ParameterOrderLimits,
) -> Result<Ordering> {
    let wrap = |exact: &AlgebraicEventPosition| AlgebraicEventNode {
        source_cells: [0, 0],
        pair: 0,
        factor: None,
        root: 0,
        stationary: false,
        exact: exact.clone(),
    };
    parameter_cmp(&wrap(a), side, &wrap(b), other, l)
}
const MAX_SOURCES: usize = 32;
const MAX_REFINEMENTS: usize = 512;
const MAX_RESULTANT_SIZE: usize = 128;
const MAX_PARTITIONS: usize = 4096;
const MAX_SEGMENTS: usize = 4096;
const MATCHING_WORK: usize = 65536;
/// One segment of one common partition, as (partition, segment).
type Candidate = (usize, usize);
/// Other candidates on the same edge, each with whether it runs reversed.
type Links = Vec<Vec<(usize, bool)>>;
/// Every source pair has one verified owner over exactly those two sources;
/// returns the (pair, component) of every common component, in order.
fn expected_partitions(o: &OwnedOriginalCommonDictionary) -> Result<Vec<(usize, usize)>> {
    let expected = (0..o.sources.len())
        .flat_map(|a| (a..o.sources.len()).map(move |b| [a, b]))
        .collect::<Vec<_>>();
    if o.pairs != expected || o.owners.len() != expected.len() {
        return Err(refused("Original common dictionary omits a source pair"));
    }
    let mut parts = Vec::new();
    for (pair, (&ids, owner)) in o.pairs.iter().zip(&o.owners).enumerate() {
        owner.reverify_source()?;
        if owner.owner().sources() != &[o.sources[ids[0]].clone(), o.sources[ids[1]].clone()]
            || owner.owner().census().self_pair != (ids[0] == ids[1])
            || owner.owner().census().maximum_parameter_width != o.width
        {
            return Err(refused(
                "Original common dictionary pair rebound to different sources",
            ));
        }
        parts.extend((0..owner.components().len()).map(|component| (pair, component)));
    }
    if parts.len() != o.partitions.len() || o.partitions.len() > MAX_PARTITIONS {
        return Err(refused(
            "Original common dictionary component coverage is incomplete",
        ));
    }
    Ok(parts)
}
/// Every stored partition is the verified partition of its expected
/// component; returns all their segments.
fn segment_candidates(
    o: &OwnedOriginalCommonDictionary,
    expected: &[(usize, usize)],
) -> Result<Vec<Candidate>> {
    let mut candidates = Vec::new();
    for (partition, (p, &(pair, component))) in o.partitions.iter().zip(expected).enumerate() {
        if p.pair != pair
            || p.component != component
            || !Arc::ptr_eq(p.owner.base(), &o.owners[pair])
            || p.owner.component() != component
            || p.owner.sources() != o.sources
        {
            return Err(refused(
                "Original common dictionary partition provenance changed",
            ));
        }
        p.owner.reverify_source()?;
        candidates.extend((0..p.owner.segments().len()).map(|segment| (partition, segment)));
    }
    if candidates.len() > MAX_SEGMENTS {
        return Err(refused(
            "Original common dictionary edge allowance exceeded",
        ));
    }
    Ok(candidates)
}
/// How two segments relate on one shared source. `Some(reverse)` when they
/// are the same interval, `None` when they do not overlap; a partial overlap
/// is refused.
fn relate(
    x: [&AlgebraicEventPosition; 2],
    sa: usize,
    y: [&AlgebraicEventPosition; 2],
    sb: usize,
    l: ParameterOrderLimits,
) -> Result<Option<bool>> {
    let same = |a: &AlgebraicEventPosition, b: &AlgebraicEventPosition| -> Result<bool> {
        Ok(compare(a, sa, b, sb, l)? == Ordering::Equal)
    };
    let forward = same(x[0], y[0])? && same(x[1], y[1])?;
    let reverse = same(x[0], y[1])? && same(x[1], y[0])?;
    if forward || reverse {
        return Ok(Some(reverse));
    }
    let xi = if compare(x[0], sa, x[1], sa, l)? == Ordering::Less {
        [0, 1]
    } else {
        [1, 0]
    };
    let yi = if compare(y[0], sb, y[1], sb, l)? == Ordering::Less {
        [0, 1]
    } else {
        [1, 0]
    };
    if compare(x[xi[0]], sa, y[yi[1]], sb, l)? == Ordering::Less
        && compare(y[yi[0]], sb, x[xi[1]], sa, l)? == Ordering::Less
    {
        return Err(refused(
            "Original common dictionary has an unaligned partial source interval",
        ));
    }
    Ok(None)
}
/// Links every two segments that are the same interval of a shared source.
fn link_candidates(o: &OwnedOriginalCommonDictionary, candidates: &[Candidate]) -> Result<Links> {
    let endpoints = |candidate: usize| {
        let (p, s) = candidates[candidate];
        let owner = &o.partitions[p].owner;
        let cuts = owner.segments()[s].cuts;
        [&owner.cuts()[cuts[0]].exact, &owner.cuts()[cuts[1]].exact]
    };
    let sources = |candidate: usize| o.pairs[o.partitions[candidates[candidate].0].pair];
    let mut links = vec![Vec::new(); candidates.len()];
    let mut work = 0usize;
    for a in 0..candidates.len() {
        for b in a + 1..candidates.len() {
            let (pa, pb) = (sources(a), sources(b));
            for (sa, sb) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
                if pa[sa] != pb[sb] {
                    continue;
                }
                work += 1;
                if work > MATCHING_WORK {
                    return Err(refused(
                        "Original common dictionary matching allowance exceeded",
                    ));
                }
                if let Some(reverse) = relate(endpoints(a), sa, endpoints(b), sb, o.limits)? {
                    links[a].push((b, reverse));
                    links[b].push((a, reverse));
                }
            }
        }
    }
    Ok(links)
}
/// One edge per connected group of linked segments, with a consistent
/// orientation of every member relative to the group's first segment.
fn orient_edges(candidates: &[Candidate], links: &Links) -> Result<Vec<OriginalCommonEdge>> {
    let mut assigned: Vec<Option<bool>> = vec![None; candidates.len()];
    let mut edges = Vec::new();
    for seed in 0..candidates.len() {
        if assigned[seed].is_some() {
            continue;
        }
        assigned[seed] = Some(false);
        let mut queue = vec![(seed, false)];
        let mut members = Vec::new();
        while let Some((a, parity)) = queue.pop() {
            members.push((a, parity));
            for &(b, reverse) in &links[a] {
                let wanted = parity ^ reverse;
                match assigned[b] {
                    Some(actual) if actual != wanted => {
                        return Err(refused(
                            "Original common-edge orientation cycle is inconsistent",
                        ));
                    }
                    Some(_) => {}
                    None => {
                        assigned[b] = Some(wanted);
                        queue.push((b, wanted));
                    }
                }
            }
        }
        members.sort_unstable();
        let uses = members
            .into_iter()
            .map(|(a, reversed)| {
                let (partition, segment) = candidates[a];
                OriginalCommonEdgeUse {
                    partition,
                    segment,
                    reversed,
                }
            })
            .collect();
        edges.push(OriginalCommonEdge { uses });
    }
    Ok(edges)
}
fn derive(o: &OwnedOriginalCommonDictionary) -> Result<Vec<OriginalCommonEdge>> {
    if o.sources.len() > MAX_SOURCES
        || !o.width.is_finite()
        || !(1e-18..=1e-3).contains(&o.width)
        || o.limits.max_refinements > MAX_REFINEMENTS
        || !(1..=MAX_RESULTANT_SIZE).contains(&o.limits.max_resultant_size)
    {
        return Err(refused(
            "Invalid original common dictionary source census or limits",
        ));
    }
    let expected = expected_partitions(o)?;
    let candidates = segment_candidates(o, &expected)?;
    let links = link_candidates(o, &candidates)?;
    orient_edges(&candidates, &links)
}
impl OwnedOriginalCommonDictionary {
    pub fn sources(&self) -> &[Curve] {
        &self.sources
    }
    pub fn pairs(&self) -> &[[usize; 2]] {
        &self.pairs
    }
    pub fn owners(&self) -> &[Arc<OwnedOriginalOverlapComponents>] {
        &self.owners
    }
    pub fn partitions(&self) -> &[OriginalCommonPartitionRef] {
        &self.partitions
    }
    pub fn edges(&self) -> &[OriginalCommonEdge] {
        &self.edges
    }
    pub fn reverify_source(&self) -> Result<()> {
        if self.edges != derive(self)? {
            return Err(refused(
                "Original common dictionary edges or source uses changed",
            ));
        }
        Ok(())
    }
}
/// Complete original pair and common-component census, including self pairs.
/// This dictionary owns common edges only; isolated/single-source edges and
/// cyclic graph topology still require the global arrangement owner.
pub fn own_original_common_dictionary(
    sources: &[Curve],
    width: f64,
    limits: ParameterOrderLimits,
) -> Result<OwnedOriginalCommonDictionary> {
    if sources.len() > 32 || !width.is_finite() || !(1e-18..=1e-3).contains(&width) {
        return Err(refused("Invalid original common dictionary source census"));
    }
    let mut out = OwnedOriginalCommonDictionary {
        sources: sources.to_vec(),
        pairs: Vec::new(),
        owners: Vec::new(),
        partitions: Vec::new(),
        width,
        limits,
        edges: Vec::new(),
    };
    for a in 0..sources.len() {
        for b in a..sources.len() {
            let census = certify_span_intersections(&sources[a], &sources[b], a == b, width)?;
            let owner = Arc::new(own_original_overlap_components(Arc::new(
                own_curve_event_nodes(&sources[a], &sources[b], &census)?,
            ))?);
            let pair = out.pairs.len();
            out.pairs.push([a, b]);
            out.owners.push(owner.clone());
            for component in 0..owner.components().len() {
                if out.partitions.len() >= 4096 {
                    return Err(refused(
                        "Original common dictionary partition allowance exceeded",
                    ));
                }
                let partition = Arc::new(own_original_common_partition(
                    owner.clone(),
                    component,
                    sources,
                    width,
                    limits,
                )?);
                out.partitions.push(OriginalCommonPartitionRef {
                    pair,
                    component,
                    owner: partition,
                });
            }
        }
    }
    out.edges = derive(&out)?;
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_rejects_pair_omission_alias_omission_and_orientation_change() {
        let a = Curve::from_polyline(vec![vec![0., 0.], vec![2., 0.]]).unwrap();
        let b = Curve::from_polyline(vec![vec![2., 0.], vec![0., 0.]]).unwrap();
        let x = own_original_common_dictionary(&[a, b], 1e-12, ParameterOrderLimits::default())
            .unwrap();
        x.reverify_source().unwrap();
        let mut bad = x.clone();
        bad.pairs.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.edges[0].uses.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.edges[0].uses[0].reversed = true;
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.partitions[0].component = 99;
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.edges[0].uses[0].segment = 99;
        assert!(bad.reverify_source().is_err());
    }
}
