//! Contact-aware chart patches from exact original germ rotations and winding.
//! Original chart contacts remain explicit; no world sewing is inferred here.
use super::*;
use std::{collections::BTreeMap, sync::Arc};
#[derive(Clone, Copy, Debug)]
pub struct PeriodicChartFanPatchLimits {
    pub sides: PeriodicChartCycleSideLimits,
    pub max_relation_work: usize,
}
impl Default for PeriodicChartFanPatchLimits {
    fn default() -> Self {
        Self {
            sides: Default::default(),
            max_relation_work: 1_000_000,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Data {
    sides: Vec<OriginalPeriodicChartCycleSides>,
    contains: Vec<Vec<bool>>,
    parents: Vec<Option<usize>>,
    patches: Vec<OriginalPeriodicChartStrictPatch>,
}
#[derive(Clone, Debug)]
pub struct OwnedPeriodicChartFanPatches {
    owner: Arc<OwnedPeriodicChartLoopWinding>,
    limits: PeriodicChartFanPatchLimits,
    data: Data,
}
fn alternating(a: [usize; 2], b: [usize; 2]) -> bool {
    let inside = |p| {
        if a[0] < a[1] {
            a[0] < p && p < a[1]
        } else {
            p > a[0] || p < a[1]
        }
    };
    inside(b[0]) != inside(b[1])
}
fn derive(o: &OwnedPeriodicChartFanPatches) -> Result<Data> {
    let _replay = super::periodic_replay::scope();
    if !(1..=1_000_000).contains(&o.limits.max_relation_work) {
        return Err(refused("Invalid complete fan patch relation allowance"));
    }
    o.owner.reverify_source()?;
    let loops = o.owner.owner();
    let cycles = loops.owner();
    let n = loops.loops().len();
    let square = n
        .checked_mul(n)
        .ok_or_else(|| refused("Fan patch relation overflow"))?;
    let mut work = square
        .checked_mul(n)
        .and_then(|x| x.checked_mul(2))
        .and_then(|x| square.checked_mul(10).and_then(|y| x.checked_add(y)))
        .and_then(|x| x.checked_add(cycles.edges().len().checked_mul(2)?))
        .ok_or_else(|| refused("Fan patch relation overflow"))?;
    if work > o.limits.max_relation_work {
        return Err(refused(
            "Complete fan patch relation work exceeds allowance",
        ));
    }
    let mut turns: BTreeMap<usize, Vec<(usize, [usize; 2])>> = BTreeMap::new();
    let mut positions = BTreeMap::new();
    for (v, rotation) in cycles.rotations().iter().enumerate() {
        for (rank, &port) in rotation.iter().enumerate() {
            positions.insert((v, port), rank);
        }
    }
    for (id, boundary) in loops.loops().iter().enumerate() {
        for (j, &outgoing) in boundary.edges.iter().enumerate() {
            let incoming = boundary.edges[(j + boundary.edges.len() - 1) % boundary.edges.len()];
            let v = cycles.edges()[outgoing].vertices[0];
            let rank = |edge, end| {
                positions
                    .get(&(v, [edge, end]))
                    .copied()
                    .ok_or_else(|| refused("Loop contact lost its original high-germ port"))
            };
            turns
                .entry(v)
                .or_default()
                .push((id, [rank(incoming, 1)?, rank(outgoing, 0)?]));
        }
    }
    for at in turns.values() {
        for i in 0..at.len() {
            for j in i + 1..at.len() {
                work = work
                    .checked_add(1)
                    .filter(|x| *x <= o.limits.max_relation_work)
                    .ok_or_else(|| refused("Complete contact fan pair census exceeds allowance"))?;
                if at[i].0 == at[j].0 || alternating(at[i].1, at[j].1) {
                    return Err(refused(
                        "Decomposed loop contact crosses an original material fan",
                    ));
                }
            }
        }
    }
    let walks = loops
        .loops()
        .iter()
        .map(|b| b.edges.clone())
        .collect::<Vec<_>>();
    let sides =
        super::periodic_chart_cycle_sides::derive_sides(cycles, &walks, o.limits.sides, |p| {
            o.owner.windings(p)
        })?;
    let positive = sides
        .iter()
        .map(|s| s.orientation == PeriodicChartCycleOrientation::Positive)
        .collect::<Vec<_>>();
    let mut contains = vec![vec![false; n]; n];
    // Original embedding excludes contacts in open edges; complete exact port
    // rotations exclude alternating loop germs at every shared vertex. Each
    // vertex-simple graph loop is thus Jordan, and pairs can touch but cannot
    // cross. A loop's side relative to another loop cannot change at a contact.
    // The source-qualified transverse witness crosses only its OWN edge, so
    // its winding in every other loop represents that entire boundary's side.
    for i in 0..n {
        for j in 0..n {
            if i != j {
                let value = sides[j].windings[0][i];
                if value != 0 && value != if positive[i] { 1 } else { -1 } {
                    return Err(refused(
                        "Noncrossing Jordan loop has inconsistent winding orientation",
                    ));
                }
                contains[i][j] = value != 0;
            }
        }
    }
    let (parents, patches) =
        super::periodic_chart_strict_patches::hierarchy_parts(contains.clone(), &positive)?;
    Ok(Data {
        sides,
        contains,
        parents,
        patches,
    })
}
impl OwnedPeriodicChartFanPatches {
    pub fn owner(&self) -> &Arc<OwnedPeriodicChartLoopWinding> {
        &self.owner
    }
    pub fn sides(&self) -> &[OriginalPeriodicChartCycleSides] {
        &self.data.sides
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
            return Err(refused(
                "Original fan patch contact or containment ownership changed",
            ));
        }
        Ok(())
    }
}
pub fn own_periodic_chart_fan_patches(
    owner: Arc<OwnedPeriodicChartLoopWinding>,
    limits: PeriodicChartFanPatchLimits,
) -> Result<OwnedPeriodicChartFanPatches> {
    let mut o = OwnedPeriodicChartFanPatches {
        owner,
        limits,
        data: Data {
            sides: vec![],
            contains: vec![],
            parents: vec![],
            patches: vec![],
        },
    };
    o.data = derive(&o)?;
    Ok(o)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_cyclic_port_alternation_distinguishes_crossing_nested_and_separate_fans() {
        assert!(alternating([0, 2], [1, 3]));
        assert!(alternating([2, 0], [1, 3]));
        assert!(!alternating([0, 3], [1, 2]));
        assert!(!alternating([3, 0], [1, 2]));
        assert!(!alternating([0, 1], [2, 3]));
    }
    fn build(w: OwnedPeriodicChartWinding) -> OwnedPeriodicChartFanPatches {
        let loops = Arc::new(
            own_periodic_chart_boundary_loops(w.owner().clone(), Default::default()).unwrap(),
        );
        let winding = Arc::new(own_periodic_chart_loop_winding(loops, Default::default()).unwrap());
        own_periodic_chart_fan_patches(winding, Default::default()).unwrap()
    }
    #[test]
    fn actual_touching_material_fans_publish_two_patches_with_one_owned_contact() {
        let o = build(
            super::super::periodic_chart_cycles::actual_fan_tests::build(
                super::super::periodic_support_regions::tests::discs(true),
                2,
            ),
        );
        assert_eq!(o.patches().len(), 2);
        assert!(o.patches().iter().all(|p| p.holes.is_empty()));
        assert_eq!(o.owner().owner().contacts().len(), 1);
        o.reverify_source().unwrap();
        let mut bad = o.clone();
        bad.data.parents[0] = Some(0);
        assert!(bad.reverify_source().is_err());
    }
    #[test]
    fn actual_tangent_hole_retains_one_patch_negative_hole_and_original_contact() {
        let o = build(
            super::super::periodic_chart_cycles::actual_fan_tests::build(
                super::super::periodic_support_material::tests::tangent_hole(),
                1,
            ),
        );
        assert_eq!(o.patches().len(), 1);
        assert_eq!(o.patches()[0].holes.len(), 1);
        let p = &o.patches()[0];
        assert_eq!(o.parents()[p.holes[0]], Some(p.outer));
        assert_eq!(
            o.sides()[p.outer].orientation,
            PeriodicChartCycleOrientation::Positive
        );
        assert_eq!(
            o.sides()[p.holes[0]].orientation,
            PeriodicChartCycleOrientation::Negative
        );
        assert_eq!(o.owner().owner().contacts().len(), 1);
        o.reverify_source().unwrap();
    }
}
