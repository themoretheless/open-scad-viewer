//! Every directed physical quotient dart in exactly one left-successor orbit.
use super::*;
use std::sync::Arc;
#[derive(Clone, Debug)]
pub struct OwnedPeriodicCycles {
    owner: Arc<OwnedPeriodicRotation>,
    max_darts: usize,
    successor: Vec<usize>,
    cycles: Vec<Vec<usize>>,
    dart_cycles: Vec<usize>,
}
fn derive(o: &OwnedPeriodicCycles) -> Result<(Vec<usize>, Vec<Vec<usize>>, Vec<usize>)> {
    let n = o.owner.owner().ports().len();
    if o.max_darts == 0 || o.max_darts > 1000000 || n > o.max_darts {
        return Err(refused(
            "Complete physical quotient dart census exceeds allowance",
        ));
    }
    super::periodic_replay::verify_arc(&o.owner, |x| x.reverify_source())?;
    let ports = o.owner.owner().ports();
    let rows = o.owner.rotations();
    let vertices = o.owner.owner().vertices();
    let graph = o.owner.owner().owner().owner();
    if rows.len() != vertices.len() {
        return Err(refused("Physical quotient rotation omits vertices"));
    }
    let mut positions = vec![None; n];
    for (vertex, row) in rows.iter().enumerate() {
        if row.len() != vertices[vertex].len() {
            return Err(refused(
                "Physical quotient rotation omits vertex incidences",
            ));
        }
        for (position, &dart) in row.iter().enumerate() {
            let p = ports
                .get(dart)
                .ok_or_else(|| refused("Physical cyclic order names an unknown dart"))?;
            if p.vertex != vertex || positions[dart].replace((vertex, position)).is_some() {
                return Err(refused(
                    "Physical cyclic order duplicates or misplaces an endpoint dart",
                ));
            }
        }
    }
    if positions.iter().any(Option::is_none) {
        return Err(refused("Physical cyclic order has missing endpoint darts"));
    }
    let mut successor = Vec::new();
    let mut incoming = vec![0usize; n];
    for (dart, p) in ports.iter().enumerate() {
        if p.end > 1 || p.edge.checked_mul(2).and_then(|v| v.checked_add(p.end)) != Some(dart) {
            return Err(refused(
                "Physical dart index does not match actual edge endpoint",
            ));
        }
        let twin = p.edge * 2 + (1 - p.end);
        let opposite = ports
            .get(twin)
            .ok_or_else(|| refused("Physical edge has no opposite endpoint dart"))?;
        if opposite.edge != p.edge || opposite.end == p.end {
            return Err(refused(
                "Physical edge twin is not its distinct opposite endpoint",
            ));
        }
        let (vertex, position) = positions[twin]
            .ok_or_else(|| refused("Incoming physical twin has no cyclic position"))?;
        let row = &rows[vertex];
        if row.is_empty() {
            return Err(refused(
                "Incoming physical twin has an empty vertex rotation",
            ));
        }
        let next = row[(position + row.len() - 1) % row.len()];
        let edge = graph
            .edges()
            .get(p.edge)
            .ok_or_else(|| refused("Physical successor lost its original quotient edge"))?;
        if ports[next].vertex != edge.vertices[1 - p.end] {
            return Err(refused(
                "Physical left-successor leaves the actual arrival vertex",
            ));
        }
        successor.push(next);
        incoming[next] += 1;
    }
    if incoming.iter().any(|&v| v != 1) {
        return Err(refused(
            "Physical left-successor map is not a complete dart permutation",
        ));
    }
    let mut dart_cycles = vec![usize::MAX; n];
    let mut cycles = Vec::new();
    for start in 0..n {
        if dart_cycles[start] != usize::MAX {
            continue;
        }
        let mut orbit = Vec::new();
        let mut current = start;
        while dart_cycles[current] == usize::MAX {
            dart_cycles[current] = cycles.len();
            orbit.push(current);
            current = successor[current];
        }
        if current != start || orbit.is_empty() {
            return Err(refused(
                "Physical left-successor walk does not close at its original dart",
            ));
        }
        cycles.push(orbit);
    }
    Ok((successor, cycles, dart_cycles))
}
impl OwnedPeriodicCycles {
    pub fn owner(&self) -> &Arc<OwnedPeriodicRotation> {
        &self.owner
    }
    pub fn successor(&self) -> &[usize] {
        &self.successor
    }
    /// Complete directed boundary WALK orbits, not a claim of material faces.
    /// A walk may repeat a vertex or contain both directions of a bridge.
    pub fn cycles(&self) -> &[Vec<usize>] {
        &self.cycles
    }
    pub fn dart_cycles(&self) -> &[usize] {
        &self.dart_cycles
    }
    pub fn reverify_source(&self) -> Result<()> {
        let _replay = super::periodic_replay::scope();
        let (s, c, d) = derive(self)?;
        if s != self.successor || c != self.cycles || d != self.dart_cycles {
            return Err(refused(
                "Original quotient successor permutation or boundary walk changed",
            ));
        }
        Ok(())
    }
}
pub fn own_periodic_cycles(
    owner: Arc<OwnedPeriodicRotation>,
    max_darts: usize,
) -> Result<OwnedPeriodicCycles> {
    let _replay = super::periodic_replay::scope();
    let mut o = OwnedPeriodicCycles {
        owner,
        max_darts,
        successor: Vec::new(),
        cycles: Vec::new(),
        dart_cycles: Vec::new(),
    };
    let (s, c, d) = derive(&o)?;
    o.successor = s;
    o.cycles = c;
    o.dart_cycles = d;
    Ok(o)
}
#[cfg(test)]
mod tests {
    use super::*;
    use periodic_rotation::tests::ports;
    #[test]
    fn a_physical_loop_has_two_single_dart_orbits_not_one_collapsed_endpoint() {
        let a = Curve::from_polyline(vec![vec![0., 0.5], vec![4., 0.5]]).unwrap();
        let rotation = Arc::new(
            own_periodic_rotation(ports(&[a]), ContactPortOrderLimits::default()).unwrap(),
        );
        assert!(own_periodic_cycles(rotation.clone(), 1).is_err());
        let o = own_periodic_cycles(rotation, 16).unwrap();
        o.reverify_source().unwrap();
        assert_eq!(o.successor(), &[0, 1]);
        assert_eq!(o.cycles(), &[vec![0], vec![1]]);
        let mut bad = o.clone();
        bad.successor[0] = 1;
        assert!(bad.reverify_source().is_err());
        let mut bad = o.clone();
        bad.dart_cycles[0] = 99;
        assert!(bad.reverify_source().is_err());
        let mut bad = o;
        bad.cycles.swap(0, 1);
        assert!(bad.reverify_source().is_err());
    }
    #[test]
    fn tangent_seam_boundary_walks_cover_each_original_directed_edge_once() {
        let a = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 2., 2., 2.],
            control_points: vec![
                vec![0., 0.5],
                vec![0., 0.75],
                vec![1., 0.75],
                vec![0.5, 0.625],
                vec![0., 0.5],
            ],
            weights: vec![1.; 5],
            periodic: false,
        };
        let mut b = a.clone();
        for p in &mut b.control_points {
            p[0] = 4. - p[0];
        }
        let rotation = Arc::new(
            own_periodic_rotation(ports(&[a, b]), ContactPortOrderLimits::default()).unwrap(),
        );
        let o = own_periodic_cycles(rotation, 64).unwrap();
        o.reverify_source().unwrap();
        assert_eq!(o.cycles(), &[vec![0, 2, 7, 5], vec![1, 3], vec![4, 6]]);
        let mut all = o.cycles().iter().flatten().copied().collect::<Vec<_>>();
        all.sort_unstable();
        assert_eq!(all, (0..8).collect::<Vec<_>>());
        let mut bad = o;
        bad.cycles[0].pop();
        assert!(bad.reverify_source().is_err());
    }
}
