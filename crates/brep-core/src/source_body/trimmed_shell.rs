//! Assemble an explicitly owned trimmed patch network, with independent gates.
//! No automatic tolerance sewing, branch inference or solid embedding is claimed.
use crate::{Coedge, Edge, Face, FaceUse, Loop, Model, Shell, Vertex, face_domain, watertight};
use nurbs_core::{Error, Result, surface::Surface, surface_regularity};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
pub struct Patch {
    pub surface: Surface,
    /// Pcurves follow traversal order; reversed refers only to the owned edge.
    pub outer: Vec<Coedge>,
    pub holes: Vec<Vec<Coedge>>,
    pub reversed: bool,
}
#[derive(Clone, Copy)]
pub struct Limits {
    pub tolerance_mm: f64,
    pub tolerance_uv: f64,
    pub agreement_cells: usize,
    pub trim_pairs: usize,
    pub trim_cells: usize,
    pub trim_domain_cells: usize,
    pub regularity_cells: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            tolerance_mm: 1e-6,
            tolerance_uv: 1e-8,
            agreement_cells: 1000000,
            trim_pairs: 100000,
            trim_cells: 100000,
            trim_domain_cells: 1000000,
            regularity_cells: 1000000,
        }
    }
}
pub struct Report {
    /// Present only after every gate passes. No Body/solid is manufactured.
    pub model: Option<Model>,
    pub closure: watertight::Report,
    pub trims: face_domain::TrimAudit,
    pub regularity: Vec<Option<surface_regularity::Report>>,
    pub regularity_cells: usize,
    pub vertex_links_valid: bool,
    pub connected: bool,
}
fn invalid(message: &str) -> Error {
    Error::new("BREP_TRIMMED_SHELL_INPUT", message)
}
fn incidence(model: &Model) -> (bool, bool) {
    let mut edge_faces = vec![vec![]; model.edges.len()];
    let mut links = vec![BTreeMap::<usize, Vec<usize>>::new(); model.vertices.len()];
    for (face_id, face) in model.faces.iter().enumerate() {
        for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            let coedges = &model.loops[wire].coedges;
            for (i, c) in coedges.iter().enumerate() {
                edge_faces[c.edge].push(face_id);
                let next = &coedges[(i + 1) % coedges.len()];
                let edge = &model.edges[c.edge];
                let vertex = edge.vertices[usize::from(!c.reversed)];
                links[vertex].entry(c.edge).or_default().push(next.edge);
                links[vertex].entry(next.edge).or_default().push(c.edge);
            }
        }
    }
    let mut valid = edge_faces.iter().all(|f| f.len() == 2);
    for (vertex, link) in links.iter().enumerate() {
        let incident: BTreeSet<_> = model
            .edges
            .iter()
            .enumerate()
            .filter(|(_, e)| e.vertices.contains(&vertex))
            .map(|(i, _)| i)
            .collect();
        if incident.is_empty()
            || link.len() != incident.len()
            || link.iter().any(|(e, n)| {
                !incident.contains(e) || n.len() != 2 || n.iter().any(|x| !incident.contains(x))
            })
        {
            valid = false;
            continue;
        }
        let start = *incident.first().unwrap();
        let mut seen = BTreeSet::new();
        let mut queue = vec![start];
        while let Some(e) = queue.pop() {
            if seen.insert(e) {
                queue.extend(&link[&e]);
            }
        }
        valid &= seen == incident;
    }
    let mut adjacency = vec![BTreeSet::new(); model.faces.len()];
    for faces in &edge_faces {
        for &a in faces {
            for &b in faces {
                adjacency[a].insert(b);
            }
        }
    }
    let mut seen = BTreeSet::new();
    let mut queue = VecDeque::from([0]);
    while let Some(face) = queue.pop_front() {
        if seen.insert(face) {
            queue.extend(&adjacency[face]);
        }
    }
    (valid, seen.len() == model.faces.len())
}
/// Build one connected closed shell from authored edge ownership. Input edges
/// must have distinct endpoint vertex IDs; degenerate pole edges are refused.
/// All edges and vertices must be used. Up to 512 faces and the kernel's entity
/// limits are admitted. Shared work limits apply independently to each audit.
/// Unresolved proof returns model=None with all diagnostic reports, never an
/// apparently complete shell. Source fitting and automatic sewing are separate.
pub fn assemble(
    vertices: Vec<Vertex>,
    edges: Vec<Edge>,
    patches: Vec<Patch>,
    limits: Limits,
) -> Result<Report> {
    if patches.is_empty()
        || patches.len() > 512
        || vertices.is_empty()
        || edges.is_empty()
        || vertices.len() + edges.len() > crate::MAX_ENTITIES
        || !limits.tolerance_mm.is_finite()
        || !(1e-10..=1e-2).contains(&limits.tolerance_mm)
        || !limits.tolerance_uv.is_finite()
        || limits.tolerance_uv <= 0.
        || !(1..=1000000).contains(&limits.agreement_cells)
        || !(1..=100000).contains(&limits.trim_pairs)
        || !(1..=100000).contains(&limits.trim_cells)
        || !(1..=1000000).contains(&limits.trim_domain_cells)
        || !(1..=1000000).contains(&limits.regularity_cells)
        || edges
            .iter()
            .any(|e| e.degenerate || e.vertices[0] == e.vertices[1])
    {
        return Err(invalid(
            "Trimmed shell needs bounded audits, 1..512 faces and nondegenerate owned edges",
        ));
    }
    let mut model = Model::empty(limits.tolerance_mm)?;
    model.vertices = vertices;
    model.edges = edges;
    let mut face_uses = vec![];
    for patch in patches {
        if patch.holes.len() > 15 {
            return Err(invalid("At most 15 holes are admitted per face"));
        }
        let outer = model.loops.len();
        model.loops.push(Loop {
            coedges: patch.outer,
        });
        let mut holes = vec![];
        for ring in patch.holes {
            holes.push(model.loops.len());
            model.loops.push(Loop { coedges: ring });
        }
        let face = model.faces.len();
        model.faces.push(Face {
            surface: patch.surface,
            outer,
            holes,
        });
        face_uses.push(FaceUse {
            face,
            reversed: patch.reversed,
        });
    }
    model.shells.push(Shell {
        faces: face_uses,
        closed: false,
    });
    model.rebuild_topology_ids();
    model.validate_boundary_diagnostic_inputs()?;

    let (vertex_links_valid, connected) = incidence(&model);
    let closure = watertight::inspect(&model, limits.agreement_cells)?;
    let trims = face_domain::audit_trim_regions(
        &model,
        limits.tolerance_uv,
        limits.trim_pairs,
        limits.trim_cells,
        limits.trim_domain_cells,
    )?;
    let mut regularity = vec![];
    let mut regularity_cells = 0;
    let mut regular = true;
    for face in &model.faces {
        let remaining = limits.regularity_cells - regularity_cells;
        let r = if remaining == 0 {
            None
        } else {
            match surface_regularity::inspect(&face.surface, remaining.min(100000)) {
                Ok(r) => Some(r),
                Err(e) if e.code == "NURBS_RESOURCE_LIMIT" => None,
                Err(e) => return Err(e),
            }
        };
        if let Some(r) = &r {
            regularity_cells += r.cells;
        }
        regular &= r.as_ref().is_some_and(|r| r.spanwise_regular);
        regularity.push(r);
    }
    let admitted = vertex_links_valid
        && connected
        && trims.all_valid
        && regular
        && closure.status == watertight::Closure::WithinTolerance;
    let model = if admitted {
        model.shells[0].closed = true;
        model.validate()?;
        Some(model)
    } else {
        None
    };
    Ok(Report {
        model,
        closure,
        trims,
        regularity,
        regularity_cells,
        vertex_links_valid,
        connected,
    })
}
