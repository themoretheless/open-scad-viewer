//! Source-proof graph of the exact translated UV arrangement, before seam quotient.
use super::*;
use periodic_events::compare;
use std::{cmp::Ordering, collections::BTreeMap, sync::Arc};
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OriginalPeriodicChartCutRef {
    pub piece: usize,
    pub cut: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalPeriodicChartRelationOrigin {
    Event {
        pair: usize,
        event: usize,
    },
    Common {
        pair: usize,
        clip: usize,
        end: usize,
    },
    Alias {
        alias: usize,
        end: usize,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicChartRelation {
    pub cuts: [OriginalPeriodicChartCutRef; 2],
    pub origin: OriginalPeriodicChartRelationOrigin,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicChartIncidence {
    pub edge: usize,
    pub end: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicChartVertex {
    pub uses: Vec<OriginalPeriodicChartCutRef>,
    pub incidences: Vec<OriginalPeriodicChartIncidence>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicChartCarrier {
    pub piece: usize,
    pub interval: usize,
    pub source: usize,
    pub cell: usize,
    pub reversed: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicChartEdge {
    pub vertices: [usize; 2],
    pub carriers: Vec<OriginalPeriodicChartCarrier>,
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicChartGraph {
    owner: Arc<OwnedPeriodicAlignedIntervals>,
    limits: ParameterOrderLimits,
    vertices: Vec<OriginalPeriodicChartVertex>,
    edges: Vec<OriginalPeriodicChartEdge>,
    relations: Vec<OriginalPeriodicChartRelation>,
}
fn value(n: &AlgebraicEventNode, side: usize) -> OriginalSourceParameter {
    OriginalSourceParameter {
        primitive: n.exact.primitive.clone(),
        interval: n.exact.isolating_interval.clone(),
        parameter: n.exact.parameters[side].clone(),
    }
}
fn cut(
    o: &OwnedPeriodicChartGraph,
    piece: usize,
    v: &OriginalSourceParameter,
) -> Result<OriginalPeriodicChartCutRef> {
    for (cut, c) in o.owner.cuts()[piece].iter().enumerate() {
        if compare(&c.value, v, o.limits)? == Ordering::Equal {
            return Ok(OriginalPeriodicChartCutRef { piece, cut });
        }
    }
    Err(refused(
        "Exact event missing from periodic graph source cuts",
    ))
}
fn root(p: &[usize], mut i: usize) -> usize {
    while p[i] != i {
        i = p[i];
    }
    i
}
fn union(p: &mut [usize], a: usize, b: usize) {
    let a = root(p, a);
    let b = root(p, b);
    if a != b {
        p[a.max(b)] = a.min(b);
    }
}
type Data = (
    Vec<OriginalPeriodicChartVertex>,
    Vec<OriginalPeriodicChartEdge>,
    Vec<OriginalPeriodicChartRelation>,
);
fn derive(o: &OwnedPeriodicChartGraph) -> Result<Data> {
    if o.limits.max_refinements > 512 || !(1..=128).contains(&o.limits.max_resultant_size) {
        return Err(refused(
            "Invalid exact periodic chart graph comparison allowance",
        ));
    }
    super::periodic_replay::verify_arc(&o.owner, |x| x.reverify_source())?;
    let mut ids = BTreeMap::new();
    let mut refs = Vec::new();
    let mut raw_edges = Vec::new();
    let mut edge_ids = BTreeMap::new();
    for (piece, cuts) in o.owner.cuts().iter().enumerate() {
        for cut in 0..cuts.len() {
            let r = OriginalPeriodicChartCutRef { piece, cut };
            ids.insert(r, refs.len());
            refs.push(r);
        }
        for interval in 0..cuts.len() - 1 {
            edge_ids.insert((piece, interval), raw_edges.len());
            raw_edges.push((piece, interval));
        }
    }
    let mut parents = (0..refs.len()).collect::<Vec<_>>();
    let mut relations = Vec::new();
    let mut relate = |cuts: [OriginalPeriodicChartCutRef; 2],
                      origin: OriginalPeriodicChartRelationOrigin|
     -> Result<()> {
        let a = *ids
            .get(&cuts[0])
            .ok_or_else(|| refused("Missing source graph cut"))?;
        let b = *ids
            .get(&cuts[1])
            .ok_or_else(|| refused("Missing counterpart graph cut"))?;
        union(&mut parents, a, b);
        relations.push(OriginalPeriodicChartRelation { cuts, origin });
        Ok(())
    };
    for (pair, p) in o.owner.owner().pairs().iter().enumerate() {
        let pieces = p.owner().pair();
        for (event, n) in p.owner().events().iter().enumerate() {
            if n.in_both_closed_pieces() {
                relate(
                    [
                        cut(o, pieces[0], &value(&n.node, 0))?,
                        cut(o, pieces[1], &value(&n.node, 1))?,
                    ],
                    OriginalPeriodicChartRelationOrigin::Event { pair, event },
                )?;
            }
        }
        for (clip, c) in p.clips().iter().enumerate() {
            for end in 0..2 {
                relate(
                    [
                        cut(o, pieces[0], &value(&c.endpoints[end], 0))?,
                        cut(o, pieces[1], &value(&c.endpoints[end], 1))?,
                    ],
                    OriginalPeriodicChartRelationOrigin::Common { pair, clip, end },
                )?;
            }
        }
    }
    let mut adjacency = vec![Vec::new(); raw_edges.len()];
    for (alias, a) in o.owner.aliases().iter().enumerate() {
        let x = *edge_ids
            .get(&(a.pieces[0], a.intervals[0]))
            .ok_or_else(|| refused("Missing whole source edge interval"))?;
        let y = *edge_ids
            .get(&(a.pieces[1], a.intervals[1]))
            .ok_or_else(|| refused("Missing whole counterpart edge interval"))?;
        adjacency[x].push((y, a.reversed));
        adjacency[y].push((x, a.reversed));
        for end in 0..2 {
            let target_end = if a.reversed { 1 - end } else { end };
            relate(
                [
                    OriginalPeriodicChartCutRef {
                        piece: a.pieces[0],
                        cut: a.intervals[0] + end,
                    },
                    OriginalPeriodicChartCutRef {
                        piece: a.pieces[1],
                        cut: a.intervals[1] + target_end,
                    },
                ],
                OriginalPeriodicChartRelationOrigin::Alias { alias, end },
            )?;
        }
    }
    drop(relate);
    let mut vertex_ids = BTreeMap::new();
    let mut vertices: Vec<OriginalPeriodicChartVertex> = Vec::new();
    for (id, r) in refs.iter().enumerate() {
        let representative = root(&parents, id);
        let next = vertices.len();
        let vertex = *vertex_ids.entry(representative).or_insert_with(|| {
            vertices.push(OriginalPeriodicChartVertex {
                uses: Vec::new(),
                incidences: Vec::new(),
            });
            next
        });
        vertices[vertex].uses.push(*r);
    }
    let vertex = |piece: usize, cut: usize| -> usize {
        vertex_ids[&root(&parents, ids[&OriginalPeriodicChartCutRef { piece, cut }])]
    };
    let mut groups = vec![None; raw_edges.len()];
    let mut edges: Vec<OriginalPeriodicChartEdge> = Vec::new();
    for first in 0..raw_edges.len() {
        if groups[first].is_some() {
            continue;
        }
        let edge = edges.len();
        let (piece, interval) = raw_edges[first];
        let ends = [vertex(piece, interval), vertex(piece, interval + 1)];
        edges.push(OriginalPeriodicChartEdge {
            vertices: ends,
            carriers: Vec::new(),
        });
        let mut stack = vec![(first, false)];
        while let Some((id, reversed)) = stack.pop() {
            if let Some((old_edge, old_reversed)) = groups[id] {
                if old_edge != edge || old_reversed != reversed {
                    return Err(refused(
                        "Common whole-edge orientation cycle is inconsistent",
                    ));
                }
                continue;
            }
            groups[id] = Some((edge, reversed));
            let (piece, interval) = raw_edges[id];
            let piece_source = &o.owner.owner().pieces().pieces()[piece];
            let actual = [vertex(piece, interval), vertex(piece, interval + 1)];
            let expected = if reversed { [ends[1], ends[0]] } else { ends };
            if actual != expected {
                return Err(refused(
                    "Whole common edge endpoints do not agree with its direction proof",
                ));
            }
            edges[edge].carriers.push(OriginalPeriodicChartCarrier {
                piece,
                interval,
                source: piece_source.source,
                cell: piece_source.cell,
                reversed,
            });
            for &(other, parity) in &adjacency[id] {
                stack.push((other, reversed ^ parity));
            }
        }
        edges[edge].carriers.sort_by_key(|c| (c.piece, c.interval));
        for end in 0..2 {
            vertices[ends[end]]
                .incidences
                .push(OriginalPeriodicChartIncidence { edge, end });
        }
    }
    Ok((vertices, edges, relations))
}
impl OwnedPeriodicChartGraph {
    pub fn owner(&self) -> &Arc<OwnedPeriodicAlignedIntervals> {
        &self.owner
    }
    pub fn vertices(&self) -> &[OriginalPeriodicChartVertex] {
        &self.vertices
    }
    pub fn edges(&self) -> &[OriginalPeriodicChartEdge] {
        &self.edges
    }
    pub fn relations(&self) -> &[OriginalPeriodicChartRelation] {
        &self.relations
    }
    pub fn reverify_source(&self) -> Result<()> {
        let _replay = super::periodic_replay::scope();
        let (v, e, r) = derive(self)?;
        if v != self.vertices || e != self.edges || r != self.relations {
            return Err(refused(
                "Periodic source chart graph topology or ownership changed",
            ));
        }
        Ok(())
    }
}
/// Deduplicates ONLY proved whole common intervals; retains every original
/// carrier and both endpoint incidences of a loop. Opposite-shore physical
/// quotient relations, port rotation and face assembly remain separate.
pub fn own_periodic_chart_graph(
    owner: Arc<OwnedPeriodicAlignedIntervals>,
    limits: ParameterOrderLimits,
) -> Result<OwnedPeriodicChartGraph> {
    let _replay = super::periodic_replay::scope();
    if owner.owner().seam_contacts().is_some() {
        return Err(refused(
            "Chart-only graph cannot silently include physical seam identifications",
        ));
    }
    build_graph(owner, limits)
}
fn build_graph(
    owner: Arc<OwnedPeriodicAlignedIntervals>,
    limits: ParameterOrderLimits,
) -> Result<OwnedPeriodicChartGraph> {
    let _replay = super::periodic_replay::scope();
    let mut o = OwnedPeriodicChartGraph {
        owner,
        limits,
        vertices: Vec::new(),
        edges: Vec::new(),
        relations: Vec::new(),
    };
    let (v, e, r) = derive(&o)?;
    o.vertices = v;
    o.edges = e;
    o.relations = r;
    Ok(o)
}
/// Actual-support quotient graph; no manifold material sectors or faces yet.
#[derive(Clone, Debug)]
pub struct OwnedPeriodicQuotientGraph {
    graph: OwnedPeriodicChartGraph,
}
impl OwnedPeriodicQuotientGraph {
    pub fn owner(&self) -> &Arc<OwnedPeriodicAlignedIntervals> {
        self.graph.owner()
    }
    pub fn seam_contacts(&self) -> &Arc<OwnedPeriodicSeamContacts> {
        self.owner()
            .owner()
            .seam_contacts()
            .expect("qualified quotient owns actual seam contacts")
    }
    pub fn vertices(&self) -> &[OriginalPeriodicChartVertex] {
        self.graph.vertices()
    }
    pub fn edges(&self) -> &[OriginalPeriodicChartEdge] {
        self.graph.edges()
    }
    pub fn relations(&self) -> &[OriginalPeriodicChartRelation] {
        self.graph.relations()
    }
    pub fn reverify_source(&self) -> Result<()> {
        let _replay = super::periodic_replay::scope();
        if self.owner().owner().seam_contacts().is_none() {
            return Err(refused(
                "Physical quotient lost actual surface seam ownership",
            ));
        }
        self.graph.reverify_source()
    }
}
pub fn own_periodic_quotient_graph(
    owner: Arc<OwnedPeriodicAlignedIntervals>,
    limits: ParameterOrderLimits,
) -> Result<OwnedPeriodicQuotientGraph> {
    let _replay = super::periodic_replay::scope();
    if owner.owner().seam_contacts().is_none() {
        return Err(refused(
            "Physical quotient graph requires complete actual-support seam contacts",
        ));
    }
    Ok(OwnedPeriodicQuotientGraph {
        graph: build_graph(owner, limits)?,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn graph(sources: &[Curve], max_pairs: usize) -> OwnedPeriodicChartGraph {
        let pieces = Arc::new(
            own_original_periodic_pieces(
                sources,
                [[0., 1.], [0., 1.]],
                [false, false],
                1e-12,
                ParameterOrderLimits::default(),
                32,
            )
            .unwrap(),
        );
        let seeds = Arc::new(
            own_periodic_arrangement_cuts(
                pieces,
                1e-12,
                ParameterOrderLimits::default(),
                max_pairs,
            )
            .unwrap(),
        );
        let aligned = Arc::new(
            own_periodic_aligned_intervals(seeds, ParameterOrderLimits::default(), 4096).unwrap(),
        );
        own_periodic_chart_graph(aligned, ParameterOrderLimits::default()).unwrap()
    }
    #[test]
    fn crossing_graph_has_one_four_port_vertex_and_all_original_intervals() {
        let a = Curve::from_polyline(vec![vec![0., 0.5], vec![1., 0.5]]).unwrap();
        let b = Curve::from_polyline(vec![vec![0.5, 0.], vec![0.5, 1.]]).unwrap();
        let o = graph(&[a, b], 3);
        o.reverify_source().unwrap();
        assert_eq!(o.vertices().len(), 5);
        assert_eq!(o.edges().len(), 4);
        assert_eq!(
            o.vertices()
                .iter()
                .filter(|v| v.incidences.len() == 4)
                .count(),
            1
        );
        assert!(o.edges().iter().all(|e| e.carriers.len() == 1));
        let mut bad = o.clone();
        bad.vertices[0].uses.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = o.clone();
        bad.relations.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = o;
        bad.edges[0].carriers[0].source = 99;
        assert!(bad.reverify_source().is_err());
    }
    #[test]
    fn common_edge_dedup_keeps_reverse_carriers_without_false_crossing_ports() {
        let a = Curve::from_polyline(vec![vec![0., 0.5], vec![1., 0.5]]).unwrap();
        let b = Curve::from_polyline(vec![vec![1., 0.5], vec![0., 0.5]]).unwrap();
        let c = Curve::from_polyline(vec![vec![0.5, 0.], vec![0.5, 1.]]).unwrap();
        let o = graph(&[a, b, c], 6);
        o.reverify_source().unwrap();
        assert_eq!(o.vertices().len(), 5);
        assert_eq!(o.edges().len(), 4);
        assert_eq!(o.edges().iter().map(|e| e.carriers.len()).sum::<usize>(), 6);
        assert_eq!(
            o.vertices()
                .iter()
                .filter(|v| v.incidences.len() == 4)
                .count(),
            1
        );
        let common = o
            .edges()
            .iter()
            .filter(|e| e.carriers.len() == 2)
            .collect::<Vec<_>>();
        assert_eq!(common.len(), 2);
        assert!(common.iter().all(|e| e.carriers[0].source == 0
            && !e.carriers[0].reversed
            && e.carriers[1].source == 1
            && e.carriers[1].reversed));
        let mut bad = o;
        bad.edges[0].carriers[0].reversed = true;
        assert!(bad.reverify_source().is_err());
    }
    #[test]
    fn original_closed_native_knot_cycle_has_four_vertices_and_eight_endpoint_darts() {
        let a = Curve::from_polyline(vec![
            vec![0., 0.],
            vec![1., 0.],
            vec![1., 1.],
            vec![0., 1.],
            vec![0., 0.],
        ])
        .unwrap();
        let o = graph(&[a], 10);
        o.reverify_source().unwrap();
        assert_eq!(o.vertices().len(), 4);
        assert_eq!(o.edges().len(), 4);
        assert!(o.vertices().iter().all(|v| v.incidences.len() == 2));
        assert_eq!(
            o.vertices()
                .iter()
                .map(|v| v.incidences.len())
                .sum::<usize>(),
            8
        );
        assert_eq!(o.edges().iter().map(|e| e.carriers.len()).sum::<usize>(), 4);
    }
}
#[cfg(test)]
pub(super) mod quotient_tests {
    use super::*;
    pub(in super::super) fn sources(
        sources: &[Curve],
    ) -> (
        Arc<OwnedPeriodicArrangementCuts>,
        Arc<OwnedPeriodicSeamContacts>,
    ) {
        let rim = Curve::from_polyline(vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![1., 1., 0.],
            vec![0., 1., 0.],
            vec![0., 0., 0.],
        ])
        .unwrap();
        let surface = nurbs_core::surface::extrude(&rim, [0., 0., 1.]).unwrap();
        let cover = Arc::new(
            own_original_periodic_pieces(
                sources,
                [[0., 4.], [0., 1.]],
                [true, false],
                1e-12,
                ParameterOrderLimits::default(),
                64,
            )
            .unwrap(),
        );
        let n = cover.pieces().len();
        let count = n * (n + 1) / 2;
        let support = Arc::new(
            bind_original_periodic_pieces_to_surface(
                cover.clone(),
                &surface,
                ParameterOrderLimits::default(),
            )
            .unwrap(),
        );
        let uses = (0..sources.len())
            .map(|source| {
                vec![OriginalPeriodicSourceUse {
                    source,
                    reversed: false,
                }]
            })
            .collect::<Vec<_>>();
        let wires = Arc::new(own_original_periodic_wires(support, &uses).unwrap());
        let embedding = Arc::new(
            own_original_periodic_embedding(
                wires,
                4096,
                OriginalInjectiveAtlasLimits::default(),
                OriginalAtlasPairLimits::default(),
            )
            .unwrap(),
        );
        let chart = Arc::new(
            own_periodic_arrangement_cuts(cover, 1e-12, ParameterOrderLimits::default(), count)
                .unwrap(),
        );
        let seams = Arc::new(
            own_periodic_seam_contacts(
                embedding,
                1e-12,
                ParameterOrderLimits::default(),
                count * 2,
            )
            .unwrap(),
        );
        (chart, seams)
    }
    pub(in super::super) fn aligned(
        chart: Arc<OwnedPeriodicArrangementCuts>,
        seams: Arc<OwnedPeriodicSeamContacts>,
    ) -> Arc<OwnedPeriodicAlignedIntervals> {
        let max = chart.pairs().len() + seams.pairs().len();
        let combined = Arc::new(
            own_periodic_quotient_arrangement_cuts(
                chart,
                seams,
                ParameterOrderLimits::default(),
                max,
            )
            .unwrap(),
        );
        Arc::new(
            own_periodic_aligned_intervals(combined, ParameterOrderLimits::default(), 4096)
                .unwrap(),
        )
    }
    #[test]
    fn actual_periodic_loop_retains_two_distinct_endpoint_darts_at_one_vertex() {
        let a = Curve {
            degree: 2,
            knots: vec![2., 2., 2., 6., 6., 6.],
            control_points: vec![vec![0., 0.5], vec![0., 0.5], vec![4., 0.5]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let (chart, seams) = sources(&[a]);
        assert!(
            own_periodic_quotient_arrangement_cuts(
                chart.clone(),
                seams.clone(),
                ParameterOrderLimits::default(),
                2
            )
            .is_err()
        );
        let bare = Arc::new(
            own_periodic_aligned_intervals(chart.clone(), ParameterOrderLimits::default(), 4096)
                .unwrap(),
        );
        let uv = own_periodic_chart_graph(bare.clone(), ParameterOrderLimits::default()).unwrap();
        assert_eq!(uv.vertices().len(), 2);
        assert!(
            own_periodic_quotient_graph(bare.clone(), ParameterOrderLimits::default()).is_err()
        );
        let aligned = aligned(chart, seams);
        assert!(
            own_periodic_chart_graph(aligned.clone(), ParameterOrderLimits::default()).is_err()
        );
        let o = own_periodic_quotient_graph(aligned, ParameterOrderLimits::default()).unwrap();
        o.reverify_source().unwrap();
        assert_eq!(o.vertices().len(), 1);
        assert_eq!(o.edges().len(), 1);
        assert_eq!(o.edges()[0].vertices, [0, 0]);
        assert_eq!(
            o.vertices()[0].incidences,
            vec![
                OriginalPeriodicChartIncidence { edge: 0, end: 0 },
                OriginalPeriodicChartIncidence { edge: 0, end: 1 }
            ]
        );
        assert_eq!(o.edges()[0].carriers.len(), 1);
        let mut bad = o.clone();
        bad.graph.vertices[0].incidences.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = o.clone();
        bad.graph.edges[0].carriers[0].reversed = true;
        assert!(bad.reverify_source().is_err());
        let mut bad = o;
        bad.graph.owner = bare;
        assert!(bad.reverify_source().is_err());
    }
    #[test]
    fn opposite_shore_contact_splits_actual_carrier_before_physical_graph_union() {
        let a = Curve::from_polyline(vec![
            vec![0., 0.2],
            vec![1., 0.2],
            vec![1., 0.8],
            vec![0., 0.8],
            vec![0., 0.2],
        ])
        .unwrap();
        let b = Curve::from_polyline(vec![
            vec![2., 0.4],
            vec![4., 0.5],
            vec![2., 0.6],
            vec![2., 0.4],
        ])
        .unwrap();
        let (chart, seams) = sources(&[a, b]);
        assert_eq!(chart.cuts()[3].len(), 2);
        let aligned = aligned(chart, seams);
        assert_eq!(aligned.cuts()[3].len(), 3);
        let o = own_periodic_quotient_graph(aligned, ParameterOrderLimits::default()).unwrap();
        o.reverify_source().unwrap();
        assert_eq!(o.vertices().len(), 7);
        assert_eq!(o.edges().len(), 8);
        assert_eq!(
            o.vertices()
                .iter()
                .filter(|v| v.incidences.len() == 4)
                .count(),
            1
        );
        assert_eq!(o.edges().iter().map(|e| e.carriers.len()).sum::<usize>(), 8);
        assert!(
            o.vertices()
                .iter()
                .find(|v| v.incidences.len() == 4)
                .unwrap()
                .uses
                .iter()
                .any(|r| r.piece == 3 && r.cut == 1)
        );
    }
}
