//! One certified material component for an entire Cartesian source enclosure.
use super::material_seed::SurfaceMaterialLimits;
use super::*;
use nurbs_core::surface_distance;
mod trimmed;
pub(super) struct Material {
    pub witness: [f64; 3],
    pub inside: bool,
    pub distance_cells: usize,
    pub domain_cells: usize,
    pub parity_cells: usize,
    pub parity_domain_cells: usize,
}
fn unproved(s: &str) -> Error {
    Error::new("BREP_MATERIAL_ENCLOSURE_UNPROVED", s)
}
pub(super) fn qualify(
    solid: &QualifiedClosedSolid,
    bounds: [[f64; 2]; 3],
    directions: &[[f64; 3]],
    limits: SurfaceMaterialLimits,
    max_trim_cells: usize,
) -> Result<Material> {
    if directions.is_empty()
        || directions.len() > 16
        || directions
            .iter()
            .any(|d| !d.iter().all(|v| v.is_finite()) || d.iter().all(|v| *v == 0.))
    {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Material enclosure needs 1..16 finite nonzero parity directions",
        ));
    }
    let witness = bounds.map(|d| (d[0] * 0.5 + d[1] * 0.5).max(d[0]).min(d[1]));
    let radius = surface_distance::enclosure_distance(&bounds, &witness.map(|x| [x; 2]))?.1;
    let mut cells = 0;
    let mut domain_cells = 0;
    for (i, face) in solid.model.faces.iter().enumerate() {
        for (axis, periodic) in [face.surface.periodic_u, face.surface.periodic_v]
            .into_iter()
            .enumerate()
        {
            if periodic
                && nurbs_core::periodic_trim::verify_surface_seam_identity(&face.surface, axis)?
                    .is_none()
            {
                return Err(unproved(
                    "Original material boundary has no exact native periodic seam identity",
                ));
            }
        }
        let remaining = limits.distance_cells - cells;
        if remaining == 0 {
            return Err(unproved("Original boundary separation work exhausted"));
        }
        let share = (remaining / (solid.model.faces.len() - i)).max(1);
        let fast = if max_trim_cells > 0 {
            (share / 2).max(1)
        } else {
            share
        }
        .min(100000);
        let distance = surface_distance::nearest_point(
            &face.surface,
            witness,
            solid.model.tolerance_mm,
            fast,
        )?;
        cells += distance.cells;
        if distance.distance_interval[0] > radius {
            continue;
        }
        if max_trim_cells == 0 || domain_cells == max_trim_cells || share <= distance.cells {
            return Err(unproved(
                "Source enclosure is not separated from the closed material boundary",
            ));
        }
        let work = trimmed::exclude(
            solid,
            i,
            witness,
            radius,
            share - distance.cells,
            max_trim_cells - domain_cells,
        )?;
        cells += work[0];
        domain_cells += work[1];
    }
    let report = solid.classify_point(
        witness,
        directions,
        limits.parity_cells,
        limits.parity_domain_cells,
    )?;
    let inside = report
        .parity
        .ok_or_else(|| unproved("Material enclosure witness parity remains unresolved"))?;
    Ok(Material {
        witness,
        inside,
        distance_cells: cells,
        domain_cells,
        parity_cells: report.cells,
        parity_domain_cells: report.domain_cells,
    })
}
