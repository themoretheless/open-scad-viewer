//! Source-owned half-edge permutations using exact contact rotations.
use super::*;
use std::sync::Arc;
#[derive(Clone, Debug)]
pub struct OwnedOriginalCarrierCycles {
    graph: Arc<OwnedOriginalCarrierGraph>,
    limits: ContactPortOrderLimits,
    rotations: Vec<Vec<usize>>,
    successor: Vec<usize>,
    cycles: Vec<Vec<usize>>,
}
fn halfedge(p: OriginalCarrierIncidence) -> usize {
    2 * p.carrier + usize::from(p.direction < 0)
}
type CycleData = (Vec<Vec<usize>>, Vec<usize>, Vec<Vec<usize>>);
fn derive(graph: &OwnedOriginalCarrierGraph, limits: ContactPortOrderLimits) -> Result<CycleData> {
    graph.reverify_source()?;
    let mut rotations = Vec::new();
    for (vertex, incidence) in graph.vertices().iter().zip(graph.incidence()) {
        let rotation = match vertex {
            OriginalCarrierVertex::Contact(group) => {
                let order =
                    own_original_contact_port_order(graph.owner().atlas().clone(), *group, limits)?;
                let mut rotation = Vec::new();
                for port in order.ports() {
                    let matches=incidence.iter().filter(|p| {
                        if p.direction!=port.direction {return false;}
                        let carrier=&graph.owner().carriers()[p.carrier];
                        if carrier.source!=port.source {return false;}
                        let cut=&carrier.cuts[usize::from(p.direction<0)];
                        matches!(cut,OriginalCarrierCut::Contact{parameter_group,point_group} if *parameter_group==port.parameter_group && point_group==group)
                    }).copied().collect::<Vec<_>>();
                    if matches.len() != 1 {
                        return Err(refused(
                            "Contact rotation is not bijective with original carrier ends",
                        ));
                    }
                    rotation.push(halfedge(matches[0]));
                }
                if rotation.len() != incidence.len() {
                    return Err(refused(
                        "Original endpoint branch is missing from contact rotation",
                    ));
                }
                rotation
            }
            OriginalCarrierVertex::RationalEndpoint(_) => {
                if incidence.len() > 2 {
                    return Err(refused(
                        "Original endpoint needs a complete geometric branch rotation",
                    ));
                }
                incidence.iter().copied().map(halfedge).collect()
            }
        };
        rotations.push(rotation);
    }
    let n = 2 * graph.ends().len();
    let mut successor = vec![usize::MAX; n];
    for h in 0..n {
        let target = graph.ends()[h / 2][if h % 2 == 0 { 1 } else { 0 }];
        let rotation = &rotations[target];
        let twin = h ^ 1;
        let position = rotation
            .iter()
            .position(|p| *p == twin)
            .ok_or_else(|| refused("Original half-edge has no target twin"))?;
        successor[h] = rotation[(position + rotation.len() - 1) % rotation.len()];
    }
    let mut indegree = vec![0; n];
    for next in &successor {
        indegree[*next] += 1;
    }
    if indegree.iter().any(|d| *d != 1) {
        return Err(refused("Original face successor is not a permutation"));
    }
    let mut visited = vec![false; n];
    let mut cycles = Vec::new();
    for start in 0..n {
        if visited[start] {
            continue;
        }
        let mut cycle = Vec::new();
        let mut h = start;
        while !visited[h] {
            visited[h] = true;
            cycle.push(h);
            h = successor[h];
        }
        if h != start {
            return Err(refused("Original face cycle joins an earlier orbit"));
        }
        cycles.push(cycle);
    }
    Ok((rotations, successor, cycles))
}
impl OwnedOriginalCarrierCycles {
    pub fn graph(&self) -> &Arc<OwnedOriginalCarrierGraph> {
        &self.graph
    }
    /// Half-edge 2*k follows original carrier k, 2*k+1 reverses it.
    pub fn rotations(&self) -> &[Vec<usize>] {
        &self.rotations
    }
    pub fn successor(&self) -> &[usize] {
        &self.successor
    }
    /// Oriented boundary walks; disconnected face components are not grouped.
    pub fn cycles(&self) -> &[Vec<usize>] {
        &self.cycles
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (rotations, successor, cycles) = derive(&self.graph, self.limits)?;
        if rotations != self.rotations || successor != self.successor || cycles != self.cycles {
            return Err(refused(
                "Original contact rotations or boundary walks changed",
            ));
        }
        Ok(())
    }
}
/// Construct left-side boundary walks from exact original contact rotations.
/// Walks retain bridges and both loop sides. Winding, holes and filled-face
/// selection require further proof; these walks alone never admit material.
pub fn own_original_carrier_cycles(
    graph: Arc<OwnedOriginalCarrierGraph>,
    limits: ContactPortOrderLimits,
) -> Result<OwnedOriginalCarrierCycles> {
    let (rotations, successor, cycles) = derive(&graph, limits)?;
    Ok(OwnedOriginalCarrierCycles {
        graph,
        limits,
        rotations,
        successor,
        cycles,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn boundary_walk_replay_rejects_permutation_and_rotation_tampering() {
        let source = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
        let a = Arc::new(
            assemble_original_contact_atlas(&[source], 1e-12, ContactAtlasLimits::default())
                .unwrap(),
        );
        let carriers = Arc::new(own_original_carriers(a).unwrap());
        let graph = Arc::new(own_original_carrier_graph(carriers).unwrap());
        let c = own_original_carrier_cycles(graph, ContactPortOrderLimits::default()).unwrap();
        c.reverify_source().unwrap();
        let mut bad = c.clone();
        bad.successor[0] = 0;
        assert!(bad.reverify_source().is_err());
        let mut bad = c.clone();
        bad.rotations[0].clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = c;
        bad.cycles[0].reverse();
        assert!(bad.reverify_source().is_err());
    }
}
