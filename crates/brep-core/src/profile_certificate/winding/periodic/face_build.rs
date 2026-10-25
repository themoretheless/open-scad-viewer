//! Common physical face obligations after proof-kind-specific source admission.
//! Inputs are private to periodic ownership; callers must freshly verify their
//! original source context before constructing this borrowed view.
use super::*;
use crate::{Coedge, Edge, Face, FaceUse, Loop, Model, Shell, Surface, Vertex};
use nurbs_core::{curve_surface_agreement as agreement, surface_injectivity};
use std::collections::{BTreeMap, BTreeSet};
pub(super) struct FaceContext<'a> {
    pub surface: &'a Surface,
    pub wrapped: [bool; 2],
    pub chart: [[f64; 2]; 2],
    pub components: &'a [WindingComponent],
    pub cycle_edges: &'a [Vec<usize>],
    pub graph: &'a WeightedChartGraph,
    pub error_upper: f64,
}
#[derive(Clone)]
pub(super) struct FaceBuild {
    pub model: Model,
    pub domains: Vec<[[f64; 2]; 2]>,
    pub injectivity: Vec<surface_injectivity::Report>,
    pub seam_edges: Vec<usize>,
    pub agreement: crate::boundary_agreement::Report,
    pub boundary_error_upper: f64,
}
fn clamped(c: &Curve) -> bool {
    let n = c.control_points.len();
    !c.periodic
        && c.knots[..=c.degree].iter().all(|&k| k == c.knots[c.degree])
        && c.knots[n..].iter().all(|&k| k == c.knots[n])
}
fn key(p: [f64; 2]) -> [u64; 2] {
    p.map(|x| if x == 0. { 0 } else { x.to_bits() })
}
fn canonical(mut p: [f64; 2], chart: [[f64; 2]; 2], wrapped: [bool; 2]) -> [f64; 2] {
    for a in 0..2 {
        if wrapped[a] && p[a] == chart[a][1] {
            p[a] = chart[a][0];
        }
    }
    p
}
fn seam(p: &Curve, chart: [[f64; 2]; 2], wrapped: [bool; 2]) -> Option<(usize, usize)> {
    (0..2).find_map(|a| {
        if wrapped[a] {
            (0..2).find_map(|s| {
                p.control_points
                    .iter()
                    .all(|p| p[a] == chart[a][s])
                    .then_some((a, s))
            })
        } else {
            None
        }
    })
}
fn verify_vertex_links(model: &Model) -> Result<()> {
    let mut links = vec![BTreeMap::<usize, Vec<usize>>::new(); model.vertices.len()];
    for face in &model.faces {
        for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            let cs = &model.loops[wire].coedges;
            for (i, c) in cs.iter().enumerate() {
                let next = &cs[(i + 1) % cs.len()];
                let vertex = model.edges[c.edge].vertices[usize::from(!c.reversed)];
                links[vertex].entry(c.edge).or_default().push(next.edge);
                links[vertex].entry(next.edge).or_default().push(c.edge);
            }
        }
    }
    for link in links {
        if link.is_empty() {
            return Err(refused("Unused periodic quotient vertex"));
        }
        let ends = link.values().filter(|n| n.len() == 1).count();
        if (ends != 0 && ends != 2) || link.values().any(|n| n.is_empty() || n.len() > 2) {
            return Err(refused(
                "Periodic quotient vertex has a nonmanifold edge link",
            ));
        }
        let mut pending = vec![*link.keys().next().unwrap()];
        let mut visited = BTreeSet::new();
        while let Some(e) = pending.pop() {
            if visited.insert(e) {
                pending.extend(link[&e].iter().copied());
            }
        }
        if visited.len() != link.len() {
            return Err(refused(
                "Periodic quotient vertex joins disconnected face fans",
            ));
        }
    }
    Ok(())
}
pub(super) fn build(source: FaceContext<'_>, limits: PeriodicFaceLimits) -> Result<FaceBuild> {
    limits.validate()?;
    let components = source.components;
    if components.is_empty() || components.len() > 512 {
        return Err(refused(
            "Periodic face component census exceeds its owner limit",
        ));
    }
    let graph = source.graph;
    let chart = source.chart;
    let mut model = Model::empty(limits.tolerance_world)?;
    let mut vertices = BTreeMap::new();
    let mut connectors = BTreeMap::<([usize; 2], usize), (usize, usize)>::new();
    let mut edge_uses = Vec::<Vec<(usize, bool)>>::new();
    let mut seam_edges = Vec::new();
    let mut domains = Vec::new();
    let mut proofs = Vec::new();
    let mut span_work = 0;
    let mut lift_work = 0;
    for (face, component) in components.iter().enumerate() {
        let mut domain = [[f64::INFINITY, f64::NEG_INFINITY]; 2];
        for p in component
            .wires()
            .iter()
            .flatten()
            .flat_map(|c| &c.control_points)
        {
            for a in 0..2 {
                domain[a][0] = domain[a][0].min(p[a]);
                domain[a][1] = domain[a][1].max(p[a]);
            }
        }
        if span_work == limits.injectivity_spans {
            return Err(refused(
                "Shared original-chart injectivity allowance exhausted",
            ));
        }
        let proof = surface_injectivity::certify_on_refined(
            source.surface,
            domain,
            limits.injectivity_spans - span_work,
            limits.injectivity_depth,
        )?;
        span_work += proof.spans;
        if !proof.proven {
            return Err(refused(
                "Original supporting-surface region injectivity/rank remains unproven",
            ));
        }
        domains.push(domain);
        proofs.push(proof);
        let mut loop_ids = Vec::new();
        for (wi, wire) in component.wires().iter().enumerate() {
            let cycle = &source.cycle_edges[component.source_loops()[wi]];
            let mut coedges = Vec::new();
            for (ci, pc) in wire.iter().enumerate() {
                if !clamped(pc) {
                    return Err(refused(
                        "Owned periodic edge endpoints require full clamping",
                    ));
                }
                let eid = cycle[if component.reversed()[wi] {
                    cycle.len() - 1 - ci
                } else {
                    ci
                }];
                let generated = graph.edges[eid]
                    .contributions
                    .iter()
                    .any(|c| c.source.is_none());
                let uv = [&pc.control_points[0], pc.control_points.last().unwrap()];
                let mut vs = [0; 2];
                for end in 0..2 {
                    let p = canonical([uv[end][0], uv[end][1]], chart, source.wrapped);
                    let id = if let Some(&id) = vertices.get(&key(p)) {
                        id
                    } else {
                        let q = source.surface.evaluate(p[0], p[1])?.point;
                        let id = model.vertices.len();
                        model.vertices.push(Vertex { point: q });
                        vertices.insert(key(p), id);
                        id
                    };
                    vs[end] = id;
                }
                if vs[0] == vs[1] {
                    return Err(refused(
                        "Closed or collapsed periodic edge needs an explicit chart partition",
                    ));
                }
                let pair = if vs[0] < vs[1] { vs } else { [vs[1], vs[0]] };
                let tag = if generated {
                    let (axis, side) = seam(pc, chart, source.wrapped).ok_or_else(|| {
                        refused("Generated connector is not on a wrapped original chart boundary")
                    })?;
                    if pc.degree != 1
                        || pc.control_points.len() != 2
                        || pc.weights[0] != pc.weights[1]
                    {
                        return Err(refused(
                            "Nonlinear generated seam correspondence remains unproven",
                        ));
                    }
                    Some((axis, side))
                } else {
                    None
                };
                let found = tag.and_then(|(axis, side)| {
                    connectors
                        .get(&(pair, axis))
                        .map(|&(edge, other)| (edge, side, other))
                });
                let (edge, reversed) = if let Some((edge, side, other)) = found {
                    if side == other || edge_uses[edge].len() != 1 {
                        return Err(refused(
                            "Periodic connector has ambiguous chart-side ownership",
                        ));
                    }
                    let rev = model.edges[edge].vertices != vs;
                    if !rev {
                        return Err(refused("Periodic seam traversals are not opposite"));
                    }
                    (edge, true)
                } else {
                    if lift_work == limits.lift_cells {
                        return Err(refused("Shared owned-edge lift allowance exhausted"));
                    }
                    let endpoints = vs.map(|id| model.vertices[id].point);
                    let lift = agreement::lift(
                        pc,
                        source.surface,
                        Some(endpoints),
                        limits.tolerance_world,
                        limits.lift_cells - lift_work,
                    )?;
                    lift_work += lift.cells;
                    let curve = lift
                        .curve
                        .ok_or_else(|| refused("Original pcurve/support lift remains unproven"))?;
                    let edge = model.edges.len();
                    model.edges.push(Edge {
                        vertices: vs,
                        curve,
                        degenerate: false,
                    });
                    edge_uses.push(Vec::new());
                    if let Some((axis, side)) = tag {
                        connectors.insert((pair, axis), (edge, side));
                        seam_edges.push(edge);
                    }
                    (edge, false)
                };
                edge_uses[edge].push((face, reversed));
                coedges.push(Coedge::new(
                    edge,
                    reversed,
                    pc.clone(),
                ));
            }
            loop_ids.push(model.loops.len());
            model.loops.push(Loop { coedges });
        }
        model.faces.push(Face {
            surface: source.surface.clone(),
            outer: loop_ids[0],
            holes: loop_ids[1..].to_vec(),
        });
    }
    if seam_edges.iter().any(|&e| edge_uses[e].len() != 2) {
        return Err(refused(
            "An original generated connector lacks its opposite owned face",
        ));
    }
    let mut neighbors = vec![Vec::new(); model.faces.len()];
    for uses in &edge_uses {
        if uses.len() == 2 {
            let (a, b) = (uses[0].0, uses[1].0);
            neighbors[a].push(b);
            neighbors[b].push(a);
        }
    }
    let mut seen = BTreeSet::new();
    for start in 0..model.faces.len() {
        if seen.contains(&start) {
            continue;
        }
        let mut pending = vec![start];
        let mut faces = Vec::new();
        let mut closed = true;
        while let Some(face) = pending.pop() {
            if seen.insert(face) {
                faces.push(FaceUse {
                    face,
                    reversed: false,
                });
                pending.extend(neighbors[face].iter().copied());
                for wire in std::iter::once(model.faces[face].outer)
                    .chain(model.faces[face].holes.iter().copied())
                {
                    closed &= model.loops[wire]
                        .coedges
                        .iter()
                        .all(|c| edge_uses[c.edge].len() == 2);
                }
            }
        }
        model.shells.push(Shell { faces, closed });
    }
    model.inherit_topology_ids(&[]);
    model.validate()?;
    verify_vertex_links(&model)?;
    let agreement = crate::boundary_agreement::verify(&model, limits.agreement_cells)?;
    if !agreement.complete {
        return Err(refused(
            "Complete saved owned-periodic edge/support agreement remains unproven",
        ));
    }
    let boundary_error_upper = nurbs_core::interval_eval::Interval::point(source.error_upper)
        .add(nurbs_core::interval_eval::Interval::point(
            limits.tolerance_world,
        ))?
        .hi;
    Ok(FaceBuild {
        model,
        domains,
        injectivity: proofs,
        seam_edges,
        agreement,
        boundary_error_upper,
    })
}

