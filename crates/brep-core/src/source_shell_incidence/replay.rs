//! Fresh material regions and canonical incidence with one shared exact budget.
use super::*;
use crate::{source_contour_proposal::SourceRegion, source_shell_restore::Limits};
fn invalid(message: &str) -> Error {
    Error::new("BREP_SOURCE_SHELL_REPLAY", message)
}
fn regions<E>(shell: &Shell<E>, limits: &Limits, projective: bool) -> Result<Vec<SourceRegion>> {
    let source = shell
        .regions()
        .ok_or_else(|| invalid("Qualified source material regions required"))?;
    if !(2..=4096).contains(&source.len())
        || shell.edges.len() != shell.uses.len()
        || shell.edges.len() > if projective { 32768 } else { 524288 }
        || shell.poles.len() > if projective { 65536 } else { 1048576 }
        || !(1..=100_000_000).contains(&limits.exact_work)
        || limits.driver_cells > 100000
    {
        return Err(invalid(
            "Bound source regions, pairs, poles and exact/driver work",
        ));
    }
    source
        .iter()
        .map(|r| r.requalify(&limits.regions))
        .collect()
}
fn recorded(
    regions: &[SourceRegion],
    addresses: [Address; 2],
    fragments: [&crate::source_boundary_fragment::Fragment; 2],
    mapping: usize,
) -> Result<()> {
    for (address, fragment) in addresses.into_iter().zip(fragments) {
        let actual = regions
            .get(address.face)
            .and_then(|r| r.loops().get(address.wire))
            .and_then(|w| w.get(address.edge))
            .ok_or_else(|| invalid("Unknown source use address"))?;
        let fresh = fragment.requalify(mapping)?;
        if !actual.same_definition(&fresh) {
            return Err(invalid(
                "Recorded edge use disagrees with replayed material boundary",
            ));
        }
    }
    Ok(())
}
fn poles<E>(shell: &Shell<E>) -> Vec<Pole> {
    shell
        .poles
        .iter()
        .map(|(a, p)| Pole {
            use_: *a,
            point: p.point(),
        })
        .collect()
}
impl Shell {
    /// Re-admit original material regions, all recorded uses and shared edges.
    /// Cached face wires, vertex IDs and direction/range flags give no authority.
    pub fn requalify(&self, limits: &Limits) -> Result<Report> {
        let regions = regions(self, limits, false)?;
        let mut pairs = Vec::new();
        let mut planes = Vec::new();
        let mut maps = Vec::new();
        let mut candidates = Vec::new();
        for (index, (edge, &uses)) in self.edges.iter().zip(&self.uses).enumerate() {
            recorded(
                &regions,
                uses,
                edge.uses().each_ref(),
                limits.regions.mapping_cells,
            )?;
            let (world_reversed, cutters, root_planes) = match edge.original_recipe() {
                crate::source_shared_edge::OriginalRecipe::Direct {
                    world_reversed,
                    cutters,
                    planes,
                } => (*world_reversed, cutters.clone(), *planes),
                crate::source_shared_edge::OriginalRecipe::Mapped {
                    ranges,
                    planes,
                    candidates: roots,
                } => {
                    maps.push(AffineMaps {
                        pair: index,
                        ranges: *ranges,
                    });
                    candidates.push(RootCandidates {
                        pair: index,
                        candidates: *roots,
                    });
                    ([false; 2], [None, None], *planes)
                }
            };
            planes.push(RootPlanes {
                pair: index,
                planes: root_planes,
            });
            pairs.push(Pair {
                uses,
                world: edge.world().clone(),
                world_reversed,
                cutters,
            });
        }
        assemble_regions_with_pole_inputs(
            &regions,
            &pairs,
            &planes,
            &maps,
            &candidates,
            &poles(self),
            limits.exact_work,
            limits.driver_cells,
        )
    }
}
impl Shell<crate::source_projective_edge::Edge> {
    /// Fresh rational maps and endpoint candidates share one shell work budget.
    pub fn requalify(
        &self,
        limits: &Limits,
    ) -> Result<Report<crate::source_projective_edge::Edge>> {
        let regions = regions(self, limits, true)?;
        let mut pairs = Vec::new();
        let mut planes = Vec::new();
        let mut maps = Vec::new();
        let mut candidates = Vec::new();
        let mut fractions = Vec::new();
        for (index, (edge, &uses)) in self.edges.iter().zip(&self.uses).enumerate() {
            recorded(
                &regions,
                uses,
                edge.uses().each_ref().map(|u| u.fragment()),
                limits.regions.mapping_cells,
            )?;
            if edge.uses()[0].world() != edge.uses()[1].world() {
                return Err(invalid("Original projective worlds disagree"));
            }
            pairs.push(Pair {
                uses,
                world: edge.world().clone(),
                world_reversed: [false; 2],
                cutters: [None, None],
            });
            maps.push(ProjectiveMaps {
                pair: index,
                ranges: edge.uses().each_ref().map(|u| u.range()),
                weights: edge.uses().each_ref().map(|u| u.weights()),
            });
            let (raw_planes, roots, raw_fractions) = edge.original_inputs()?;
            planes.push(RootPlanes {
                pair: index,
                planes: raw_planes,
            });
            candidates.push(RootCandidates {
                pair: index,
                candidates: roots,
            });
            if let Some(raw) = raw_fractions {
                fractions.push(RootFractionCandidates {
                    pair: index,
                    candidates: raw,
                });
            }
        }
        assemble_regions_projective_with_fraction_candidates(
            &regions,
            &pairs,
            &planes,
            &maps,
            &candidates,
            &fractions,
            &poles(self),
            limits.exact_work,
            limits.driver_cells,
        )
    }
}
#[cfg(test)]
impl Shell {
    pub(crate) fn assert_native_replay_cache_and_missing_ownership(&self, limits: &Limits) {
        let report = self.requalify(limits).unwrap();
        let mut cached = report.shell.expect(report.reason);
        let expected = cached.vertices.clone();
        cached.vertices.clear();
        cached.faces.clear();
        let report = cached.requalify(limits).unwrap();
        assert_eq!(report.shell.expect(report.reason).vertices, expected);
        cached.uses.pop();
        assert!(cached.requalify(limits).is_err());
        if !self.poles.is_empty() {
            let report = self.requalify(limits).unwrap();
            let mut missing = report.shell.expect(report.reason);
            missing.poles.pop();
            assert!(missing.requalify(limits).is_err());
        }
    }
}
