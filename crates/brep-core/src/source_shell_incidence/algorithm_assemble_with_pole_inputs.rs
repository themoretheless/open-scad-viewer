//! source shell incidence algorithm: assemble_with_pole_inputs.
use super::*;
pub fn assemble_with_pole_inputs(
    faces: &[Vec<Wire>],
    pairs: &[Pair],
    planes: &[RootPlanes],
    maps: &[AffineMaps],
    candidates: &[RootCandidates],
    poles: &[Pole],
    max_work: u64,
    max_driver_cells: usize,
) -> Result<Report> {
    assemble_core(
        faces,
        pairs,
        planes,
        maps,
        candidates,
        poles,
        max_work,
        max_driver_cells,
        |input, work, driver| {
            let p = input.pair;
            let report = if input.ranges.is_some() || input.candidates.is_some() {
                let full = p.world_reversed.map(|r| {
                    if r {
                        [[1., 1.], [0., 1.]]
                    } else {
                        [[0., 1.], [1., 1.]]
                    }
                });
                crate::source_mapped_edge::qualify_with_candidates(
                    &p.world,
                    input.sources,
                    input.ranges.unwrap_or(full),
                    input.planes,
                    input.candidates.unwrap_or([[None; 2]; 2]),
                    work,
                    driver,
                )?
            } else {
                source_shared_edge::qualify_with_cutters_and_planes(
                    &p.world,
                    input.sources,
                    p.world_reversed,
                    [p.cutters[0].as_ref(), p.cutters[1].as_ref()],
                    input.planes,
                    work,
                    driver,
                )?
            };
            Ok(EdgeReport {
                edge: report.edge,
                work_used: report.work_used,
                root_checks: report.root_checks,
                driver_cells: report.driver_cells,
                reason: report.reason,
            })
        },
    )
}
