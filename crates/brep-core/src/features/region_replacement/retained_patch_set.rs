//! One transaction from original-face material planning to a qualified patch net.
use super::{ContactQualificationLimits, SewPatch, SewnPatchSet, refused};
use crate::{Coedge, Edge, Model, Vertex, face_cut};
use nurbs_core::{Result, surface::Surface};

/// Authored against the private partition's topology, never a proximity weld.
pub struct OwnedSewPatch {
    pub surface: Surface,
    pub loops: Vec<Vec<Coedge>>,
    pub reversed: bool,
}
pub struct OwnedPatchSet {
    /// New edge endpoints index partition.vertices followed by these vertices.
    pub vertices: Vec<Vertex>,
    /// Patch coedges index partition.edges followed by these edges.
    pub edges: Vec<Edge>,
    pub patches: Vec<OwnedSewPatch>,
}
/// Evidence stays bound to the actual saved closed body. Original supports may
/// have several surviving sectors; maps refer to the caller's original model.
pub struct RetainedPatchBody {
    assembly: SewnPatchSet,
    source_faces: Vec<Vec<usize>>,
    source_edges: Vec<Option<usize>>,
    source_vertices: Vec<Option<usize>>,
    contact_edges: Vec<Vec<Option<usize>>>,
    work: (usize, u64, usize, usize),
}
impl RetainedPatchBody {
    pub fn model(&self) -> &Model {
        self.assembly.model()
    }
    pub fn patch_faces(&self) -> &[usize] {
        self.assembly.patch_faces()
    }
    pub fn source_faces(&self) -> &[Vec<usize>] {
        &self.source_faces
    }
    pub fn source_edges(&self) -> &[Option<usize>] {
        &self.source_edges
    }
    pub fn source_vertices(&self) -> &[Option<usize>] {
        &self.source_vertices
    }
    pub fn contact_edges(&self) -> &[Vec<Option<usize>>] {
        &self.contact_edges
    }
    pub fn seams(&self) -> &crate::sweep_smoothness::geometric::Report {
        self.assembly.seams()
    }
    pub fn fields(&self) -> &[crate::face_curvature::Report] {
        self.assembly.fields()
    }
    pub fn closure(&self) -> &crate::watertight::QualifiedReport {
        self.assembly.closure()
    }
    pub fn work(&self) -> (usize, u64, usize, usize) {
        self.work
    }
}
/// Cut immutable source NURBS supports, select retained material with strict
/// interior witnesses, and sew a simultaneous transition net. Only the derived
/// discarded sectors can be replaced. The constructor sees a read-only partition
/// and returns geometry/roles, not a model or trusted evidence. Every saved patch
/// rim must pass G2 and every saved face must pass curvature <=10/mm. All planning
/// and admission share one ledger; refusal never mutates the original.
///
/// Prepared root endpoints remain required. Contact discovery, radius laws,
/// thickness and global embedding are NOT certified by this transaction.
pub fn trim_and_sew_g2_patch_set<F>(
    source: &Model,
    requests: &[face_cut::FaceRetention<'_>],
    key: &str,
    tolerance_uv: f64,
    limits: ContactQualificationLimits,
    construct: F,
) -> Result<RetainedPatchBody>
where
    F: FnOnce(&face_cut::RetentionNetwork) -> Result<OwnedPatchSet>,
{
    if !(8..=100_000).contains(&limits.cells)
        || limits.exact_work > 1_000_000
        || !(1..=1_000_000).contains(&limits.domain_cells)
        || !(1..=1_000_000).contains(&limits.shape_cells)
        || key.is_empty()
        || key.len() > 128
    {
        return Err(refused(
            "Need bounded shared trim/sew work and an operation key",
        ));
    }
    let plan = face_cut::plan_retention_network(
        source,
        requests,
        tolerance_uv,
        limits.cells,
        limits.exact_work,
        limits.domain_cells.min(100_000),
    )?;
    let selected = plan
        .regions
        .iter()
        .flat_map(|r| r.discard_faces.iter().copied())
        .collect::<Vec<_>>();
    if selected.is_empty()
        || limits.cells.saturating_sub(plan.cells) < 8
        || plan.exact_work > limits.exact_work
        || plan.domain_cells >= limits.domain_cells
    {
        return Err(refused(
            "No replaceable material or remaining trim/sew work",
        ));
    }
    let authored = construct(&plan)?;
    let patches = authored
        .patches
        .iter()
        .map(|p| SewPatch {
            surface: &p.surface,
            loops: &p.loops,
            reversed: p.reversed,
        })
        .collect::<Vec<_>>();
    let mut assembly = super::sew_g2_patch_set(
        &plan.model,
        super::PatchReplacement {
            selected: &selected,
            new_vertices: &authored.vertices,
            new_edges: &authored.edges,
            patches: &patches,
        },
        key,
        tolerance_uv,
        ContactQualificationLimits {
            cells: limits.cells - plan.cells,
            exact_work: limits.exact_work - plan.exact_work,
            domain_cells: limits.domain_cells - plan.domain_cells,
            shape_cells: limits.shape_cells,
        },
    )?;
    let mut source_faces = Vec::with_capacity(source.faces.len());
    for f in 0..source.faces.len() {
        let kept = match plan.regions.iter().find(|r| r.source_face == f) {
            Some(r) => r.retained_faces.clone(),
            None => vec![f],
        };
        let mapped = kept
            .into_iter()
            .map(|f| {
                assembly.source_faces()[f]
                    .ok_or_else(|| refused("A planned retained support was removed"))
            })
            .collect::<Result<Vec<_>>>()?;
        source_faces.push(mapped);
    }
    let source_edges = assembly.source_edges()[..source.edges.len()].to_vec();
    let source_vertices = assembly.source_vertices()[..source.vertices.len()].to_vec();
    let contact_edges = plan
        .regions
        .iter()
        .map(|r| {
            r.cut_edges
                .iter()
                .map(|&e| assembly.source_edges()[e])
                .collect()
        })
        .collect();
    let (c, e, d, s) = assembly.work();
    let work = (
        plan.cells + c,
        plan.exact_work + e,
        plan.domain_cells + d,
        s,
    );
    assembly.rebase_change_set(source);
    Ok(RetainedPatchBody {
        assembly,
        source_faces,
        source_edges,
        source_vertices,
        contact_edges,
        work,
    })
}
