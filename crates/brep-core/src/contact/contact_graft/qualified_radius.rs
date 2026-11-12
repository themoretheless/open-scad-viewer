//! Atomic saved-skin radius gate after full owned graft admission.
use super::*;
pub struct RadiusQualifiedGraft {
    pub graft: Graft,
    pub radius: nurbs_core::skin_radius::Report,
}
/// Which patch vertices are grafted onto which source targets.
#[derive(Clone, Copy)]
pub struct GraftMapping<'a> {
    pub vertices: &'a [usize],
    pub targets: &'a [Target],
}
/// UV and radius (mm) tolerances of the radius gate.
#[derive(Clone, Copy, Debug)]
pub struct RadiusBounds {
    pub uv: f64,
    pub radius_mm: f64,
}
/// Assemble privately, then inspect the actual saved skin against explicit
/// affine world fields. Returns no intermediate model on failed radius proof.
/// Contact/G2/global embedding remain distinct from whole-skin radial error.
pub fn attach_radius_qualified(
    source: &Model,
    patch: &Model,
    mapping: GraftMapping<'_>,
    law: RadiusLaw,
    bounds: RadiusBounds,
    work: GraftWork,
) -> Result<RadiusQualifiedGraft> {
    let GraftMapping { vertices, targets } = mapping;
    let RadiusLaw { center, radius } = law;
    let RadiusBounds { uv: tolerance_uv, radius_mm: radius_tolerance_mm } = bounds;
    let GraftWork { max_cells, max_exact, max_coefficients } = work;
    let graft = attach(
        source,
        patch,
        vertices,
        targets,
        crate::contact_graft::ContactWork {
            tolerance_uv,
            max_cells,
            max_exact,
        },
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
/// Which patch vertices and edges are grafted onto which source targets.
#[derive(Clone, Copy)]
pub struct GraftTopology<'a> {
    pub vertices: &'a [usize],
    pub targets: &'a [Target],
    pub patch_contacts: &'a [usize],
}
/// The rolling-ball law: cubic center and radius coefficients.
#[derive(Clone, Copy, Debug)]
pub struct RadiusLaw {
    pub center: [[f64; 4]; 3],
    pub radius: [f64; 4],
}
/// UV, radius (mm) and normal tolerances of the radius and G1 gate.
#[derive(Clone, Copy, Debug)]
pub struct RadiusTolerance {
    pub uv: f64,
    pub radius_mm: f64,
    pub normal: f64,
}
/// Cell, exact-work and coefficient limits shared by assembly and contact checks.
#[derive(Clone, Copy, Debug)]
pub struct GraftWork {
    pub max_cells: usize,
    pub max_exact: u64,
    pub max_coefficients: usize,
}
/// Radius and G1 gate over explicit PATCH edge indices, mapped to the actual
/// saved graft edges. Other edges (including sharp caps) are outside this scope.
/// All assembly/contact cells and exact work share the original caller limits.
pub fn attach_radius_tangent_qualified(
    source: &Model,
    patch: &Model,
    topology: GraftTopology<'_>,
    law: RadiusLaw,
    tolerance: RadiusTolerance,
    work: GraftWork,
) -> Result<RadiusTangentGraft> {
    let GraftTopology { vertices, targets, patch_contacts } = topology;
    let RadiusLaw { center, radius } = law;
    let RadiusTolerance { uv: tolerance_uv, radius_mm: radius_tolerance_mm, normal: normal_tolerance } = tolerance;
    let GraftWork { max_cells, max_exact, max_coefficients } = work;
    let unique: std::collections::BTreeSet<_> = patch_contacts.iter().copied().collect();
    if patch_contacts.is_empty()
        || unique.len() != patch_contacts.len()
        || patch_contacts.iter().any(|&e| e >= patch.edges.len())
    {
        return Err(invalid(
            "Radius/G1 gate needs a distinct explicit patch contact scope",
        ));
    }
    let qualified = attach_radius_qualified(source, patch, GraftMapping { vertices, targets }, RadiusLaw { center, radius }, RadiusBounds { uv: tolerance_uv, radius_mm: radius_tolerance_mm }, GraftWork { max_cells, max_exact, max_coefficients })?;
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
