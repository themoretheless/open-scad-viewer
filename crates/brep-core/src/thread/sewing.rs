use super::*;

#[derive(Debug)]
pub struct SharedPatchEdge {
    /// One retained world curve, oriented as the source boundary.
    pub curve: nurbs_core::curve::Curve,
    pub source_pcurve: nurbs_core::curve::Curve,
    pub target_pcurve: nurbs_core::curve::Curve,
    pub target_reversed: bool,
    pub agreements: [Report; 2],
    pub verification_cells: usize,
}

/// Sew two boundary uses within one open shell. Retains the left world curve;
/// exact opposite endpoints and continuous agreement on both faces are needed.
/// This does not certify surface embedding or turn the shell into a solid.
pub fn sew_patch_boundaries(
    model: &Model,
    left: [usize; 2],
    right: [usize; 2],
    max_verification_cells: usize,
) -> Result<(Model, SharedPatchEdge)> {
    sew_boundaries_impl(model, left, right, max_verification_cells, true)
}
pub(super) fn sew_boundaries_impl(
    model: &Model,
    left: [usize; 2],
    right: [usize; 2],
    max_verification_cells: usize,
    validate_input: bool,
) -> Result<(Model, SharedPatchEdge)> {
    if validate_input {
        model.validate()?;
    }
    let invalid = || {
        Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Sew needs distinct forward boundary uses in one open shell",
        )
    };
    if model.shells.len() != 1 || model.shells[0].closed || !model.bodies.is_empty() {
        return Err(invalid());
    }
    let coedge = |address: [usize; 2]| {
        model
            .loops
            .get(address[0])
            .and_then(|l| l.coedges.get(address[1]))
            .ok_or_else(invalid)
    };
    let lc = coedge(left)?;
    let rc = coedge(right)?;
    if lc.edge == rc.edge || lc.reversed || rc.reversed {
        return Err(invalid());
    }
    for edge in [lc.edge, rc.edge] {
        if model
            .loops
            .iter()
            .flat_map(|l| &l.coedges)
            .filter(|c| c.edge == edge)
            .count()
            != 1
        {
            return Err(invalid());
        }
    }
    let owner = |wire| {
        model
            .faces
            .iter()
            .find(|f| f.outer == wire)
            .ok_or_else(invalid)
    };
    let lf = owner(left[0])?;
    let rf = owner(right[0])?;
    let le = &model.edges[lc.edge];
    let re = &model.edges[rc.edge];
    let mut vertex_map: Vec<_> = (0..model.vertices.len()).collect();
    for i in 0..2 {
        let old = re.vertices[i];
        let new = le.vertices[1 - i];
        if model.vertices[old].point != model.vertices[new].point {
            return Err(invalid());
        }
        vertex_map[old] = new;
    }
    let provisional = nurbs_core::thread::MappedEdge {
        curve: le.curve.clone(),
        pcurve: lc.pcurve.clone(),
        agreement: Report {
            status: nurbs_core::curve_surface_agreement::Status::Unresolved,
            cells: 0,
            witness: None,
            witness_distance: None,
        },
        requested_world_tolerance: model.tolerance_mm,
    };
    let proof = share_patch_edge(
        &provisional,
        &lf.surface,
        &rc.pcurve,
        &rf.surface,
        true,
        model.tolerance_mm,
        max_verification_cells,
    )?;
    let mut result = model.clone();
    result.loops[right[0]].coedges[right[1]].edge = lc.edge;
    result.loops[right[0]].coedges[right[1]].reversed = true;
    result.edges.remove(rc.edge);
    for c in result.loops.iter_mut().flat_map(|l| &mut l.coedges) {
        if c.edge > rc.edge {
            c.edge -= 1;
        }
    }
    // Resolve endpoint unions before compacting; reject cycles rather than
    // silently assigning a different coordinate or breaking incidence.
    for i in 0..vertex_map.len() {
        let mut v = i;
        let mut steps = 0;
        while vertex_map[v] != v {
            v = vertex_map[v];
            steps += 1;
            if steps > vertex_map.len() {
                return Err(invalid());
            }
        }
        vertex_map[i] = v;
    }
    let mut compact = vec![usize::MAX; vertex_map.len()];
    let mut vertices = Vec::new();
    for (i, &root) in vertex_map.iter().enumerate() {
        if i == root {
            compact[i] = vertices.len();
            vertices.push(model.vertices[i].clone());
        }
    }
    for edge in &mut result.edges {
        edge.vertices = edge.vertices.map(|v| compact[vertex_map[v]]);
    }
    result.vertices = vertices;
    result.rebuild_topology_ids();
    if validate_input {
        result.inherit_topology_ids(&[model]);
    }
    if validate_input {
        result.validate()?;
    }
    Ok((result, proof))
}

/// Join two single-face patch sheets using one retained source edge. Opposite
/// loop traversal and exact endpoint equality are required; no vertex healing.
/// All source inputs remain immutable, including on failed verification.
pub fn join_patch_sheets(
    source: &Model,
    target: &Model,
    source_coedge: usize,
    target_coedge: usize,
    max_verification_cells: usize,
) -> Result<(Model, SharedPatchEdge)> {
    attach_patch_sheet(
        source,
        target,
        0,
        source_coedge,
        target_coedge,
        max_verification_cells,
    )
}

