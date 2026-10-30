//! Replace connected retained-face sectors by one authored transition chart.
//! Boundary identity is topological; no fitting, welding or tolerance growth.
use crate::{Coedge, Face, FaceUse, Loop, Model, TopologyLineageRecord};
use nurbs_core::{Error, Result, surface::Surface};
use std::collections::{BTreeMap, BTreeSet};
#[path = "region_replacement/mapped.rs"]
mod mapped;
pub use mapped::{MappedReplacement, replace_g2_mapped};
#[path = "region_replacement/network.rs"]
mod network;
pub use network::{NetworkReplacement, RegionPatch, replace_g2_network};
#[path = "region_replacement/cap.rs"]
mod cap;
pub use cap::{SurfaceCap, cap_region};

pub struct Replacement {
    pub model: Model,
    pub face: usize,
    pub source_faces: Vec<Option<usize>>,
    pub source_edges: Vec<Option<usize>>,
    pub source_vertices: Vec<Option<usize>>,
    pub trims: crate::face_domain::TrimAudit,
    pub closure: crate::watertight::QualifiedReport,
    pub cells: usize,
    pub exact_work: u64,
}
fn refused(message: &str) -> Error {
    Error::new("BREP_REGION_REPLACEMENT_UNPROVEN", message)
}
pub struct SmoothReplacement {
    pub replacement: Replacement,
    pub corner: nurbs_core::corner_blend::Report,
    pub cells: usize,
    pub exact_work: u64,
}
pub struct QualifiedCandidate {
    pub replacement: Replacement,
    pub seams: Vec<(usize, nurbs_core::surface_g2_pcurves::Report)>,
    pub interior: nurbs_core::surface_linear_monotonicity::Report,
    pub cells: usize,
    pub exact_work: u64,
}
/// Admit an authored transition with arbitrary original face-local pcurves.
/// Every boundary edge must have one surviving original support use, with fresh
/// whole-curve position/G2 proof. The complete candidate chart also needs an
/// injectivity certificate. Geometry construction and radius laws are separate.
pub fn replace_g2_candidate(
    source: &Model,
    selected: &[usize],
    surface: &Surface,
    loops: &[Vec<Coedge>],
    reversed: bool,
    tolerance_uv: f64,
    tolerances: [f64; 3],
    max_cells: usize,
    max_exact: u64,
) -> Result<QualifiedCandidate> {
    source.validate()?;
    surface.validate()?;
    if !(4..=100000).contains(&max_cells)
        || max_exact > 1000000
        || tolerances.iter().any(|x| !x.is_finite() || *x < 0.)
        || tolerances[0] > source.tolerance_mm
        || tolerances[1] > 2.
        || !(1..=256).contains(&selected.len())
        || !(1..=16).contains(&loops.len())
        || loops.iter().map(Vec::len).sum::<usize>() > 256
        || surface.periodic_u
        || surface.periodic_v
        || surface
            .control_points
            .len()
            .checked_mul(surface.control_points[0].len())
            .is_none_or(|n| n > 4096)
    {
        return Err(refused(
            "Need bounded finite transition qualification inputs",
        ));
    }
    for (knots, degree, n) in [
        (
            &surface.knots_u,
            surface.degree_u,
            surface.control_points.len(),
        ),
        (
            &surface.knots_v,
            surface.degree_v,
            surface.control_points[0].len(),
        ),
    ] {
        if knots.iter().any(|&k| {
            knots[degree] < k
                && k < knots[n]
                && knots.iter().filter(|&&x| x == k).count() > degree.saturating_sub(2)
        }) {
            return Err(refused("Transition interior C2 basis remains unproven"));
        }
    }
    let interior =
        nurbs_core::surface_linear_monotonicity::inspect_candidate(surface, max_cells / 4)?;
    if !interior.certified {
        return Err(refused(
            "Whole transition chart injectivity remains unproven",
        ));
    }
    let mut cells = interior.cells;
    let mut exact_work = 0;
    let mut seams = Vec::new();
    let mut seen = BTreeSet::new();
    let count = loops.iter().map(Vec::len).sum::<usize>();
    for pc in loops.iter().flatten() {
        if pc.edge >= source.edges.len() || !seen.insert(pc.edge) || cells == max_cells {
            return Err(refused(
                "Transition boundary has invalid/repeated edges or no remaining work",
            ));
        }
        let supports = source
            .faces
            .iter()
            .enumerate()
            .filter(|(f, _)| !selected.contains(f))
            .flat_map(|(f, face)| {
                std::iter::once(face.outer)
                    .chain(face.holes.iter().copied())
                    .flat_map(move |w| {
                        source.loops[w]
                            .coedges
                            .iter()
                            .filter(move |c| c.edge == pc.edge)
                            .map(move |c| (f, c))
                    })
            })
            .collect::<Vec<_>>();
        if supports.len() != 1 {
            return Err(refused(
                "Transition edge needs exactly one retained original support use",
            ));
        }
        let (f, old) = supports[0];
        let s = &source.faces[f].surface;
        if s.control_points
            .len()
            .checked_mul(s.control_points[0].len())
            .is_none_or(|n| n > 4096)
        {
            return Err(refused("Original support control budget exceeded"));
        }
        let budget = ((max_cells - cells) / (count - seams.len() + 1)).max(1);
        let proof = nurbs_core::surface_g2_pcurves::inspect(
            &source.edges[pc.edge].curve,
            [&old.pcurve, &pc.pcurve],
            [&source.faces[f].surface, surface],
            [old.reversed, pc.reversed],
            tolerances[0],
            tolerances[1],
            tolerances[2],
            budget,
            max_exact - exact_work,
        )?;
        cells += proof.geometric.cells;
        exact_work += proof.positions.exact_work;
        if !proof.positions.proven
            || proof.geometric.decision != nurbs_core::surface_join::Decision::WithinTolerance
        {
            return Err(refused(
                "Original-pcurve G2 transition seam remains unproven",
            ));
        }
        seams.push((pc.edge, proof));
    }
    if max_cells - cells < 3 {
        return Err(refused(
            "No shared work remains for qualified transition assembly",
        ));
    }
    let replacement = replace(
        source,
        selected,
        surface,
        loops,
        reversed,
        tolerance_uv,
        max_cells - cells,
        max_exact - exact_work,
    )?;
    cells += replacement.cells;
    exact_work += replacement.exact_work;
    Ok(QualifiedCandidate {
        replacement,
        seams,
        interior,
        cells,
        exact_work,
    })
}
/// Construct the replacement from original retained support jets. Each rim use
/// must belong to a surviving face; complete original-trace G2 and chart
/// injectivity admission precede atomic region replacement and closure checks.
/// Current rim correspondence is affine isoparametric, with four sides.
pub fn replace_g2(
    source: &Model,
    selected: &[usize],
    supports: [crate::corner_close::BoundaryUse; 4],
    options: nurbs_core::corner_blend::Options,
    tolerance_uv: f64,
    tolerances: [f64; 3],
    max_cells: usize,
    max_exact: u64,
) -> Result<SmoothReplacement> {
    replace_g2_sides(
        source,
        selected,
        supports.map(|u| vec![u]),
        options,
        tolerance_uv,
        tolerances,
        max_cells,
        max_exact,
    )
}
/// Each of the four sides may consist of several root-bound original edges in
/// one support chart. Gaps, overlaps and nonidentical shared vertices refuse.
pub fn replace_g2_sides(
    source: &Model,
    selected: &[usize],
    supports: [Vec<crate::corner_close::BoundaryUse>; 4],
    options: nurbs_core::corner_blend::Options,
    tolerance_uv: f64,
    tolerances: [f64; 3],
    max_cells: usize,
    max_exact: u64,
) -> Result<SmoothReplacement> {
    source.validate()?;
    let first = *selected
        .first()
        .ok_or_else(|| refused("Need selected replacement sectors"))?;
    if first >= source.faces.len()
        || supports
            .iter()
            .flatten()
            .any(|u| selected.contains(&u.face))
    {
        return Err(refused(
            "G2 support jets must come from surviving source faces",
        ));
    }
    let owners = source
        .shells
        .iter()
        .enumerate()
        .filter(|(_, s)| s.faces.iter().any(|u| u.face == first))
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    if owners.len() != 1 {
        return Err(refused("Replacement sectors need one shell owner"));
    }
    let prepared = crate::corner_close::prepare_sides(
        source, owners[0], supports, options, tolerances, max_cells, max_exact, 2,
    )?;
    if max_cells - prepared.cells < 3 {
        return Err(refused("No shared work remains for corner region assembly"));
    }
    let replacement = replace(
        source,
        selected,
        &prepared.corner.surface,
        &[prepared.rim],
        prepared.reversed,
        tolerance_uv,
        max_cells - prepared.cells,
        max_exact - prepared.exact_work,
    )?;
    let cells = prepared.cells + replacement.cells;
    let exact_work = prepared.exact_work + replacement.exact_work;
    Ok(SmoothReplacement {
        replacement,
        corner: prepared.corner,
        cells,
        exact_work,
    })
}
/// The supplied loops reference existing source edges and use the replacement
/// surface chart. They must equal the complete oriented exposed boundary of
/// the selected connected region. All selected faces belong to one closed shell.
/// Internal edges are removed; original rim curves and retained surfaces survive.
/// A result requires fresh whole-model trim and continuous closure admission.
/// This does not establish global embedding, wall thickness or a radius law.
pub fn replace(
    source: &Model,
    selected: &[usize],
    surface: &Surface,
    loops: &[Vec<Coedge>],
    reversed: bool,
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<Replacement> {
    source.validate()?;
    surface.validate()?;
    let chosen: BTreeSet<_> = selected.iter().copied().collect();
    if !source.persistent_naming_complete()
        || chosen.len() != selected.len()
        || !(1..=256).contains(&selected.len())
        || selected.iter().any(|&f| f >= source.faces.len())
        || !(1..=16).contains(&loops.len())
        || loops.iter().any(|l| l.len() < 2)
        || loops.iter().map(Vec::len).sum::<usize>() > 256
        || surface.periodic_u
        || surface.periodic_v
        || surface
            .control_points
            .len()
            .checked_mul(surface.control_points[0].len())
            .is_none_or(|n| n > 4096)
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || !(3..=100000).contains(&max_cells)
        || max_exact > 1000000
    {
        return Err(refused(
            "Need owned sectors, nonperiodic transition and bounded work",
        ));
    }
    let owners = source
        .shells
        .iter()
        .enumerate()
        .filter(|(_, s)| s.faces.iter().any(|u| chosen.contains(&u.face)))
        .collect::<Vec<_>>();
    if owners.len() != 1 || !owners[0].1.closed {
        return Err(refused("Selected sectors must belong to one closed shell"));
    }
    let shell = owners[0].0;
    let mut incidence: BTreeMap<usize, Vec<(usize, bool)>> = BTreeMap::new();
    for &f in selected {
        let uses = source.shells[shell]
            .faces
            .iter()
            .filter(|u| u.face == f)
            .collect::<Vec<_>>();
        if uses.len() != 1 {
            return Err(refused("Selected sector needs unique shell ownership"));
        }
        let face = &source.faces[f];
        for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            for c in &source.loops[wire].coedges {
                if source.edges[c.edge].degenerate {
                    return Err(refused(
                        "Collapsed region boundaries need separate pole planning",
                    ));
                }
                incidence
                    .entry(c.edge)
                    .or_default()
                    .push((f, c.reversed ^ uses[0].reversed));
            }
        }
    }
    let mut boundary = BTreeMap::new();
    let mut adjacent: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (&edge, uses) in &incidence {
        match uses.as_slice() {
            [(_, r)] => {
                boundary.insert(edge, *r);
            }
            [(a, ra), (b, rb)] if a != b && ra != rb => {
                adjacent.entry(*a).or_default().push(*b);
                adjacent.entry(*b).or_default().push(*a);
            }
            _ => return Err(refused("Region incidence is not an oriented manifold")),
        }
    }
    let mut seen = BTreeSet::new();
    let mut queue = vec![selected[0]];
    while let Some(f) = queue.pop() {
        if seen.insert(f) {
            queue.extend(adjacent.get(&f).into_iter().flatten().copied());
        }
    }
    if seen != chosen {
        return Err(refused("Replacement region is disconnected"));
    }
    let mut proposed = BTreeMap::new();
    for c in loops.iter().flatten() {
        c.pcurve.validate()?;
        if c.edge >= source.edges.len()
            || c.pcurve.control_points[0].len() != 2
            || proposed.insert(c.edge, c.reversed ^ reversed).is_some()
        {
            return Err(refused(
                "Replacement rim must use each existing boundary edge once",
            ));
        }
    }
    if boundary.is_empty() || proposed != boundary {
        return Err(refused(
            "Replacement rim differs from the selected region boundary",
        ));
    }
    let mut model = source.clone();
    let mut source_faces = vec![None; source.faces.len()];
    model.faces.clear();
    model.loops.clear();
    for (i, f) in source.faces.iter().enumerate() {
        if chosen.contains(&i) {
            continue;
        }
        let mut retained = f.clone();
        retained.outer = model.loops.len();
        model.loops.push(source.loops[f.outer].clone());
        retained.holes.clear();
        for &w in &f.holes {
            retained.holes.push(model.loops.len());
            model.loops.push(source.loops[w].clone());
        }
        source_faces[i] = Some(model.faces.len());
        model.faces.push(retained);
    }
    let face = model.faces.len();
    let outer = model.loops.len();
    model
        .loops
        .extend(loops.iter().map(|l| Loop { coedges: l.clone() }));
    let holes = (outer + 1..model.loops.len()).collect();
    model.faces.push(Face {
        surface: surface.clone(),
        outer,
        holes,
    });
    for s in &mut model.shells {
        s.faces = s
            .faces
            .iter()
            .filter_map(|u| {
                source_faces[u.face].map(|f| FaceUse {
                    face: f,
                    reversed: u.reversed,
                })
            })
            .collect();
    }
    model.shells[shell].faces.push(FaceUse { face, reversed });
    let mut used_edges = vec![false; source.edges.len()];
    for c in model.loops.iter().flat_map(|l| &l.coedges) {
        used_edges[c.edge] = true;
    }
    let mut used_vertices = vec![false; source.vertices.len()];
    for (i, e) in source.edges.iter().enumerate() {
        if used_edges[i] {
            for &v in &e.vertices {
                used_vertices[v] = true;
            }
        }
    }
    let mut source_vertices = vec![None; source.vertices.len()];
    model.vertices.clear();
    for (i, v) in source.vertices.iter().enumerate() {
        if used_vertices[i] {
            source_vertices[i] = Some(model.vertices.len());
            model.vertices.push(v.clone());
        }
    }
    let mut source_edges = vec![None; source.edges.len()];
    model.edges.clear();
    for (i, e) in source.edges.iter().enumerate() {
        if used_edges[i] {
            source_edges[i] = Some(model.edges.len());
            let mut e = e.clone();
            e.vertices = e.vertices.map(|v| source_vertices[v].unwrap());
            model.edges.push(e);
        }
    }
    for c in model.loops.iter_mut().flat_map(|l| &mut l.coedges) {
        c.edge = source_edges[c.edge].unwrap();
    }
    model.inherit_topology_ids(&[source]);
    if selected.len() > 1 || model.1.faces[face] != source.1.faces[selected[0]] {
        model.1.lineage.push(TopologyLineageRecord {
            operation: if selected.len() > 1 {
                "merge"
            } else {
                "modify"
            }
            .into(),
            entity_kind: "face".into(),
            parents: selected.iter().map(|&f| source.1.faces[f]).collect(),
            children: vec![model.1.faces[face]],
        });
    }
    if model.1.shells[shell] != source.1.shells[shell] {
        model.1.lineage.push(TopologyLineageRecord {
            operation: "modify".into(),
            entity_kind: "shell".into(),
            parents: vec![source.1.shells[shell]],
            children: vec![model.1.shells[shell]],
        });
    }
    for (i, b) in source.bodies.iter().enumerate() {
        if (b.outer_shell == shell || b.inner_shells.contains(&shell))
            && source.1.bodies[i] != model.1.bodies[i]
        {
            model.1.lineage.push(TopologyLineageRecord {
                operation: "modify".into(),
                entity_kind: "body".into(),
                parents: vec![source.1.bodies[i]],
                children: vec![model.1.bodies[i]],
            });
        }
    }
    model.refresh_change_set(&[source]);
    model.validate()?;
    let stage = ((max_cells - 1) / 2).min(100000);
    let trims = crate::face_domain::audit_trim_regions(&model, tolerance_uv, 10000, stage, stage)?;
    let mut cells = trims.cells + trims.domain_cells;
    if !trims.all_valid || cells >= max_cells {
        let problem=trims.faces.iter().enumerate().find(|(_,r)|
            !r.as_ref().is_some_and(|r|r.valid==Some(true)));
        let detail=problem.map(|(face,r)|match r {
            Some(r)=>format!("face {face}: {}, loops {:?}",r.reason,
                r.loops.iter().map(|l|l.outcome()).collect::<Vec<_>>()),
            None=>format!("face {face}: unvisited"),
        }).unwrap_or_else(||"shared work exhausted".into());
        return Err(refused(&format!("Replacement trim regions remain unproven ({detail})")));
    }
    let closure = crate::watertight::inspect_with_exact(&model, max_cells - cells, max_exact)?;
    cells += closure.closure.agreement.cells;
    if closure.closure.status != crate::watertight::Closure::WithinTolerance
        || !model.persistent_naming_complete()
    {
        return Err(refused("Replacement closure or naming remains unproven"));
    }
    let exact_work = closure.exact.work;
    Ok(Replacement {
        model,
        face,
        source_faces,
        source_edges,
        source_vertices,
        trims,
        closure,
        cells,
        exact_work,
    })
}

