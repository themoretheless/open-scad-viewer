//! Simultaneous replacement with topology-authored NEW interior interfaces.
use super::{ContactQualificationLimits, refused};
use crate::{Coedge, Edge, Face, FaceUse, Loop, Model, TopoId, TopoKind, Vertex, face_curvature};
use nurbs_core::{Result, surface::Surface};
use std::collections::{BTreeMap, BTreeSet};

pub(super) const MAX_SEW_PATCHES: usize = 64;
pub(super) const G2_NORMAL_TOLERANCE: f64 = 1e-5;

pub struct SewPatch<'a> {
    pub surface: &'a Surface,
    /// Edges index source.edges followed by the supplied new_edges.
    pub loops: &'a [Vec<Coedge>],
    pub reversed: bool,
}
/// Read-only admission of the actual simultaneous assembly. Every patch rim,
/// including every new internal seam, passes G2; every saved face passes the
/// 10/mm curvature bound. Global embedding, thickness and radius are separate.
pub struct SewnPatchSet {
    model: Model,
    source_faces: Vec<Option<usize>>,
    source_edges: Vec<Option<usize>>,
    source_vertices: Vec<Option<usize>>,
    patch_faces: Vec<usize>,
    new_edges: Vec<usize>,
    new_vertices: Vec<usize>,
    seams: crate::sweep_smoothness::geometric::Report,
    fields: Vec<face_curvature::Report>,
    closure: crate::watertight::QualifiedReport,
    work: (usize, u64, usize, usize),
    seam_coefficient_work: usize,
}
impl SewnPatchSet {
    pub(super) fn rebase_change_set(&mut self, source: &Model) {
        self.model.refresh_change_set(&[source]);
    }
    pub fn source_faces(&self) -> &[Option<usize>] {
        &self.source_faces
    }
    pub fn source_edges(&self) -> &[Option<usize>] {
        &self.source_edges
    }
    pub fn source_vertices(&self) -> &[Option<usize>] {
        &self.source_vertices
    }

    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn patch_faces(&self) -> &[usize] {
        &self.patch_faces
    }
    pub fn new_edges(&self) -> &[usize] {
        &self.new_edges
    }
    pub fn new_vertices(&self) -> &[usize] {
        &self.new_vertices
    }
    pub fn seams(&self) -> &crate::sweep_smoothness::geometric::Report {
        &self.seams
    }
    pub fn fields(&self) -> &[face_curvature::Report] {
        &self.fields
    }
    pub fn closure(&self) -> &crate::watertight::QualifiedReport {
        &self.closure
    }
    pub fn seam_coefficient_work(&self) -> usize {
        self.seam_coefficient_work
    }
    pub fn work(&self) -> (usize, u64, usize, usize) {
        self.work
    }
}
fn c2_basis(s: &Surface) -> bool {
    for (knots, p, n) in [
        (&s.knots_u, s.degree_u, s.control_points.len()),
        (&s.knots_v, s.degree_v, s.control_points[0].len()),
    ] {
        let mut i = 0;
        while i < knots.len() {
            let mut end = i + 1;
            while end < knots.len() && knots[end] == knots[i] {
                end += 1;
            }
            if knots[p] < knots[i] && knots[i] < knots[n] && end - i > p.saturating_sub(2) {
                return false;
            }
            i = end;
        }
    }
    true
}
fn connected(chosen: &BTreeSet<usize>, adj: &BTreeMap<usize, Vec<usize>>) -> bool {
    let mut seen = BTreeSet::new();
    let mut pending = vec![*chosen.first().unwrap()];
    while let Some(f) = pending.pop() {
        if seen.insert(f) {
            pending.extend(adj.get(&f).into_iter().flatten().copied());
        }
    }
    &seen == chosen
}
/// One atomic connected source-region replacement. Original boundary edge
/// identities and vertices are reused, never matched by spatial proximity.
/// New edge vertices index source.vertices followed by new_vertices. Every
/// supplied new role must be used. Intermediate nonclosed models are private.
/// All rim interfaces are G2: a sharp omitted cap cannot authorize admission.
pub fn sew_g2_patch_set(
    source: &Model,
    selected: &[usize],
    new_vertices: &[Vertex],
    new_edges: &[Edge],
    patches: &[SewPatch<'_>],
    key: &str,
    tolerance_uv: f64,
    limits: ContactQualificationLimits,
) -> Result<SewnPatchSet> {
    sew_g2_patch_set_impl(
        source,
        selected,
        new_vertices,
        new_edges,
        patches,
        key,
        tolerance_uv,
        limits,
        None,
    )
}
/// Same saved-body admission with an additional explicit coefficient ledger.
/// No proposal reports are accepted; every graph proof inspects saved coedges.
pub fn sew_g2_patch_set_with_graph_work(
    source: &Model,
    selected: &[usize],
    new_vertices: &[Vertex],
    new_edges: &[Edge],
    patches: &[SewPatch<'_>],
    key: &str,
    tolerance_uv: f64,
    limits: ContactQualificationLimits,
    max_coefficients: usize,
) -> Result<SewnPatchSet> {
    if (max_coefficients as u64) > 50_000_000_000 {
        return Err(refused("Invalid saved-seam coefficient budget"));
    }
    sew_g2_patch_set_impl(
        source,
        selected,
        new_vertices,
        new_edges,
        patches,
        key,
        tolerance_uv,
        limits,
        Some(max_coefficients),
    )
}
fn sew_g2_patch_set_impl(
    source: &Model,
    selected: &[usize],
    new_vertices: &[Vertex],
    new_edges: &[Edge],
    patches: &[SewPatch<'_>],
    key: &str,
    tolerance_uv: f64,
    limits: ContactQualificationLimits,
    graph_work: Option<usize>,
) -> Result<SewnPatchSet> {
    source.validate()?;
    let chosen: BTreeSet<_> = selected.iter().copied().collect();
    if chosen.len() != selected.len()
        || !(1..=256).contains(&chosen.len())
        || selected.iter().any(|&f| f >= source.faces.len())
        || !(1..=MAX_SEW_PATCHES).contains(&patches.len())
        || new_vertices.len() > 4096
        || new_edges.len() > 4096
        || key.is_empty()
        || key.len() > 128
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || !(8..=100_000).contains(&limits.cells)
        || limits.exact_work > 1_000_000
        || !(1..=1_000_000).contains(&limits.domain_cells)
        || !(1..=1_000_000).contains(&limits.shape_cells)
    {
        return Err(refused(
            "Need bounded owned source region, patch roles and work",
        ));
    }
    let owners = source
        .shells
        .iter()
        .enumerate()
        .filter(|(_, s)| s.faces.iter().any(|u| chosen.contains(&u.face)))
        .collect::<Vec<_>>();
    if owners.len() != 1 || !owners[0].1.closed {
        return Err(refused("Patch set needs one original closed shell"));
    }
    let shell = owners[0].0;
    let mut old = BTreeMap::<usize, Vec<(usize, bool)>>::new();
    for &f in selected {
        let uses = source.shells[shell]
            .faces
            .iter()
            .filter(|u| u.face == f)
            .collect::<Vec<_>>();
        if uses.len() != 1 {
            return Err(refused("Source sectors need unique shell ownership"));
        }
        for w in std::iter::once(source.faces[f].outer).chain(source.faces[f].holes.iter().copied())
        {
            for c in &source.loops[w].coedges {
                if source.edges[c.edge].degenerate {
                    return Err(refused("Collapsed region edges need a separate pole plan"));
                }
                old.entry(c.edge)
                    .or_default()
                    .push((f, c.reversed ^ uses[0].reversed));
            }
        }
    }
    let mut boundary = BTreeMap::new();
    let mut adjacency = BTreeMap::<usize, Vec<usize>>::new();
    for (&e, u) in &old {
        match u.as_slice() {
            [(_, r)] => {
                boundary.insert(e, *r);
            }
            [(a, ra), (b, rb)] if a != b && ra != rb => {
                adjacency.entry(*a).or_default().push(*b);
                adjacency.entry(*b).or_default().push(*a);
            }
            _ => return Err(refused("Original region is not an oriented manifold")),
        }
    }
    if boundary.is_empty() || !connected(&chosen, &adjacency) {
        return Err(refused(
            "Need one connected source region with an external rim",
        ));
    }
    let mut geometry = source.clone();
    geometry.vertices.extend_from_slice(new_vertices);
    for e in new_edges {
        e.curve.validate()?;
        if e.degenerate
            || e.vertices[0] == e.vertices[1]
            || e.vertices.iter().any(|&v| v >= geometry.vertices.len())
            || e.curve.control_points[0].len() != 3
        {
            return Err(refused(
                "New interface needs regular 3D geometry and explicit vertices",
            ));
        }
    }
    geometry.edges.extend_from_slice(new_edges);
    let mut incidence = BTreeMap::<usize, Vec<(usize, bool)>>::new();
    for (i, p) in patches.iter().enumerate() {
        p.surface.validate()?;
        if p.surface.periodic_u
            || p.surface.periodic_v
            || p.surface.control_points[0][0].len() != 3
            || p.surface
                .control_points
                .len()
                .checked_mul(p.surface.control_points[0].len())
                .is_none_or(|n| n > 4096)
            || !(1..=16).contains(&p.loops.len())
            || p.loops.iter().any(|l| l.len() < 2)
            || p.loops.iter().map(Vec::len).sum::<usize>() > 256
        {
            return Err(refused(
                "Patch needs bounded regular charts and explicit loops",
            ));
        }
        if !c2_basis(p.surface) {
            return Err(refused("Patch interior C2 basis remains unproven"));
        }
        for c in p.loops.iter().flatten() {
            c.pcurve.validate()?;
            if c.edge >= geometry.edges.len() || c.pcurve.control_points[0].len() != 2 {
                return Err(refused("Patch references an invalid interface"));
            }
            incidence
                .entry(c.edge)
                .or_default()
                .push((i, c.reversed ^ p.reversed));
        }
    }
    let mut patch_adj = BTreeMap::<usize, Vec<usize>>::new();
    let mut actual_boundary = BTreeMap::new();
    for (&e, u) in &incidence {
        if boundary.contains_key(&e) {
            if let [(_, r)] = u.as_slice() {
                actual_boundary.insert(e, *r);
            } else {
                return Err(refused("External rim must occur once in the patch set"));
            }
        } else {
            if e < source.edges.len() && !old.contains_key(&e) {
                return Err(refused("Patch cannot consume a retained non-rim edge"));
            }
            match u.as_slice() {
                [(a, ra), (b, rb)] if a != b && ra != rb => {
                    patch_adj.entry(*a).or_default().push(*b);
                    patch_adj.entry(*b).or_default().push(*a);
                }
                _ => {
                    return Err(refused(
                        "Internal interface needs two distinct opposite patch uses",
                    ));
                }
            }
        }
    }
    if actual_boundary != boundary
        || !connected(&(0..patches.len()).collect(), &patch_adj)
        || (source.edges.len()..geometry.edges.len()).any(|e| !incidence.contains_key(&e))
    {
        return Err(refused(
            "Patch set must cover the exact rim and use every new edge in one connected network",
        ));
    }
    let mut model = geometry.clone();
    model.faces.clear();
    model.loops.clear();
    let mut source_faces = vec![None; source.faces.len()];
    for (i, f) in source.faces.iter().enumerate() {
        if chosen.contains(&i) {
            continue;
        }
        let mut face = f.clone();
        face.outer = model.loops.len();
        model.loops.push(source.loops[f.outer].clone());
        face.holes.clear();
        for &w in &f.holes {
            face.holes.push(model.loops.len());
            model.loops.push(source.loops[w].clone());
        }
        source_faces[i] = Some(model.faces.len());
        model.faces.push(face);
    }
    let mut patch_faces = Vec::new();
    for p in patches {
        let outer = model.loops.len();
        model
            .loops
            .extend(p.loops.iter().map(|c| Loop { coedges: c.clone() }));
        patch_faces.push(model.faces.len());
        let holes = (outer + 1..model.loops.len()).collect();
        model.faces.push(Face {
            surface: p.surface.clone(),
            outer,
            holes,
        });
    }
    for s in &mut model.shells {
        s.faces = s
            .faces
            .iter()
            .filter_map(|u| {
                source_faces[u.face].map(|f| FaceUse {
                    face: f,
                    reversed: u.reversed,
                })
            })
            .collect();
    }
    model.shells[shell]
        .faces
        .extend(patch_faces.iter().zip(patches).map(|(&face, p)| FaceUse {
            face,
            reversed: p.reversed,
        }));
    let mut used_edges: BTreeSet<_> = model
        .loops
        .iter()
        .flat_map(|l| l.coedges.iter().map(|c| c.edge))
        .collect();
    // Free original geometry outside the replaced sectors is not garbage.
    // Only old region entities which lost every use may be removed.
    used_edges.extend((0..source.edges.len()).filter(|e| !old.contains_key(e)));
    let mut used_vertices: BTreeSet<_> = used_edges
        .iter()
        .flat_map(|&e| geometry.edges[e].vertices)
        .collect();
    let region_vertices: BTreeSet<_> = old.keys().flat_map(|&e| source.edges[e].vertices).collect();
    used_vertices.extend((0..source.vertices.len()).filter(|v| !region_vertices.contains(v)));
    if (source.vertices.len()..geometry.vertices.len()).any(|v| !used_vertices.contains(&v)) {
        return Err(refused("Unused new vertex roles are not permitted"));
    }
    let mut vertices = vec![None; geometry.vertices.len()];
    model.vertices.clear();
    for &v in &used_vertices {
        vertices[v] = Some(model.vertices.len());
        model.vertices.push(geometry.vertices[v].clone());
    }
    let mut edges = vec![None; geometry.edges.len()];
    model.edges.clear();
    for &e in &used_edges {
        edges[e] = Some(model.edges.len());
        let mut edge = geometry.edges[e].clone();
        edge.vertices = edge.vertices.map(|v| vertices[v].unwrap());
        model.edges.push(edge);
    }
    for c in model.loops.iter_mut().flat_map(|l| &mut l.coedges) {
        c.edge = edges[c.edge].unwrap();
    }
    model.inherit_topology_ids(&[source]);
    let mut parents = selected
        .iter()
        .map(|&f| source.1.faces[f].to_string())
        .collect::<Vec<_>>();
    parents.sort();
    let signature = parents.join("/");
    let derive =
        |kind, role: &str| TopoId::derive(kind, "patch-set", key, role, signature.as_bytes());
    let new_vertices = (source.vertices.len()..vertices.len())
        .map(|i| vertices[i].unwrap())
        .collect::<Vec<_>>();
    let new_edges = (source.edges.len()..edges.len())
        .map(|i| edges[i].unwrap())
        .collect::<Vec<_>>();
    for (i, &v) in new_vertices.iter().enumerate() {
        let old = model.1.vertices[v];
        let id = derive(TopoKind::Vertex, &format!("vertex-{i}"));
        if let Some(t) = model.1.local_tolerances_mm.remove(&old) {
            model.1.local_tolerances_mm.insert(id, t);
        }
        model.1.vertices[v] = id;
    }
    for (i, &e) in new_edges.iter().enumerate() {
        let old = model.1.edges[e];
        let id = derive(TopoKind::Edge, &format!("interface-{i}"));
        if let Some(t) = model.1.local_tolerances_mm.remove(&old) {
            model.1.local_tolerances_mm.insert(id, t);
        }
        if model.1.sewed_edges.remove(&old) {
            model.1.sewed_edges.insert(id);
        }
        model.1.edges[e] = id;
    }
    for (i, &f) in patch_faces.iter().enumerate() {
        model.1.faces[f] = derive(TopoKind::Face, &format!("patch-{i}"));
    }
    let parents = selected
        .iter()
        .map(|&f| source.1.faces[f])
        .collect::<Vec<_>>();
    let children = patch_faces
        .iter()
        .map(|&f| model.1.faces[f])
        .collect::<Vec<_>>();
    let mut relation = |operation: &str, parents: Vec<TopoId>, children: Vec<TopoId>| {
        model.1.lineage.push(crate::TopologyLineageRecord {
            operation: operation.into(),
            entity_kind: "face".into(),
            parents,
            children,
        });
    };
    match (parents.len(), children.len()) {
        (1, 1) => relation("persist", parents, children),
        (1, _) => relation("split", parents, children),
        (_, 1) => relation("merge", parents, children),
        _ => {
            let region = derive(TopoKind::Face, "merged-source-region");
            relation("merge", parents, vec![region]);
            relation("split", vec![region], children);
        }
    }
    model.refresh_change_set(&[source]);
    model.validate()?;
    let mut cells = 0;
    let mut exact = 0;
    let mut domain = 0;
    let mut shape = 0;
    for &f in &patch_faces {
        let proof = nurbs_core::surface_linear_monotonicity::inspect_candidate(
            &model.faces[f].surface,
            limits.cells - cells,
        )?;
        cells += proof.cells;
        if !proof.certified {
            return Err(refused("Saved patch injectivity remains unproven"));
        }
        if cells >= limits.cells {
            return Err(refused("Patch set interior work exhausted"));
        }
    }
    let allowance = (limits.cells - cells) / 2;
    if allowance == 0 {
        return Err(refused("No trim audit work remains"));
    }
    let trims =
        crate::face_domain::audit_trim_regions(&model, tolerance_uv, 10000, allowance, allowance)?;
    cells += trims.cells + trims.domain_cells;
    if !trims.all_valid || cells >= limits.cells {
        return Err(refused("Final patch set trim audit remains unproven"));
    }
    let closure = crate::watertight::inspect_with_exact(
        &model,
        limits.cells - cells,
        limits.exact_work - exact,
    )?;
    cells += closure.closure.agreement.cells;
    exact += closure.exact.work;
    if closure.closure.status != crate::watertight::Closure::WithinTolerance
        || cells >= limits.cells
    {
        return Err(refused("Final patch set closure remains unproven"));
    }
    let seam_edges = incidence
        .keys()
        .map(|e| edges[*e].unwrap())
        .collect::<Vec<_>>();
    let tolerances = crate::sweep_smoothness::geometric::Tolerances {
        position: source.tolerance_mm,
        normal: G2_NORMAL_TOLERANCE,
        curvature: 1e-4,
    };
    let (seams, seam_coefficient_work) = if let Some(max) = graph_work {
        let proof = crate::sweep_smoothness::geometric::inspect_edges_with_graph_work(
            &model,
            &seam_edges,
            tolerances,
            limits.cells - cells,
            limits.exact_work - exact,
            max,
        )?;
        (proof.report, proof.coefficient_work)
    } else {
        (
            crate::sweep_smoothness::geometric::inspect_edges_with_exact(
                &model,
                &seam_edges,
                tolerances,
                limits.cells - cells,
                limits.exact_work - exact,
            )?,
            0,
        )
    };
    cells += seams.cells;
    exact += seams.exact_work;
    if !seams.complete || !seams.within_tolerance {
        let failures = seams
            .seams
            .iter()
            .filter(|s| {
                s.inspection.as_ref().is_none_or(|r| {
                    r.decision != nurbs_core::surface_join::Decision::WithinTolerance
                })
            })
            .take(4)
            .map(|s| {
                format!(
                    "edge={} faces={:?} decision={:?} positions={:?} bounds={:?}",
                    s.edge,
                    s.faces,
                    s.inspection.as_ref().map(|r| r.decision),
                    s.positions.as_ref().map(|p| p.proven),
                    s.inspection.as_ref().map(|r| r
                        .branches
                        .each_ref()
                        .map(|b| (b.normal_upper, b.tensor_upper)))
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        return Err(refused(&format!(
            "Final patch set G2 interfaces remain unproven: cells={cells}/{} exact={exact}/{} coefficients={seam_coefficient_work}/{:?}; {failures}",
            limits.cells, limits.exact_work, graph_work
        )));
    }
    let mut fields = Vec::new();
    for face in 0..model.faces.len() {
        if !c2_basis(&model.faces[face].surface) {
            return Err(refused(
                "Saved face curvature across internal knots remains unproven",
            ));
        }
        if cells >= limits.cells || domain >= limits.domain_cells || shape >= limits.shape_cells {
            return Err(refused("Patch set curvature work exhausted"));
        }
        let field = face_curvature::inspect(
            &model,
            face,
            10.,
            tolerance_uv,
            face_curvature::Limits {
                cells: (limits.cells - cells).min(50_000),
                domain_cells: limits.domain_cells - domain,
                shape_cells: limits.shape_cells - shape,
            },
            &[],
        )?;
        cells += field.cells;
        domain += field.domain_cells;
        shape += field.shape_cells;
        if field.decision != face_curvature::Decision::Within
            || field
                .absolute_curvature_upper
                .is_none_or(|x| !x.is_finite() || x > 10.)
        {
            return Err(refused(
                "Saved patch set curvature <=10/mm remains unproven",
            ));
        }
        fields.push(field);
    }
    Ok(SewnPatchSet {
        model,
        source_faces,
        source_edges: edges[..source.edges.len()].to_vec(),
        source_vertices: vertices[..source.vertices.len()].to_vec(),
        patch_faces,
        new_edges,
        new_vertices,
        seams,
        fields,
        closure,
        work: (cells, exact, domain, shape),
        seam_coefficient_work,
    })
}
