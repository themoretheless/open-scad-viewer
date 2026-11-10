//! Boundary-moving face-region replacement with topology-derived sector intent.
//! Root isolation/section construction belongs to face_cut; closed cap admission
//! belongs to region_replacement. No new SS implementation or proximity weld.
use crate::{Error, Model, Result, TopoId, face_cut, region_replacement, solid_audit};
use nurbs_core::surface::Surface;
use std::collections::{BTreeMap, BTreeSet};

fn refused(message: &str) -> Error {
    Error::new("BREP_RETRIM_REPLACEMENT_REFUSED", message)
}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub tolerance_uv: f64,
    pub max_cells: usize,
    pub max_exact: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            tolerance_uv: 1e-8,
            max_cells: 100000,
            max_exact: 1000000,
        }
    }
}
/// Closed incidence and continuous original trim/lift agreement, with a private
/// actual result. This is NOT a global embedding or prescribed-radius receipt.
pub struct Replacement {
    cap: region_replacement::SurfaceCap,
    audit: solid_audit::SolidAuditCertificate,
    removed_faces: Vec<TopoId>,
    neighbors: Vec<(TopoId, usize)>,
    roots: Vec<(TopoId, usize, usize)>,
    cells: usize,
    exact_work: u64,
}
impl Replacement {
    pub fn model(&self) -> &Model {
        &self.cap.replacement.model
    }
    pub fn into_model(self) -> Model {
        self.cap.replacement.model
    }
    pub fn replacement_face(&self) -> usize {
        self.cap.replacement.face
    }
    pub fn removed_faces(&self) -> &[TopoId] {
        &self.removed_faces
    }
    /// Original support identity -> actual retained trimmed face index.
    pub fn neighbors(&self) -> &[(TopoId, usize)] {
        &self.neighbors
    }
    /// Original edge identity, original root ordinal, final shared vertex index.
    pub fn roots(&self) -> &[(TopoId, usize, usize)] {
        &self.roots
    }
    pub fn closure(&self) -> &crate::watertight::QualifiedReport {
        &self.cap.replacement.closure
    }
    pub fn audit(&self) -> &solid_audit::SolidAuditCertificate {
        &self.audit
    }
    pub fn work(&self) -> (usize, u64) {
        (self.cells, self.exact_work)
    }
}
fn edges_of(model: &Model, face: usize) -> BTreeSet<usize> {
    let f = &model.faces[face];
    std::iter::once(f.outer)
        .chain(f.holes.iter().copied())
        .flat_map(|w| model.loops[w].coedges.iter().map(|c| c.edge))
        .collect()
}

