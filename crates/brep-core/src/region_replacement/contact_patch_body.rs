//! Root-owned contact cuts and fresh G2/global admission of their replacement.
use super::{EmbeddedPatchLimits, OwnedPatchSet, SewPatch, SewnPatchSet};
use crate::{Model, boundary_embedding, face_cut::ContactRetention};
use nurbs_core::Result;

pub enum ContactPatchFailure {
    Construction(nurbs_core::Error),
    EndRegion(super::ContactEndNormals),
    GlobalAudit(nurbs_core::Error),
    Unproven(Box<boundary_embedding::Report>),
}
impl std::fmt::Debug for ContactPatchFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Construction(e) => f.debug_tuple("Construction").field(e).finish(),
            Self::EndRegion(r) => f.debug_tuple("ExpandContactEndRegion").field(r).finish(),
            Self::GlobalAudit(e) => f.debug_tuple("GlobalAudit").field(e).finish(),
            Self::Unproven(r) => f.debug_struct("GlobalUnproven")
                .field("exact_lifts", &r.agreement.all_equal)
                .field("injective_faces", &r.intersections.faces.all_faces_injective)
                .field("pairs_classified", &r.intersections.pairs.all_pairs_classified).finish(),
        }
    }
}
impl std::fmt::Display for ContactPatchFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{self:?}") }
}
impl std::error::Error for ContactPatchFailure {}

/// Actual saved body, G2/curvature <=10/mm and fresh whole-boundary embedding.
/// Radius, material nesting/volume and thickness still require separate proofs.
pub struct ContactPatchBody {
    retention: ContactRetention,
    assembly: SewnPatchSet,
    global: boundary_embedding::Report,
    contact_edges: [usize; 2],
}
impl ContactPatchBody {
    /// Saved contact edges, ordered by the original support-face identities.
    /// These are topology bindings, not certificates of a prescribed radius.
    pub fn contact_edges(&self) -> [usize; 2] { self.contact_edges }
    pub fn model(&self) -> &Model { self.assembly.model() }
    pub fn retention(&self) -> &ContactRetention { &self.retention }
    pub fn patch_faces(&self) -> &[usize] { self.assembly.patch_faces() }
    pub fn seams(&self) -> &crate::sweep_smoothness::geometric::Report { self.assembly.seams() }
    pub fn fields(&self) -> &[crate::face_curvature::Report] { self.assembly.fields() }
    pub fn closure(&self) -> &crate::watertight::QualifiedReport { self.assembly.closure() }
    pub fn global(&self) -> &boundary_embedding::Report { &self.global }
}
/// Consume the privately bound material plan. Only its selected discarded
/// sectors may be replaced; all original retained surfaces remain unchanged.
/// Construction and global work limits apply to these fresh admission stages;
/// root/face-planning ledgers remain separately available through retention.
/// No trusted geometry flags or caller-provided G2/global reports are accepted.
pub fn sew_contact_g2_patch_set<F>(
    retention: ContactRetention, key: &str, tolerance_uv: f64,
    limits: EmbeddedPatchLimits, construct: F,
) -> std::result::Result<ContactPatchBody, ContactPatchFailure>
where F: FnOnce(&ContactRetention) -> Result<OwnedPatchSet> {
    // Reconstruct root ownership and every cut before using their saved roles.
    retention.reverify().map_err(ContactPatchFailure::Construction)?;
    let ends = super::inspect_contact_end_normals(&retention)
        .map_err(ContactPatchFailure::Construction)?;
    if ends.needs_expansion() { return Err(ContactPatchFailure::EndRegion(ends)); }
    let plan = retention.plan();
    let selected = plan.regions.iter().flat_map(|r| r.discard_faces.iter().copied()).collect::<Vec<_>>();
    let authored = construct(&retention).map_err(ContactPatchFailure::Construction)?;
    let patches = authored.patches.iter().map(|p| SewPatch {
        surface: &p.surface, loops: &p.loops, reversed: p.reversed,
    }).collect::<Vec<_>>();
    let mut assembly = super::sew_g2_patch_set(&plan.model, &selected,
        &authored.vertices, &authored.edges, &patches, key, tolerance_uv,
        limits.construction).map_err(ContactPatchFailure::Construction)?;
    // Map every retained descendant through the actual replacement, never a
    // count or caller identity claim. Check literal source coefficients again.
    for region in &plan.regions {
        for &face in &region.retained_faces {
            let Some(saved) = assembly.source_faces()[face] else {
                return Err(ContactPatchFailure::Construction(super::refused("A planned retained contact support was removed")));
            };
            if assembly.model().faces[saved].surface != plan.model.faces[face].surface {
                return Err(ContactPatchFailure::Construction(super::refused("Saved retained contact support coefficients changed")));
            }
        }
    }
    // Bind complete original contacts through both cuts and actual sewing maps.
    // Cut request order is not the original support order.
    let mut contact_edges = [0; 2];
    for side in 0..2 {
        let original_face = retention.roots().branch().contacts[side].face;
        let regions = plan.regions.iter().filter(|r|
            retention.roots().model().1.faces[r.source_face] == original_face
        ).collect::<Vec<_>>();
        if regions.len() != 1 {
            return Err(ContactPatchFailure::Construction(super::refused("Contact support lost its unique original face role")));
        }
        let candidates = regions[0].cut_edges.iter().copied().filter(|&e|
            plan.model.edges[e].curve == retention.roots().contacts()[side].curve
        ).collect::<Vec<_>>();
        if candidates.len() != 1 {
            return Err(ContactPatchFailure::Construction(super::refused("Complete original contact has no unique cut edge")));
        }
        let saved = assembly.source_edges()[candidates[0]].ok_or_else(||
            ContactPatchFailure::Construction(super::refused("Sewing removed an original contact edge")))?;
        let mut hits = Vec::new();
        for (face, f) in assembly.model().faces.iter().enumerate() {
            for w in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
                for c in &assembly.model().loops[w].coedges {
                    if c.edge == saved { hits.push(face); }
                }
            }
        }
        if hits.len() != 2 || hits.iter().filter(|f| assembly.patch_faces().contains(f)).count() != 1
            || !regions[0].retained_faces.iter().any(|&f|
                assembly.source_faces()[f].is_some_and(|mapped| hits.contains(&mapped))) {
            return Err(ContactPatchFailure::Construction(super::refused("Saved contact does not join its retained support to one replacement face")));
        }
        contact_edges[side] = saved;
    }
    if contact_edges[0] == contact_edges[1] {
        return Err(ContactPatchFailure::Construction(super::refused("Distinct contact sides collapsed to one edge")));
    }
    let global = boundary_embedding::inspect_with_linear(assembly.model(), tolerance_uv,
        limits.global, limits.linear_cells).map_err(ContactPatchFailure::GlobalAudit)?;
    if !global.proven { return Err(ContactPatchFailure::Unproven(Box::new(global))); }
    assembly.rebase_change_set(retention.roots().partition().source());
    Ok(ContactPatchBody { retention, assembly, global, contact_edges })
}
