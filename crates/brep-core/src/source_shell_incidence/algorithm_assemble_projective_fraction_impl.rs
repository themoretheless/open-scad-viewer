//! source shell incidence algorithm: assemble_projective_fraction_impl.
use super::*;
pub fn assemble_projective_fraction_impl(
    faces: &[Vec<Wire>],
    pairs: &[Pair],
    planes: &[RootPlanes],
    maps: &[ProjectiveMaps],
    candidates: &[RootCandidates],
    fraction_candidates: &[RootFractionCandidates],
    poles: &[Pole],
    discovery: Option<[usize; 2]>,
    max_work: u64,
    max_driver_cells: usize,
) -> Result<Report<crate::source_projective_edge::Edge>> {
    let mut fractions = BTreeMap::new();
    for proposal in fraction_candidates {
        if proposal.pair >= pairs.len()
            || fractions
                .insert(proposal.pair, proposal.candidates)
                .is_some()
            || proposal
                .candidates
                .iter()
                .flatten()
                .flatten()
                .flatten()
                .any(|q| q.iter().any(|v| !v.is_finite()) || q[1] <= 0.)
        {
            return Err(error(
                "Require unique addressed finite rational root candidates",
            ));
        }
    }
    let mut weights = BTreeMap::new();
    for map in maps {
        if map.pair >= pairs.len()
            || weights.insert(map.pair, map.weights).is_some()
            || map
                .weights
                .iter()
                .flatten()
                .any(|w| !w.is_finite() || *w <= 0.)
        {
            return Err(error("Require unique positive projective map weights"));
        }
    }
    if pairs.iter().any(|p| p.cutters.iter().any(Option::is_some)) {
        return Err(error(
            "Projective incidence requires original root/plane inputs rather than cutter hints",
        ));
    }
    let affine: Vec<_> = maps
        .iter()
        .map(|m| AffineMaps {
            pair: m.pair,
            ranges: m.ranges,
        })
        .collect();
    assemble_core(
        faces,
        pairs,
        planes,
        &affine,
        candidates,
        poles,
        max_work,
        max_driver_cells,
        |input, work, driver| {
            let full = std::array::from_fn(|side| {
                if input.pair.world_reversed[side] ^ input.sources[side].reversed() {
                    [[1., 1.], [0., 1.]]
                } else {
                    [[0., 1.], [1., 1.]]
                }
            });
            let ranges = input.ranges.unwrap_or(full);
            let weights = weights.get(&input.index).copied().unwrap_or([[1., 1.]; 2]);
            let candidates = input.candidates.unwrap_or([[None; 2]; 2]);
            let r = if let Some(fractions) = fractions.get(&input.index) {
                crate::source_projective_edge::qualify_with_fraction_candidates(
                    &input.pair.world,
                    input.sources,
                    ranges,
                    weights,
                    input.planes,
                    candidates,
                    *fractions,
                    work,
                    driver,
                )?
            } else if let Some([denominator, count]) = discovery {
                crate::source_projective_edge::qualify_with_discovered_fraction_candidates(
                    &input.pair.world,
                    input.sources,
                    ranges,
                    weights,
                    input.planes,
                    candidates,
                    denominator,
                    count,
                    work,
                    driver,
                )?
            } else {
                crate::source_projective_edge::qualify(
                    &input.pair.world,
                    input.sources,
                    ranges,
                    weights,
                    input.planes,
                    candidates,
                    work,
                    driver,
                )?
            };
            Ok(EdgeReport {
                edge: r.edge,
                work_used: r.exact_work,
                root_checks: r.root_checks,
                driver_cells: r.driver_cells,
                reason: r.reason,
            })
        },
    )
}