/// Replace selected original faces by a supplied freeform surface, MOVING their
/// boundary by trimming all adjacent supports. No face IDs, UV material seeds,
/// search boxes, roots or positive receipts for the adjacent sectors are inputs.
/// Removed sectors are inferred solely from original shared-rim incidence.
///
/// Admitted domain: one closed body/shell, a connected selected region, one
/// certifiable original-root fiber across each adjacent support, with the old
/// rim untouched by the cutter. The supplied chart must cover all contacts and
/// admit one injective replacement cap. Extension outside original support
/// charts, hidden-branch census and multi-loop cap construction remain separate.
pub fn replace_region(
    source: &Model,
    selected: &[usize],
    surface: &Surface,
    limits: Limits,
) -> Result<Replacement> {
    source.validate()?;
    surface.validate()?;
    if source.bodies.len() != 1
        || source.shells.len() != 1
        || selected.is_empty()
        || selected.len() > 128
        || selected.iter().any(|&f| f >= source.faces.len())
        || selected.iter().copied().collect::<BTreeSet<_>>().len() != selected.len()
        || !(32..=100000).contains(&limits.max_cells)
        || limits.max_exact > 1000000
        || !limits.tolerance_uv.is_finite()
        || limits.tolerance_uv <= 0.
    {
        return Err(refused(
            "Need distinct original faces, one closed body and bounded work",
        ));
    }
    solid_audit::audit_solid(source)?;
    let chosen = selected.iter().copied().collect::<BTreeSet<_>>();
    let source_edges = source
        .faces
        .iter()
        .enumerate()
        .map(|(i, _)| edges_of(source, i))
        .collect::<Vec<_>>();
    let mut rim = BTreeMap::<usize, Vec<usize>>::new();
    let mut adjacent_selected = BTreeMap::<usize, BTreeSet<usize>>::new();
    for &f in selected {
        for &edge in &source_edges[f] {
            let owners = source_edges
                .iter()
                .enumerate()
                .filter(|(_, e)| e.contains(&edge))
                .map(|(i, _)| i)
                .collect::<Vec<_>>();
            if owners.len() != 2 {
                return Err(refused(
                    "Selected rim requires exactly two original face owners",
                ));
            }
            let other = *owners
                .iter()
                .find(|&&i| i != f)
                .ok_or_else(|| refused("Missing adjacent face"))?;
            if chosen.contains(&other) {
                adjacent_selected.entry(f).or_default().insert(other);
            } else {
                rim.entry(other).or_default().push(edge);
            }
        }
    }
    // Reject disconnected original selections before expensive root queries.
    let mut connected = BTreeSet::from([selected[0]]);
    loop {
        let before = connected.len();
        let old = connected.iter().copied().collect::<Vec<_>>();
        for f in old {
            if let Some(next) = adjacent_selected.get(&f) {
                connected.extend(next);
            }
        }
        if before == connected.len() {
            break;
        }
    }
    if connected.len() != selected.len() || rim.is_empty() || rim.len() + selected.len() > 256 {
        return Err(refused(
            "Selected replacement must be one bounded connected region with a retained rim",
        ));
    }
    for edges in rim.values_mut() {
        edges.sort_unstable();
        edges.dedup();
    }
    let faces = rim.keys().copied().collect::<Vec<_>>();
    let cutter = super::freeform::sheet(surface, source.tolerance_mm)?;
    let network = face_cut::partition_fiber_network_automatic(
        source,
        &faces,
        &cutter,
        0,
        limits.tolerance_uv,
        limits.max_cells,
        limits.max_exact,
    )?;
    if network
        .boundary_coverage
        .iter()
        .any(|(_, r)| !r.complete || !r.source_is_owned)
    {
        return Err(refused(
            "Complete original boundary root coverage is required",
        ));
    }
    let mut remove = selected.to_vec();
    let mut retained = Vec::new();
    for section in &network.sections {
        let original_rim = rim
            .get(&section.source_face)
            .ok_or_else(|| refused("Unrequested section face"))?;
        // Original region intent survives the partition by identity/incidence,
        // never by a nearest vertex or an approximate UV sampling decision.
        if original_rim
            .iter()
            .any(|&e| source.edges[e] != network.model.edges[e])
        {
            return Err(refused(
                "Cutter intersects the old selected rim; multi-region intent is required",
            ));
        }
        let owners = section
            .faces
            .into_iter()
            .filter(|&f| {
                let owned = edges_of(&network.model, f);
                original_rim.iter().all(|e| owned.contains(e))
            })
            .collect::<Vec<_>>();
        if owners.len() != 1 {
            return Err(refused(
                "Old region rim does not select exactly one complete adjacent sector",
            ));
        }
        remove.push(owners[0]);
        let kept = section
            .faces
            .into_iter()
            .find(|&f| f != owners[0])
            .ok_or_else(|| refused("No retained adjacent sector"))?;
        retained.push((section.source_face, kept));
    }
    if limits.max_cells.saturating_sub(network.cells) < 8 {
        return Err(refused("No shared work remains for closed cap admission"));
    }
    let cap = region_replacement::cap_region(
        &network.model,
        &remove,
        surface,
        limits.tolerance_uv,
        limits.max_cells - network.cells,
        limits.max_exact - network.exact_work,
    )?;
    if cap.replacement.closure.closure.status != crate::watertight::Closure::WithinTolerance
        || !cap.replacement.trims.all_valid
    {
        return Err(refused(
            "Replacement is not continuously closed and trim-admitted",
        ));
    }
    let mut neighbors = Vec::new();
    for (original, kept) in retained {
        let final_face = cap.replacement.source_faces[kept]
            .ok_or_else(|| refused("Replacement discarded a retained support sector"))?;
        if cap.replacement.model.faces[final_face].surface != source.faces[original].surface {
            return Err(refused(
                "Retained support geometry changed during retrimming",
            ));
        }
        neighbors.push((source.1.faces[original], final_face));
    }
    let roots = network
        .root_vertices
        .iter()
        .map(|&(edge, ordinal, vertex)| {
            let final_vertex = cap.replacement.source_vertices[vertex]
                .ok_or_else(|| refused("Replacement discarded an original shared root"))?;
            Ok((source.1.edges[edge], ordinal, final_vertex))
        })
        .collect::<Result<Vec<_>>>()?;
    for &f in &remove {
        if cap.replacement.source_faces[f].is_some() {
            return Err(refused("A removed sector survived replacement"));
        }
    }
    let audit = solid_audit::audit_solid(&cap.replacement.model)?;
    let cells = network.cells + cap.cells;
    let exact_work = network.exact_work + cap.replacement.exact_work;
    Ok(Replacement {
        cap,
        audit,
        removed_faces: selected.iter().map(|&f| source.1.faces[f]).collect(),
        neighbors,
        roots,
        cells,
        exact_work,
    })
}
