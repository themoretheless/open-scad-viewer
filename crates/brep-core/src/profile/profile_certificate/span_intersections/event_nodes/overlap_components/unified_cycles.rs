//! Complete directed-edge permutation and left-side walks of the original graph.
use super::*;
#[derive(Clone, Debug)]
pub struct OwnedOriginalUnifiedCycles {
    owner: Arc<OwnedOriginalUnifiedRotation>,
    successor: Vec<usize>,
    cycles: Vec<Vec<usize>>,
    dart_cycles: Vec<usize>,
}
fn derive(
    owner: &OwnedOriginalUnifiedRotation,
) -> Result<(Vec<usize>, Vec<Vec<usize>>, Vec<usize>)> {
    owner.reverify_source()?;
    let ports = owner.owner();
    let graph = ports.graph();
    let count = ports.ports().len();
    if count > 16384 {
        return Err(refused("Unified original boundary dart allowance exceeded"));
    }
    let mut lookup = vec![[usize::MAX; 2]; graph.edges().len()];
    for (dart, p) in ports.ports().iter().enumerate() {
        if p.edge >= lookup.len() || p.endpoint > 1 || lookup[p.edge][p.endpoint] != usize::MAX {
            return Err(refused("Unified original dart ownership is not bijective"));
        }
        lookup[p.edge][p.endpoint] = dart;
    }
    if lookup.iter().flatten().any(|&v| v == usize::MAX) {
        return Err(refused("Unified original edge lacks one endpoint dart"));
    }
    let mut previous = vec![usize::MAX; count];
    let mut visited = vec![false; count];
    for (vertex, rotation) in owner.rotations().iter().enumerate() {
        if rotation.is_empty() {
            return Err(refused(
                "Unified original graph vertex has no incident port",
            ));
        }
        for (i, &dart) in rotation.iter().enumerate() {
            if dart >= count || visited[dart] || ports.ports()[dart].vertex != vertex {
                return Err(refused("Unified original vertex rotation loses a dart"));
            }
            visited[dart] = true;
            previous[dart] = rotation[(i + rotation.len() - 1) % rotation.len()];
        }
    }
    if visited.iter().any(|v| !v) {
        return Err(refused("Unified original vertex rotations omit darts"));
    }
    let mut successor = Vec::new();
    let mut incoming = vec![0usize; count];
    for p in ports.ports() {
        let twin = lookup[p.edge][1 - p.endpoint];
        let next = previous[twin];
        if ports.ports()[next].vertex != graph.edges()[p.edge].vertices[1 - p.endpoint] {
            return Err(refused(
                "Unified original boundary successor leaves its actual vertex",
            ));
        }
        successor.push(next);
        incoming[next] += 1;
    }
    if incoming.iter().any(|&n| n != 1) {
        return Err(refused(
            "Unified original left-boundary successor is not a permutation",
        ));
    }
    let mut dart_cycles = vec![usize::MAX; count];
    let mut cycles = Vec::new();
    for seed in 0..count {
        if dart_cycles[seed] != usize::MAX {
            continue;
        }
        let mut current = seed;
        let mut cycle = Vec::new();
        loop {
            if dart_cycles[current] != usize::MAX {
                if current != seed {
                    return Err(refused("Unified original boundary enters another cycle"));
                }
                break;
            }
            dart_cycles[current] = cycles.len();
            cycle.push(current);
            current = successor[current];
            if cycle.len() > count {
                return Err(refused("Unified original boundary walk does not close"));
            }
        }
        cycles.push(cycle);
    }
    Ok((successor, cycles, dart_cycles))
}
impl OwnedOriginalUnifiedCycles {
    pub fn owner(&self) -> &Arc<OwnedOriginalUnifiedRotation> {
        &self.owner
    }
    /// Dart indices refer to the source-owned unified ports, not fitted edges.
    pub fn successor(&self) -> &[usize] {
        &self.successor
    }
    pub fn cycles(&self) -> &[Vec<usize>] {
        &self.cycles
    }
    pub fn dart_cycles(&self) -> &[usize] {
        &self.dart_cycles
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (s, c, d) = derive(&self.owner)?;
        if s != self.successor || c != self.cycles || d != self.dart_cycles {
            return Err(refused(
                "Unified original boundary cycles or directed-edge ownership changed",
            ));
        }
        Ok(())
    }
}
/// Complete geometric left-side boundary walks. These are not material faces:
/// dangling edges, multiple components and hole containment need winding labels.
pub fn own_original_unified_cycles(
    owner: Arc<OwnedOriginalUnifiedRotation>,
) -> Result<OwnedOriginalUnifiedCycles> {
    let (successor, cycles, dart_cycles) = derive(&owner)?;
    Ok(OwnedOriginalUnifiedCycles {
        owner,
        successor,
        cycles,
        dart_cycles,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_rejects_cycle_omission_successor_and_dart_owner_tampering() {
        let a = Curve::from_polyline(vec![
            vec![0., 0.],
            vec![1., 0.],
            vec![1., 1.],
            vec![0., 1.],
            vec![0., 0.],
        ])
        .unwrap();
        let d = Arc::new(
            own_original_common_dictionary(&[a], 1e-12, ParameterOrderLimits::default()).unwrap(),
        );
        let c = Arc::new(own_original_source_carriers(d, ParameterOrderLimits::default()).unwrap());
        let g = Arc::new(own_original_unified_graph(c).unwrap());
        let p = Arc::new(own_original_unified_ports(g, 192).unwrap());
        let r =
            Arc::new(own_original_unified_rotation(p, ContactPortOrderLimits::default()).unwrap());
        let x = own_original_unified_cycles(r).unwrap();
        x.reverify_source().unwrap();
        let mut bad = x.clone();
        bad.cycles.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.successor[0] = 0;
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.dart_cycles[0] = 99;
        assert!(bad.reverify_source().is_err());
    }
}
