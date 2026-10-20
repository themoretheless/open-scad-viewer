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
fn derive(o: &OwnedOriginalCommonDictionary) -> Result<Vec<OriginalCommonEdge>> {
    if o.sources.len() > 32
        || !o.width.is_finite()
        || !(1e-18..=1e-3).contains(&o.width)
        || o.limits.max_refinements > 512
        || !(1..=128).contains(&o.limits.max_resultant_size)
    {
        return Err(refused(
            "Invalid original common dictionary source census or limits",
        ));
    }
    let expected = (0..o.sources.len())
        .flat_map(|a| (a..o.sources.len()).map(move |b| [a, b]))
        .collect::<Vec<_>>();
    if o.pairs != expected || o.owners.len() != expected.len() {
        return Err(refused("Original common dictionary omits a source pair"));
    }
    let mut expected_parts = Vec::new();
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
        for component in 0..owner.components().len() {
            expected_parts.push((pair, component));
        }
    }
    if expected_parts.len() != o.partitions.len() || o.partitions.len() > 4096 {
        return Err(refused(
            "Original common dictionary component coverage is incomplete",
        ));
    }
    let mut candidates = Vec::new();
    for (partition, (p, &(pair, component))) in o.partitions.iter().zip(&expected_parts).enumerate()
    {
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
        for segment in 0..p.owner.segments().len() {
            candidates.push((partition, segment));
        }
    }
    if candidates.len() > 4096 {
        return Err(refused(
            "Original common dictionary edge allowance exceeded",
        ));
    }
    let endpoints = |candidate: usize| {
        let (p, s) = candidates[candidate];
        let owner = &o.partitions[p].owner;
        let cuts = owner.segments()[s].cuts;
        [&owner.cuts()[cuts[0]].exact, &owner.cuts()[cuts[1]].exact]
    };
    let mut links = vec![Vec::<(usize, bool)>::new(); candidates.len()];
    let mut work = 0usize;
    for a in 0..candidates.len() {
        for b in a + 1..candidates.len() {
            let pa = o.pairs[o.partitions[candidates[a].0].pair];
            let pb = o.pairs[o.partitions[candidates[b].0].pair];
            for sa in 0..2 {
                for sb in 0..2 {
                    if pa[sa] != pb[sb] {
                        continue;
                    }
                    work += 1;
                    if work > 65536 {
                        return Err(refused(
                            "Original common dictionary matching allowance exceeded",
                        ));
                    }
                    let x = endpoints(a);
                    let y = endpoints(b);
                    let forward = compare(x[0], sa, y[0], sb, o.limits)? == Ordering::Equal
                        && compare(x[1], sa, y[1], sb, o.limits)? == Ordering::Equal;
                    let reverse = compare(x[0], sa, y[1], sb, o.limits)? == Ordering::Equal
                        && compare(x[1], sa, y[0], sb, o.limits)? == Ordering::Equal;
                    if forward || reverse {
                        links[a].push((b, reverse));
                        links[b].push((a, reverse));
                        continue;
                    }
                    let xi = if compare(x[0], sa, x[1], sa, o.limits)? == Ordering::Less {
                        [0, 1]
                    } else {
                        [1, 0]
                    };
                    let yi = if compare(y[0], sb, y[1], sb, o.limits)? == Ordering::Less {
                        [0, 1]
                    } else {
                        [1, 0]
                    };
                    if compare(x[xi[0]], sa, y[yi[1]], sb, o.limits)? == Ordering::Less
                        && compare(y[yi[0]], sb, x[xi[1]], sa, o.limits)? == Ordering::Less
                    {
                        return Err(refused(
                            "Original common dictionary has an unaligned partial source interval",
                        ));
                    }
                }
            }
        }
    }
    let mut assigned = vec![None; candidates.len()];
    let mut edges = Vec::new();
    for seed in 0..candidates.len() {
        if assigned[seed].is_some() {
            continue;
        }
        assigned[seed] = Some(false);
        let mut queue = vec![seed];
        let mut members = Vec::new();
        while let Some(a) = queue.pop() {
            members.push(a);
            let parity = assigned[a].unwrap();
            for &(b, reverse) in &links[a] {
                let wanted = parity ^ reverse;
                if let Some(actual) = assigned[b] {
                    if actual != wanted {
                        return Err(refused(
                            "Original common-edge orientation cycle is inconsistent",
                        ));
                    }
                } else {
                    assigned[b] = Some(wanted);
                    queue.push(b);
                }
            }
        }
        members.sort_unstable();
        let uses = members
            .into_iter()
            .map(|a| {
                let (partition, segment) = candidates[a];
                OriginalCommonEdgeUse {
                    partition,
                    segment,
                    reversed: assigned[a].unwrap(),
                }
            })
            .collect();
        edges.push(OriginalCommonEdge { uses });
    }
    Ok(edges)
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
