//! Nonzero material boundary selection on the unified common/single-source graph.
use super::*;
#[derive(Clone, Debug)]
pub struct OwnedOriginalUnifiedFilled {
    cycles: Arc<OwnedOriginalUnifiedCycles>,
    sides: Vec<OwnedOriginalUnifiedSides>,
    coefficients: Vec<i32>,
    winding: Vec<[i64; 2]>,
    retained: Vec<usize>,
    successor: Vec<Option<usize>>,
    boundaries: Vec<Vec<usize>>,
}
type Data = (
    Vec<[i64; 2]>,
    Vec<usize>,
    Vec<Option<usize>>,
    Vec<Vec<usize>>,
);
fn derive(
    cycles: &OwnedOriginalUnifiedCycles,
    sides: &[OwnedOriginalUnifiedSides],
    coefficients: &[i32],
) -> Result<Data> {
    cycles.reverify_source()?;
    let rotation = cycles.owner();
    let ports = rotation.owner();
    let graph = ports.graph();
    if sides.len() != graph.edges().len()
        || coefficients.iter().any(|&v| i64::from(v).abs() > 1_000_000)
        || (sides.is_empty() && !coefficients.is_empty())
    {
        return Err(refused(
            "Unified material selection needs every original edge side and bounded wire coefficients",
        ));
    }
    let mut winding = Vec::new();
    for (edge, side) in sides.iter().enumerate() {
        if !Arc::ptr_eq(side.graph(), graph)
            || side.edge() != edge
            || side.loops().len() != coefficients.len()
            || side.loops() != sides[0].loops()
        {
            return Err(refused(
                "Unified material side proof source or graph context changed",
            ));
        }
        side.reverify_source()?;
        let weighted = |values: &[i64]| {
            values
                .iter()
                .zip(coefficients)
                .try_fold(0i64, |sum, (&w, &c)| {
                    sum.checked_add(
                        w.checked_mul(i64::from(c))
                            .ok_or_else(|| refused("Unified material winding product overflow"))?,
                    )
                    .ok_or_else(|| refused("Unified material winding total overflow"))
                })
        };
        winding.push([
            weighted(&side.certificates()[0].windings)?,
            weighted(&side.certificates()[1].windings)?,
        ]);
    }
    let left = |h: usize| {
        let p = &ports.ports()[h];
        winding[p.edge][p.endpoint]
    };
    let right = |h: usize| {
        let p = &ports.ports()[h];
        winding[p.edge][1 - p.endpoint]
    };
    for r in rotation.rotations() {
        for i in 0..r.len() {
            if left(r[i]) != right(r[(i + 1) % r.len()]) {
                return Err(refused(
                    "Unified exact vertex sectors disagree on material winding",
                ));
            }
        }
    }
    let count = ports.ports().len();
    let retained = (0..count)
        .filter(|&h| left(h) != 0 && right(h) == 0)
        .collect::<Vec<_>>();
    let mut selected = vec![false; count];
    for &h in &retained {
        selected[h] = true;
    }
    let mut endpoint_ports = vec![[usize::MAX; 2]; graph.edges().len()];
    for (h, p) in ports.ports().iter().enumerate() {
        endpoint_ports[p.edge][p.endpoint] = h;
    }
    let mut successor = vec![None; count];
    let mut indegree = vec![0usize; count];
    for &h in &retained {
        let p = &ports.ports()[h];
        let target = graph.edges()[p.edge].vertices[1 - p.endpoint];
        let r = &rotation.rotations()[target];
        let twin = endpoint_ports[p.edge][1 - p.endpoint];
        let at = r
            .iter()
            .position(|&v| v == twin)
            .ok_or_else(|| refused("Unified filled incoming twin is missing"))?;
        let next = (1..=r.len())
            .map(|step| r[(at + r.len() - step) % r.len()])
            .find(|&v| selected[v])
            .ok_or_else(|| refused("Unified filled boundary has no outgoing material branch"))?;
        successor[h] = Some(next);
        indegree[next] += 1;
    }
    if (0..count).any(|h| indegree[h] != usize::from(selected[h])) {
        return Err(refused(
            "Unified filled successor is not a complete selected permutation",
        ));
    }
    let mut visited = vec![false; count];
    let mut boundaries = Vec::new();
    for &seed in &retained {
        if visited[seed] {
            continue;
        }
        let mut h = seed;
        let mut boundary = Vec::new();
        while !visited[h] {
            visited[h] = true;
            boundary.push(h);
            h = successor[h].ok_or_else(|| refused("Unified filled successor is missing"))?;
        }
        if h != seed {
            return Err(refused("Unified filled boundary joins a different orbit"));
        }
        boundaries.push(boundary);
    }
    Ok((winding, retained, successor, boundaries))
}
impl OwnedOriginalUnifiedFilled {
    pub fn cycles(&self) -> &Arc<OwnedOriginalUnifiedCycles> {
        &self.cycles
    }
    pub fn sides(&self) -> &[OwnedOriginalUnifiedSides] {
        &self.sides
    }
    pub fn coefficients(&self) -> &[i32] {
        &self.coefficients
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
    /// Material lies on the left. Hole/outer containment remains a separate proof.
    pub fn boundaries(&self) -> &[Vec<usize>] {
        &self.boundaries
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (w, r, s, b) = derive(&self.cycles, &self.sides, &self.coefficients)?;
        if w != self.winding || r != self.retained || s != self.successor || b != self.boundaries {
            return Err(refused(
                "Unified original material boundaries or winding changed",
            ));
        }
        Ok(())
    }
}
pub fn own_original_unified_filled(
    cycles: Arc<OwnedOriginalUnifiedCycles>,
    sides: Vec<OwnedOriginalUnifiedSides>,
    coefficients: &[i32],
) -> Result<OwnedOriginalUnifiedFilled> {
    let (winding, retained, successor, boundaries) = derive(&cycles, &sides, coefficients)?;
    Ok(OwnedOriginalUnifiedFilled {
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
    fn replay_rejects_winding_boundary_and_selected_successor_tampering() {
        let a = Curve::from_polyline(vec![
            vec![0., 0.],
            vec![1., 0.],
            vec![1., 1.],
            vec![0., 1.],
            vec![0., 0.],
        ])
        .unwrap();
        let loops = vec![vec![a.clone()]];
        let d = Arc::new(
            own_original_common_dictionary(&[a], 1e-12, ParameterOrderLimits::default()).unwrap(),
        );
        let c = Arc::new(own_original_source_carriers(d, ParameterOrderLimits::default()).unwrap());
        let g = Arc::new(own_original_unified_graph(c).unwrap());
        let p = Arc::new(own_original_unified_ports(g.clone(), 192).unwrap());
        let r =
            Arc::new(own_original_unified_rotation(p, ContactPortOrderLimits::default()).unwrap());
        let cycles = Arc::new(own_original_unified_cycles(r).unwrap());
        let sides = own_original_unified_sides_automatically(
            g,
            &loops,
            ParameterOrderLimits::default(),
            32,
        )
        .unwrap();
        let x = own_original_unified_filled(cycles, sides, &[1]).unwrap();
        x.reverify_source().unwrap();
        let mut bad = x.clone();
        bad.winding[0][0] += 1;
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.retained.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.boundaries[0].pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.successor[x.retained[0]] = None;
        assert!(bad.reverify_source().is_err());
    }
}
