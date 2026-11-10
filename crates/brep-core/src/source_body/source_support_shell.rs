//! Rebuild source regions and shared incidence from authored support topology.
//! Closed flags and Body records in the input confer no geometric authority.
use crate::source_shell_incidence::{Address, Pair, Pole};
use std::collections::BTreeMap;
pub struct Limits {
    pub tolerance_uv: f64,
    pub faces: usize,
    pub uses: usize,
    /// Independently bounded original-region work per face.
    pub regions: crate::trimmed_face_recipe::Limits,
    pub exact_work: u64,
}
pub fn prepare(
    model: &crate::Model,
    limits: Limits,
) -> crate::Result<crate::source_shell_incidence::Report> {
    prepare_using(model,limits,|regions,pairs,poles,work| {
        crate::source_shell_incidence::assemble_regions_with_poles(regions,pairs,poles,work)
    })
}
/// Rebuild authored support sheets using exact projective edge correspondence.
/// Pole contractions are proved from original surfaces; flags grant no authority.
pub fn prepare_projective(model:&crate::Model,limits:Limits)
    ->crate::Result<crate::source_shell_incidence::Report<crate::source_projective_edge::Edge>> {
    prepare_using(model,limits,|regions,pairs,poles,work| {
        crate::source_shell_incidence::assemble_regions_projective_with_poles(regions,pairs,&[],&[],&[],poles,work,0)
    })
}
fn prepare_using<E>(model:&crate::Model,limits:Limits,
    assemble:impl FnOnce(&[crate::source_contour_proposal::SourceRegion],&[Pair],&[Pole],u64)->nurbs_core::Result<crate::source_shell_incidence::Report<E>>,
)->crate::Result<crate::source_shell_incidence::Report<E>> {
    if !limits.tolerance_uv.is_finite()
        || limits.tolerance_uv <= 0.
        || !(1..=4096).contains(&limits.faces)
        || !(1..=100000).contains(&limits.uses)
        || !(1..=100_000_000).contains(&limits.exact_work)
    {
        return Err(crate::invalid(
            "Choose bounded source support work and positive UV tolerance",
        ));
    }
    if model.faces.len() > limits.faces
        || model.shells.len() != 1
        || model.shells[0].faces.len() != model.faces.len()
    {
        return Err(crate::invalid(
            "Source support assembly requires one complete connected shell",
        ));
    }
    let count = model.loops.iter().map(|w| w.coedges.len()).sum::<usize>();
    if count > limits.uses {
        return Err(crate::invalid("Source support use limit"));
    }
    model.validate()?;
    // Propose consistently oriented charts. Every proposed lift and shared
    // carrier is freshly checked below; reflected floats grant no authority.
    let oriented = crate::face_senses::canonical_face_senses(model)?;
    let model = &oriented;
    let mut regions = Vec::new();
    let mut pairs = Vec::new();
    let mut poles = Vec::new();
    let mut pending = BTreeMap::new();
    for (face, support) in model.faces.iter().enumerate() {
        let mut wires = Vec::new();
        for (wire, index) in std::iter::once(support.outer)
            .chain(support.holes.iter().copied())
            .enumerate()
        {
            let mut boundaries = Vec::new();
            for (edge, use_) in model.loops[index].coedges.iter().enumerate() {
                let address = Address { face, wire, edge };
                let original = &model.edges[use_.edge];
                let pcurve = use_.pcurve.clone();
                let reversed = use_.reversed;
                boundaries.push(crate::trimmed_face_recipe::Boundary {
                    curve: original.curve.clone(),
                    pcurve,
                    reversed,
                });
                if original.degenerate {
                    poles.push(Pole {
                        use_: address,
                        point: model.vertices[original.vertices[0]].point,
                    });
                } else if let Some((other, reversed)) = pending.remove(&use_.edge) {
                    pairs.push(Pair {
                        uses: [other, address],
                        world: original.curve.clone(),
                        world_reversed: [reversed, use_.reversed],
                        cutters: [None, None],
                    });
                } else {
                    pending.insert(use_.edge, (address, reversed));
                }
            }
            wires.push(boundaries);
        }
        let qualified = crate::source_contour_proposal::qualify_original_region(
            &cad_predicates::ToleranceContext::default_valid(),
            &support.surface,
            &wires,
            limits.tolerance_uv,
            crate::trimmed_face_recipe::Limits {
                pairs: limits.regions.pairs,
                region_cells: limits.regions.region_cells,
                domain_cells: limits.regions.domain_cells,
                agreement_cells: limits.regions.agreement_cells,
            },
        )?;
        let Some(region) = qualified.region else {
            return Err(crate::invalid(format!(
                "Source support face {face}: {}",
                qualified.audit.reason
            )));
        };
        regions.push(region);
    }
    if !pending.is_empty() {
        return Err(crate::invalid(
            "Source supports retain unpaired ordinary edges",
        ));
    }
    assemble(
        &regions,
        &pairs,
        &poles,
        limits.exact_work,
    )
    .map_err(|e| crate::Error::new(e.code, e.message))
}
#[cfg(test)]
#[path = "source_support_shell/tests/mod.rs"]
mod tests;
