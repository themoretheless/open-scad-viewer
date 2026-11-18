use super::*;
use crate::mixed_root_model::closed_cut::jordan::census::sew::volume::cavity::coverage::{
    roots, split,
};
pub(super) struct Cover {
    pub inside: bool,
    pub work: RootRelationWork,
    _seed: QualifiedRootMaterialBox,
    _patches: Vec<QualifiedRootSurfaceMaterialBox>,
}
pub(super) fn qualify(
    source: &Arc<QualifiedPairedRootSolid>,
    target: &Arc<QualifiedPairedRootSolid>,
    directions: &[[f64; 3]],
    limits: RootRelationLimits,
) -> Result<Cover> {
    let p = limits.material.material.point;
    if !(1..=64).contains(&p.arc_subdivisions)
        || !p.arc_subdivisions.is_power_of_two()
        || p.band_queries > 1000000
        || p.winding_queries > 65536
    {
        return Err(unproved("Invalid root relation domain allowance"));
    }
    // This is a source-owned exact boundary selector, not a fitted vertex or
    // the midpoint of an unproved source image. Its whole enclosure is tested.
    let Some(vertex) = source.model().topology().vertices.first() else {
        return Err(unproved("Root relation source has no boundary selector"));
    };
    let MixedVertex::Root(point) = &vertex.point else {
        return Err(unproved("Root relation source contains an unowned vertex"));
    };
    let bounds = point.enclose(p.geometry)?.world;
    let seed = target.qualify_material_box(bounds, directions, limits.material.material)?;
    let inside = seed.inside();
    let dl = RootBoundaryVolumeLimits {
        arc_subdivisions: p.arc_subdivisions,
        band_queries: p.band_queries,
        winding_queries: p.winding_queries,
        winding: p.winding,
        geometry: p.geometry,
        ..Default::default()
    };
    let mut dw = RootBoundaryVolumeReport {
        signed_bounds: None,
        accuracy_proven: false,
        cells: 0,
        spans: 0,
        band_queries: 0,
        winding_queries: 0,
        seeds: 0,
        propagations: 0,
        reason: "root-relation-domain-work",
    };
    let mut region = domain::Domains::new(source.boundary(), dl, &mut dw)?
        .ok_or_else(|| unproved("Root relation source boundary bands incomplete"))?;
    let mut work = RootRelationWork {
        cells: 0,
        accepted: 0,
        domain_queries: 0,
        material_attempts: 1,
        band_queries: 0,
        winding_queries: 0,
    };
    let mut patches = Vec::new();
    for (face, f) in source.model().topology().faces.iter().enumerate() {
        let mut pending = roots(&f.surface, Some(region.roots[face]), limits.coverage_cells)?;
        while let Some((d, depth)) = pending.pop_front() {
            if work.cells == limits.coverage_cells {
                return Err(unproved("Root relation complete source cover exhausted"));
            }
            work.cells += 1;
            work.material_attempts += 1;
            let attempt =
                target.qualify_surface_material_box(&f.surface, d, directions, limits.material);
            let contradictory = match attempt {
                Ok(proof) if proof.inside() == inside => {
                    patches.push(proof);
                    work.accepted += 1;
                    continue;
                }
                Ok(_) => true,
                Err(_) => false,
            };
            if work.domain_queries < limits.domain_queries {
                work.domain_queries += 1;
                let mut q = region.classify(face, d, dl, &mut dw)?;
                if q.is_none() {
                    q = region.classify_box(face, d, dl, &mut dw)?;
                }
                if q == Some(false) {
                    continue;
                }
                if q == Some(true) && contradictory {
                    return Err(unproved(
                        "Retained root boundary has inconsistent material signatures",
                    ));
                }
            }
            split(&mut pending, d, depth)?;
        }
    }
    if work.accepted == 0 {
        return Err(unproved("Root relation has no complete material cover"));
    }
    work.band_queries = dw.band_queries;
    work.winding_queries = dw.winding_queries;
    Ok(Cover {
        inside,
        work,
        _seed: seed,
        _patches: patches,
    })
}
