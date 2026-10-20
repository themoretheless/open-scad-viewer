//! Exact chart-local oriented material boundary cycles, before face containment/sewing.
use super::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OriginalPeriodicChartVertexKey {
    Corner(usize),
    SourceCopy(usize),
    Transported { copy: usize, shore: usize },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicChartCycleVertex {
    pub key: OriginalPeriodicChartVertexKey,
    /// ORIGINAL quotient graph ID, not a selected-incidence vertex index.
    pub source_vertex: Option<usize>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalPeriodicChartCycleEdgeKind {
    SourceTrace(usize),
    NativeShore { shore: usize, interval: usize },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicChartCycleEdge {
    pub vertices: [usize; 2],
    pub kind: OriginalPeriodicChartCycleEdgeKind,
}
#[derive(Clone, Copy, Debug)]
pub struct PeriodicChartCycleLimits {
    pub max_edges: usize,
    pub max_vertices: usize,
    pub max_ports: usize,
    pub max_source_uses: usize,
    pub max_walk_steps: usize,
}
impl Default for PeriodicChartCycleLimits {
    fn default() -> Self {
        Self {
            max_edges: 1000000,
            max_vertices: 1000000,
            max_ports: 1000000,
            max_source_uses: 1000000,
            max_walk_steps: 1000000,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Data {
    vertices: Vec<OriginalPeriodicChartCycleVertex>,
    edges: Vec<OriginalPeriodicChartCycleEdge>,
    /// Original selected traces represented by their source-qualified frame image.
    coincident_traces: Vec<usize>,
    successor: Vec<usize>,
    rotations: Vec<Vec<[usize; 2]>>,
    walks: Vec<Vec<usize>>,
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicChartCycles {
    owner: Arc<OwnedPeriodicShoreMaterialIntervals>,
    limits: PeriodicChartCycleLimits,
    data: Data,
}
#[derive(Clone, Debug)]
struct Port {
    edge: usize,
    end: usize,
    sector: u8,
    rank: Option<usize>,
}
fn sector(signs: [i8; 2]) -> Result<u8> {
    match signs {
        [1, 0] => Ok(0),
        [1, 1] => Ok(1),
        [0, 1] => Ok(2),
        [-1, 1] => Ok(3),
        [-1, 0] => Ok(4),
        [-1, -1] => Ok(5),
        [0, -1] => Ok(6),
        [1, -1] => Ok(7),
        _ => Err(refused(
            "Local chart port has no exact nonzero displacement sector",
        )),
    }
}
fn permutations(
    edges: &[OriginalPeriodicChartCycleEdge],
    ports: &mut [Vec<Port>],
    max_steps: usize,
) -> Result<(Vec<usize>, Vec<Vec<usize>>)> {
    let mut successor = vec![usize::MAX; edges.len()];
    let mut predecessors = vec![0usize; edges.len()];
    let mut uses = vec![[false; 2]; edges.len()];
    for (vertex, rotation) in ports.iter_mut().enumerate() {
        rotation.sort_by_key(|p| (p.sector, p.rank));
        for pair in rotation.windows(2) {
            if pair[0].sector == pair[1].sector
                && (pair[0].rank.is_none()
                    || pair[1].rank.is_none()
                    || pair[0].rank == pair[1].rank)
            {
                return Err(refused(
                    "Local chart frame/source germ coincidence was not source-qualified and deduplicated",
                ));
            }
        }
        for (i, p) in rotation.iter().enumerate() {
            if p.edge >= edges.len()
                || p.end > 1
                || edges[p.edge].vertices[p.end] != vertex
                || std::mem::replace(&mut uses[p.edge][p.end], true)
            {
                return Err(refused(
                    "Local chart port is repeated or belongs to another actual endpoint",
                ));
            }
            if p.end == 1 {
                let next = &rotation[(i + rotation.len() - 1) % rotation.len()];
                if next.end != 0 {
                    return Err(refused(
                        "Local material fan does not have an outgoing clockwise boundary successor",
                    ));
                }
                successor[p.edge] = next.edge;
                predecessors[next.edge] += 1;
            }
        }
    }
    if uses.iter().any(|v| *v != [true, true])
        || successor.contains(&usize::MAX)
        || predecessors.iter().any(|n| *n != 1)
    {
        return Err(refused(
            "Local chart material successor is not a complete boundary permutation",
        ));
    }
    let mut seen = vec![false; edges.len()];
    let mut walks = Vec::new();
    let mut steps = 0usize;
    for start in 0..edges.len() {
        if seen[start] {
            continue;
        }
        let mut walk = Vec::new();
        let mut current = start;
        loop {
            steps = steps
                .checked_add(1)
                .filter(|n| *n <= max_steps)
                .ok_or_else(|| refused("Local chart cycle walk allowance exhausted"))?;
            if current >= edges.len() || std::mem::replace(&mut seen[current], true) {
                return Err(refused("Local chart cycles merge or repeat an owned edge"));
            }
            walk.push(current);
            let next = successor[current];
            if edges[current].vertices[1] != edges[next].vertices[0] {
                return Err(refused("Local chart cycle loses exact endpoint ownership"));
            }
            current = next;
            if current == start {
                break;
            }
        }
        walks.push(walk);
    }
    Ok((successor, walks))
}
fn copy_key(
    corners: &OwnedPeriodicNativeChartCorners,
    copy: usize,
) -> Result<OriginalPeriodicChartVertexKey> {
    let c = corners
        .owner()
        .copies()
        .get(copy)
        .ok_or_else(|| refused("Local chart endpoint lost its actual source copy"))?;
    if let [Some(u), Some(v)] = c.bounds {
        let corner = corners
            .corners()
            .iter()
            .position(|k| k.ends == [u, v])
            .ok_or_else(|| refused("Local source corner lacks its actual native corner"))?;
        Ok(OriginalPeriodicChartVertexKey::Corner(corner))
    } else {
        Ok(OriginalPeriodicChartVertexKey::SourceCopy(copy))
    }
}
fn frame_key(
    corners: &OwnedPeriodicNativeChartCorners,
    shore: usize,
    p: OriginalPeriodicNativeShorePoint,
) -> Result<OriginalPeriodicChartVertexKey> {
    match p {
        OriginalPeriodicNativeShorePoint::Corner(c) => {
            Ok(OriginalPeriodicChartVertexKey::Corner(c))
        }
        OriginalPeriodicNativeShorePoint::SourceCopy {
            copy,
            transported: false,
        } => copy_key(corners, copy),
        OriginalPeriodicNativeShorePoint::SourceCopy {
            copy,
            transported: true,
        } => Ok(OriginalPeriodicChartVertexKey::Transported { copy, shore }),
    }
}
fn vertex(
    key: OriginalPeriodicChartVertexKey,
    corners: &OwnedPeriodicNativeChartCorners,
    ids: &mut BTreeMap<OriginalPeriodicChartVertexKey, usize>,
    vertices: &mut Vec<OriginalPeriodicChartCycleVertex>,
    limit: usize,
) -> Result<usize> {
    if let Some(&i) = ids.get(&key) {
        return Ok(i);
    }
    if vertices.len() >= limit {
        return Err(refused("Local chart vertex allowance exhausted"));
    }
    let source_vertex = match key {
        OriginalPeriodicChartVertexKey::Corner(i) => {
            let c = corners
                .corners()
                .get(i)
                .ok_or_else(|| refused("Unknown local native corner"))?;
            corners.vertices()[c.physical_vertex].source_vertex
        }
        OriginalPeriodicChartVertexKey::SourceCopy(i)
        | OriginalPeriodicChartVertexKey::Transported { copy: i, .. } => Some(
            corners
                .owner()
                .copies()
                .get(i)
                .ok_or_else(|| refused("Unknown local source chart copy"))?
                .source_vertex,
        ),
    };
    let i = vertices.len();
    vertices.push(OriginalPeriodicChartCycleVertex { key, source_vertex });
    ids.insert(key, i);
    Ok(i)
}
fn derive(o: &OwnedPeriodicChartCycles) -> Result<Data> {
    let _replay = super::periodic_replay::scope();
    let l = o.limits;
    if [
        l.max_edges,
        l.max_vertices,
        l.max_ports,
        l.max_source_uses,
        l.max_walk_steps,
    ]
    .iter()
    .any(|n| !(1..=1000000).contains(n))
    {
        return Err(refused("Invalid exact local chart cycle allowance"));
    }
    o.owner.reverify_source()?;
    let samples = o.owner.owner();
    let shores = samples.owner();
    let corners = shores.owner();
    let copies = corners.owner();
    let incidence = copies.owner();
    let traces = incidence.owner();
    let regions = traces.owner();
    let lifts = regions.owner().owner();
    let cycles = lifts.owner();
    let rotation = cycles.rotation();
    let ports = rotation.owner();
    if ports.ports().len() > l.max_ports
        || traces
            .traces()
            .len()
            .checked_add(shores.interval_count())
            .is_none_or(|n| n > l.max_edges)
    {
        return Err(refused(
            "Complete local chart source/frame edge census exceeds allowance",
        ));
    }
    let mut ranks = vec![usize::MAX; ports.ports().len()];
    for (v, row) in rotation.rotations().iter().enumerate() {
        let mut previous = None;
        for (rank, &p) in row.iter().enumerate() {
            let port = &ports.ports()[p];
            let s = sector(port.component_signs)?;
            if port.vertex != v || ranks[p] != usize::MAX || previous.is_some_and(|old| old > s) {
                return Err(refused(
                    "Actual source rotation is not anchored in native axis/quadrant order",
                ));
            }
            previous = Some(s);
            ranks[p] = rank;
        }
    }
    if ranks.contains(&usize::MAX) {
        return Err(refused("Local chart source rotation omits original ports"));
    }
    let mut aliases = BTreeMap::new();
    let mut alias_uses = 0usize;
    for (copy, c) in copies.copies().iter().enumerate() {
        let key = copy_key(corners, copy)?;
        for &use_ in &c.uses {
            alias_uses = alias_uses
                .checked_add(1)
                .filter(|n| *n <= l.max_source_uses)
                .ok_or_else(|| refused("Local chart source use allowance exhausted"))?;
            if aliases
                .insert((c.source_vertex, use_), (copy, key))
                .is_some()
            {
                return Err(refused(
                    "Local chart original use belongs to multiple canonical copies",
                ));
            }
        }
    }
    let coincident = o
        .owner
        .contacts()
        .iter()
        .filter_map(|c| {
            if c.period_shift == [0, 0]
                && matches!(c.kind, OriginalPeriodicShoreContactKind::Coincident { .. })
            {
                Some(c.piece)
            } else {
                None
            }
        })
        .collect::<BTreeSet<_>>();
    let mut ids = BTreeMap::new();
    let mut vertices = Vec::new();
    let mut edges = Vec::new();
    let mut edge_ports = Vec::new();
    let mut coincident_traces = Vec::new();
    for (trace, t) in traces.traces().iter().enumerate() {
        let part = &t.original;
        if coincident.contains(&part.piece) {
            coincident_traces.push(trace);
            continue;
        }
        let a = ports
            .ports()
            .get(part.dart)
            .ok_or_else(|| refused("Local trace lost its actual original dart"))?;
        if a.edge != part.edge {
            return Err(refused(
                "Local trace dart belongs to a different physical source edge",
            ));
        }
        let twin = part
            .edge
            .checked_mul(2)
            .and_then(|n| n.checked_add(1 - a.end))
            .ok_or_else(|| refused("Local trace twin overflow"))?;
        let mut ends = [0; 2];
        for (end, pid) in [part.dart, twin].into_iter().enumerate() {
            let p = ports
                .ports()
                .get(pid)
                .ok_or_else(|| refused("Local trace lost its actual twin port"))?;
            let &(copy, key) = aliases
                .get(&(p.vertex, p.chart_use))
                .ok_or_else(|| refused("Local source port lacks its actual canonical chart use"))?;
            ends[end] = vertex(key, corners, &mut ids, &mut vertices, l.max_vertices)?;
            if vertices[ends[end]].source_vertex != Some(p.vertex) {
                return Err(refused(
                    "Local source port changed physical vertex ownership",
                ));
            }
            let c = &copies.copies()[copy];
            for axis in 0..2 {
                if let Some(bound) = c.bounds[axis] {
                    if (bound == 0 && p.component_signs[axis] < 0)
                        || (bound == 1 && p.component_signs[axis] > 0)
                    {
                        return Err(refused(
                            "Selected original germ points outside its canonical native chart",
                        ));
                    }
                }
            }
        }
        edges.push(OriginalPeriodicChartCycleEdge {
            vertices: ends,
            kind: OriginalPeriodicChartCycleEdgeKind::SourceTrace(trace),
        });
        edge_ports.push([
            (sector(a.component_signs)?, Some(ranks[part.dart])),
            (
                sector(ports.ports()[twin].component_signs)?,
                Some(ranks[twin]),
            ),
        ]);
    }
    for (shore, s) in shores.shores().iter().enumerate() {
        for (interval, at) in s.intervals.iter().enumerate() {
            if !o.owner.selected(shore, interval)? {
                continue;
            }
            let mut points = *at;
            if s.direction < 0 {
                points.swap(0, 1)
            }
            if s.direction != 1 && s.direction != -1 {
                return Err(refused("Invalid native frame boundary direction"));
            }
            let mut ends = [0; 2];
            let mut germ = [(0, None); 2];
            for end in 0..2 {
                let key = frame_key(corners, shore, s.points[points[end]])?;
                ends[end] = vertex(key, corners, &mut ids, &mut vertices, l.max_vertices)?;
                let mut signs = [0i8; 2];
                signs[1 - s.fixed_axis] = s.direction * if end == 0 { 1 } else { -1 };
                germ[end] = (sector(signs)?, None);
            }
            edges.push(OriginalPeriodicChartCycleEdge {
                vertices: ends,
                kind: OriginalPeriodicChartCycleEdgeKind::NativeShore { shore, interval },
            });
            edge_ports.push(germ);
        }
    }
    if edges.len().checked_mul(2).is_none_or(|n| n > l.max_ports) {
        return Err(refused("Local chart boundary port allowance exhausted"));
    }
    let mut local = vec![Vec::new(); vertices.len()];
    for (edge, e) in edges.iter().enumerate() {
        for end in 0..2 {
            local[e.vertices[end]].push(Port {
                edge,
                end,
                sector: edge_ports[edge][end].0,
                rank: edge_ports[edge][end].1,
            })
        }
    }
    let (successor, walks) = permutations(&edges, &mut local, l.max_walk_steps)?;
    Ok(Data {
        vertices,
        edges,
        coincident_traces,
        successor,
        rotations: local
            .iter()
            .map(|r| r.iter().map(|p| [p.edge, p.end]).collect())
            .collect(),
        walks,
    })
}
impl OwnedPeriodicChartCycles {
    pub fn owner(&self) -> &Arc<OwnedPeriodicShoreMaterialIntervals> {
        &self.owner
    }
    pub fn vertices(&self) -> &[OriginalPeriodicChartCycleVertex] {
        &self.data.vertices
    }
    pub fn edges(&self) -> &[OriginalPeriodicChartCycleEdge] {
        &self.data.edges
    }
    pub fn coincident_traces(&self) -> &[usize] {
        &self.data.coincident_traces
    }
    pub fn successor(&self) -> &[usize] {
        &self.data.successor
    }
    /// Counterclockwise actual source/frame germ rotation: [original edge, end].
    pub fn rotations(&self) -> &[Vec<[usize; 2]>] {
        &self.data.rotations
    }
    pub fn walks(&self) -> &[Vec<usize>] {
        &self.data.walks
    }
    pub fn reverify_source(&self) -> Result<()> {
        if derive(self)? != self.data {
            return Err(refused(
                "Local chart source/frame vertices,material fans or cycle permutation changed",
            ));
        }
        Ok(())
    }
}
/// All chart-local oriented boundary cycles with exact original source or native
/// frame provenance. Does not infer outer/hole nesting, face patches, world seam
/// sewing, manifold closure or a manufactured Body from these cycles.
pub fn own_periodic_chart_cycles(
    owner: Arc<OwnedPeriodicShoreMaterialIntervals>,
    limits: PeriodicChartCycleLimits,
) -> Result<OwnedPeriodicChartCycles> {
    let mut o = OwnedPeriodicChartCycles {
        owner,
        limits,
        data: Data {
            vertices: Vec::new(),
            edges: Vec::new(),
            coincident_traces: Vec::new(),
            successor: Vec::new(),
            rotations: Vec::new(),
            walks: Vec::new(),
        },
    };
    o.data = derive(&o)?;
    Ok(o)
}
#[cfg(test)]
pub(super) fn check_tampering(o: &OwnedPeriodicChartCycles) {
    let mut bad = o.clone();
    bad.data.successor[0] = usize::MAX;
    assert!(bad.reverify_source().is_err());
    assert!(
        own_periodic_chart_cycles(
            o.owner.clone(),
            PeriodicChartCycleLimits {
                max_edges: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[cfg(test)]
mod tests {
    use super::*;
    fn touching() -> (Vec<OriginalPeriodicChartCycleEdge>, Vec<Vec<Port>>) {
        let endpoints = [
            [0, 1],
            [1, 2],
            [2, 3],
            [3, 0],
            [0, 4],
            [4, 5],
            [5, 6],
            [6, 0],
        ];
        let directions = [
            [1, 0],
            [0, 1],
            [-1, 0],
            [0, -1],
            [-1, 0],
            [0, -1],
            [1, 0],
            [0, 1],
        ];
        let mut ports = vec![Vec::new(); 7];
        let edges = endpoints
            .iter()
            .enumerate()
            .map(|(i, &vertices)| {
                for end in 0..2 {
                    let s = directions[i].map(|n| if end == 0 { n } else { -n });
                    ports[vertices[end]].push(Port {
                        edge: i,
                        end,
                        sector: sector(s).unwrap(),
                        rank: None,
                    });
                }
                OriginalPeriodicChartCycleEdge {
                    vertices,
                    kind: OriginalPeriodicChartCycleEdgeKind::SourceTrace(i),
                }
            })
            .collect();
        (edges, ports)
    }
    #[test]
    fn exact_clockwise_successors_keep_two_touching_material_fans_separate() {
        let (edges, mut ports) = touching();
        let (successor, walks) = permutations(&edges, &mut ports, 8).unwrap();
        assert_eq!(walks, vec![vec![0, 1, 2, 3], vec![4, 5, 6, 7]]);
        assert_eq!(successor[3], 0);
        assert_eq!(successor[7], 4);
        assert_eq!(ports[0].len(), 4);
    }
    #[test]
    fn incomplete_or_duplicated_ports_and_walk_exhaustion_never_publish_partial_cycles() {
        let (edges, mut ports) = touching();
        assert!(permutations(&edges, &mut ports, 7).is_err());
        let (_, mut ports) = touching();
        ports[0].pop();
        assert!(permutations(&edges, &mut ports, 8).is_err());
        let (_, mut ports) = touching();
        let duplicate = ports[0][0].clone();
        ports[0].push(duplicate);
        assert!(permutations(&edges, &mut ports, 8).is_err());
    }
}

#[cfg(test)]
pub(super) mod actual_fan_tests {
    use super::super::periodic_chart_shores::tests as frame;
    use super::super::periodic_support_regions::tests as fixtures;
    use super::*;
    pub(in super::super) fn build(
        selection: Arc<OwnedPeriodicSupportSelection>,
        components: usize,
    ) -> OwnedPeriodicChartWinding {
        let signatures = fixtures::signatures(selection);
        let regions = own_periodic_support_regions(signatures, &[], 100000).unwrap();
        assert_eq!(regions.components().len(), components);
        let shores = Arc::new(frame::shores(frame::corners(Arc::new(regions))));
        let samples = Arc::new(
            own_periodic_shore_material_samples(
                shores,
                ParameterOrderLimits::default(),
                PeriodicShoreMaterialSampleLimits::default(),
            )
            .unwrap(),
        );
        let intervals = Arc::new(
            own_periodic_shore_material_intervals(
                samples,
                ParameterOrderLimits::default(),
                PeriodicShoreMaterialIntervalLimits::default(),
            )
            .unwrap(),
        );
        construct_periodic_chart_winding(
            intervals,
            PeriodicChartCycleLimits::default(),
            PeriodicChartWindingLimits::default(),
        )
        .unwrap()
    }
    #[test]
    fn actual_touching_filled_regions_keep_two_cycles_and_one_shared_chart_vertex() {
        let winding = build(fixtures::discs(true), 2);
        super::super::periodic_chart_cycle_sides::check_batch(&winding);
        let o = winding.owner();
        let q = |x: &str, y: &str| {
            [x, y].map(|v| {
                crate::profile_certificate::algebraic_intersections::ratio(&Q::new(
                    v.parse().unwrap(),
                    8.into(),
                ))
            })
        };
        assert!(
            winding
                .material(&[["2".into(), "8".into()], ["2".into(), "8".into()]])
                .is_err()
        );
        assert_eq!(winding.material(&q("2", "2")).unwrap(), Some(true));
        assert_eq!(winding.material(&q("6", "6")).unwrap(), Some(true));
        assert_eq!(winding.material(&q("2", "6")).unwrap(), Some(false));
        assert_eq!(winding.material(&q("4", "4")).unwrap(), None);
        assert_eq!(
            (o.vertices().len(), o.edges().len(), o.walks().len()),
            (7, 8, 2)
        );
        assert!(o.walks().iter().all(|w| w.len() == 4));
        assert!(o.coincident_traces().is_empty());
        let shared = (0..o.vertices().len())
            .filter(|&v| o.edges().iter().filter(|e| e.vertices.contains(&v)).count() == 4)
            .collect::<Vec<_>>();
        assert_eq!(shared.len(), 1);
        assert!(
            o.walks()
                .iter()
                .all(|w| w.iter().any(|&e| o.edges()[e].vertices[0] == shared[0]))
        );
    }
    #[test]
    fn actual_tangent_hole_uses_higher_germs_and_retains_a_repeated_vertex_walk() {
        let winding = build(
            super::super::periodic_support_material::tests::tangent_hole(),
            1,
        );
        super::super::periodic_chart_cycle_sides::check_batch(&winding);
        let o = winding.owner();
        let q = |x: &str, y: &str| {
            [x, y].map(|v| {
                crate::profile_certificate::algebraic_intersections::ratio(&Q::new(
                    v.parse().unwrap(),
                    4.into(),
                ))
            })
        };
        assert_eq!(winding.material(&q("2", "1")).unwrap(), Some(false));
        assert_eq!(winding.material(&q("0", "2")).unwrap(), None);
        assert_eq!(winding.material(&q("-4", "1")).unwrap(), Some(false));
        assert_eq!(winding.material(&q("2", "0")).unwrap(), None);
        assert_eq!(
            (o.vertices().len(), o.edges().len(), o.walks().len()),
            (8, 9, 1)
        );
        assert_eq!(o.walks()[0].len(), 9);
        assert_eq!(o.coincident_traces().len(), 5);
        let mut visits = BTreeMap::new();
        for &e in &o.walks()[0] {
            *visits.entry(o.edges()[e].vertices[0]).or_insert(0usize) += 1;
        }
        assert_eq!(visits.values().filter(|&&n| n == 2).count(), 1);
        assert!(visits.values().all(|&n| n == 1 || n == 2));
        assert_eq!(
            o.edges()
                .iter()
                .filter(|e| matches!(
                    e.kind,
                    OriginalPeriodicChartCycleEdgeKind::NativeShore { .. }
                ))
                .count(),
            5
        );
    }
}

// Only this module can issue the capability, immediately after a complete fresh
// local-cycle constructor. Its immutable Arc is retained by the winding batch.
pub(super) struct FreshChartCycles<'a> {
    owner: &'a Arc<OwnedPeriodicChartCycles>,
}
impl FreshChartCycles<'_> {
    pub(super) fn owner(&self) -> &Arc<OwnedPeriodicChartCycles> {
        self.owner
    }
}
/// One fresh source replay for the combined local-cycle and winding operation.
pub fn construct_periodic_chart_winding(
    owner: Arc<OwnedPeriodicShoreMaterialIntervals>,
    cycles: PeriodicChartCycleLimits,
    limits: PeriodicChartWindingLimits,
) -> Result<OwnedPeriodicChartWinding> {
    let owner = Arc::new(own_periodic_chart_cycles(owner, cycles)?);
    OwnedPeriodicChartWinding::from_fresh_cycles(&FreshChartCycles { owner: &owner }, limits)
}
