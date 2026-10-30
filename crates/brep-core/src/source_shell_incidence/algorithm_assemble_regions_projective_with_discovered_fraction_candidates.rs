//! source shell incidence algorithm: assemble_regions_projective_with_discovered_fraction_candidates.
use super::*;
/// Preserve material ownership while discovering exact rational root proposals.
pub fn assemble_regions_projective_with_discovered_fraction_candidates(
    regions: &[crate::source_contour_proposal::SourceRegion],
    pairs: &[Pair],
    planes: &[RootPlanes],
    maps: &[ProjectiveMaps],
    candidates: &[RootCandidates],
    poles: &[Pole],
    max_denominator: usize,
    max_candidates_per_axis: usize,
    max_work: u64,
    max_driver_cells: usize,
) -> Result<Report<crate::source_projective_edge::Edge>> {
    if !(2..=4096).contains(&regions.len()) {
        return Err(error("Choose bounded qualified source regions"));
    }
    let faces = regions
        .iter()
        .map(|r| r.world_wires())
        .collect::<Result<Vec<_>>>()?;
    let mut report = assemble_projective_with_discovered_fraction_candidates(
        &faces,
        pairs,
        planes,
        maps,
        candidates,
        poles,
        max_denominator,
        max_candidates_per_axis,
        max_work,
        max_driver_cells,
    )?;
    if let Some(shell) = report.shell.as_mut() {
        shell.regions = Some(regions.to_vec());
    }
    Ok(report)
}
