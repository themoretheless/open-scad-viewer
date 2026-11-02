//! Topology-owned support discovery and cyclic corner neighborhoods.
//! Surface-pair proposals are not admitted trimmed contacts or sewn solids.
use crate::{
    Model,
    region_replacement::{EdgeRadius, RadiusNetwork, plan_radius_network},
};
use nurbs_core::{Error, Result, offset::fillet_surfaces::FilletOptions};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, PartialEq)]
pub struct CornerFan {
    pub vertex: usize,
    /// Cyclic original edges; rotation starts at the smallest edge index.
    pub edges: Vec<usize>,
    /// faces[i] joins edges[i] to edges[(i+1)%n].
    pub faces: Vec<usize>,
    /// Prescribed radius for each original edge, None for unselected edges.
    pub radii_mm: Vec<Option<f64>>,
}
pub struct Discovery {
    pub network: RadiusNetwork,
    pub fans: Vec<CornerFan>,
    pub topology_visits: usize,
}
fn refuse(message: &str) -> Error {
    Error::new("BREP_CONTACT_TOPOLOGY", message)
}
/// Discover without coordinate sorting, normal fitting, or vertex welding.
/// Supports arbitrary valid source NURBS definitions; this is combinatorial,
/// not evidence that a geometric blend exists for the prescribed radii.
pub fn discover(
    source: &Model,
    shell: usize,
    requested: &[EdgeRadius],
    max_visits: usize,
) -> Result<Discovery> {
    let network = plan_radius_network(source, shell, requested, max_visits)?;
    let mut visits = network.topology_visits;
    let mut tick = || -> Result<()> {
        if visits >= max_visits {
            return Err(refuse("Corner fan work exhausted"));
        }
        visits += 1;
        Ok(())
    };
    let radii: BTreeMap<_, _> = network
        .edges
        .iter()
        .map(|e| (e.edge, e.radius_mm))
        .collect();
    let mut fans = Vec::new();
    for corner in &network.corners {
        let incident: BTreeSet<_> = corner.incident.iter().copied().collect();
        let mut adjacency = BTreeMap::<usize, Vec<(usize, usize)>>::new();
        for owned in &source.shells[shell].faces {
            tick()?;
            let face = &source.faces[owned.face];
            for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                let coedges = &source.loops[wire].coedges;
                for i in 0..coedges.len() {
                    tick()?;
                    let a = &coedges[i];
                    let b = &coedges[(i + 1) % coedges.len()];
                    let ea = &source.edges[a.edge];
                    let eb = &source.edges[b.edge];
                    if ea.vertices[usize::from(!a.reversed)] == corner.vertex
                        && eb.vertices[usize::from(b.reversed)] == corner.vertex
                    {
                        if a.edge == b.edge
                            || !incident.contains(&a.edge)
                            || !incident.contains(&b.edge)
                        {
                            return Err(refuse("Ambiguous corner incidence"));
                        }
                        adjacency
                            .entry(a.edge)
                            .or_default()
                            .push((b.edge, owned.face));
                        adjacency
                            .entry(b.edge)
                            .or_default()
                            .push((a.edge, owned.face));
                    }
                }
            }
        }
        if incident.len() < 3
            || adjacency.len() != incident.len()
            || adjacency.values().any(|a| a.len() != 2 || a[0].0 == a[1].0)
        {
            return Err(refuse("Corner needs one simple closed face fan"));
        }
        let first = *incident.first().unwrap();
        let mut edges = vec![first];
        let mut faces = Vec::new();
        let mut previous = None;
        let mut current = first;
        loop {
            tick()?;
            let next = adjacency[&current]
                .iter()
                .filter(|(edge, _)| Some(*edge) != previous)
                .min()
                .copied()
                .ok_or_else(|| refuse("Broken fan"))?;
            faces.push(next.1);
            if next.0 == first {
                break;
            }
            if edges.contains(&next.0) {
                return Err(refuse("Disconnected or repeated fan"));
            }
            edges.push(next.0);
            previous = Some(current);
            current = next.0;
        }
        if edges.len() != incident.len() {
            return Err(refuse("Disconnected corner fan"));
        }
        fans.push(CornerFan {
            vertex: corner.vertex,
            radii_mm: edges.iter().map(|e| radii.get(e).copied()).collect(),
            edges,
            faces,
        });
    }
    drop(tick);
    Ok(Discovery {
        network,
        fans,
        topology_visits: visits,
    })
}

