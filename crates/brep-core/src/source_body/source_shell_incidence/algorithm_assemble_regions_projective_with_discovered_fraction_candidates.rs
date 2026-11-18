//! source shell incidence algorithm: assemble_regions_projective_with_discovered_fraction_candidates.
use super::*;
/// Preserve material ownership while discovering exact rational root proposals.
pub fn assemble_regions_projective_with_discovered_fraction_candidates(
    regions: &[crate::source_contour_proposal::SourceRegion],
    inputs: ProjectiveInputs<'_>,
    discovery: FractionDiscovery,
    work: AssemblyWork,
) -> Result<Report<crate::source_projective_edge::Edge>> {
    if !(2..=4096).contains(&regions.len()) {
        return Err(error("Choose bounded qualified source regions"));
    }
    let faces = regions
        .iter()
        .map(|r| r.world_wires())
        .collect::<Result<Vec<_>>>()?;
    let mut report = assemble_projective_with_discovered_fraction_candidates(&faces, inputs, discovery, work)?;
    if let Some(shell) = report.shell.as_mut() {
        shell.regions = Some(regions.to_vec());
    }
    Ok(report)
}
