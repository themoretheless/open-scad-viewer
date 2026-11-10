//! Actual supporting-surface correspondence for owned coherent UV regions.
use super::*;
use crate::Surface;
use periodic::{SurfaceCorrespondence, SurfaceCorrespondenceLimits};
mod faces;
pub use faces::{OwnedPeriodicFaces, PeriodicFaceLimits, assemble_periodic_surface_faces};
#[derive(Clone, Debug)]
pub struct SurfacePeriodicRegions {
    surface: Surface,
    wrapped: [bool; 2],
    regions: CoherentPeriodicWindingRegions,
    limits: SurfaceCorrespondenceLimits,
    correspondence: SurfaceCorrespondence,
}
impl SurfacePeriodicRegions {
    pub fn surface(&self) -> &Surface {
        &self.surface
    }
    pub fn wrapped(&self) -> [bool; 2] {
        self.wrapped
    }
    pub fn regions(&self) -> &CoherentPeriodicWindingRegions {
        &self.regions
    }
    pub fn correspondence(&self) -> &SurfaceCorrespondence {
        &self.correspondence
    }
    /// Fresh original UV topology, actual supporting-surface seam/error
    /// qualification and retained candidate boundary roles. No face claim.
    pub fn reverify_source(&self) -> Result<()> {
        self.regions.reverify_source()?;
        let fresh = correspondence(&self.surface, &self.regions, self.wrapped, self.limits)?;
        if fresh.seams != self.correspondence.seams
            || fresh.error_upper != self.correspondence.error_upper
            || !self.correspondence.within_world_tolerance
        {
            return Err(refused("Supporting-surface correspondence receipt changed"));
        }
        Ok(())
    }
}
fn correspondence(
    surface: &Surface,
    regions: &CoherentPeriodicWindingRegions,
    wrapped: [bool; 2],
    limits: SurfaceCorrespondenceLimits,
) -> Result<SurfaceCorrespondence> {
    let report = periodic::verify_surface_correspondence(
        surface,
        &regions.source,
        &regions.partitions,
        regions.chart,
        wrapped,
        limits,
    )?;
    if !report.within_world_tolerance {
        return Err(refused(&format!(
            "Actual supporting-surface source error or seam remains unproved: {:?}, error upper {}",
            report.reason, report.error_upper
        )));
    }
    let graph = admitted(&regions.assembly)?;
    // Only retained region boundaries are published. Removed internal borders
    // need not become physical supporting-surface boundaries.
    for component in regions.regions.components() {
        for &wire in component.source_loops() {
            for &edge in &regions.cycle_edges[wire] {
                let e = &graph.edges[edge];
                if e.contributions.iter().any(|p| p.source.is_none())
                    && (0..2).any(|a| {
                        !wrapped[a]
                            && (0..2).any(|side| {
                                e.curve
                                    .control_points
                                    .iter()
                                    .all(|p| p[a] == regions.chart[a][side])
                            })
                    })
                {
                    return Err(refused(
                        "Retained periodic connector lies on a nonwrapped surface axis",
                    ));
                }
            }
        }
    }
    Ok(report)
}
/// Bind already owned coherent candidate UV regions to the actual saved
/// surface. Original/candidate world deviation, exact chart domains, wrapped
/// seam identity and retained connector axes are recomputed independently.
/// Both source context and surface are owned. Surface regularity/injectivity,
/// lifted boundary curves, sewn topology and 3D face admission are separate.
pub fn bind_periodic_regions_to_surface(
    surface: &Surface,
    regions: &CoherentPeriodicWindingRegions,
    wrapped: [bool; 2],
    limits: SurfaceCorrespondenceLimits,
) -> Result<SurfacePeriodicRegions> {
    regions.reverify_source()?;
    let correspondence = correspondence(surface, regions, wrapped, limits)?;
    Ok(SurfacePeriodicRegions {
        surface: surface.clone(),
        wrapped,
        regions: regions.clone(),
        limits,
        correspondence,
    })
}