#[path = "region_replacement/radius_network.rs"]
mod radius_network;
pub use radius_network::{
    EdgeRadius, RadiusCorner, RadiusEdge, RadiusNetwork, plan_radius_network,
};
#[path = "region_replacement/trihedral_setbacks.rs"]
mod trihedral_setbacks;
pub use trihedral_setbacks::{EdgeSetback, FacePort, TrihedralSetbacks, plan_trihedral_setbacks};

#[path = "region_replacement/trihedral_skins.rs"]
mod trihedral_skins;
pub use trihedral_skins::{
    StripLimits, StripPatch, StripSeparation, TrihedralStrips, build_trihedral_strips,
};

#[path = "region_replacement/trihedral_rim.rs"]
mod trihedral_rim;
pub use trihedral_rim::{
    RimConnector, RimSupports, TrihedralRim, build_trihedral_rim, inspect_rim_supports,
};

#[path = "region_replacement/spatial_rim.rs"]
mod spatial_rim;
pub use spatial_rim::{SpatialRimReport, inspect_spatial_rim};

#[path = "region_replacement/trihedral_collars.rs"]
mod trihedral_collars;
pub use trihedral_collars::{CornerCollar, CornerCollars, prepare_trihedral_collars};

#[path = "region_replacement/central_graph.rs"]
mod central_graph;
#[path = "region_replacement/trihedral_graft.rs"]
mod trihedral_graft;
pub use trihedral_graft::{TrihedralGraftCandidate,propose_trihedral_graft};
pub use central_graph::{
    CentralGraph, CentralGraphInspection, GraphOptions, GraphGuide, inspect_central_graph, propose_central_graph,
    propose_central_graph_guided,
    propose_central_graph_fair,
    propose_central_graph_refined,
    propose_central_graph_constrained,
    inspect_central_graph_correlated,
    trim_central_graph,
};

mod interior_lift;
pub use interior_lift::{InteriorLift, InteriorLiftProposal, propose_interior_lift};

mod interior_tile;
pub use interior_tile::{InteriorTileLimits, InteriorTileProposal, propose_interior_tile, propose_curvature_limited_tile};