/// Attach a single-loop face to a boundary edge of an existing open shell.
/// Edges already used twice refuse; no third incident face is admitted.
pub fn attach_patch_sheet(
    source: &Model,
    target: &Model,
    source_loop: usize,
    source_coedge: usize,
    target_coedge: usize,
    max_verification_cells: usize,
) -> Result<(Model, SharedPatchEdge)> {
    attach_impl(
        source,
        target,
        source_loop,
        source_coedge,
        target_coedge,
        max_verification_cells,
        true,
    )
}
/// Attach a face along two boundaries atomically; the intermediate face fan
/// may be disconnected and is never returned or admitted as a Model.
pub fn attach_patch_sheet_two_edges(
    source: &Model,
    target: &Model,
    first: [[usize; 2]; 2],
    second: [[usize; 2]; 2],
    max_verification_cells: usize,
) -> Result<Model> {
    Ok(attach_two_edges_impl(source, target, first, second, max_verification_cells)?.0)
}
pub(super) fn attach_two_edges_impl(
    source: &Model,
    target: &Model,
    first: [[usize; 2]; 2],
    second: [[usize; 2]; 2],
    max_verification_cells: usize,
) -> Result<(Model, usize)> {
    if first[1][0] != 0 || second[1][0] != 0 {
        return Err(Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Target uses must belong to loop zero",
        ));
    }
    attach_patch_sheet_edges(
        source,
        target,
        &[(first[0], first[1][1]), (second[0], second[1][1])],
        max_verification_cells,
    )
}
/// Attach a complete single-loop face along 1..254 selected boundaries,
/// checking all seams under one shared budget before admitting the result.
pub fn attach_patch_sheet_edges(
    source: &Model,
    target: &Model,
    pairs: &[([usize; 2], usize)],
    max_verification_cells: usize,
) -> Result<(Model, usize)> {
    if pairs.is_empty() || pairs.len() > 254 || max_verification_cells > 100000 {
        return Err(Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Attach needs 1..254 distinct boundary pairs and at most 100000 cells",
        ));
    }
    let mut source_uses = std::collections::BTreeSet::new();
    let mut target_uses = std::collections::BTreeSet::new();
    for &(s, t) in pairs {
        if !source_uses.insert(s) || !target_uses.insert(t) {
            return Err(Error::new(
                "BREP_INVALID_THREAD_PATCH",
                "Repeated boundary correspondence",
            ));
        }
    }
    let (first_source, first_target) = pairs[0];
    let (mut result, proof) = attach_impl(
        source,
        target,
        first_source[0],
        first_source[1],
        first_target,
        max_verification_cells,
        false,
    )?;
    let target_loop = source.loops.len();
    let mut used = proof.verification_cells;
    for &(s, t) in &pairs[1..] {
        let (next, proof) = sew_boundaries_impl(
            &result,
            s,
            [target_loop, t],
            max_verification_cells - used,
            false,
        )?;
        used += proof.verification_cells;
        result = next;
    }
    result.validate()?;
    Ok((result, used))
}