impl FaceBuild {
    pub(super) fn verify_against(&self, fresh: &Self) -> Result<()> {
        let injectivity_same = self.injectivity.len() == fresh.injectivity.len()
            && self
                .injectivity
                .iter()
                .zip(&fresh.injectivity)
                .all(|(a, b)| {
                    a.proven == b.proven
                        && a.projection == b.projection
                        && a.contraction_upper == b.contraction_upper
                        && a.spans == b.spans
                        && a.reason == b.reason
                });
        let a = &self.agreement;
        let b = &fresh.agreement;
        let agreement_same = a.complete == b.complete
            && a.cells == b.cells
            && a.uses.len() == b.uses.len()
            && a.uses.iter().zip(&b.uses).all(|(a, b)| {
                a.face == b.face
                    && a.wire == b.wire
                    && a.coedge == b.coedge
                    && a.edge == b.edge
                    && a.status == b.status
                    && a.witness == b.witness
                    && a.witness_distance == b.witness_distance
            });
        if self.model != fresh.model
            || self.domains != fresh.domains
            || self.seam_edges != fresh.seam_edges
            || !injectivity_same
            || !agreement_same
            || self.boundary_error_upper != fresh.boundary_error_upper
        {
            return Err(refused(
                "Original periodic model or complete physical proof receipt changed",
            ));
        }
        Ok(())
    }
}
