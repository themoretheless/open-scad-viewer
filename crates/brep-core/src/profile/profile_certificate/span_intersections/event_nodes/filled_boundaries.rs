//! Nonzero material boundaries from exact source-side winding and rotations.
use super::*;
use std::sync::Arc;
#[derive(Clone, Debug)]
pub struct OwnedOriginalFilledBoundaries {
    cycles: Arc<OwnedOriginalCarrierCycles>,
    sides: Vec<OwnedOriginalCarrierSides>,
    coefficients: Vec<i32>,
    winding: Vec<[i64; 2]>,
    retained: Vec<usize>,
    successor: Vec<Option<usize>>,
    boundaries: Vec<Vec<usize>>,
}
type BoundaryData = (
    Vec<[i64; 2]>,
    Vec<usize>,
    Vec<Option<usize>>,
    Vec<Vec<usize>>,
);
fn derive(
    cycles: &OwnedOriginalCarrierCycles,
    sides: &[OwnedOriginalCarrierSides],
    coefficients: &[i32],
) -> Result<BoundaryData> {
    cycles.reverify_source()?;
    let graph = cycles.graph();
    let owner = graph.owner();
    if sides.len() != owner.carriers().len()
        || coefficients.iter().any(|c| i64::from(*c).abs() > 1_000_000)
    {
        return Err(refused(
            "Filled boundaries need the complete bounded original side census",
        ));
    }
    let mut winding = Vec::new();
    for (carrier, side) in sides.iter().enumerate() {
        if !Arc::ptr_eq(side.owner(), owner)
            || side.carrier() != carrier
            || side.loops().len() != coefficients.len()
            || side.loops() != sides[0].loops()
        {
            return Err(refused(
                "Filled boundary side ownership or original wire grouping differs",
            ));
        }
        side.reverify_source()?;
        let wire = side
            .loops()
            .iter()
            .enumerate()
            .flat_map(|(w, c)| c.iter().map(move |_| w))
            .nth(owner.carriers()[carrier].source)
            .ok_or_else(|| refused("Filled carrier has no original source wire"))?;
        let [left, right] = side.certificates();
        for w in 0..coefficients.len() {
            if left.windings[w].checked_sub(right.windings[w]) != Some(i64::from(w == wire)) {
                return Err(refused(
                    "Original carrier side census has an inconsistent winding jump",
                ));
            }
        }
        let weighted = |values: &[i64]| -> Result<i64> {
            values
                .iter()
                .zip(coefficients)
                .try_fold(0i64, |sum, (&w, &c)| {
                    sum.checked_add(
                        w.checked_mul(i64::from(c))
                            .ok_or_else(|| refused("Filled winding product overflow"))?,
                    )
                    .ok_or_else(|| refused("Filled winding total overflow"))
                })
        };
        winding.push([weighted(&left.windings)?, weighted(&right.windings)?]);
    }
    if sides.is_empty() && !coefficients.is_empty() {
        return Err(refused(
            "Empty carrier census cannot own nonempty wire coefficients",
        ));
    }
    let left = |h: usize| winding[h / 2][h % 2];
    let right = |h: usize| winding[h / 2][1 - h % 2];
    for rotation in cycles.rotations() {
        for i in 0..rotation.len() {
            if left(rotation[i]) != right(rotation[(i + 1) % rotation.len()]) {
                return Err(refused(
                    "Exact vertex sectors disagree with source-side winding labels",
                ));
            }
        }
    }
    let n = 2 * winding.len();
    let retained = (0..n)
        .filter(|h| left(*h) != 0 && right(*h) == 0)
        .collect::<Vec<_>>();
    let mut selected = vec![false; n];
    for h in &retained {
        selected[*h] = true;
    }
    let mut successor = vec![None; n];
    for &h in &retained {
        let target = graph.ends()[h / 2][if h % 2 == 0 { 1 } else { 0 }];
        let rotation = &cycles.rotations()[target];
        let twin = rotation
            .iter()
            .position(|p| *p == h ^ 1)
            .ok_or_else(|| refused("Filled incoming carrier twin is missing"))?;
        let next = (1..=rotation.len())
            .map(|step| rotation[(twin + rotation.len() - step) % rotation.len()])
            .find(|p| selected[*p])
            .ok_or_else(|| refused("Filled boundary has no outgoing material edge"))?;
        successor[h] = Some(next);
    }
    let mut indegree = vec![0; n];
    for h in &retained {
        indegree[successor[*h].unwrap()] += 1;
    }
    if (0..n).any(|h| indegree[h] != usize::from(selected[h])) {
        return Err(refused(
            "Filled boundary successor is not a complete selected permutation",
        ));
    }
    let mut visited = vec![false; n];
    let mut boundaries = Vec::new();
    for &start in &retained {
        if visited[start] {
            continue;
        }
        let mut boundary = Vec::new();
        let mut h = start;
        while !visited[h] {
            visited[h] = true;
            boundary.push(h);
            h = successor[h].unwrap();
        }
        if h != start {
            return Err(refused("Filled boundary walk joins a different orbit"));
        }
        boundaries.push(boundary);
    }
    Ok((winding, retained, successor, boundaries))
}
impl OwnedOriginalFilledBoundaries {
    pub(super) fn side_witnesses(&self)->&[OwnedOriginalCarrierSides]{&self.sides}
    pub fn cycles(&self) -> &Arc<OwnedOriginalCarrierCycles> {
        &self.cycles
    }
    pub fn winding(&self) -> &[[i64; 2]] {
        &self.winding
    }
    pub fn retained(&self) -> &[usize] {
        &self.retained
    }
    pub fn successor(&self) -> &[Option<usize>] {
        &self.successor
    }
    /// Material is on the left of every retained original half-edge. Outer
    /// boundaries and holes still require source-owned component grouping.
    pub fn boundaries(&self) -> &[Vec<usize>] {
        &self.boundaries
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (winding, retained, successor, boundaries) =
            derive(&self.cycles, &self.sides, &self.coefficients)?;
        if winding != self.winding
            || retained != self.retained
            || successor != self.successor
            || boundaries != self.boundaries
        {
            return Err(refused(
                "Original nonzero material boundary selection changed",
            ));
        }
        Ok(())
    }
}
/// Select exact original nonzero winding boundaries and bypass internal edges.
/// Every side receipt must own this exact carrier owner in carrier order. No
/// sampled polygon or unbound winding integer is accepted as region evidence.
pub fn own_original_filled_boundaries(
    cycles: Arc<OwnedOriginalCarrierCycles>,
    sides: Vec<OwnedOriginalCarrierSides>,
    coefficients: &[i32],
) -> Result<OwnedOriginalFilledBoundaries> {
    let (winding, retained, successor, boundaries) = derive(&cycles, &sides, coefficients)?;
    Ok(OwnedOriginalFilledBoundaries {
        cycles,
        sides,
        coefficients: coefficients.to_vec(),
        winding,
        retained,
        successor,
        boundaries,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filled_boundary_replay_rejects_winding_edges_successors_and_cycles() {
        let points = [[0., 0.], [1., 0.], [1., 1.], [0., 1.], [0., 0.]];
        let wire = points
            .windows(2)
            .map(|p| Curve::from_polyline(vec![p[0].to_vec(), p[1].to_vec()]).unwrap())
            .collect::<Vec<_>>();
        let loops = vec![wire.clone()];
        let atlas = Arc::new(
            assemble_original_contact_atlas(&wire, 1e-12, ContactAtlasLimits::default()).unwrap(),
        );
        let owner = Arc::new(own_original_carriers(atlas).unwrap());
        let graph = Arc::new(own_original_carrier_graph(owner.clone()).unwrap());
        let cycles = Arc::new(
            own_original_carrier_cycles(graph, ContactPortOrderLimits::default()).unwrap(),
        );
        let q = |a: &str, b: &str| [a.to_string(), b.to_string()];
        let sides = (0..4)
            .map(|carrier| {
                own_original_carrier_sides(
                    owner.clone(),
                    &loops,
                    carrier,
                    q("1", "2"),
                    q("1", "8"),
                    192,
                )
                .unwrap()
            })
            .collect();
        let region = own_original_filled_boundaries(cycles, sides, &[1]).unwrap();
        region.reverify_source().unwrap();
        let mut bad = region.clone();
        bad.winding[0][0] = 0;
        assert!(bad.reverify_source().is_err());
        let mut bad = region.clone();
        bad.retained.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = region.clone();
        bad.successor[0] = None;
        assert!(bad.reverify_source().is_err());
        let mut bad = region.clone();
        bad.boundaries[0].reverse();
        assert!(bad.reverify_source().is_err());
        let mut bad = region;
        bad.coefficients[0] = 0;
        assert!(bad.reverify_source().is_err());
    }
}
