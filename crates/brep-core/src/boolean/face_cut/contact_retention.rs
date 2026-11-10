//! Atomic contact-root refinement followed by two original-support material cuts.
use super::{FaceRetention, OwnedCut, RetentionNetwork};
use crate::{Model, edge_partition::contact_interval, fillet_contact::Contact,
    region_replacement::EdgeRadius};
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};

pub struct ContactFaceCut {
    pub face: usize,
    /// Ordered in the complete contact's traversal direction: root 0 to 1,
    /// or root 1 to 0 when its source clock is reversed. Must include the
    /// literal complete contact curve; no refit or knot reversal is required.
    pub curves: Vec<Curve>,
    pub keep_uv: Vec<[f64; 2]>,
}
#[derive(Clone, Copy)]
pub struct ContactRetentionLimits {
    pub roots: contact_interval::Limits,
    /// Face projection/cutting exact work, separate from branch exact work.
    pub cut_exact_work: u64,
    pub domain_cells: usize,
}
impl Default for ContactRetentionLimits {
    fn default() -> Self {
        Self { roots: Default::default(), cut_exact_work: 1_000_000, domain_cells: 100_000 }
    }
}
fn refuse(s: &str) -> Error { Error::new("BREP_CONTACT_RETENTION_UNPROVEN", s) }

/// Every sector remains present until replacement surfaces pass closed-body
/// admission. Neither a caller's choice nor a discard list deletes material.
pub struct ContactRetention {
    roots: contact_interval::ContactInterval,
    plan: RetentionNetwork,
    cuts: Vec<ContactFaceCut>,
    tolerance_uv: f64,
    limits: ContactRetentionLimits,
    cells: usize,
}
impl ContactRetention {
    pub fn roots(&self) -> &contact_interval::ContactInterval { &self.roots }
    pub fn plan(&self) -> &RetentionNetwork { &self.plan }
    pub fn cells(&self) -> usize { self.cells }
    pub fn reverify(&self) -> Result<()> {
        self.roots.reverify()?;
        let actual = plan_cuts(&self.roots, &self.cuts, self.tolerance_uv, self.limits)?;
        let roles = |p: &RetentionNetwork| p.regions.iter().map(|r|
            (r.source_face, r.descendants.clone(), r.retained_faces.clone(),
             r.discard_faces.clone(), r.cut_edges.clone())).collect::<Vec<_>>();
        if actual.model != self.plan.model || roles(&actual) != roles(&self.plan) {
            return Err(refuse("Saved material sectors differ from fresh root-bound reconstruction"));
        }
        Ok(())
    }
}
fn incident(model: &Model, face: usize, edge: usize) -> bool {
    let f = &model.faces[face];
    std::iter::once(f.outer).chain(f.holes.iter().copied()).any(|w|
        model.loops[w].coedges.iter().any(|c| c.edge == edge))
}
fn plan_cuts(
    roots: &contact_interval::ContactInterval, cuts: &[ContactFaceCut],
    tolerance_uv: f64, limits: ContactRetentionLimits,
) -> Result<RetentionNetwork> {
    let original = roots.model();
    if cuts.len() != 2 || cuts[0].face == cuts[1].face {
        return Err(refuse("Need one cut on each of the two distinct original supports"));
    }
    for cut in cuts {
        if cut.face >= original.faces.len() || !incident(original, cut.face, roots.middle_edge())
            || !(1..=256).contains(&cut.curves.len()) {
            return Err(refuse("Contact cuts must use the two actual root-edge supports"));
        }
    }
    let owned = cuts.iter().map(|c| {
        let ownership=roots.branch().contacts.iter().find(|o|
            o.face == original.1.faces[c.face]).ok_or_else(||
                refuse("Cut face has no original contact ownership"))?;
        let mut endpoints=roots.endpoints();
        if ownership.contact_reversed { endpoints.reverse(); }
        Ok([OwnedCut { endpoints, curves: c.curves.clone() }])
    }).collect::<Result<Vec<_>>>()?;
    let requests = cuts.iter().zip(&owned).map(|(c, o)| FaceRetention {
        face: c.face, cuts: o, keep_uv: &c.keep_uv,
    }).collect::<Vec<_>>();
    let remaining = limits.roots.refinement.max_cells.saturating_sub(roots.cells());
    if remaining == 0 { return Err(refuse("Contact roots exhausted the material-cut cell ledger")); }
    let mut plan = super::plan_retention_network(original, &requests, tolerance_uv,
        remaining, limits.cut_exact_work, limits.domain_cells)?;
    for region in &plan.regions {
        if region.discard_faces.len() != 1 || region.retained_faces.is_empty()
            || !incident(&plan.model, region.discard_faces[0], roots.middle_edge())
            || region.retained_faces.iter().any(|&f| incident(&plan.model, f, roots.middle_edge())) {
            return Err(refuse("The discarded sector must be the unique middle-edge side on each support"));
        }
        // A wider contact may extend beyond a common root, but its connectors
        // must not discard either original source-edge piece outside the roots.
        for &edge in roots.partition().edges() {
            if edge == roots.middle_edge() { continue; }
            if region.discard_faces.iter().any(|&f| incident(&plan.model,f,edge))
                || !region.retained_faces.iter().any(|&f| incident(&plan.model,f,edge)) {
                return Err(refuse("Contact cut does not retain an outside source-edge piece"));
            }
        }
        if region.descendants.iter().any(|&f|
            plan.model.faces[f].surface != original.faces[region.source_face].surface) {
            return Err(refuse("Original support coefficients changed during rooted trimming"));
        }
    }
    plan.model.refresh_change_set(&[roots.partition().source()]);
    Ok(plan)
}
/// Root ownership is recomputed first. The constructor receives read-only
/// actual shared vertices and may propose connector/contact paths and strict
/// retained-side witnesses. Both source faces are cut privately; no candidate
/// escapes when either cut or material-side classification is incomplete.
/// Roots bound the guaranteed common source interval. Complete contacts may
/// have unequal extents; their actual connector paths must still pass fresh
/// source projection and partition admission. Both outside source-edge pieces
/// must remain on retained sectors. No contact is restricted or refitted.
pub fn plan_contact_retention<F>(
    source: &Model, shell: usize, request: EdgeRadius, skin: &Surface,
    contacts: [&Contact; 2], key: &str, tolerance_uv: f64,
    limits: ContactRetentionLimits, construct: F,
) -> Result<ContactRetention>
where F: FnOnce(&contact_interval::ContactInterval) -> Result<Vec<ContactFaceCut>> {
    if limits.cut_exact_work > 1_000_000 || !(1..=100_000).contains(&limits.domain_cells) {
        return Err(refuse("Need bounded contact material planning work"));
    }
    let roots = contact_interval::between_contacts(source, shell, request, skin,
        contacts, key, tolerance_uv, limits.roots)?;
    let cuts = construct(&roots)?;
    for (side, ownership) in roots.branch().contacts.iter().enumerate() {
        let Some(cut) = cuts.iter().find(|c|
            roots.model().1.faces.get(c.face) == Some(&ownership.face)) else {
            return Err(refuse("A root-owned support contact is missing from the proposed cuts"));
        };
        if !cut.curves.iter().any(|c| c == &contacts[side].curve) {
            return Err(refuse("Actual complete contact curve is missing or was refitted"));
        }
    }
    let plan = plan_cuts(&roots, &cuts, tolerance_uv, limits)?;
    let cells = roots.cells() + plan.cells;
    Ok(ContactRetention { roots, plan, cuts, tolerance_uv, limits, cells })
}
