//! Vertex-link cycles of immutable closed source incidence.
use crate::source_shell_incidence::Shell;
use nurbs_core::{Error, Result};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Node {
    pub edge: usize,
    pub end: usize,
}
pub struct VertexLink {
    pub vertex: usize,
    pub cycle: Vec<Node>,
}
pub struct Report {
    pub all_manifold: bool,
    pub corners: usize,
    pub links: Vec<VertexLink>,
    pub euler_characteristic: Option<i64>,
    pub genus: Option<usize>,
    pub uncertain_vertex: Option<usize>,
    pub reason: &'static str,
}
fn cycle(graph: &BTreeMap<Node, Vec<Node>>) -> Option<Vec<Node>> {
    if graph.is_empty() || graph.values().any(|n| n.len() != 2) {
        return None;
    }
    let first = *graph.first_key_value()?.0;
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    let mut previous = None;
    let mut current = first;
    loop {
        if !seen.insert(current) {
            return (current == first && seen.len() == graph.len()).then_some(out);
        }
        out.push(current);
        let neighbors = graph.get(&current)?;
        let next = if previous == Some(neighbors[0]) {
            neighbors[1]
        } else {
            neighbors[0]
        };
        previous = Some(current);
        current = next;
    }
}
pub fn inspect(shell: &Shell, max_corners: usize) -> Result<Report> {
    if !(1..=100000).contains(&max_corners) {
        return Err(Error::new(
            "BREP_SOURCE_VERTEX_LINK",
            "Choose bounded source corner work",
        ));
    }
    let mut out = Report {
        all_manifold: false,
        corners: 0,
        links: Vec::new(),
        euler_characteristic: None,
        genus: None,
        uncertain_vertex: None,
        reason: "source-vertex-incidence-unproven",
    };
    let mut uses = BTreeMap::new();
    let mut graphs = BTreeMap::<usize, BTreeMap<Node, Vec<Node>>>::new();
    for (edge, pair) in shell.uses().iter().enumerate() {
        for (index, a) in pair.iter().enumerate() {
            uses.insert((a.face, a.wire, a.edge), (edge, index));
        }
        for end in 0..2 {
            let a = pair[0];
            let b = pair[1];
            if shell.vertices()[a.face][a.wire][a.edge][end]
                != shell.vertices()[b.face][b.wire][b.edge][1 - end]
            {
                return Ok(out);
            }
        }
    }
    let poles = shell
        .poles()
        .iter()
        .map(|(a, _)| (a.face, a.wire, a.edge))
        .collect::<BTreeSet<_>>();
    for (face, wires) in shell.faces().iter().enumerate() {
        for (wire, w) in wires.iter().enumerate() {
            if (0..w.edges().len()).all(|edge| poles.contains(&(face, wire, edge))) {
                out.reason = "source-vertex-link-collapsed-wire";
                return Ok(out);
            }
            for edge in 0..w.edges().len() {
                if poles.contains(&(face, wire, edge)) {
                    if out.corners == max_corners {
                        out.reason = "source-vertex-link-work-limit";
                        return Ok(out);
                    }
                    out.corners += 1;
                    if shell.vertices()[face][wire][edge][0]
                        != shell.vertices()[face][wire][edge][1]
                    {
                        return Ok(out);
                    }
                    continue;
                }
                let vertex = shell.vertices()[face][wire][edge][0];
                out.uncertain_vertex = Some(vertex);
                if out.corners == max_corners {
                    out.reason = "source-vertex-link-work-limit";
                    return Ok(out);
                }
                out.corners += 1;
                let mut prev = (edge + w.edges().len() - 1) % w.edges().len();
                while poles.contains(&(face, wire, prev)) {
                    prev = (prev + w.edges().len() - 1) % w.edges().len();
                }
                if shell.vertices()[face][wire][prev][1] != vertex {
                    return Ok(out);
                }
                let node = |local, end| {
                    uses.get(&(face, wire, local)).map(|&(global, index)| Node {
                        edge: global,
                        end: if index == 0 { end } else { 1 - end },
                    })
                };
                let (Some(a), Some(b)) = (node(prev, 1), node(edge, 0)) else {
                    return Ok(out);
                };
                let graph = graphs.entry(vertex).or_default();
                graph.entry(a).or_default().push(b);
                graph.entry(b).or_default().push(a);
            }
        }
    }
    for (vertex, graph) in graphs {
        out.uncertain_vertex = Some(vertex);
        let Some(cycle) = cycle(&graph) else {
            out.reason = "source-vertex-link-not-one-cycle";
            return Ok(out);
        };
        out.links.push(VertexLink { vertex, cycle });
    }
    if shell.regions().is_some() {
        // One bounded material region per face: disk minus its retained holes.
        let faces: i64 = shell.faces().iter().map(|f| 2 - f.len() as i64).sum();
        let chi = out.links.len() as i64 - shell.edges().len() as i64 + faces;
        out.euler_characteristic = Some(chi);
        if chi > 2 || (2 - chi) % 2 != 0 {
            out.reason = "source-shell-euler-inconsistent";
            return Ok(out);
        }
        out.genus = Some(((2 - chi) / 2) as usize);
    }
    out.uncertain_vertex = None;
    out.all_manifold = true;
    out.reason = "source-vertex-links-qualified";
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn link_requires_one_cycle_and_preserves_parallel_edges() {
        let n = |edge| Node { edge, end: 0 };
        let graph = BTreeMap::from([(n(0), vec![n(1), n(1)]), (n(1), vec![n(0), n(0)])]);
        assert_eq!(cycle(&graph).unwrap().len(), 2);
        let mut disconnected = graph.clone();
        disconnected.insert(n(2), vec![n(3), n(3)]);
        disconnected.insert(n(3), vec![n(2), n(2)]);
        assert!(cycle(&disconnected).is_none());
        let mut open = graph.clone();
        open.get_mut(&n(0)).unwrap().pop();
        assert!(cycle(&open).is_none());
        let mut branched = graph;
        branched.get_mut(&n(0)).unwrap().push(n(1));
        assert!(cycle(&branched).is_none());
    }
}
