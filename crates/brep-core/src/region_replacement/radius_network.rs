//! Bind independently prescribed radii to original shell edge/vertex roles.
use crate::{Model, TopoId};
use nurbs_core::{Error, Result};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeRadius {
    pub edge: usize,
    pub radius_mm: f64,
}
#[derive(Debug, PartialEq)]
pub struct RadiusEdge {
    pub edge: usize,
    pub identity: TopoId,
    pub radius_mm: f64,
    pub supports: [usize; 2],
    pub support_identities: [TopoId; 2],
}
#[derive(Debug, PartialEq)]
pub struct RadiusCorner {
    pub vertex: usize,
    pub identity: TopoId,
    pub selected: Vec<usize>,
    pub incident: Vec<usize>,
    pub unequal_radii: bool,
}
#[derive(Debug, PartialEq)]
pub struct RadiusNetwork {
    pub shell: usize,
    pub identity: TopoId,
    pub edges: Vec<RadiusEdge>,
    pub corners: Vec<RadiusCorner>,
    pub topology_visits: usize,
}
fn refuse(s: &str) -> Error {
    Error::new("BREP_RADIUS_NETWORK_INPUT", s)
}
/// Topology planning only: no assertion that radii fit, and no fillet surfaces.
/// Common model validation owns vertex-fan admission. Equality of radii is
/// exact caller intent; no tolerance merge or coordinate-based vertex welding.
pub fn plan_radius_network(
    source: &Model,
    shell: usize,
    requested: &[EdgeRadius],
    max_visits: usize,
) -> Result<RadiusNetwork> {
    source.validate()?;
    if !source.persistent_naming_complete()
        || requested.is_empty()
        || requested.len() > 10000
        || !(1..=1000000).contains(&max_visits)
    {
        return Err(refuse(
            "Need named original edges and bounded topology work",
        ));
    }
    let owned = source
        .shells
        .get(shell)
        .ok_or_else(|| refuse("Missing shell"))?;
    if !owned.closed {
        return Err(refuse("Radius network needs a closed source shell"));
    }
    let mut visits = 0;
    let mut visit = || -> Result<()> {
        if visits == max_visits {
            return Err(refuse("Radius network topology work exhausted"));
        }
        visits += 1;
        Ok(())
    };
    let mut radii = BTreeMap::new();
    for r in requested {
        visit()?;
        let e = source
            .edges
            .get(r.edge)
            .ok_or_else(|| refuse("Missing requested edge"))?;
        if !r.radius_mm.is_finite()
            || r.radius_mm <= 0.
            || e.degenerate
            || e.vertices[0] == e.vertices[1]
            || radii.insert(r.edge, r.radius_mm).is_some()
        {
            return Err(refuse("Need unique open edges and finite positive radii"));
        }
    }
    let mut incidence = BTreeMap::<usize, Vec<(usize, bool)>>::new();
    let mut vertices = BTreeMap::<usize, BTreeSet<usize>>::new();
    for f in &owned.faces {
        visit()?;
        let face = &source.faces[f.face];
        for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            visit()?;
            for c in &source.loops[wire].coedges {
                visit()?;
                incidence
                    .entry(c.edge)
                    .or_default()
                    .push((f.face, c.reversed ^ f.reversed));
                for v in source.edges[c.edge].vertices {
                    vertices.entry(v).or_default().insert(c.edge);
                }
            }
        }
    }
    let mut edges = Vec::new();
    let mut selected = BTreeMap::<usize, Vec<usize>>::new();
    for (edge, radius_mm) in radii {
        visit()?;
        let uses = incidence
            .get(&edge)
            .ok_or_else(|| refuse("Requested edge is outside the shell"))?;
        if uses.len() != 2 || uses[0].0 == uses[1].0 || uses[0].1 == uses[1].1 {
            return Err(refuse(
                "Requested edge needs opposite uses on two distinct supports",
            ));
        }
        let mut supports = [uses[0].0, uses[1].0];
        supports.sort();
        for v in source.edges[edge].vertices {
            selected.entry(v).or_default().push(edges.len());
        }
        edges.push(RadiusEdge {
            edge,
            identity: source.1.edges[edge],
            radius_mm,
            supports,
            support_identities: supports.map(|f| source.1.faces[f]),
        });
    }
    let mut corners = Vec::new();
    for (vertex, selected) in selected {
        visit()?;
        let incident = vertices.remove(&vertex).unwrap();
        for edge in &incident {
            visit()?;
            let uses = &incidence[edge];
            if uses.len() != 2 || uses[0].0 == uses[1].0 || uses[0].1 == uses[1].1 {
                return Err(refuse(
                    "Corner neighborhood needs two distinct opposite face uses",
                ));
            }
        }
        let radius = edges[selected[0]].radius_mm;
        let unequal_radii = selected.iter().any(|&e| edges[e].radius_mm != radius);
        corners.push(RadiusCorner {
            vertex,
            identity: source.1.vertices[vertex],
            selected,
            incident: incident.into_iter().collect(),
            unequal_radii,
        });
    }
    Ok(RadiusNetwork {
        shell,
        identity: source.1.shells[shell],
        edges,
        corners,
        topology_visits: visits,
    })
}
