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
fn derive(o: &OwnedPeriodicSewnFaces) -> Result<Data> {
    let _replay = super::periodic_replay::scope();
    if !(1..=1000000).contains(&o.limits.max_edges)
        || !(1..=1000000).contains(&o.limits.max_seam_comparisons)
    {
        return Err(refused("Invalid complete sewn face allowance"));
    }
    o.owner.reverify_source()?;
    o.embedding.reverify_source()?;
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
    let loops = o.owner.owner().owner();
    let mut loop_patch = vec![usize::MAX; loops.loops().len()];
    for (patch, p) in patches.iter().enumerate() {
        for boundary in std::iter::once(p.outer).chain(p.holes.iter().copied()) {
            if std::mem::replace(&mut loop_patch[boundary], patch) != usize::MAX {
                return Err(refused("Loop belongs to multiple actual material patches"));
            }
        }
    }
    let mut edge_patch = vec![usize::MAX; n];
    for (id, b) in loops.loops().iter().enumerate() {
        if loop_patch[id] == usize::MAX {
            return Err(refused("Sewn face omits an original boundary loop"));
        }
        for &edge in &b.edges {
            edge_patch[edge] = loop_patch[id];
        }
    }
    let mut patch_boundaries = vec![0usize; patches.len()];
    for walk in c.walks() {
        let patch = edge_patch[*walk
            .first()
            .ok_or_else(|| refused("Empty original material boundary"))?];
        if walk.iter().any(|&e| edge_patch[e] != patch) {
            return Err(refused(
                "Original contact fan walk needs further owned material patch separation",
            ));
        }
        patch_boundaries[patch] += 1;
    }
    if patch_boundaries.iter().any(|b| *b == 0) {
        return Err(refused("Unbounded or boundary-free chart patch is unowned"));
    }
    let mut auxiliary = Vec::new();
    for (e, edge) in c.edges().iter().enumerate() {
        if let OriginalPeriodicChartCycleEdgeKind::NativeShore { shore, interval } = edge.kind {
            if shores.shores()[shore].role == OriginalPeriodicNativeShoreRole::AuxiliarySeam {
                auxiliary.push((e, [shore, interval]));
            }
        }
    }
    let mut paired = vec![None; n];
    let mut seams = vec![];
    let mut comparisons = 0usize;
    for &(e, a) in &auxiliary {
        if paired[e].is_some() {
            continue;
        }
        let mut matched = None;
        for &(f, b) in &auxiliary {
            if f != e {
                comparisons = comparisons
                    .checked_add(1)
                    .filter(|v| *v <= o.limits.max_seam_comparisons)
                    .ok_or_else(|| {
                        refused("Complete exact auxiliary interval pairing exceeds allowance")
                    })?;
                if shores.matching_opposite_interval(a, b)? {
                    if matched.replace(f).is_some() {
                        return Err(refused(
                            "Auxiliary interval has multiple exact original counterparts",
                        ));
                    }
                }
            }
        }
        let f = matched.ok_or_else(|| {
            refused("Unpaired auxiliary interval: authored seam-boundary admission is required")
        })?;
        if paired[f].is_some() {
            return Err(refused("Auxiliary seam counterpart is already owned"));
        }
        paired[e] = Some(f);
        paired[f] = Some(e);
        seams.push([e, f]);
    }
    let mut vertex_parent = (0..n).collect::<Vec<_>>();
    let mut patch_parent = (0..patches.len()).collect::<Vec<_>>();
    for &[e, f] in &seams {
        // Original CCW directions are opposite in the exact world seam.
        // A fan is the interval link between its original incoming/outgoing
        // boundary ports. Joining its two seam link ends gives connected
        // paths/circles, preserving distinct unrelated contact fans.
        join(&mut vertex_parent, e, c.successor()[f]);
        join(&mut vertex_parent, c.successor()[e], f);
        join(&mut patch_parent, edge_patch[e], edge_patch[f]);
    }
    let mut vertices: Vec<OriginalPeriodicSewnVertex> = vec![];
    let mut vertex_ids = BTreeMap::new();
    let mut fan_vertex = vec![0; n];
    for fan in 0..n {
        let root = find(&mut vertex_parent, fan);
        let next = vertices.len();
        let id = *vertex_ids.entry(root).or_insert_with(|| {
            vertices.push(OriginalPeriodicSewnVertex {
                fans: vec![],
                chart_vertices: vec![],
            });
            next
        });
        vertices[id].fans.push(fan);
        vertices[id].chart_vertices.push(c.edges()[fan].vertices[0]);
        fan_vertex[fan] = id;
    }
    for v in &mut vertices {
        v.chart_vertices.sort();
        v.chart_vertices.dedup();
    }
    let mut edges = vec![];
    let mut edge_ids = vec![usize::MAX; n];
    let mut reversed = vec![false; n];
    for e in 0..n {
        if edge_ids[e] == usize::MAX {
            let id = edges.len();
            let mut chart_edges = vec![e];
            edge_ids[e] = id;
            if let Some(f) = paired[e] {
                edge_ids[f] = id;
                reversed[f] = true;
                chart_edges.push(f);
                if fan_vertex[e] != fan_vertex[c.successor()[f]]
                    || fan_vertex[c.successor()[e]] != fan_vertex[f]
                {
                    return Err(refused(
                        "Sewn edge endpoints disagree with exact opposite interval orientation",
                    ));
                }
            }
            edges.push(OriginalPeriodicSewnEdge {
                vertices: [fan_vertex[e], fan_vertex[c.successor()[e]]],
                chart_edges,
                original: c.edges()[e].kind,
            });
        }
    }
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
    let mut next = vec![usize::MAX; n];
    let mut predecessor = vec![0usize; n];
    for e in 0..n {
        if paired[e].is_none() {
            let mut at = c.successor()[e];
            let mut steps = 0;
            while let Some(f) = paired[at] {
                steps += 1;
                if steps > n {
                    return Err(refused(
                        "Sewn boundary successor never reaches an actual boundary",
                    ));
                }
                at = c.successor()[f];
            }
            if fan_vertex[c.successor()[e]] != fan_vertex[at] {
                return Err(refused(
                    "Sewn boundary successor misses its original quotient fan vertex",
                ));
            }
            next[e] = at;
            predecessor[at] += 1;
        }
    }
    if (0..n).any(|e| paired[e].is_none() && predecessor[e] != 1) {
        return Err(refused("Sewn material boundary successor is not bijective"));
    }
    let mut face_boundaries: BTreeMap<usize, Vec<Vec<usize>>> = BTreeMap::new();
    let mut seen = vec![false; n];
    for e in 0..n {
        if paired[e].is_none() && !seen[e] {
            let root = find(&mut patch_parent, edge_patch[e]);
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
                if find(&mut patch_parent, edge_patch[at]) != root {
                    return Err(refused("Sewn boundary crosses an unrelated material face"));
                }
                seen[at] = true;
                walk.push(edge_ids[at]);
                at = next[at];
            }
            face_boundaries.entry(root).or_default().push(walk);
        }
    }
    let mut faces = vec![];
    for (root, ps) in face_patches {
        let selected = ps.iter().copied().collect::<BTreeSet<_>>();
        let mut vs = BTreeSet::new();
        let mut es = BTreeSet::new();
        for e in 0..n {
            if selected.contains(&edge_patch[e]) {
                vs.insert(fan_vertex[e]);
                vs.insert(fan_vertex[c.successor()[e]]);
                es.insert(edge_ids[e]);
            }
        }
        // Each normalized chart patch is planar with b actual material-fan
        // boundary cycles, hence chi=2-b, INCLUDING tangent hole contacts.
        // V0=E0 on those boundary circles. Exact seam quotient gives V,E here.
        let correction = ps
            .iter()
            .map(|&p| 2 - patch_boundaries[p] as i64)
            .sum::<i64>();
        let chi = vs.len() as i64 - es.len() as i64 + correction;
        let boundary_cycles = face_boundaries.remove(&root).unwrap_or_default();
        let twice_genus = 2 - boundary_cycles.len() as i64 - chi;
        if twice_genus < 0 || twice_genus % 2 != 0 {
            return Err(refused(
                "Sewn oriented material fan topology has inconsistent Euler data",
            ));
        }
        faces.push(OriginalPeriodicSewnFace {
            patches: ps,
            boundary_cycles,
            euler_characteristic: chi,
            genus: (twice_genus / 2) as usize,
        });
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
    owner.reverify_source()?;
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
