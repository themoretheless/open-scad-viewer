//! General certificate for an authored rational boundary and its retained body.
//! Frame orientation and filled cap shape impose no special cases. Every face
//! is compared on its complete common UV region, including holes and cap interiors.
//! A source motion law is not inferred from an authored rational reference.
use crate::{Error, Model, Result, boundary_embedding::regularity, volume_validity};
use nurbs_core::surface_correspondence;

#[derive(Clone, Copy)]
pub struct Limits {
    /// Shared by every reference/retained face comparison.
    pub correspondence: surface_correspondence::Limits,
    /// Shared by both complete boundaries, never renewed per face or body.
    pub regularity_cells: usize,
    pub linear_cells_per_boundary: usize,
    /// Explicit independent stages; neither may borrow success from the other.
    pub reference_volume: volume_validity::Limits,
    pub retained_volume: volume_validity::Limits,
}
pub struct Report {
    pub certified: bool,
    pub continuous_bound: bool,
    pub error_upper: Option<f64>,
    pub within_budget: bool,
    pub correspondence_cells: usize,
    pub correspondence_products: usize,
    pub face_bounds: Vec<surface_correspondence::Report>,
    pub reference_regularity: Option<regularity::Report>,
    pub retained_regularity: Option<regularity::Report>,
    pub reference_volume: Option<volume_validity::Report>,
    pub retained_volume: Option<volume_validity::Report>,
    pub reason: Option<&'static str>,
}
fn common_regions(a: &Model, b: &Model) -> bool {
    // An explicit index correspondence owns the same complete trimmed domains.
    // Equal edge incidence and shell roles bind that correspondence globally.
    a.vertices.len() == b.vertices.len()
        && a.edges.len() == b.edges.len()
        && a.loops == b.loops
        && a.shells == b.shells
        && a.bodies == b.bodies
        && a.edges
            .iter()
            .zip(&b.edges)
            .all(|(x, y)| x.vertices == y.vertices && x.degenerate == y.degenerate)
        && a.faces.len() == b.faces.len()
        && a.faces
            .iter()
            .zip(&b.faces)
            .all(|(x, y)| x.outer == y.outer && x.holes == y.holes)
}
/// Recomputes all evidence from borrowed original inputs; accepts no supplied
/// certificate flags or cached successful stages. Crease seams remain C0.
pub fn inspect(
    reference: &Model,
    retained: &Model,
    tolerance_uv: f64,
    error_budget: f64,
    limits: Limits,
) -> Result<Report> {
    reference.validate()?;
    retained.validate()?;
    if !error_budget.is_finite()
        || error_budget < 0.
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || limits.correspondence.cells > 100000
        || limits.correspondence.products > 1000000
        || limits.regularity_cells > 100000
        || limits.linear_cells_per_boundary > 100000
    {
        return Err(Error::new(
            "BREP_BOUNDARY_CERTIFICATE_INVALID",
            "Invalid certificate tolerance/work limits",
        ));
    }
    let mut out = Report {
        certified: false,
        continuous_bound: false,
        error_upper: None,
        within_budget: false,
        correspondence_cells: 0,
        correspondence_products: 0,
        face_bounds: Vec::new(),
        reference_regularity: None,
        retained_regularity: None,
        reference_volume: None,
        retained_volume: None,
        reason: Some("boundary-correspondence-mismatch"),
    };
    if !common_regions(reference, retained) || reference.faces.is_empty() {
        return Ok(out);
    }
    let mut maximum = 0_f64;
    let mut complete = true;
    for (a, b) in reference.faces.iter().zip(&retained.faces) {
        let proof = surface_correspondence::inspect(
            &a.surface,
            &b.surface,
            error_budget,
            surface_correspondence::Limits {
                cells: limits.correspondence.cells - out.correspondence_cells,
                products: limits.correspondence.products - out.correspondence_products,
            },
        )?;
        out.correspondence_cells += proof.cells;
        out.correspondence_products += proof.products;
        if let Some(bound) = proof.error_upper {
            maximum = maximum.max(bound);
        } else {
            complete = false;
        }
        out.face_bounds.push(proof);
    }
    if complete {
        out.continuous_bound = true;
        out.error_upper = Some(maximum);
        out.within_budget = maximum <= error_budget;
    }
    // A complete positional bound can coexist with a folded/self-intersecting
    // boundary. Publish these distinct facts; only the conjunction certifies.
    let reference_regularity = regularity::inspect(reference, limits.regularity_cells)?;
    let retained_regularity = regularity::inspect(
        retained,
        limits.regularity_cells - reference_regularity.cells,
    )?;
    let regular = reference_regularity.all_faces_regular_modulo_poles
        && retained_regularity.all_faces_regular_modulo_poles;
    out.reference_regularity = Some(reference_regularity);
    out.retained_regularity = Some(retained_regularity);
    if !complete || !out.within_budget || !regular {
        out.reason = if !complete {
            Some("whole-boundary-error-unproved")
        } else if !out.within_budget {
            Some("boundary-error-budget-exceeded")
        } else {
            Some("whole-boundary-regularity-unproved")
        };
        return Ok(out);
    }
    let reference_volume = volume_validity::inspect_with_linear(
        reference,
        tolerance_uv,
        limits.reference_volume,
        limits.linear_cells_per_boundary,
    )?;
    let retained_volume = volume_validity::inspect_with_linear(
        retained,
        tolerance_uv,
        limits.retained_volume,
        limits.linear_cells_per_boundary,
    )?;
    let volumes = reference_volume.proven && retained_volume.proven;
    out.reference_volume = Some(reference_volume);
    out.retained_volume = Some(retained_volume);
    out.certified = out.continuous_bound && out.within_budget && regular && volumes;
    out.reason = if !complete {
        Some("whole-boundary-error-unproved")
    } else if !out.within_budget {
        Some("boundary-error-budget-exceeded")
    } else if !regular {
        Some("whole-boundary-regularity-unproved")
    } else if !volumes {
        Some("solid-geometry-unproved")
    } else {
        None
    };
    Ok(out)
}

mod loft;
pub use loft::{Attempt, CertifiedBody, Loft, construct};

#[cfg(test)]
mod tests;
