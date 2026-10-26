//! Atomic supported section, winding selection, and owned cap construction.
use super::{invalid, unproven};
use crate::{Model, face_domain::FaceDomain, region_replacement};
use nurbs_core::{Result, trim_domain::Location};

#[derive(Clone, Copy)]
pub struct CapSelection {
    /// Original face and a point strictly inside its sector to remove.
    pub face: usize,
    pub remove_uv: [f64; 2],
}
pub struct AutomaticCap {
    pub cap: region_replacement::SurfaceCap,
    /// Removed sector indices in the private partition, for diagnostics only.
    pub selected_sectors: Vec<usize>,
    pub cells: usize,
    pub exact_work: u64,
}
/// All construction remains private until a closed, trim-audited result exists.
/// Uses the supported ruled/plane section solver, not an arbitrary surface
/// intersection solver. Seeds choose material intent, never certify a sector.
/// Uncut faces belonging to the region must be supplied explicitly.
/// No radius, smoothness, or global embedding certificate is implied.
pub fn trim_ruled_and_cap(
    source: &Model,
    selections: &[CapSelection],
    remove_uncut: &[usize],
    cutter: &Model,
    cutter_face: usize,
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<AutomaticCap> {
    source.validate()?;
    let faces = selections.iter().map(|s| s.face).collect::<Vec<_>>();
    let mut seen = std::collections::BTreeSet::new();
    if selections.is_empty()
        || selections.len() + remove_uncut.len() > 256
        || !(16..=100000).contains(&max_cells)
        || max_exact > 1000000
        || selections.iter().any(|s| {
            s.face >= source.faces.len()
                || !seen.insert(s.face)
                || s.remove_uv.iter().any(|v| !v.is_finite())
        })
        || remove_uncut.iter().any(|&f| f >= source.faces.len() || !seen.insert(f))
    {
        return Err(invalid("Need distinct original faces and finite removal seeds"));
    }
    let partition = super::partition_ruled_network(
        source, &faces, cutter, cutter_face, tolerance_uv, max_cells, max_exact,
    )?;
    let mut cells = partition.cells;
    let mut selected = remove_uncut.to_vec();
    for (request, section) in selections.iter().zip(&partition.sections) {
        let mut owner = None;
        for &face in &section.faces {
            if cells >= max_cells {
                return Err(unproven("Shared automatic cap selection work exhausted"));
            }
            let r = FaceDomain::new(&partition.model, face, tolerance_uv)?
                .classify(request.remove_uv.map(|v| [v, v]), max_cells - cells)?;
            cells += r.cells;
            match r.location {
                Location::Inside if r.winding == Some(1) => {
                    if owner.replace(face).is_some() {
                        return Err(unproven("Removal seed has multiple sector owners"));
                    }
                }
                Location::Outside => {}
                _ => {
                    return Err(unproven(
                        "Removal seed is on or unresolved near a cut boundary",
                    ));
                }
            }
        }
        selected.push(owner.ok_or_else(|| unproven("Removal seed owns no cut sector"))?);
    }
    if max_cells.saturating_sub(cells) < 8 {
        return Err(unproven("Shared automatic cap closure work exhausted"));
    }
    let cap = region_replacement::cap_region(
        &partition.model, &selected, &cutter.faces[cutter_face].surface,
        tolerance_uv, max_cells - cells, max_exact - partition.exact_work,
    )?;
    cells += cap.cells;
    let exact_work = partition.exact_work + cap.replacement.exact_work;
    Ok(AutomaticCap { cap, selected_sectors: selected, cells, exact_work })
}
