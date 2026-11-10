//! Edge-exhaustive decomposition of source-owned weak chart walks.
//! Loops retain original edge orientation and contacts. They grant no patch,
//! containment, world embedding or sewing authority.
use super::*;
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Copy, Debug)]
pub struct PeriodicChartBoundaryLoopLimits {
    pub max_edge_uses: usize,
    pub max_loops: usize,
}
impl Default for PeriodicChartBoundaryLoopLimits {
    fn default() -> Self {
        Self {
            max_edge_uses: 1_000_000,
            max_loops: 1_000_000,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicChartBoundaryLoop {
    pub source_walk: usize,
    pub edges: Vec<usize>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicChartLoopContact {
    pub vertex: usize,
    /// Loop IDs incident to this original chart vertex. Not a fan ordering.
    pub loops: Vec<usize>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Data {
    loops: Vec<OriginalPeriodicChartBoundaryLoop>,
    contacts: Vec<OriginalPeriodicChartLoopContact>,
    /// Every original edge owns exactly one (loop, position).
    edge_locations: Vec<[usize; 2]>,
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicChartBoundaryLoops {
    owner: Arc<OwnedPeriodicChartCycles>,
    limits: PeriodicChartBoundaryLoopLimits,
    data: Data,
}
fn decompose(
    edges: &[OriginalPeriodicChartCycleEdge],
    vertex_count: usize,
    walks: &[Vec<usize>],
    limits: PeriodicChartBoundaryLoopLimits,
) -> Result<Data> {
    if !(1..=1_000_000).contains(&limits.max_edge_uses)
        || !(1..=1_000_000).contains(&limits.max_loops)
        || edges.len() > limits.max_edge_uses
        || vertex_count > 1_000_000
        || walks.len() > limits.max_edge_uses
    {
        return Err(refused(
            "Invalid or exhausted chart boundary loop allowance",
        ));
    }
    let mut seen = vec![false; edges.len()];
    let mut loops = Vec::new();
    for (source_walk, walk) in walks.iter().enumerate() {
        let first = *walk
            .first()
            .ok_or_else(|| refused("Empty source boundary walk"))?;
        let start = edges
            .get(first)
            .ok_or_else(|| refused("Invalid source edge"))?
            .vertices[0];
        let mut vertices = vec![start];
        let mut stack = Vec::new();
        let mut positions = BTreeMap::from([(start, 0usize)]);
        for &edge in walk {
            let e = edges
                .get(edge)
                .ok_or_else(|| refused("Invalid source edge"))?;
            if e.vertices.iter().any(|&v| v >= vertex_count)
                || vertices.last().copied() != Some(e.vertices[0])
                || std::mem::replace(&mut seen[edge], true)
            {
                return Err(refused("Boundary walk incidence or edge census changed"));
            }
            stack.push(edge);
            if let Some(&at) = positions.get(&e.vertices[1]) {
                if loops.len() >= limits.max_loops {
                    return Err(refused("Complete boundary loop census exceeds allowance"));
                }
                // The active stack has unique vertices. Its suffix closes
                // precisely at the repeated ORIGINAL chart vertex.
                loops.push(OriginalPeriodicChartBoundaryLoop {
                    source_walk,
                    edges: stack.split_off(at),
                });
                for v in vertices.drain(at + 1..) {
                    positions.remove(&v);
                }
            } else {
                positions.insert(e.vertices[1], vertices.len());
                vertices.push(e.vertices[1]);
            }
        }
        if !stack.is_empty() || vertices != [start] {
            return Err(refused("Original chart walk is not completely closed"));
        }
    }
    if seen.iter().any(|v| !v) {
        return Err(refused("Boundary loop census omits an original edge"));
    }
    let mut edge_locations = vec![[usize::MAX; 2]; edges.len()];
    let mut incidence: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (id, boundary) in loops.iter().enumerate() {
        for (position, &edge) in boundary.edges.iter().enumerate() {
            edge_locations[edge] = [id, position];
            incidence
                .entry(edges[edge].vertices[0])
                .or_default()
                .push(id);
        }
    }
    let contacts = incidence
        .into_iter()
        .filter_map(|(vertex, loops)| {
            (loops.len() > 1).then_some(OriginalPeriodicChartLoopContact { vertex, loops })
        })
        .collect();
    Ok(Data {
        loops,
        contacts,
        edge_locations,
    })
}
impl OwnedPeriodicChartBoundaryLoops {
    pub fn owner(&self) -> &Arc<OwnedPeriodicChartCycles> {
        &self.owner
    }
    pub fn loops(&self) -> &[OriginalPeriodicChartBoundaryLoop] {
        &self.data.loops
    }
    pub fn contacts(&self) -> &[OriginalPeriodicChartLoopContact] {
        &self.data.contacts
    }
    pub fn edge_locations(&self) -> &[[usize; 2]] {
        &self.data.edge_locations
    }
    pub fn reverify_source(&self) -> Result<()> {
        let _replay = super::periodic_replay::scope();
        super::periodic_replay::verify_arc(&self.owner, |owner| owner.reverify_source())?;
        if decompose(
            self.owner.edges(),
            self.owner.vertices().len(),
            self.owner.walks(),
            self.limits,
        )? != self.data
        {
            return Err(refused("Original chart boundary loop ownership changed"));
        }
        Ok(())
    }
}
pub fn own_periodic_chart_boundary_loops(
    owner: Arc<OwnedPeriodicChartCycles>,
    limits: PeriodicChartBoundaryLoopLimits,
) -> Result<OwnedPeriodicChartBoundaryLoops> {
    let _replay = super::periodic_replay::scope();
    super::periodic_replay::verify_arc(&owner, |owner| owner.reverify_source())?;
    let data = decompose(owner.edges(), owner.vertices().len(), owner.walks(), limits)?;
    Ok(OwnedPeriodicChartBoundaryLoops {
        owner,
        limits,
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn edges(pairs: &[[usize; 2]]) -> Vec<OriginalPeriodicChartCycleEdge> {
        pairs
            .iter()
            .enumerate()
            .map(|(i, &vertices)| OriginalPeriodicChartCycleEdge {
                vertices,
                kind: OriginalPeriodicChartCycleEdgeKind::SourceTrace(i),
            })
            .collect()
    }
    #[test]
    fn repeated_vertex_retains_two_loops_all_edges_and_contact() {
        let e = edges(&[[0, 1], [1, 2], [2, 0], [0, 3], [3, 4], [4, 0]]);
        let d = decompose(&e, 5, &[vec![0, 1, 2, 3, 4, 5]], Default::default()).unwrap();
        assert_eq!(
            d.loops.iter().map(|l| l.edges.clone()).collect::<Vec<_>>(),
            vec![vec![0, 1, 2], vec![3, 4, 5]]
        );
        assert_eq!(
            d.contacts,
            vec![OriginalPeriodicChartLoopContact {
                vertex: 0,
                loops: vec![0, 1]
            }]
        );
        for (i, loc) in d.edge_locations.iter().enumerate() {
            assert_eq!(d.loops[loc[0]].edges[loc[1]], i);
        }
    }
    #[test]
    fn nested_excursions_keep_source_walk_and_original_directions() {
        let e = edges(&[[0, 1], [1, 2], [2, 3], [3, 2], [2, 1], [1, 0], [4, 4]]);
        let d = decompose(
            &e,
            5,
            &[vec![0, 1, 2, 3, 4, 5], vec![6]],
            Default::default(),
        )
        .unwrap();
        assert_eq!(
            d.loops.iter().map(|l| l.edges.clone()).collect::<Vec<_>>(),
            vec![vec![2, 3], vec![1, 4], vec![0, 5], vec![6]]
        );
        assert_eq!(d.loops[3].source_walk, 1);
    }
    #[test]
    fn omissions_duplicates_open_walks_and_limits_never_publish_partial_loops() {
        let e = edges(&[[0, 1], [1, 0], [0, 2], [2, 0]]);
        for w in [
            vec![vec![0, 1]],
            vec![vec![0, 1, 0, 1]],
            vec![vec![0, 2, 3, 1]],
            vec![vec![0, 1, 2]],
            vec![vec![]],
        ] {
            assert!(decompose(&e, 3, &w, Default::default()).is_err());
        }
        assert!(
            decompose(
                &e,
                3,
                &[vec![0, 1, 2, 3]],
                PeriodicChartBoundaryLoopLimits {
                    max_loops: 1,
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert!(
            decompose(
                &e,
                3,
                &[vec![0, 1, 2, 3]],
                PeriodicChartBoundaryLoopLimits {
                    max_edge_uses: 3,
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert!(decompose(&e, 2, &[vec![0, 1, 2, 3]], Default::default()).is_err());
    }
}

#[cfg(test)]
mod source_tests {
    use super::*;
    fn check(winding: OwnedPeriodicChartWinding, source_walks: usize) {
        let source = winding.owner().clone();
        assert_eq!(source.walks().len(), source_walks);
        let owner = own_periodic_chart_boundary_loops(source, Default::default()).unwrap();
        assert_eq!(owner.loops().len(), 2);
        assert_eq!(owner.contacts().len(), 1);
        assert_eq!(owner.contacts()[0].loops, vec![0, 1]);
        assert_eq!(owner.edge_locations().len(), owner.owner().edges().len());
        owner.reverify_source().unwrap();
        let mut bad = owner.clone();
        bad.data.loops[0].edges.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.data.contacts.clear();
        assert!(bad.reverify_source().is_err());
    }
    #[test]
    fn actual_touching_regions_keep_contact_and_distinct_source_walks() {
        let w = super::super::periodic_chart_cycles::actual_fan_tests::build(
            super::super::periodic_support_regions::tests::discs(true),
            2,
        );
        check(w, 2);
    }
    #[test]
    fn actual_tangent_hole_splits_weak_walk_without_claiming_patch_ownership() {
        let w = super::super::periodic_chart_cycles::actual_fan_tests::build(
            super::super::periodic_support_material::tests::tangent_hole(),
            1,
        );
        check(w, 1);
    }
}
