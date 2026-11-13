//! Source-bound contact-aware faces with exact opposite native-interval sewing.
//! Abstract material fans remain separate at owned world point contacts.
use super::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
#[derive(Clone, Copy, Debug)]
pub struct PeriodicSewnFaceLimits {
    pub max_edges: usize,
    pub max_seam_comparisons: usize,
    pub max_regularity_cells: usize,
    pub atlas: OriginalInjectiveAtlasLimits,
    pub pairs: OriginalAtlasPairLimits,
}
impl Default for PeriodicSewnFaceLimits {
    fn default() -> Self {
        Self {
            max_edges: 100000,
            max_seam_comparisons: 1000000,
            max_regularity_cells: 100000,
            atlas: Default::default(),
            pairs: OriginalAtlasPairLimits {
                max_span_work: 1000000,
                refinement_depth: 4,
                ..Default::default()
            },
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicSewnVertex {
    /// Original outgoing-edge material fans, not coordinate-snapped vertices.
    pub fans: Vec<usize>,
    pub chart_vertices: Vec<usize>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicSewnEdge {
    pub vertices: [usize; 2],
    /// One boundary expression, or two oppositely oriented seam expressions.
    pub chart_edges: Vec<usize>,
    pub original: OriginalPeriodicChartCycleEdgeKind,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicSewnCoedge {
    pub patch: usize,
    pub edge: usize,
    pub chart_edge: usize,
    pub reversed: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicSewnFace {
    pub patches: Vec<usize>,
    /// Oriented world EDGE IDs; auxiliary sewing edges have been removed.
    pub boundary_cycles: Vec<Vec<usize>>,
    pub euler_characteristic: i64,
    pub genus: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Data {
    vertices: Vec<OriginalPeriodicSewnVertex>,
    edges: Vec<OriginalPeriodicSewnEdge>,
    coedges: Vec<OriginalPeriodicSewnCoedge>,
    faces: Vec<OriginalPeriodicSewnFace>,
    seams: Vec<[usize; 2]>,
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicSewnFaces {
    owner: Arc<OwnedPeriodicChartFanPatches>,
    embedding: Arc<OwnedOriginalPeriodicEmbedding>,
    limits: PeriodicSewnFaceLimits,
    data: Data,
}
fn find(p: &mut [usize], i: usize) -> usize {
    let mut r = i;
    while p[r] != r {
        r = p[r];
    }
    let mut at = i;
    while p[at] != at {
        let next = p[at];
        p[at] = r;
        at = next;
    }
    r
}
fn join(p: &mut [usize], a: usize, b: usize) {
    let x = find(p, a);
    let y = find(p, b);
    p[y] = x;
}
fn cycles(o: &OwnedPeriodicChartFanPatches) -> &Arc<OwnedPeriodicChartCycles> {
    o.owner().owner().owner()
}
const MAX_ALLOWANCE: usize = 1000000;
/// Material patch that owns every chart edge, through the boundary loop carrying it.
fn edge_patches(
    patches: &[OriginalPeriodicChartStrictPatch],
    loops: &[OriginalPeriodicChartBoundaryLoop],
    n: usize,
) -> Result<Vec<usize>> {
    let mut loop_patch = vec![usize::MAX; loops.len()];
    for (patch, p) in patches.iter().enumerate() {
        for boundary in std::iter::once(p.outer).chain(p.holes.iter().copied()) {
            if std::mem::replace(&mut loop_patch[boundary], patch) != usize::MAX {
                return Err(refused("Loop belongs to multiple actual material patches"));
            }
        }
    }
    let mut edge_patch = vec![usize::MAX; n];
    for (id, b) in loops.iter().enumerate() {
        if loop_patch[id] == usize::MAX {
            return Err(refused("Sewn face omits an original boundary loop"));
        }
        for &edge in &b.edges {
            edge_patch[edge] = loop_patch[id];
        }
    }
    Ok(edge_patch)
}
/// Number of material boundary walks of every patch; each walk stays in one
/// patch and each patch has at least one.
fn patch_boundary_counts(
    walks: &[Vec<usize>],
    edge_patch: &[usize],
    patches: usize,
) -> Result<Vec<usize>> {
    let mut counts = vec![0usize; patches];
    for walk in walks {
        let patch = edge_patch[*walk
            .first()
            .ok_or_else(|| refused("Empty original material boundary"))?];
        if walk.iter().any(|&e| edge_patch[e] != patch) {
            return Err(refused(
                "Original contact fan walk needs further owned material patch separation",
            ));
        }
        counts[patch] += 1;
    }
    if counts.contains(&0) {
        return Err(refused("Unbounded or boundary-free chart patch is unowned"));
    }
    Ok(counts)
}
/// Auxiliary seam intervals paired with their exact opposites.
struct SeamPairs {
    /// Counterpart of every chart edge that is one side of a seam.
    paired: Vec<Option<usize>>,
    seams: Vec<[usize; 2]>,
}
/// Pairs every auxiliary interval with its single exact opposite original
/// interval. `matching` is the exact comparison; each call is charged.
fn pair_seams(
    auxiliary: &[(usize, [usize; 2])],
    n: usize,
    max_comparisons: usize,
    matching: impl Fn([usize; 2], [usize; 2]) -> Result<bool>,
) -> Result<SeamPairs> {
    let mut out = SeamPairs {
        paired: vec![None; n],
        seams: vec![],
    };
    let mut comparisons = 0usize;
    for &(e, a) in auxiliary {
        if out.paired[e].is_some() {
            continue;
        }
        let mut matched = None;
        for &(f, b) in auxiliary.iter().filter(|(f, _)| *f != e) {
            comparisons = comparisons
                .checked_add(1)
                .filter(|v| *v <= max_comparisons)
                .ok_or_else(|| {
                    refused("Complete exact auxiliary interval pairing exceeds allowance")
                })?;
            if matching(a, b)? && matched.replace(f).is_some() {
                return Err(refused(
                    "Auxiliary interval has multiple exact original counterparts",
                ));
            }
        }
        let f = matched.ok_or_else(|| {
            refused("Unpaired auxiliary interval: authored seam-boundary admission is required")
        })?;
        if out.paired[f].is_some() {
            return Err(refused("Auxiliary seam counterpart is already owned"));
        }
        out.paired[e] = Some(f);
        out.paired[f] = Some(e);
        out.seams.push([e, f]);
    }
    Ok(out)
}
/// Quotient vertices of the outgoing-edge fans under the seam identifications,
/// and the vertex of every fan.
fn sewn_vertices(
    chart_edges: &[OriginalPeriodicChartCycleEdge],
    successor: &[usize],
    seams: &[[usize; 2]],
) -> (Vec<OriginalPeriodicSewnVertex>, Vec<usize>) {
    let n = chart_edges.len();
    let mut parent = (0..n).collect::<Vec<_>>();
    for &[e, f] in seams {
        // Original CCW directions are opposite in the exact world seam.
        // A fan is the interval link between its original incoming/outgoing
        // boundary ports. Joining its two seam link ends gives connected
        // paths/circles, preserving distinct unrelated contact fans.
        join(&mut parent, e, successor[f]);
        join(&mut parent, successor[e], f);
    }
    let mut vertices: Vec<OriginalPeriodicSewnVertex> = vec![];
    let mut vertex_ids = BTreeMap::new();
    let mut fan_vertex = vec![0; n];
    for fan in 0..n {
        let root = find(&mut parent, fan);
        let next = vertices.len();
        let id = *vertex_ids.entry(root).or_insert_with(|| {
            vertices.push(OriginalPeriodicSewnVertex {
                fans: vec![],
                chart_vertices: vec![],
            });
            next
        });
        vertices[id].fans.push(fan);
        vertices[id]
            .chart_vertices
            .push(chart_edges[fan].vertices[0]);
        fan_vertex[fan] = id;
    }
    for v in &mut vertices {
        v.chart_vertices.sort();
        v.chart_vertices.dedup();
    }
    (vertices, fan_vertex)
}
/// World edges after sewing: one per boundary chart edge, one per seam pair.
struct SewnEdges {
    edges: Vec<OriginalPeriodicSewnEdge>,
    /// World edge of every chart edge.
    ids: Vec<usize>,
    /// Whether a chart edge runs against its world edge (the second seam side).
    reversed: Vec<bool>,
}
fn sewn_edges(
    chart_edges: &[OriginalPeriodicChartCycleEdge],
    successor: &[usize],
    paired: &[Option<usize>],
    fan_vertex: &[usize],
) -> Result<SewnEdges> {
    let n = chart_edges.len();
    let mut out = SewnEdges {
        edges: vec![],
        ids: vec![usize::MAX; n],
        reversed: vec![false; n],
    };
    for e in 0..n {
        if out.ids[e] != usize::MAX {
            continue;
        }
        let id = out.edges.len();
        let mut members = vec![e];
        out.ids[e] = id;
        if let Some(f) = paired[e] {
            out.ids[f] = id;
            out.reversed[f] = true;
            members.push(f);
            if fan_vertex[e] != fan_vertex[successor[f]]
                || fan_vertex[successor[e]] != fan_vertex[f]
            {
                return Err(refused(
                    "Sewn edge endpoints disagree with exact opposite interval orientation",
                ));
            }
        }
        out.edges.push(OriginalPeriodicSewnEdge {
            vertices: [fan_vertex[e], fan_vertex[successor[e]]],
            chart_edges: members,
            original: chart_edges[e].kind,
        });
    }
    Ok(out)
}
/// Next actual boundary edge after every unpaired edge, skipping across sewn
/// seams. The map must be a bijection on the unpaired edges.
fn boundary_successors(
    successor: &[usize],
    paired: &[Option<usize>],
    fan_vertex: &[usize],
) -> Result<Vec<usize>> {
    let n = successor.len();
    let mut next = vec![usize::MAX; n];
    let mut predecessor = vec![0usize; n];
    for e in (0..n).filter(|&e| paired[e].is_none()) {
        let mut at = successor[e];
        let mut steps = 0;
        while let Some(f) = paired[at] {
            steps += 1;
            if steps > n {
                return Err(refused(
                    "Sewn boundary successor never reaches an actual boundary",
                ));
            }
            at = successor[f];
        }
        if fan_vertex[successor[e]] != fan_vertex[at] {
            return Err(refused(
                "Sewn boundary successor misses its original quotient fan vertex",
            ));
        }
        next[e] = at;
        predecessor[at] += 1;
    }
    if (0..n).any(|e| paired[e].is_none() && predecessor[e] != 1) {
        return Err(refused("Sewn material boundary successor is not bijective"));
    }
    Ok(next)
}
/// Boundary cycles of every sewn face, as world edge ids keyed by face root.
fn boundary_cycles(
    paired: &[Option<usize>],
    next: &[usize],
    edge_patch: &[usize],
    edge_ids: &[usize],
    patch_parent: &mut [usize],
) -> Result<BTreeMap<usize, Vec<Vec<usize>>>> {
    let n = next.len();
    let mut cycles: BTreeMap<usize, Vec<Vec<usize>>> = BTreeMap::new();
    let mut seen = vec![false; n];
    for e in 0..n {
        if paired[e].is_some() || seen[e] {
            continue;
        }
        let root = find(patch_parent, edge_patch[e]);
        let mut at = e;
        let mut walk = vec![];
        loop {
            if seen[at] {
                if at != e {
                    return Err(refused(
                        "Sewn boundary walk joins another already owned cycle",
                    ));
                }
                break;
            }
            if find(patch_parent, edge_patch[at]) != root {
                return Err(refused("Sewn boundary crosses an unrelated material face"));
            }
            seen[at] = true;
            walk.push(edge_ids[at]);
            at = next[at];
        }
        cycles.entry(root).or_default().push(walk);
    }
    Ok(cycles)
}
/// One sewn face: its Euler characteristic from the quotient vertex and edge
/// counts, and the genus that characteristic implies.
fn sewn_face(
    patches: Vec<usize>,
    boundary_cycles: Vec<Vec<usize>>,
    patch_boundaries: &[usize],
    incident: impl Iterator<Item = ([usize; 2], usize)>,
) -> Result<OriginalPeriodicSewnFace> {
    let mut vs = BTreeSet::new();
    let mut es = BTreeSet::new();
    for (ends, edge) in incident {
        vs.extend(ends);
        es.insert(edge);
    }
    // Each normalized chart patch is planar with b actual material-fan
    // boundary cycles, hence chi=2-b, INCLUDING tangent hole contacts.
    // V0=E0 on those boundary circles. Exact seam quotient gives V,E here.
    let correction = patches
        .iter()
        .map(|&p| 2 - patch_boundaries[p] as i64)
        .sum::<i64>();
    let chi = vs.len() as i64 - es.len() as i64 + correction;
    let twice_genus = 2 - boundary_cycles.len() as i64 - chi;
    if twice_genus < 0 || twice_genus % 2 != 0 {
        return Err(refused(
            "Sewn oriented material fan topology has inconsistent Euler data",
        ));
    }
    Ok(OriginalPeriodicSewnFace {
        patches,
        boundary_cycles,
        euler_characteristic: chi,
        genus: (twice_genus / 2) as usize,
    })
}
fn derive(o: &OwnedPeriodicSewnFaces) -> Result<Data> {
    let _replay = super::periodic_replay::scope();
    if !(1..=MAX_ALLOWANCE).contains(&o.limits.max_edges)
        || !(1..=MAX_ALLOWANCE).contains(&o.limits.max_seam_comparisons)
    {
        return Err(refused("Invalid complete sewn face allowance"));
    }
    super::periodic_replay::verify_arc(&o.owner, |owner| owner.reverify_source())?;
    super::periodic_replay::verify_arc(&o.embedding, |embedding| embedding.reverify_source())?;
    let c = cycles(&o.owner);
    let shores = c.owner().owner().owner();
    let traces = shores.owner().owner().owner().owner();
    let support = traces
        .owner()
        .owner()
        .owner()
        .owner()
        .owner()
        .rule()
        .support();
    if !Arc::ptr_eq(support.augmented(), o.embedding.wires()) {
        return Err(refused(
            "Sewn faces use a different original support embedding",
        ));
    }
    let n = c.edges().len();
    let patches = o.owner.patches();
    if n > o.limits.max_edges {
        return Err(refused("Complete sewn face edge census exceeds allowance"));
    }
    let successor = c.successor();
    let edge_patch = edge_patches(patches, o.owner.owner().owner().loops(), n)?;
    let patch_boundaries = patch_boundary_counts(c.walks(), &edge_patch, patches.len())?;
    let mut auxiliary = Vec::new();
    for (e, edge) in c.edges().iter().enumerate() {
        if let OriginalPeriodicChartCycleEdgeKind::NativeShore { shore, interval } = edge.kind
            && shores.shores()[shore].role == OriginalPeriodicNativeShoreRole::AuxiliarySeam
        {
            auxiliary.push((e, [shore, interval]));
        }
    }
    let SeamPairs { paired, seams } =
        pair_seams(&auxiliary, n, o.limits.max_seam_comparisons, |a, b| {
            shores.matching_opposite_interval(a, b)
        })?;
    let mut patch_parent = (0..patches.len()).collect::<Vec<_>>();
    for &[e, f] in &seams {
        join(&mut patch_parent, edge_patch[e], edge_patch[f]);
    }
    let (vertices, fan_vertex) = sewn_vertices(c.edges(), successor, &seams);
    let SewnEdges {
        edges,
        ids: edge_ids,
        reversed,
    } = sewn_edges(c.edges(), successor, &paired, &fan_vertex)?;
    let coedges = (0..n)
        .map(|e| OriginalPeriodicSewnCoedge {
            patch: edge_patch[e],
            edge: edge_ids[e],
            chart_edge: e,
            reversed: reversed[e],
        })
        .collect();
    let mut face_patches: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for p in 0..patches.len() {
        let root = find(&mut patch_parent, p);
        face_patches.entry(root).or_default().push(p);
    }
    let next = boundary_successors(successor, &paired, &fan_vertex)?;
    let mut face_boundaries =
        boundary_cycles(&paired, &next, &edge_patch, &edge_ids, &mut patch_parent)?;
    let mut faces = vec![];
    for (root, ps) in face_patches {
        let selected = ps.iter().copied().collect::<BTreeSet<_>>();
        let incident = (0..n)
            .filter(|&e| selected.contains(&edge_patch[e]))
            .map(|e| ([fan_vertex[e], fan_vertex[successor[e]]], edge_ids[e]));
        let boundary = face_boundaries.remove(&root).unwrap_or_default();
        faces.push(sewn_face(ps, boundary, &patch_boundaries, incident)?);
    }
    Ok(Data {
        vertices,
        edges,
        coedges,
        faces,
        seams,
    })
}
impl OwnedPeriodicSewnFaces {
    pub fn owner(&self) -> &Arc<OwnedPeriodicChartFanPatches> {
        &self.owner
    }
    pub fn embedding(&self) -> &Arc<OwnedOriginalPeriodicEmbedding> {
        &self.embedding
    }
    pub fn vertices(&self) -> &[OriginalPeriodicSewnVertex] {
        &self.data.vertices
    }
    pub fn edges(&self) -> &[OriginalPeriodicSewnEdge] {
        &self.data.edges
    }
    pub fn coedges(&self) -> &[OriginalPeriodicSewnCoedge] {
        &self.data.coedges
    }
    pub fn faces(&self) -> &[OriginalPeriodicSewnFace] {
        &self.data.faces
    }
    pub fn seams(&self) -> &[[usize; 2]] {
        &self.data.seams
    }
    pub fn reverify_source(&self) -> Result<()> {
        if derive(self)? != self.data {
            return Err(refused(
                "Original sewn face, coedge or material fan ownership changed",
            ));
        }
        Ok(())
    }
    /// Exact evaluation of the retained original surface composition. Native
    /// source clocks and algebraic interval admissions remain unchanged.
    pub fn world_point(&self, edge: usize, native: &ExactRatio) -> Result<[ExactRatio; 3]> {
        let e = self
            .data
            .edges
            .get(edge)
            .ok_or_else(|| refused("Unknown original sewn world edge"))?;
        let shores = cycles(&self.owner).owner().owner().owner();
        match e.original {
            OriginalPeriodicChartCycleEdgeKind::NativeShore { shore, interval } => {
                shores.point(shore, interval, native)
            }
            OriginalPeriodicChartCycleEdgeKind::SourceTrace(trace) => Ok(shores
                .owner()
                .owner()
                .owner()
                .owner()
                .point(trace, native)?
                .world),
        }
    }
}
pub fn own_periodic_sewn_faces(
    owner: Arc<OwnedPeriodicChartFanPatches>,
    limits: PeriodicSewnFaceLimits,
) -> Result<OwnedPeriodicSewnFaces> {
    let _replay = super::periodic_replay::scope();
    super::periodic_replay::verify_arc(&owner, |owner| owner.reverify_source())?;
    let shores = cycles(&owner).owner().owner().owner();
    let traces = shores.owner().owner().owner().owner();
    let support = traces
        .owner()
        .owner()
        .owner()
        .owner()
        .owner()
        .rule()
        .support();
    let embedding = Arc::new(own_original_periodic_embedding(
        support.augmented().clone(),
        limits.max_regularity_cells,
        limits.atlas,
        limits.pairs,
    )?);
    let mut o = OwnedPeriodicSewnFaces {
        owner,
        embedding,
        limits,
        data: Data {
            vertices: vec![],
            edges: vec![],
            coedges: vec![],
            faces: vec![],
            seams: vec![],
        },
    };
    o.data = derive(&o)?;
    Ok(o)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn from_shores(shores: OwnedPeriodicNativeChartShores) -> OwnedPeriodicChartWinding {
        let samples = Arc::new(
            own_periodic_shore_material_samples(
                Arc::new(shores),
                Default::default(),
                Default::default(),
            )
            .unwrap(),
        );
        let intervals = Arc::new(
            own_periodic_shore_material_intervals(samples, Default::default(), Default::default())
                .unwrap(),
        );
        construct_periodic_chart_winding(intervals, Default::default(), Default::default()).unwrap()
    }
    fn patches(w: OwnedPeriodicChartWinding) -> Arc<OwnedPeriodicChartFanPatches> {
        let loops = Arc::new(
            own_periodic_chart_boundary_loops(w.owner().clone(), Default::default()).unwrap(),
        );
        let winding = Arc::new(own_periodic_chart_loop_winding(loops, Default::default()).unwrap());
        Arc::new(own_periodic_chart_fan_patches(winding, Default::default()).unwrap())
    }
    #[test]
    fn actual_full_torus_sews_both_axes_and_corners_without_authored_trim_edges() {
        let _replay = super::super::periodic_replay::scope();
        let p = patches(from_shores(
            super::super::periodic_chart_shores::tests::full_torus(),
        ));
        let o = own_periodic_sewn_faces(p.clone(), Default::default()).unwrap();
        assert_eq!(o.seams().len(), 2);
        assert_eq!(o.faces().len(), 1);
        assert_eq!(o.vertices().len(), 1);
        assert_eq!(o.edges().len(), 2);
        assert_eq!(o.coedges().len(), 4);
        assert!(o.faces()[0].boundary_cycles.is_empty());
        assert_eq!(o.faces()[0].genus, 1);
        assert_eq!(o.faces()[0].euler_characteristic, 0);
        o.reverify_source().unwrap();
        assert!(
            o.world_point(usize::MAX, &["1".into(), "2".into()])
                .is_err()
        );
        assert!(o.world_point(0, &["2".into(), "4".into()]).is_err());
        assert!(
            own_periodic_sewn_faces(
                p.clone(),
                PeriodicSewnFaceLimits {
                    max_seam_comparisons: 1,
                    ..Default::default()
                }
            )
            .is_err()
        );
        let mut bad = o.clone();
        bad.data.seams.pop();
        assert!(bad.reverify_source().is_err());
        assert!(
            own_periodic_sewn_faces(
                p,
                PeriodicSewnFaceLimits {
                    max_edges: 1,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
    #[test]
    fn actual_curved_seam_crossing_keeps_irrational_cuts_and_exact_world_composition() {
        let _replay = super::super::periodic_replay::scope();
        let p = patches(from_shores(
            super::super::periodic_chart_shores::tests::shores(
                super::super::periodic_chart_shores::tests::cylinder_profile(true),
            ),
        ));
        let o = own_periodic_sewn_faces(p, Default::default()).unwrap();
        assert!(!o.seams().is_empty());
        assert_eq!(o.faces().len(), 1);
        assert_eq!(o.faces()[0].genus, 0);
        assert_eq!(o.faces()[0].boundary_cycles.len(), 1);
        assert_eq!(o.faces()[0].euler_characteristic, 1);
        let shores = cycles(o.owner()).owner().owner().owner();
        let traces = shores.owner().owner().owner().owner();
        let mut saw = false;
        let mut algebraic = false;
        for (e, edge) in o.edges().iter().enumerate() {
            if let OriginalPeriodicChartCycleEdgeKind::SourceTrace(trace) = edge.original {
                let t = &traces.traces()[trace].original;
                algebraic |= t.parameters.iter().any(|p| p.primitive.len() > 2);
                if t.source == 0 {
                    if let Ok(point) = o.world_point(e, &["1".to_string(), "2".to_string()]) {
                        assert_eq!(
                            point,
                            [
                                ["0".to_string(), "1".to_string()],
                                ["1".to_string(), "8".to_string()],
                                ["1".to_string(), "2".to_string()]
                            ]
                        );
                        saw = true;
                    }
                }
            }
        }
        assert!(saw && algebraic);
        o.reverify_source().unwrap();
    }
    #[test]
    fn actual_point_touching_faces_preserve_distinct_original_material_fans() {
        let _replay = super::super::periodic_replay::scope();
        let w = super::super::periodic_chart_cycles::actual_fan_tests::build(
            super::super::periodic_support_regions::tests::discs(true),
            2,
        );
        let o = own_periodic_sewn_faces(patches(w), Default::default()).unwrap();
        assert!(o.seams().is_empty());
        assert_eq!(o.faces().len(), 2);
        assert!(
            o.faces().iter().all(|f| f.genus == 0
                && f.euler_characteristic == 1
                && f.boundary_cycles.len() == 1)
        );
        let mut shared = BTreeMap::new();
        for v in o.vertices() {
            for &chart in &v.chart_vertices {
                *shared.entry(chart).or_insert(0) += 1;
            }
        }
        assert!(shared.values().any(|&uses| uses == 2));
        o.reverify_source().unwrap();
    }
    #[test]
    fn actual_tangent_hole_keeps_contact_normalization_and_one_material_boundary_walk() {
        let _replay = super::super::periodic_replay::scope();
        let w = super::super::periodic_chart_cycles::actual_fan_tests::build(
            super::super::periodic_support_material::tests::tangent_hole(),
            1,
        );
        let o = own_periodic_sewn_faces(patches(w), Default::default()).unwrap();
        assert_eq!(o.owner().patches().len(), 1);
        assert_eq!(o.owner().patches()[0].holes.len(), 1);
        assert_eq!(o.faces().len(), 1);
        assert_eq!(o.faces()[0].boundary_cycles.len(), 1);
        assert_eq!(o.faces()[0].euler_characteristic, 1);
        assert_eq!(o.faces()[0].genus, 0);
        o.reverify_source().unwrap();
    }
}
