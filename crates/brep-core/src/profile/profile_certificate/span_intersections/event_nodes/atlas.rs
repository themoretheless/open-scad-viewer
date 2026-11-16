//! Complete original isolated/stationary event grouping through shared exact
//! source parameters. Different traversals remain distinct in source ordering.
use super::*;
use std::{cmp::Ordering, sync::Arc};
#[derive(Clone, Copy, Debug)]
pub struct ContactAtlasLimits {
    pub parameters: ParameterOrderLimits,
    pub max_events: usize,
    pub max_comparisons: usize,
}
impl Default for ContactAtlasLimits {
    fn default() -> Self {
        Self {
            parameters: ParameterOrderLimits::default(),
            max_events: 4096,
            max_comparisons: 16384,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContactOccurrence {
    pub pair: usize,
    pub node: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceContactParameter {
    pub occurrence: usize,
    pub side: usize,
}
#[derive(Clone, Debug)]
pub struct OriginalContactAtlas {
    sources: Vec<Curve>,
    pairs: Vec<[usize; 2]>,
    owners: Vec<Arc<CurveEventNodes>>,
    width: f64,
    limits: ContactAtlasLimits,
    occurrences: Vec<ContactOccurrence>,
    groups: Vec<Vec<usize>>,
    ordered: Vec<Vec<Vec<SourceContactParameter>>>,
    comparisons: usize,
}
fn root(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}
fn link(parent: &mut [usize], a: usize, b: usize) {
    let a = root(parent, a);
    let b = root(parent, b);
    parent[a.max(b)] = a.min(b);
}
const MAX_REFINEMENTS: usize = 512;
const MAX_RESULTANT_SIZE: usize = 128;
const MAX_SOURCES: usize = 32;
const MAX_EVENTS: usize = 4096;
const MAX_COMPARISONS: usize = 65536;
/// The allowance is valid and the pairs are exactly the ordered pairs
/// `i <= j` of the sources, one owner each.
fn verify_census(
    sources: &[Curve],
    pairs: &[[usize; 2]],
    owners: &[Arc<CurveEventNodes>],
    width: f64,
    l: ContactAtlasLimits,
) -> Result<()> {
    if !width.is_finite()
        || !(1e-18..=1e-3).contains(&width)
        || l.parameters.max_refinements > MAX_REFINEMENTS
        || !(1..=MAX_RESULTANT_SIZE).contains(&l.parameters.max_resultant_size)
        || sources.len() > MAX_SOURCES
        || !(1..=MAX_EVENTS).contains(&l.max_events)
        || l.max_comparisons > MAX_COMPARISONS
        || pairs.len() != sources.len() * (sources.len() + 1) / 2
        || pairs.len() != owners.len()
    {
        return Err(refused(
            "Invalid original contact atlas source census or allowance",
        ));
    }
    let expected = (0..sources.len()).flat_map(|i| (i..sources.len()).map(move |j| [i, j]));
    if !pairs.iter().copied().eq(expected) {
        return Err(refused(
            "Original contact atlas omits or reorders a source pair",
        ));
    }
    Ok(())
}
/// Every event node of every pair, after each owner is bound to its sources.
fn occurrences(
    sources: &[Curve],
    pairs: &[[usize; 2]],
    owners: &[Arc<CurveEventNodes>],
    width: f64,
    max_events: usize,
) -> Result<Vec<ContactOccurrence>> {
    let mut occurrences = Vec::new();
    for (pair, (ids, owner)) in pairs.iter().zip(owners).enumerate() {
        owner.reverify_source()?;
        if owner.sources != [sources[ids[0]].clone(), sources[ids[1]].clone()]
            || owner.census.self_pair != (ids[0] == ids[1])
            || owner.census.maximum_parameter_width != width
        {
            return Err(refused(
                "Contact pair is not bound to its actual original sources",
            ));
        }
        if owner.census.pairs.iter().any(|p| p.kind == "overlap") {
            return Err(refused(
                "Common curve branches need source-owned overlap assembly before an isolated atlas",
            ));
        }
        for node in 0..owner.nodes.len() {
            if occurrences.len() == max_events {
                return Err(refused("Original contact event allowance exhausted"));
            }
            occurrences.push(ContactOccurrence { pair, node });
        }
    }
    Ok(occurrences)
}
/// Event parameters of every source in increasing order, equal parameters
/// grouped, with occurrences that share a parameter joined.
struct ParameterOrder<'a> {
    pairs: &'a [[usize; 2]],
    owners: &'a [Arc<CurveEventNodes>],
    occurrences: &'a [ContactOccurrence],
    l: ContactAtlasLimits,
    ordered: Vec<Vec<Vec<SourceContactParameter>>>,
    parent: Vec<usize>,
    comparisons: usize,
}
impl ParameterOrder<'_> {
    fn order(&mut self, a: SourceContactParameter, b: SourceContactParameter) -> Result<Ordering> {
        if self.comparisons == self.l.max_comparisons {
            return Err(refused("Original parameter comparison allowance exhausted"));
        }
        self.comparisons += 1;
        let reference = |r: SourceContactParameter| {
            let o = self.occurrences[r.occurrence];
            (
                self.owners[o.pair].clone(),
                EventParameterRef {
                    node: o.node,
                    source: r.side,
                },
            )
        };
        let (first, x) = reference(a);
        let (second, y) = reference(b);
        Ok(compare_event_parameters(first, x, second, y, self.l.parameters)?.ordering())
    }
    /// Places one parameter by binary search: joins an equal group or opens
    /// a new one.
    fn place(&mut self, candidate: SourceContactParameter) -> Result<()> {
        let curve = self.pairs[self.occurrences[candidate.occurrence].pair][candidate.side];
        let (mut lo, mut hi) = (0, self.ordered[curve].len());
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let previous = self.ordered[curve][mid][0];
            match self.order(candidate, previous)? {
                Ordering::Less => hi = mid,
                Ordering::Greater => lo = mid + 1,
                Ordering::Equal => {
                    link(&mut self.parent, candidate.occurrence, previous.occurrence);
                    self.ordered[curve][mid].push(candidate);
                    return Ok(());
                }
            }
        }
        self.ordered[curve].insert(lo, vec![candidate]);
        Ok(())
    }
    /// Classes of joined occurrences, numbered by their first member.
    fn groups(&mut self) -> Vec<Vec<usize>> {
        let mut groups = Vec::<Vec<usize>>::new();
        let mut ids = std::collections::BTreeMap::new();
        for i in 0..self.occurrences.len() {
            let root = root(&mut self.parent, i);
            let id = *ids.entry(root).or_insert_with(|| {
                groups.push(Vec::new());
                groups.len() - 1
            });
            groups[id].push(i);
        }
        groups
    }
}
/// Occurrences, contact groups, ordered parameters and comparisons used.
type Atlas = (
    Vec<ContactOccurrence>,
    Vec<Vec<usize>>,
    Vec<Vec<Vec<SourceContactParameter>>>,
    usize,
);
fn build(
    sources: &[Curve],
    pairs: &[[usize; 2]],
    owners: &[Arc<CurveEventNodes>],
    width: f64,
    l: ContactAtlasLimits,
) -> Result<Atlas> {
    verify_census(sources, pairs, owners, width, l)?;
    let occurrences = occurrences(sources, pairs, owners, width, l.max_events)?;
    let mut order = ParameterOrder {
        pairs,
        owners,
        occurrences: &occurrences,
        l,
        ordered: vec![Vec::new(); sources.len()],
        parent: (0..occurrences.len()).collect(),
        comparisons: 0,
    };
    for occurrence in 0..occurrences.len() {
        for side in 0..2 {
            order.place(SourceContactParameter { occurrence, side })?;
        }
    }
    let groups = order.groups();
    let (ordered, comparisons) = (order.ordered, order.comparisons);
    Ok((occurrences, groups, ordered, comparisons))
}
impl OriginalContactAtlas {
    pub fn sources(&self) -> &[Curve] {
        &self.sources
    }
    pub fn pairs(&self) -> &[[usize; 2]] {
        &self.pairs
    }
    pub fn owners(&self) -> &[Arc<CurveEventNodes>] {
        &self.owners
    }
    pub fn occurrences(&self) -> &[ContactOccurrence] {
        &self.occurrences
    }
    /// Groups of exact coincident isolated/stationary event occurrences, linked
    /// by their actual shared original parameter. No spatial proximity merge.
    pub fn groups(&self) -> &[Vec<usize>] {
        &self.groups
    }
    /// Per original curve: strictly ordered parameter groups. A single physical
    /// event may occur at several DIFFERENT traversal parameters here.
    pub fn ordered_parameters(&self) -> &[Vec<Vec<SourceContactParameter>>] {
        &self.ordered
    }
    pub fn comparisons_used(&self) -> usize {
        self.comparisons
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (occurrences, groups, ordered, comparisons) = build(
            &self.sources,
            &self.pairs,
            &self.owners,
            self.width,
            self.limits,
        )?;
        if occurrences != self.occurrences
            || groups != self.groups
            || ordered != self.ordered
            || comparisons != self.comparisons
        {
            return Err(refused(
                "Original contact occurrence, parameter order or node partition changed",
            ));
        }
        Ok(())
    }
}
/// Complete all-original-curve-pair isolated/stationary atlas. Source parameter
/// identity links event occurrences into point groups while retaining distinct
/// traversals, including self intersections. Every unordered source pair and
/// every actual knot-cell pair is independently checked. Common-component
/// branches explicitly refuse here; their ownership is not inferred from an
/// empty isolated-event list. This is not cyclic port order, carrier cuts,
/// filled-region topology or physical face admission.
pub fn assemble_original_contact_atlas(
    sources: &[Curve],
    width: f64,
    limits: ContactAtlasLimits,
) -> Result<OriginalContactAtlas> {
    if sources.len() > 32 {
        return Err(refused("Original contact source allowance exceeded"));
    }
    let mut pairs = Vec::new();
    let mut owners = Vec::new();
    for i in 0..sources.len() {
        for j in i..sources.len() {
            let c = certify_span_intersections(&sources[i], &sources[j], i == j, width)?;
            owners.push(Arc::new(own_curve_event_nodes(
                &sources[i],
                &sources[j],
                &c,
            )?));
            pairs.push([i, j]);
        }
    }
    let (occurrences, groups, ordered, comparisons) =
        build(sources, &pairs, &owners, width, limits)?;
    Ok(OriginalContactAtlas {
        sources: sources.to_vec(),
        pairs,
        owners,
        limits,
        width,
        occurrences,
        groups,
        ordered,
        comparisons,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_contact_partition_replays_all_pairs_ordered_parameters_and_work() {
        let a = Curve::from_polyline(vec![vec![-1., -1.], vec![1., 1.]]).unwrap();
        let b = Curve::from_polyline(vec![vec![-1., 1.], vec![1., -1.]]).unwrap();
        let owner =
            assemble_original_contact_atlas(&[a, b], 1e-12, ContactAtlasLimits::default()).unwrap();
        owner.reverify_source().unwrap();
        let mut bad = owner.clone();
        bad.groups.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.groups[0].push(0);
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.ordered[0].clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.pairs.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.occurrences[0].node = 99;
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.comparisons += 1;
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.width = 1e-10;
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.limits.parameters.max_refinements = 513;
        assert!(bad.reverify_source().is_err());
        let mut bad = owner;
        Arc::make_mut(&mut bad.owners[1]).nodes.clear();
        assert!(bad.reverify_source().is_err());
    }
}
