//! Source-owned contact graph, deduplicating common atomic branches.
use super::*;
use std::collections::BTreeMap;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalUnifiedVertex {
    pub cuts: Vec<[usize; 2]>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OriginalUnifiedEdgeCarrier {
    pub carrier: usize,
    pub reversed: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalUnifiedEdge {
    pub vertices: [usize; 2],
    pub common_edge: Option<usize>,
    pub carriers: Vec<OriginalUnifiedEdgeCarrier>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OriginalUnifiedIncidence {
    pub edge: usize,
    pub endpoint: usize,
}
#[derive(Clone, Debug)]
pub struct OwnedOriginalUnifiedGraph {
    owner: Arc<OwnedOriginalSourceCarriers>,
    vertices: Vec<OriginalUnifiedVertex>,
    cut_vertices: Vec<Vec<usize>>,
    edges: Vec<OriginalUnifiedEdge>,
    incidences: Vec<Vec<OriginalUnifiedIncidence>>,
}
fn root(p: &mut [usize], mut i: usize) -> usize {
    while p[i] != i {
        p[i] = p[p[i]];
        i = p[i];
    }
    i
}
fn link(p: &mut [usize], a: usize, b: usize) {
    let a = root(p, a);
    let b = root(p, b);
    p[a.max(b)] = a.min(b);
}
fn record(
    map: &mut BTreeMap<(usize, usize), [Option<usize>; 2]>,
    key: (usize, usize),
    side: usize,
    vertex: usize,
) -> Result<()> {
    let slot = &mut map.entry(key).or_insert([None, None])[side];
    if slot.is_some_and(|v| v != vertex) {
        return Err(refused(
            "Original graph event parameter has multiple cut owners",
        ));
    }
    *slot = Some(vertex);
    Ok(())
}
const MAX_GRAPH_ITEMS: usize = 8192;
/// The two cut-vertex slots of every event, keyed by owner and cut.
type EventSides = BTreeMap<(usize, usize), [Option<usize>; 2]>;
/// A scalar knot cut can join left/right coedges only when the ORIGINAL
/// source is exactly C0. Discontinuous shores need separate cut-side owners.
fn verify_continuity(owner: &OwnedOriginalSourceCarriers) -> Result<()> {
    for source in owner.dictionary().sources() {
        let c = cells(source)?;
        for pair in c.windows(2) {
            if pair[0].domain[1] != pair[1].domain[0]
                || point(&pair[0].net, 1) != point(&pair[1].net, 0)
            {
                return Err(refused(
                    "Unified original contact graph needs separately owned discontinuous knot shores",
                ));
            }
        }
    }
    Ok(())
}
/// First slot of every source in the flat cut numbering, and the cut count.
fn cut_offsets(owner: &OwnedOriginalSourceCarriers) -> Result<(Vec<usize>, usize)> {
    let mut offsets = Vec::new();
    let mut count = 0;
    for cuts in owner.cuts() {
        offsets.push(count);
        count += cuts.len();
    }
    if count > MAX_GRAPH_ITEMS || owner.carriers().len() > MAX_GRAPH_ITEMS {
        return Err(refused("Unified original contact graph allowance exceeded"));
    }
    Ok((offsets, count))
}
/// Cut slots named by pair events and by common cuts.
fn event_sides(
    owner: &OwnedOriginalSourceCarriers,
    offsets: &[usize],
) -> Result<(EventSides, EventSides)> {
    let mut pairs = BTreeMap::new();
    let mut common = BTreeMap::new();
    for (source, cuts) in owner.cuts().iter().enumerate() {
        for (cut, c) in cuts.iter().enumerate() {
            let vertex = offsets[source] + cut;
            for origin in &c.origins {
                match *origin {
                    OriginalSourceCutOrigin::Pair { pair, cut, side } => {
                        record(&mut pairs, (pair, cut), side, vertex)?
                    }
                    OriginalSourceCutOrigin::Common {
                        partition,
                        cut,
                        side,
                    } => record(&mut common, (partition, cut), side, vertex)?,
                    OriginalSourceCutOrigin::Knot { .. } => {}
                }
            }
        }
    }
    Ok((pairs, common))
}
/// Joins the two cut slots of one event. The messages name a missing
/// event, a missing first side and a missing second side.
fn join(
    parent: &mut [usize],
    sides: &EventSides,
    key: (usize, usize),
    [missing, first, second]: [&'static str; 3],
) -> Result<()> {
    let p = sides.get(&key).ok_or_else(|| refused(missing))?;
    let a = p[0].ok_or_else(|| refused(first))?;
    let b = p[1].ok_or_else(|| refused(second))?;
    link(parent, a, b);
    Ok(())
}
/// Union-find over cut slots after every pair event and common cut joined
/// its two sides.
fn unify(
    owner: &OwnedOriginalSourceCarriers,
    offsets: &[usize],
    count: usize,
) -> Result<Vec<usize>> {
    let (pairs, common) = event_sides(owner, offsets)?;
    let mut parent = (0..count).collect::<Vec<_>>();
    for (pair, cuts) in owner.pair_cuts().iter().enumerate() {
        for cut in 0..cuts.cuts().len() {
            join(
                &mut parent,
                &pairs,
                (pair, cut),
                [
                    "Unified graph omits an original pair event",
                    "Unified graph omits first original event parameter",
                    "Unified graph omits second original event parameter",
                ],
            )?;
        }
    }
    for (partition, p) in owner.dictionary().partitions().iter().enumerate() {
        for cut in 0..p.owner.cuts().len() {
            join(
                &mut parent,
                &common,
                (partition, cut),
                [
                    "Unified graph omits a common cut",
                    "Unified graph omits first original common parameter",
                    "Unified graph omits second original common parameter",
                ],
            )?;
        }
    }
    Ok(parent)
}
/// Vertices as classes of joined cuts, and the vertex of every cut.
fn vertices(
    owner: &OwnedOriginalSourceCarriers,
    offsets: &[usize],
    parent: &mut [usize],
) -> (Vec<OriginalUnifiedVertex>, Vec<Vec<usize>>) {
    let mut groups = BTreeMap::<usize, Vec<[usize; 2]>>::new();
    for (source, cuts) in owner.cuts().iter().enumerate() {
        for cut in 0..cuts.len() {
            let r = root(parent, offsets[source] + cut);
            groups.entry(r).or_default().push([source, cut]);
        }
    }
    let vertices = groups
        .into_values()
        .map(|cuts| OriginalUnifiedVertex { cuts })
        .collect::<Vec<_>>();
    let mut cut_vertices = owner
        .cuts()
        .iter()
        .map(|c| vec![usize::MAX; c.len()])
        .collect::<Vec<_>>();
    for (v, vertex) in vertices.iter().enumerate() {
        for &[source, cut] in &vertex.cuts {
            cut_vertices[source][cut] = v;
        }
    }
    (vertices, cut_vertices)
}
/// Carriers grouped into edges: one group per common edge of the
/// dictionary, then one per carrier that is not common.
fn carrier_groups(
    owner: &OwnedOriginalSourceCarriers,
) -> Result<Vec<(Option<usize>, Vec<OriginalUnifiedEdgeCarrier>)>> {
    let mut common_groups = vec![Vec::new(); owner.dictionary().edges().len()];
    let mut single = Vec::new();
    for (carrier, c) in owner.carriers().iter().enumerate() {
        if let Some(edge) = c.common_edge {
            let reversed = c
                .common_uses
                .first()
                .ok_or_else(|| refused("Unified common carrier lacks a proved direction"))?
                .reversed;
            common_groups[edge].push(OriginalUnifiedEdgeCarrier { carrier, reversed });
        } else {
            single.push(vec![OriginalUnifiedEdgeCarrier {
                carrier,
                reversed: false,
            }]);
        }
    }
    Ok(common_groups
        .into_iter()
        .enumerate()
        .map(|(e, c)| (Some(e), c))
        .chain(single.into_iter().map(|c| (None, c)))
        .collect())
}
/// Edges whose carriers all agree on the two end vertices.
fn edges(
    owner: &OwnedOriginalSourceCarriers,
    cut_vertices: &[Vec<usize>],
) -> Result<Vec<OriginalUnifiedEdge>> {
    let ends = |use_: &OriginalUnifiedEdgeCarrier| {
        let c = &owner.carriers()[use_.carrier];
        let mut v = c.cuts.map(|cut| cut_vertices[c.source][cut]);
        if use_.reversed {
            v.swap(0, 1);
        }
        v
    };
    let mut edges = Vec::new();
    for (common_edge, carriers) in carrier_groups(owner)? {
        let first = carriers
            .first()
            .ok_or_else(|| refused("Unified graph loses an original common edge"))?;
        let vertices = ends(first);
        if carriers.iter().any(|c| ends(c) != vertices) {
            return Err(refused(
                "Unified graph common branches disagree on actual endpoint vertices",
            ));
        }
        edges.push(OriginalUnifiedEdge {
            vertices,
            common_edge,
            carriers,
        });
    }
    Ok(edges)
}
/// Vertices, the vertex of every cut, edges and edge ends at every vertex.
type UnifiedGraph = (
    Vec<OriginalUnifiedVertex>,
    Vec<Vec<usize>>,
    Vec<OriginalUnifiedEdge>,
    Vec<Vec<OriginalUnifiedIncidence>>,
);
fn derive(owner: &OwnedOriginalSourceCarriers) -> Result<UnifiedGraph> {
    owner.reverify_source()?;
    verify_continuity(owner)?;
    let (offsets, count) = cut_offsets(owner)?;
    let mut parent = unify(owner, &offsets, count)?;
    let (vertices, cut_vertices) = vertices(owner, &offsets, &mut parent);
    let edges = edges(owner, &cut_vertices)?;
    let mut incidences = vec![Vec::new(); vertices.len()];
    for (edge, e) in edges.iter().enumerate() {
        for endpoint in 0..2 {
            incidences[e.vertices[endpoint]].push(OriginalUnifiedIncidence { edge, endpoint });
        }
    }
    Ok((vertices, cut_vertices, edges, incidences))
}
impl OwnedOriginalUnifiedGraph {
    pub fn owner(&self) -> &Arc<OwnedOriginalSourceCarriers> {
        &self.owner
    }
    pub fn vertices(&self) -> &[OriginalUnifiedVertex] {
        &self.vertices
    }
    pub fn cut_vertices(&self) -> &[Vec<usize>] {
        &self.cut_vertices
    }
    pub fn edges(&self) -> &[OriginalUnifiedEdge] {
        &self.edges
    }
    /// Loop edges retain BOTH endpoint darts at the same vertex.
    pub fn incidences(&self) -> &[Vec<OriginalUnifiedIncidence>] {
        &self.incidences
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (v, c, e, i) = derive(&self.owner)?;
        if v != self.vertices || c != self.cut_vertices || e != self.edges || i != self.incidences {
            return Err(refused(
                "Unified original contact graph vertices branches or incidence changed",
            ));
        }
        Ok(())
    }
}
pub fn own_original_unified_graph(
    owner: Arc<OwnedOriginalSourceCarriers>,
) -> Result<OwnedOriginalUnifiedGraph> {
    let (vertices, cut_vertices, edges, incidences) = derive(&owner)?;
    Ok(OwnedOriginalUnifiedGraph {
        owner,
        vertices,
        cut_vertices,
        edges,
        incidences,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_rejects_vertex_cut_membership_edge_end_and_port_changes() {
        let a = Curve::from_polyline(vec![vec![0., 0.], vec![2., 2.]]).unwrap();
        let b = Curve::from_polyline(vec![vec![0., 2.], vec![2., 0.]]).unwrap();
        let d = Arc::new(
            own_original_common_dictionary(&[a, b], 1e-12, ParameterOrderLimits::default())
                .unwrap(),
        );
        let c = Arc::new(own_original_source_carriers(d, ParameterOrderLimits::default()).unwrap());
        let x = own_original_unified_graph(c).unwrap();
        x.reverify_source().unwrap();
        let mut bad = x.clone();
        bad.vertices[0].cuts.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.edges[0].vertices.swap(0, 1);
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.edges[0].carriers.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.incidences[0].clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.cut_vertices[0][0] = 99;
        assert!(bad.reverify_source().is_err());
    }
}
