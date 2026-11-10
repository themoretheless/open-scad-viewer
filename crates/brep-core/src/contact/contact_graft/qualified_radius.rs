//! Atomic saved-skin radius gate after full owned graft admission.
use super::*;
pub struct RadiusQualifiedGraft {
    pub graft: Graft,
    pub radius: nurbs_core::skin_radius::Report,
}
/// Assemble privately, then inspect the actual saved skin against explicit
/// affine world fields. Returns no intermediate model on failed radius proof.
/// Contact/G2/global embedding remain distinct from whole-skin radial error.
pub fn attach_radius_qualified(
    source: &Model,
    patch: &Model,
    vertices: &[usize],
    targets: &[Target],
    center: [[f64; 4]; 3],
    radius: [f64; 4],
    tolerance_uv: f64,
    radius_tolerance_mm: f64,
    max_cells: usize,
    max_exact: u64,
    max_coefficients: usize,
) -> Result<RadiusQualifiedGraft> {
    let graft = attach(
        source,
        patch,
        vertices,
        targets,
        tolerance_uv,
        max_cells,
        max_exact,
    )?;
    let radius = crate::fillet_radius::inspect_saved_skin(
        &graft.model,
        graft.patch_face,
        center,
        radius,
        radius_tolerance_mm,
        max_coefficients,
    )?;
    if !radius.within_tolerance {
        return Err(unproven(
            "Actual saved skin radius remains unproven after graft",
        ));
    }
    Ok(RadiusQualifiedGraft { graft, radius })
}

pub struct RadiusTangentGraft {
    pub qualified: RadiusQualifiedGraft,
    pub tangency: crate::sweep_smoothness::tangent::Report,
    pub cells: usize,
    pub exact_work: u64,
}
/// Radius and G1 gate over explicit PATCH edge indices, mapped to the actual
/// saved graft edges. Other edges (including sharp caps) are outside this scope.
/// All assembly/contact cells and exact work share the original caller limits.
pub fn attach_radius_tangent_qualified(
    source: &Model,
    patch: &Model,
    vertices: &[usize],
    targets: &[Target],
    patch_contacts: &[usize],
    center: [[f64; 4]; 3],
    radius: [f64; 4],
    tolerance_uv: f64,
    radius_tolerance_mm: f64,
    normal_tolerance: f64,
    max_cells: usize,
    max_exact: u64,
    max_coefficients: usize,
) -> Result<RadiusTangentGraft> {
    let unique: std::collections::BTreeSet<_> = patch_contacts.iter().copied().collect();
    if patch_contacts.is_empty()
        || unique.len() != patch_contacts.len()
        || patch_contacts.iter().any(|&e| e >= patch.edges.len())
    {
        return Err(invalid(
            "Radius/G1 gate needs a distinct explicit patch contact scope",
        ));
    }
    let qualified = attach_radius_qualified(
        source,
        patch,
        vertices,
        targets,
        center,
        radius,
        tolerance_uv,
        radius_tolerance_mm,
        max_cells,
        max_exact,
        max_coefficients,
    )?;
    let edges = patch_contacts
        .iter()
        .map(|&i| qualified.graft.patch_edges[i])
        .collect::<Vec<_>>();
    let tangency = crate::sweep_smoothness::tangent::inspect_edges(
        &qualified.graft.model,
        &edges,
        normal_tolerance,
        max_cells - qualified.graft.cells,
        max_exact - qualified.graft.exact_work,
    )?;
    if !tangency.within_tolerance {
        return Err(unproven(
            "Actual saved contact seam tangency remains unproven",
        ));
    }
    let cells = qualified.graft.cells + tangency.cells;
    let exact_work = qualified.graft.exact_work + tangency.exact_work;
    Ok(RadiusTangentGraft {
        qualified,
        tangency,
        cells,
        exact_work,
    })
}