pub(super) fn attach_impl(
    source: &Model,
    target: &Model,
    source_loop: usize,
    source_coedge: usize,
    target_coedge: usize,
    max_verification_cells: usize,
    validate_result: bool,
) -> Result<(Model, SharedPatchEdge)> {
    source.validate()?;
    target.validate()?;
    if target.faces.len() != 1 || target.loops.len() != 1 {
        return Err(Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Target must be a single-loop face",
        ));
    }
    for model in [source, target] {
        if model.shells.len() != 1
            || model.shells[0].closed
            || !model.bodies.is_empty()
            || model.faces.iter().any(|f| !f.holes.is_empty())
        {
            return Err(Error::new(
                "BREP_INVALID_THREAD_PATCH",
                "Join requires open shells without bodies or holes",
            ));
        }
    }
    let source_wire = source.loops.get(source_loop).ok_or_else(|| {
        Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Source loop index out of range",
        )
    })?;
    let source_face = source
        .faces
        .iter()
        .find(|f| f.outer == source_loop)
        .ok_or_else(|| Error::new("BREP_INVALID_THREAD_PATCH", "Source loop has no face owner"))?;
    let sc = source_wire.coedges.get(source_coedge).ok_or_else(|| {
        Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Source coedge index out of range",
        )
    })?;
    if source
        .loops
        .iter()
        .flat_map(|l| &l.coedges)
        .filter(|c| c.edge == sc.edge)
        .count()
        != 1
    {
        return Err(Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Selected source edge is not a boundary edge",
        ));
    }
    let tc = target.loops[0].coedges.get(target_coedge).ok_or_else(|| {
        Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Target coedge index out of range",
        )
    })?;
    if sc.reversed || tc.reversed {
        return Err(Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Join expects forward source sheet coedges",
        ));
    }
    let se = &source.edges[sc.edge];
    let te = &target.edges[tc.edge];
    if source.vertices[se.vertices[0]].point != target.vertices[te.vertices[1]].point
        || source.vertices[se.vertices[1]].point != target.vertices[te.vertices[0]].point
    {
        return Err(Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Join needs exact opposite boundary endpoints",
        ));
    }
    let tolerance = source.tolerance_mm.min(target.tolerance_mm);
    let provisional = nurbs_core::thread::MappedEdge {
        curve: se.curve.clone(),
        pcurve: sc.pcurve.clone(),
        agreement: Report {
            status: nurbs_core::curve_surface_agreement::Status::Unresolved,
            cells: 0,
            witness: None,
            witness_distance: None,
        },
        requested_world_tolerance: tolerance,
    };
    let shared = share_patch_edge(
        &provisional,
        &source_face.surface,
        &tc.pcurve,
        &target.faces[0].surface,
        true,
        tolerance,
        max_verification_cells,
    )?;
    let mut result = source.clone();
    result.tolerance_mm = tolerance;
    let mut vertices = vec![usize::MAX; target.vertices.len()];
    vertices[te.vertices[0]] = se.vertices[1];
    vertices[te.vertices[1]] = se.vertices[0];
    for (i, vertex) in target.vertices.iter().enumerate() {
        if vertices[i] == usize::MAX {
            let matches: Vec<_> = source
                .vertices
                .iter()
                .enumerate()
                .filter(|(_, v)| v.point == vertex.point)
                .map(|(i, _)| i)
                .collect();
            if matches.len() > 1 {
                return Err(Error::new(
                    "BREP_INVALID_THREAD_PATCH",
                    "Ambiguous exact vertex correspondence",
                ));
            }
            if let Some(&existing) = matches.first() {
                vertices[i] = existing;
            } else {
                vertices[i] = result.vertices.len();
                result.vertices.push(vertex.clone());
            }
        }
    }
    let mut edges = vec![usize::MAX; target.edges.len()];
    edges[tc.edge] = sc.edge;
    for (i, edge) in target.edges.iter().enumerate() {
        if i == tc.edge {
            continue;
        }
        edges[i] = result.edges.len();
        let mut edge = edge.clone();
        edge.vertices = edge.vertices.map(|v| vertices[v]);
        result.edges.push(edge);
    }
    let outer = result.loops.len();
    result.loops.push(Loop {
        coedges: target.loops[0]
            .coedges
            .iter()
            .map(|c| Coedge {
                edge: edges[c.edge],
                reversed: c.edge == tc.edge,
                pcurve: c.pcurve.clone(),
            })
            .collect(),
    });
    let face = result.faces.len();
    result.faces.push(Face {
        surface: target.faces[0].surface.clone(),
        outer,
        holes: vec![],
    });
    result.shells[0].faces.push(FaceUse {
        face,
        reversed: false,
    });
    result.rebuild_topology_ids();
    if validate_result {
        result.inherit_topology_ids(&[source, target]);
    }
    if validate_result {
        result.validate()?;
    }
    Ok((result, shared))
}

/// Verify one immutable world edge on two original patch surfaces. Both uses
/// are checked anew; cached source agreement is not trusted. This constructs
/// geometric boundary evidence, not global sewing or shell orientation.
pub fn share_patch_edge(
    source: &nurbs_core::thread::MappedEdge,
    source_surface: &nurbs_core::surface::Surface,
    target_pcurve: &nurbs_core::curve::Curve,
    target_surface: &nurbs_core::surface::Surface,
    target_reversed: bool,
    tolerance: f64,
    max_verification_cells: usize,
) -> Result<SharedPatchEdge> {
    use nurbs_core::curve_surface_agreement::{self, Status};
    if !(1e-10..=1e-2).contains(&tolerance) || max_verification_cells > 100000 {
        return Err(Error::new(
            "BREP_INVALID_THREAD_PATCH",
            "Shared boundary needs valid tolerance and at most 100000 cells",
        ));
    }
    if max_verification_cells == 0 {
        return Err(Error::new(
            "BREP_THREAD_PATCH_UNRESOLVED",
            "Shared boundary verification budget exhausted",
        ));
    }
    let first = curve_surface_agreement::verify(
        &source.curve,
        &source.pcurve,
        source_surface,
        false,
        tolerance,
        max_verification_cells,
    )?;
    if first.status != Status::WithinTolerance || first.cells == max_verification_cells {
        return Err(Error::new(
            "BREP_THREAD_PATCH_UNRESOLVED",
            "Source boundary agreement unproved or budget exhausted",
        ));
    }
    let second = curve_surface_agreement::verify(
        &source.curve,
        target_pcurve,
        target_surface,
        target_reversed,
        tolerance,
        max_verification_cells - first.cells,
    )?;
    if second.status != Status::WithinTolerance {
        return Err(Error::new(
            "BREP_THREAD_PATCH_UNRESOLVED",
            "Target boundary agreement unproved",
        ));
    }
    let cells = first.cells + second.cells;
    Ok(SharedPatchEdge {
        curve: source.curve.clone(),
        source_pcurve: source.pcurve.clone(),
        target_pcurve: target_pcurve.clone(),
        target_reversed,
        agreements: [first, second],
        verification_cells: cells,
    })
}
