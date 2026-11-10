//! Atomic multi-region assembly over source-owned, already cut boundaries.
use super::*;
pub struct RegionPatch<'a> {
    /// Original source face indices, disjoint between patches.
    pub selected: &'a [usize],
    pub surface: &'a Surface,
    /// Coedges refer to ORIGINAL source edge indices, including shared rims.
    pub loops: &'a [Vec<Coedge>],
    pub reversed: bool,
}
pub struct NetworkReplacement {
    pub model: Model,
    pub patch_faces: Vec<usize>,
    pub source_faces: Vec<Option<usize>>,
    pub source_edges: Vec<Option<usize>>,
    pub source_vertices: Vec<Option<usize>>,
    pub interiors: Vec<nurbs_core::surface_linear_monotonicity::Report>,
    pub seams: crate::sweep_smoothness::geometric::Report,
    pub trims: crate::face_domain::TrimAudit,
    pub closure: crate::watertight::QualifiedReport,
    pub cells: usize,
    pub exact_work: u64,
}
fn compose(map: &mut [Option<usize>], next: &[Option<usize>]) {
    for index in map {
        *index = index.and_then(|i| next[i]);
    }
}
/// Replace all prepared sectors as one transaction. Intermediate models stay
/// private and are never returned on refusal. New surfaces keep original rim
/// identities; shared seams are inspected once on the FINAL saved geometry.
/// The source must already contain the intended cut/partition edges. This
/// does not infer new corner boundaries or certify global body embedding.
pub fn replace_g2_network(
    source: &Model,
    patches: &[RegionPatch<'_>],
    tolerance_uv: f64,
    tolerances: [f64; 3],
    max_cells: usize,
    max_exact: u64,
) -> Result<NetworkReplacement> {
    source.validate()?;
    if !(1..=16).contains(&patches.len())
        || !(4..=100000).contains(&max_cells)
        || max_exact > 1000000
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || tolerances.iter().any(|t| !t.is_finite() || *t < 0.)
        || tolerances[0] > source.tolerance_mm
        || tolerances[1] > 2.
    {
        return Err(refused(
            "Network replacement needs bounded finite work and unchanged tolerance",
        ));
    }
    let mut chosen = BTreeSet::new();
    let mut rim = BTreeSet::new();
    for patch in patches {
        patch.surface.validate()?;
        if patch.selected.is_empty()
            || patch.selected.len() > 256
            || patch
                .selected
                .iter()
                .any(|&f| f >= source.faces.len() || !chosen.insert(f))
            || patch.loops.is_empty()
            || patch.loops.len() > 16
            || patch.loops.iter().any(|l| l.len() < 2)
            || patch.loops.iter().map(Vec::len).sum::<usize>() > 256
        {
            return Err(refused(
                "Network sectors must be disjoint and have bounded owned rims",
            ));
        }
        if patch.surface.periodic_u
            || patch.surface.periodic_v
            || patch
                .surface
                .control_points
                .len()
                .checked_mul(patch.surface.control_points[0].len())
                .is_none_or(|n| n > 4096)
        {
            return Err(refused(
                "Network candidate needs a bounded nonperiodic chart",
            ));
        }
        for (knots, degree, n) in [
            (
                &patch.surface.knots_u,
                patch.surface.degree_u,
                patch.surface.control_points.len(),
            ),
            (
                &patch.surface.knots_v,
                patch.surface.degree_v,
                patch.surface.control_points[0].len(),
            ),
        ] {
            if knots.iter().any(|&k| {
                knots[degree] < k
                    && k < knots[n]
                    && knots.iter().filter(|&&x| x == k).count() > degree.saturating_sub(2)
            }) {
                return Err(refused(
                    "Network candidate interior C2 basis remains unproven",
                ));
            }
        }
        for c in patch.loops.iter().flatten() {
            if c.edge >= source.edges.len() {
                return Err(refused("Network rim references a missing source edge"));
            }
            rim.insert(c.edge);
        }
    }
    let mut cells = 0;
    let mut exact_work = 0;
    let mut interiors = Vec::new();
    for (index, patch) in patches.iter().enumerate() {
        let proof = nurbs_core::surface_linear_monotonicity::inspect_candidate(
            patch.surface,
            max_cells - cells,
        )?;
        cells += proof.cells;
        if !proof.certified {
            return Err(refused(&format!(
                "Network patch {index} injectivity remains unproven"
            )));
        }
        interiors.push(proof);
    }
    let mut model = source.clone();
    let mut faces = (0..source.faces.len()).map(Some).collect::<Vec<_>>();
    let mut edges = (0..source.edges.len()).map(Some).collect::<Vec<_>>();
    let mut vertices = (0..source.vertices.len()).map(Some).collect::<Vec<_>>();
    let mut patch_faces = Vec::<usize>::new();
    let mut final_trims = None;
    let mut final_closure = None;
    for (index, patch) in patches.iter().enumerate() {
        if max_cells - cells < 3 {
            return Err(refused("No network assembly work remains"));
        }
        let selected = patch
            .selected
            .iter()
            .map(|&f| {
                faces[f].ok_or_else(|| refused("Network sector was consumed by another patch"))
            })
            .collect::<Result<Vec<_>>>()?;
        let loops = patch
            .loops
            .iter()
            .map(|l| {
                l.iter()
                    .map(|c| {
                        let mut c = c.clone();
                        c.edge = edges[c.edge]
                            .ok_or_else(|| refused("Network shared rim was consumed"))?;
                        Ok(c)
                    })
                    .collect::<Result<Vec<_>>>()
            })
            .collect::<Result<Vec<_>>>()?;
        let out = replace(
            &model,
            &selected,
            patch.surface,
            &loops,
            patch.reversed,
            tolerance_uv,
            max_cells - cells,
            max_exact - exact_work,
        )
        .map_err(|e| refused(&format!("Network patch {index}: {}", e.message)))?;
        cells += out.cells;
        exact_work += out.exact_work;
        for f in &mut patch_faces {
            *f = out.source_faces[*f]
                .ok_or_else(|| refused("A previously built network patch was consumed"))?;
        }
        patch_faces.push(out.face);
        compose(&mut faces, &out.source_faces);
        compose(&mut edges, &out.source_edges);
        compose(&mut vertices, &out.source_vertices);
        model = out.model;
        final_trims = Some(out.trims);
        final_closure = Some(out.closure);
    }
    let selected = rim
        .into_iter()
        .map(|e| edges[e].ok_or_else(|| refused("Final network rim is missing")))
        .collect::<Result<Vec<_>>>()?;
    let seams = crate::sweep_smoothness::geometric::inspect_edges_with_exact(
        &model,
        &selected,
        crate::sweep_smoothness::geometric::Tolerances {
            position: tolerances[0],
            normal: tolerances[1],
            curvature: tolerances[2],
        },
        max_cells - cells,
        max_exact - exact_work,
    )?;
    cells += seams.cells;
    exact_work += seams.exact_work;
    if !seams.complete || !seams.within_tolerance {
        return Err(refused("Final saved network seams remain unproven"));
    }
    // Collapse intermediate edit receipts onto the actual immutable caller source.
    model.refresh_change_set(&[source]);
    Ok(NetworkReplacement {
        model,
        patch_faces,
        source_faces: faces,
        source_edges: edges,
        source_vertices: vertices,
        interiors,
        seams,
        trims: final_trims.unwrap(),
        closure: final_closure.unwrap(),
        cells,
        exact_work,
    })
}