/// A fresh complete original-edge/support lift check, before offset marching.
/// This binds source incidence geometrically; it does not bind a future offset
/// branch to the edge or prove that contacts lie inside the trimmed faces.
pub fn inspect_source_edge(
    source: &Model,
    shell: usize,
    request: EdgeRadius,
    max_visits: usize,
    max_cells: usize,
    max_exact: u64,
) -> Result<nurbs_core::surface_g2_pcurves::Positions> {
    let plan = plan_radius_network(source, shell, &[request], max_visits)?;
    let edge = &plan.edges[0];
    let coedges = edge.supports.map(|f| {
        let face = &source.faces[f];
        std::iter::once(face.outer)
            .chain(face.holes.iter().copied())
            .flat_map(|w| &source.loops[w].coedges)
            .find(|c| c.edge == edge.edge)
            .unwrap()
    });
    nurbs_core::surface_g2_pcurves::verify_positions(
        &source.edges[edge.edge].curve,
        coedges.map(|c| &c.pcurve),
        edge.supports.map(|f| &source.faces[f].surface),
        coedges.map(|c| c.reversed),
        source.tolerance_mm,
        max_cells,
        max_exact,
    )
}

/// Automatically select the original two support charts of a requested edge.
/// The existing numerical surface-pair marcher may discover another branch:
/// even VerifiedContacts certifies lifts only, NOT association to this edge,
/// trimmed-face membership, G2, curvature <=10/mm, corner ends, or closure.
/// Callers must retain this report as a proposal until those gates pass.
pub fn propose_edge(
    source: &Model,
    shell: usize,
    request: EdgeRadius,
    options: &FilletOptions,
    max_visits: usize,
    max_cells: usize,
    max_exact: u64,
) -> Result<super::Report> {
    let plan = plan_radius_network(source, shell, &[request], max_visits)?;
    let edge = &plan.edges[0];
    let original = inspect_source_edge(source, shell, request, max_visits, max_cells, max_exact)?;
    if !original.proven {
        return Err(refuse("Original edge/support lifts remain unproven"));
    }
    let supports = edge.supports.map(|f| &source.faces[f].surface);
    super::build_g2(
        supports,
        edge.radius_mm,
        options,
        source.tolerance_mm,
        max_cells.saturating_sub(original.cells),
        max_exact.saturating_sub(original.exact_work),
    )
}

pub struct BoundEdgeProposal {
    /// Surface construction and fresh contact-lift report.
    pub candidate: super::Report,
    /// Absent when the builder could not produce both actual contacts.
    /// Bound concerns selected-edge collar ownership only, never solid admission.
    pub branch: Option<super::branch::Report>,
    /// Separate partial/full ownership status for convex nonrectangular charts.
    pub convex_branch: Option<super::convex_branch::Report>,
}
/// Propose automatically and freshly attempt selected-edge branch ownership.
/// All geometric cell/exact work is shared across proposal and binding. A
/// successful numerical proposal cannot silently acquire an ownership flag.
pub fn propose_bound_edge(
    source: &Model,
    shell: usize,
    request: EdgeRadius,
    options: &FilletOptions,
    max_visits: usize,
    max_cells: usize,
    max_exact: u64,
) -> Result<BoundEdgeProposal> {
    let original = inspect_source_edge(source, shell, request, max_visits, max_cells, max_exact)?;
    if !original.proven {
        return Err(refuse("Original edge/support lifts remain unproven"));
    }
    let plan = plan_radius_network(source, shell, &[request], max_visits)?;
    let edge = &plan.edges[0];
    let mut candidate = super::build_g2(
        edge.supports.map(|f| &source.faces[f].surface),
        request.radius_mm,
        options,
        source.tolerance_mm,
        max_cells.saturating_sub(original.cells),
        max_exact.saturating_sub(original.exact_work),
    )?;
    candidate.cells += original.cells;
    candidate.exact_work += original.exact_work;
    let branch =
        if let (Some(skin), Some(contacts)) = (&candidate.fillet.surface, &candidate.contacts) {
            Some(super::branch::inspect(
                source,
                shell,
                request,
                skin,
                [&contacts[0], &contacts[1]],
                max_visits,
                max_cells.saturating_sub(candidate.cells),
                max_exact.saturating_sub(candidate.exact_work),
            )?)
        } else {
            None
        };
    let convex_branch = if let (Some(skin), Some(contacts), Some(first)) =
        (&candidate.fillet.surface, &candidate.contacts, &branch)
    {
        if first.decision == super::branch::Decision::Unresolved {
            Some(super::convex_branch::inspect(
                source,
                shell,
                request,
                skin,
                [&contacts[0], &contacts[1]],
                max_visits,
                max_cells
                    .saturating_sub(candidate.cells)
                    .saturating_sub(first.cells),
                max_exact
                    .saturating_sub(candidate.exact_work)
                    .saturating_sub(first.exact_work),
                max_visits.min(1000000),
            )?)
        } else {
            None
        }
    } else {
        None
    };
    Ok(BoundEdgeProposal {
        candidate,
        branch,
        convex_branch,
    })
}
