//! Strictly disjoint Jordan chart cycles: exact containment and local patches.
//! Contact/repeated-vertex walks require another topology proof and refuse here.
use super::*;
use std::{collections::BTreeSet, sync::Arc};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPeriodicChartStrictPatch {
    pub outer: usize,
    pub holes: Vec<usize>,
}
#[derive(Clone, Copy, Debug)]
pub struct PeriodicChartStrictPatchLimits {
    pub max_cycles: usize,
    pub max_vertex_uses: usize,
    pub max_relation_work: usize,
}
impl Default for PeriodicChartStrictPatchLimits {
    fn default() -> Self {
        Self {
            max_cycles: 100000,
            max_vertex_uses: 1000000,
            max_relation_work: 1000000,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Data {
    contains: Vec<Vec<bool>>,
    parents: Vec<Option<usize>>,
    patches: Vec<OriginalPeriodicChartStrictPatch>,
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicChartStrictPatches {
    owner: Arc<OwnedPeriodicChartCycleSides>,
    limits: PeriodicChartStrictPatchLimits,
    data: Data,
}
fn hierarchy(contains: Vec<Vec<bool>>, positive: &[bool]) -> Result<Data> {
    let n = positive.len();
    if contains.len() != n || contains.iter().any(|r| r.len() != n) {
        return Err(refused("Incomplete strict chart containment matrix"));
    }
    let mut parents = vec![None; n];
    for child in 0..n {
        if contains[child][child] {
            return Err(refused("Strict chart cycle contains itself"));
        }
        let ancestors = (0..n).filter(|&p| contains[p][child]).collect::<Vec<_>>();
        for &a in &ancestors {
            if contains[child][a] {
                return Err(refused("Cyclic strict chart containment"));
            }
            for &b in &ancestors {
                if a != b && contains[a][b] == contains[b][a] {
                    return Err(refused("Strict Jordan ancestors are not totally nested"));
                }
            }
        }
        if !ancestors.is_empty() {
            let nearest = ancestors
                .iter()
                .copied()
                .filter(|&p| ancestors.iter().all(|&a| a == p || contains[a][p]))
                .collect::<Vec<_>>();
            if nearest.len() != 1 {
                return Err(refused(
                    "Strict chart cycle has no unique immediate container",
                ));
            }
            parents[child] = Some(nearest[0]);
        }
        if let Some(parent) = parents[child] {
            if positive[parent] == positive[child] {
                return Err(refused("Strict chart material boundaries do not alternate"));
            }
        } else if !positive[child] {
            return Err(refused("Uncontained negative chart boundary"));
        }
    }
    // Check every derived parent chain against the complete exact matrix.
    // A nearest-parent shortcut alone cannot admit a non-transitive matrix.
    for child in 0..n {
        let mut chain = vec![false; n];
        let mut p = parents[child];
        for _ in 0..n {
            let Some(parent) = p else { break };
            if chain[parent] {
                return Err(refused("Strict chart parent cycle"));
            }
            chain[parent] = true;
            p = parents[parent];
        }
        if p.is_some() || (0..n).any(|a| chain[a] != contains[a][child]) {
            return Err(refused(
                "Strict chart hierarchy omits exact containment relations",
            ));
        }
    }
    let patches = (0..n)
        .filter(|&i| positive[i])
        .map(|outer| OriginalPeriodicChartStrictPatch {
            outer,
            holes: (0..n).filter(|&i| parents[i] == Some(outer)).collect(),
        })
        .collect();
    Ok(Data {
        contains,
        parents,
        patches,
    })
}
fn derive(o: &OwnedPeriodicChartStrictPatches) -> Result<Data> {
    let _replay = super::periodic_replay::scope();
    let l = o.limits;
    if [l.max_cycles, l.max_vertex_uses, l.max_relation_work]
        .iter()
        .any(|n| !(1..=1000000).contains(n))
    {
        return Err(refused("Invalid complete strict chart patch allowance"));
    }
    o.owner.reverify_source()?;
    let cycles = o.owner.owner().owner();
    let n = cycles.walks().len();
    if n > l.max_cycles {
        return Err(refused("Strict chart cycle census exceeds allowance"));
    }
    // Upper bound for complete matrix, all ancestor comparisons and chains.
    let square = n
        .checked_mul(n)
        .ok_or_else(|| refused("Strict chart relation work overflow"))?;
    let work = square
        .checked_mul(n)
        .and_then(|v| v.checked_mul(2))
        .and_then(|v| square.checked_mul(10).and_then(|s| v.checked_add(s)))
        .ok_or_else(|| refused("Strict chart relation work overflow"))?;
    if work > l.max_relation_work {
        return Err(refused(
            "Complete strict chart relation work exceeds allowance",
        ));
    }
    let mut used = BTreeSet::new();
    let mut uses = 0usize;
    for walk in cycles.walks() {
        for &edge in walk {
            uses = uses
                .checked_add(1)
                .filter(|n| *n <= l.max_vertex_uses)
                .ok_or_else(|| refused("Strict chart vertex census exceeds allowance"))?;
            if !used.insert(cycles.edges()[edge].vertices[0]) {
                return Err(refused(
                    "Contact or repeated chart vertex requires owned fan-aware patches",
                ));
            }
        }
    }
    // Parent ownership already proves complete original embedding, exact source
    // and frame incidence and exhaustive cycles. No shared vertex now means
    // pairwise disjoint simple Jordan boundaries in this actual chart. The
    // owned transverse witness leg meets only its own edge, so its exact
    // winding in every OTHER cycle equals that entire disjoint boundary's
    // constant winding. No arbitrary point/area sampling infers containment.
    let positive = o
        .owner
        .sides()
        .iter()
        .map(|s| s.orientation == PeriodicChartCycleOrientation::Positive)
        .collect::<Vec<_>>();
    let mut contains = vec![vec![false; n]; n];
    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            let value = o.owner.sides()[j].windings[0][i];
            if value != 0 && value != if positive[i] { 1 } else { -1 } {
                return Err(refused(
                    "Disjoint Jordan winding has an inconsistent orientation",
                ));
            }
            contains[i][j] = value != 0;
        }
    }
    hierarchy(contains, &positive)
}
impl OwnedPeriodicChartStrictPatches {
    pub fn owner(&self) -> &Arc<OwnedPeriodicChartCycleSides> {
        &self.owner
    }
    pub fn contains(&self) -> &[Vec<bool>] {
        &self.data.contains
    }
    pub fn parents(&self) -> &[Option<usize>] {
        &self.data.parents
    }
    pub fn patches(&self) -> &[OriginalPeriodicChartStrictPatch] {
        &self.data.patches
    }
    pub fn reverify_source(&self) -> Result<()> {
        if derive(self)? != self.data {
            return Err(refused("Strict chart patch ownership changed"));
        }
        Ok(())
    }
}
pub fn own_periodic_chart_strict_patches(
    owner: Arc<OwnedPeriodicChartCycleSides>,
    limits: PeriodicChartStrictPatchLimits,
) -> Result<OwnedPeriodicChartStrictPatches> {
    let mut o = OwnedPeriodicChartStrictPatches {
        owner,
        limits,
        data: Data {
            contains: Vec::new(),
            parents: Vec::new(),
            patches: Vec::new(),
        },
    };
    o.data = derive(&o)?;
    Ok(o)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_island_belongs_to_its_own_patch_inside_an_outer_hole() {
        let c = vec![
            vec![false, true, true, true],
            vec![false, false, true, true],
            vec![false, false, false, true],
            vec![false, false, false, false],
        ];
        let o = hierarchy(c, &[true, false, true, false]).unwrap();
        assert_eq!(o.parents, vec![None, Some(0), Some(1), Some(2)]);
        assert_eq!(
            o.patches,
            vec![
                OriginalPeriodicChartStrictPatch {
                    outer: 0,
                    holes: vec![1]
                },
                OriginalPeriodicChartStrictPatch {
                    outer: 2,
                    holes: vec![3]
                }
            ]
        );
    }
    #[test]
    fn inconsistent_containment_or_material_alternation_refuses() {
        assert!(hierarchy(vec![vec![false]], &[false]).is_err());
        assert!(hierarchy(vec![vec![false, true], vec![true, false]], &[true, false]).is_err());
        assert!(hierarchy(vec![vec![false, true], vec![false, false]], &[true, true]).is_err());
        assert!(
            hierarchy(
                vec![
                    vec![false, true, false],
                    vec![false, false, true],
                    vec![false, false, false]
                ],
                &[true, false, true]
            )
            .is_err()
        );
    }
}

#[cfg(test)]
pub(super) fn check_tampering(o: &OwnedPeriodicChartStrictPatches) {
    let mut bad = o.clone();
    bad.data.patches[0].holes.clear();
    assert!(bad.reverify_source().is_err());
    assert!(
        own_periodic_chart_strict_patches(
            o.owner.clone(),
            PeriodicChartStrictPatchLimits {
                max_relation_work: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
}
