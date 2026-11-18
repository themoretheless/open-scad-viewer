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
use super::super::filled_selection as selection;
const MESSAGES: selection::Messages = selection::Messages {
    product_overflow: "Unified material winding product overflow",
    total_overflow: "Unified material winding total overflow",
    sectors: "Unified exact vertex sectors disagree on material winding",
    missing_twin: "Unified filled incoming twin is missing",
    no_outgoing: "Unified filled boundary has no outgoing material branch",
    permutation: "Unified filled successor is not a complete selected permutation",
    missing_successor: "Unified filled successor is missing",
    orbit: "Unified filled boundary joins a different orbit",
};
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
        let weighted = |values: &[i64]| selection::weighted(values, coefficients, &MESSAGES);
        winding.push([
            weighted(&side.certificates()[0].windings)?,
            weighted(&side.certificates()[1].windings)?,
        ]);
    }
    let mut endpoint_ports = vec![[usize::MAX; 2]; graph.edges().len()];
    for (h, p) in ports.ports().iter().enumerate() {
        endpoint_ports[p.edge][p.endpoint] = h;
    }
    let (retained, successor, boundaries) = selection::Halfedges {
        count: ports.ports().len(),
        rotations: rotation.rotations(),
        side: |h: usize| {
            let p = &ports.ports()[h];
            [winding[p.edge][p.endpoint], winding[p.edge][1 - p.endpoint]]
        },
        arrive: |h: usize| {
            let p = &ports.ports()[h];
            (
                graph.edges()[p.edge].vertices[1 - p.endpoint],
                endpoint_ports[p.edge][1 - p.endpoint],
            )
        },
        messages: &MESSAGES,
    }
    .select()?;
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
