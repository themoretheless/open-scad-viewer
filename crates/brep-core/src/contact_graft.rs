//! Attach an owned transition patch to explicitly selected retained face sectors.
//! Closed topology and continuous boundary admission are not global solid proof.
use crate::{Coedge, Face, FaceUse, Loop, Model, TopologyLineageRecord, face_domain::FaceDomain};
use nurbs_core::{
    Error, Result,
    curve::Curve,
    curve_pullback,
    curve_surface_agreement::{self, Status},
    trim_domain::Location,
};
mod boundary_census;
mod qualified_radius;
pub use qualified_radius::{
    RadiusQualifiedGraft, RadiusTangentGraft, attach_radius_qualified,
    attach_radius_tangent_qualified,
};
mod fit_boundaries;
pub use fit_boundaries::{
    BoundaryAdjustment, FittedPatch, fit_boundaries, fit_boundaries_with_charts,
};
#[derive(Clone, Copy)]
pub struct Target {
    pub face: usize,
    pub keep_vertex: usize,
}
pub struct Graft {
    pub model: Model,
    pub closure: crate::watertight::QualifiedReport,
    pub trims: crate::face_domain::TrimAudit,
    pub source_vertices: Vec<Option<usize>>,
    pub patch_edges: Vec<usize>,
    pub changed_faces: Vec<usize>,
    pub patch_face: usize,
    pub cells: usize,
    pub exact_work: u64,
}
/// Infer target faces, then atomically trim and assemble with the same fresh
/// admission gates as `attach`. Bound source roots and retention seeds remain
/// required; this is not an unrestricted arbitrary-solid fillet operation.
pub fn attach_planned(
    source: &Model,
    patch: &Model,
    vertices: &[usize],
    keep_vertices: &[usize],
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<Graft> {
    let targets = plan_targets(source, patch, vertices, keep_vertices)?;
    attach(
        source,
        patch,
        vertices,
        &targets,
        tolerance_uv,
        max_cells,
        max_exact,
    )
}
fn invalid(s: &str) -> Error {
    Error::new("BREP_CONTACT_GRAFT_INPUT", s)
}
fn unproven(s: &str) -> Error {
    Error::new("BREP_CONTACT_GRAFT_UNPROVEN", s)
}
/// Infer original target faces from owned endpoints and one retained outer
/// vertex per patch edge. No spatial proximity or surface type selects a face.
/// Ambiguous ownership refuses before changing any geometry. This automates
/// face selection, not root finding or the choice of material to retain.
pub fn plan_targets(
    source: &Model,
    patch: &Model,
    vertices: &[usize],
    keep_vertices: &[usize],
) -> Result<Vec<Target>> {
    source.validate()?;
    patch.validate()?;
    if vertices.len() != patch.vertices.len() || keep_vertices.len() != patch.edges.len() {
        return Err(invalid(
            "Need a bound vertex map and one retention seed per patch edge",
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    for (i, &v) in vertices.iter().enumerate() {
        if v >= source.vertices.len()
            || !seen.insert(v)
            || source.vertices[v].point != patch.vertices[i].point
        {
            return Err(invalid(
                "Target planning requires exact injective bound vertices",
            ));
        }
    }
    let mut targets = Vec::with_capacity(patch.edges.len());
    let mut selected = std::collections::BTreeSet::new();
    let loop_work = source.faces.iter().try_fold(0usize, |n, f| {
        n.checked_add(source.loops[f.outer].coedges.len())
    });
    if loop_work
        .and_then(|n| n.checked_mul(patch.edges.len()))
        .is_none_or(|n| n > 100000)
    {
        return Err(unproven(
            "Automatic target ownership scan exceeds its work budget",
        ));
    }
    for (edge, &keep) in patch.edges.iter().zip(keep_vertices) {
        let ends = edge.vertices.map(|v| vertices[v]);
        if keep >= source.vertices.len() || ends.contains(&keep) {
            return Err(invalid(
                "Retention seed must differ from transition endpoints",
            ));
        }
        let mut candidates = source.faces.iter().enumerate().filter_map(|(face, f)| {
            let starts = source.loops[f.outer]
                .coedges
                .iter()
                .map(|c| source.edges[c.edge].vertices[usize::from(c.reversed)])
                .collect::<Vec<_>>();
            ([ends[0], ends[1], keep]
                .iter()
                .all(|v| starts.iter().filter(|x| *x == v).count() == 1))
            .then_some(face)
        });
        let face = candidates
            .next()
            .ok_or_else(|| unproven("No original outer face owns both ends and retention seed"))?;
        if candidates.next().is_some() || !selected.insert(face) {
            return Err(unproven(
                "Target ownership is ambiguous or repeats a face; explicit region planning is required",
            ));
        }
        targets.push(Target {
            face,
            keep_vertex: keep,
        });
    }
    Ok(targets)
}
fn bounds(pc: &Curve, range: [f64; 2]) -> Result<[[f64; 2]; 2]> {
    let b = nurbs_core::interval_eval::evaluate_interval(
        pc,
        nurbs_core::interval_eval::Interval::new(range[0], range[1])?,
    )?;
    Ok(std::array::from_fn(|k| {
        [
            b[k].lo.max(
                pc.control_points
                    .iter()
                    .map(|p| p[k])
                    .fold(f64::INFINITY, f64::min),
            ),
            b[k].hi.min(
                pc.control_points
                    .iter()
                    .map(|p| p[k])
                    .fold(f64::NEG_INFINITY, f64::max),
            ),
        ]
    }))
}
fn inside(pc: &Curve, domain: &FaceDomain, cells: &mut usize, max: usize) -> Result<()> {
    let mut pending = vec![pc.domain()];
    while let Some(range) = pending.pop() {
        if *cells == max {
            return Err(unproven("Retained-face inclusion work exhausted"));
        }
        *cells += 1;
        let b = bounds(pc, range)?;
        if domain.contains_closed_rectangle(b) || domain.on_linear_boundary(b) {
            continue;
        }
        if *cells == max {
            return Err(unproven("No retained-face classification work remains"));
        }
        let r = domain.classify(b, (max - *cells).min(4096))?;
        *cells += r.cells;
        match r.location {
            Location::Inside => {}
            Location::Outside => {
                return Err(unproven(
                    "Transition boundary leaves original face material",
                ));
            }
            Location::Unresolved => {
                let mid = range[0] * 0.5 + range[1] * 0.5;
                if !(range[0] < mid && mid < range[1]) {
                    return Err(unproven("Retained-face closure membership is unresolved"));
                }
                pending.push([range[0], mid]);
                pending.push([mid, range[1]]);
            }
        }
    }
    Ok(())
}
/// `vertices` maps every patch vertex to one EXACTLY matching already-bound
/// source vertex. Each patch edge replaces one outer-boundary arc of a distinct
/// original face; keep_vertex selects the retained arc. All source surfaces and
/// untouched boundary definitions survive. No nearest-vertex welding occurs.
/// Returns a closed B-rep candidate only after fresh trim and continuous closure
/// checks. Material/branch planning, injectivity and global volume remain separate.
pub fn attach(
    source: &Model,
    patch: &Model,
    vertices: &[usize],
    targets: &[Target],
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<Graft> {
    attach_with_charts(source,patch,vertices,targets,&[],tolerance_uv,max_cells,max_exact)
}
/// Optional original-source chart definitions are proposals in 3D edge order.
/// They avoid discarding an already constructed chart to approximate it again.
/// All saved-endpoint edits, whole original lifts, membership and closure checks
/// are repeated; caller evidence never grants admission.
pub fn attach_with_charts(source:&Model,patch:&Model,vertices:&[usize],targets:&[Target],
    charts:&[Option<Curve>],tolerance_uv:f64,max_cells:usize,max_exact:u64)->Result<Graft> {
    source.validate()?;
    patch.validate()?;
    if !charts.is_empty() && charts.len()!=patch.edges.len() {return Err(invalid("Need one optional source chart per transition edge"));}
    if !source.persistent_naming_complete()
        || !patch.persistent_naming_complete()
        || source.tolerance_mm != patch.tolerance_mm
        || !(1..=100000).contains(&max_cells)
        || max_exact > 1000000
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || patch.faces.len() != 1
        || !patch.faces[0].holes.is_empty()
        || patch.shells.len() != 1
        || patch.shells[0].closed
        || !patch.bodies.is_empty()
        || vertices.len() != patch.vertices.len()
        || targets.len() != patch.edges.len()
        || !(2..=256).contains(&targets.len())
    {
        return Err(invalid(
            "Need an owned open one-face patch, matching tolerance and bounded retained-face plans",
        ));
    }
    let mut unique = std::collections::BTreeSet::new();
    for (i, &v) in vertices.iter().enumerate() {
        if v >= source.vertices.len()
            || !unique.insert(v)
            || source.vertices[v].point != patch.vertices[i].point
        {
            return Err(invalid(
                "Patch corners must map injectively to exact existing source vertices",
            ));
        }
    }
    let mut selected = std::collections::BTreeSet::new();
    let mut target_shell = None;
    for target in targets {
        if target.face >= source.faces.len() || !selected.insert(target.face) {
            return Err(invalid(
                "Each transition edge needs a distinct original target face",
            ));
        }
        let owners = source
            .shells
            .iter()
            .enumerate()
            .filter(|(_, s)| s.faces.iter().any(|u| u.face == target.face))
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        if owners.len() != 1
            || !source.shells[owners[0]].closed
            || target_shell.is_some_and(|s| s != owners[0])
        {
            return Err(invalid("Targets must share one originally closed shell"));
        }
        target_shell = Some(owners[0]);
    }
    let shell = target_shell.unwrap();
    let mut model = source.clone();
    let edge_start = model.edges.len();
    for edge in &patch.edges {
        model.edges.push(crate::Edge {
            curve: edge.curve.clone(),
            vertices: edge.vertices.map(|v| vertices[v]),
            degenerate: edge.degenerate,
        });
    }
    let mut cells = 0;
    let mut exact_work = 0;
    let mut pairs = 0;
    let mut orientation = None;
    for (edge, target) in targets.iter().enumerate() {
        let face = &source.faces[target.face];
        let outer = &source.loops[face.outer].coedges;
        let endpoints = patch.edges[edge].vertices.map(|v| vertices[v]);
        if endpoints.contains(&target.keep_vertex) || target.keep_vertex >= source.vertices.len() {
            return Err(invalid(
                "Retention seed must be a distinct original outer vertex",
            ));
        }
        let starts = outer
            .iter()
            .map(|c| source.edges[c.edge].vertices[usize::from(c.reversed)])
            .collect::<Vec<_>>();
        let mut indices = [0; 2];
        for k in 0..2 {
            let hits = starts
                .iter()
                .enumerate()
                .filter(|(_, v)| **v == endpoints[k])
                .map(|(i, _)| i)
                .collect::<Vec<_>>();
            if hits.len() != 1 {
                return Err(invalid(
                    "Transition endpoints require unique bound outer-loop occurrences",
                ));
            }
            indices[k] = hits[0];
        }
        let arc = |a: usize, b: usize| {
            let mut cs = Vec::new();
            let mut i = a;
            while i != b {
                cs.push(outer[i].clone());
                i = (i + 1) % outer.len();
            }
            cs
        };
        let choices = [arc(indices[1], indices[0]), arc(indices[0], indices[1])];
        let keeps = choices
            .iter()
            .map(|cs| {
                cs.iter().any(|c| {
                    source.edges[c.edge].vertices[usize::from(c.reversed)] == target.keep_vertex
                })
            })
            .collect::<Vec<_>>();
        let side = match keeps.as_slice() {
            [true, false] => 0,
            [false, true] => 1,
            _ => {
                return Err(invalid(
                    "Retention seed does not select exactly one original arc",
                ));
            }
        };
        if cells == max_cells {
            return Err(unproven("No new boundary pullback work remains"));
        }
        let mut pc = if let Some(Some(pc)) = charts.get(edge) {
            pc.validate()?;
            if pc.control_points[0].len() != 2 {
                return Err(invalid("Source boundary chart must be two-dimensional"));
            }
            pc.clone()
        } else {
            let projection = curve_pullback::project(
                &patch.edges[edge].curve,
                &face.surface,
                curve_pullback::Limits {
                    tolerance: source.tolerance_mm * 0.5,
                    max_segments: 256,
                    max_iterations: 32,
                    agreement_cells: max_cells - cells,
                },
            )?;
            cells += projection.agreement_cells;
            projection.pcurve.ok_or_else(|| {
                unproven(&format!(
                    "Transition edge {edge} pullback into retained face {} is unresolved",
                    target.face
                ))
            })?
        };
        // Save the exact existing chart endpoint authority, then independently
        // verify the actual edited pcurve. The pre-edit report is not inherited.
        let last = pc.control_points.len() - 1;
        pc.control_points[0] = outer[indices[0]].pcurve.control_points[0].clone();
        pc.control_points[last] = outer[indices[1]].pcurve.control_points[0].clone();
        if side == 1 {
            pc = pc.reverse()?;
        }
        if cells == max_cells {
            return Err(unproven("No saved-pcurve agreement work remains"));
        }
        let r = curve_surface_agreement::verify(
            &patch.edges[edge].curve,
            &pc,
            &face.surface,
            side == 1,
            source.tolerance_mm,
            max_cells - cells,
        )?;
        cells += r.cells;
        if r.status != Status::WithinTolerance {
            return Err(unproven("Actual saved retained-face boundary is unproven"));
        }
        let original = std::iter::once(face.outer)
            .chain(face.holes.iter().copied())
            .map(|w| {
                source.loops[w]
                    .coedges
                    .iter()
                    .map(|c| c.pcurve.clone())
                    .collect()
            })
            .collect::<Vec<Vec<_>>>();
        if max_cells - cells < 2 || pairs == 10000 {
            return Err(unproven("Original region audit budget exhausted"));
        }
        let stage = ((max_cells - cells) / 2).min(100000);
        let trim = nurbs_core::trim_region_audit::inspect(
            &original,
            tolerance_uv,
            10000 - pairs,
            stage,
            stage,
        )?;
        cells += trim.cells + trim.domain_cells;
        pairs += trim.pairs;
        if trim.valid != Some(true) {
            return Err(unproven("Original retained-face trim region is unproven"));
        }
        let domain = FaceDomain::new(source, target.face, tolerance_uv)?;
        if let Err(e) = inside(&pc, &domain, &mut cells, max_cells) {
            let census = boundary_census::inside(&pc, &original, &domain, &mut cells, &mut exact_work, max_cells, max_exact)?;
            if !census.inside {
                return Err(unproven(&format!("Transition edge {edge}, original face {}: {}; boundary census: {} {:?}, phase={:?}; exact work={exact_work}", target.face, e.message, census.reason, census.boundary, census.phase)));
            }
        }
        let use_ = Coedge::new(
            edge_start + edge,
            side == 1,
            pc,
        );
        let source_reversed = source.shells[shell]
            .faces
            .iter()
            .find(|u| u.face == target.face)
            .unwrap()
            .reversed;
        let patch_use = patch.loops[patch.faces[0].outer]
            .coedges
            .iter()
            .find(|c| c.edge == edge)
            .ok_or_else(|| invalid("Patch edge has no owned skin use"))?;
        let required = !(use_.reversed ^ source_reversed) ^ patch_use.reversed;
        if orientation.is_some_and(|o| o != required) {
            return Err(unproven(
                "Retained sectors require inconsistent skin orientation",
            ));
        }
        orientation = Some(required);
        let mut kept = vec![use_];
        kept.extend(choices[side].clone());
        model.loops[face.outer] = Loop { coedges: kept };
    }
    let outer = model.loops.len();
    model.loops.push(Loop {
        coedges: patch.loops[patch.faces[0].outer]
            .coedges
            .iter()
            .map(|c| Coedge::new(
                edge_start + c.edge,
                c.reversed,
                c.pcurve.clone(),
            ))
            .collect(),
    });
    let patch_face = model.faces.len();
    model.faces.push(Face {
        surface: patch.faces[0].surface.clone(),
        outer,
        holes: vec![],
    });
    model.shells[shell].faces.push(FaceUse {
        face: patch_face,
        reversed: orientation.unwrap(),
    });
    let mut used_edges = vec![false; model.edges.len()];
    for wire in &model.loops {
        for c in &wire.coedges {
            used_edges[c.edge] = true;
        }
    }
    let mut used_vertices = vec![false; model.vertices.len()];
    for (i, e) in model.edges.iter().enumerate() {
        if used_edges[i] {
            for &v in &e.vertices {
                used_vertices[v] = true;
            }
        }
    }
    let mut vertex_map = vec![None; model.vertices.len()];
    let mut retained = Vec::new();
    for (i, v) in model.vertices.iter().enumerate() {
        if used_vertices[i] {
            vertex_map[i] = Some(retained.len());
            retained.push(v.clone());
        }
    }
    model.vertices = retained;
    let mut edge_map = vec![None; model.edges.len()];
    let mut retained = Vec::new();
    for (i, e) in model.edges.iter().enumerate() {
        if used_edges[i] {
            edge_map[i] = Some(retained.len());
            let mut e = e.clone();
            e.vertices = e.vertices.map(|v| vertex_map[v].unwrap());
            retained.push(e);
        }
    }
    model.edges = retained;
    for wire in &mut model.loops {
        for c in &mut wire.coedges {
            c.edge = edge_map[c.edge].unwrap();
        }
    }
    model.inherit_topology_ids(&[source, patch]);
    for target in targets {
        if source.1.faces[target.face] == model.1.faces[target.face] {
            continue;
        }
        model.1.lineage.push(TopologyLineageRecord {
            operation: "modify".into(),
            entity_kind: "face".into(),
            parents: vec![source.1.faces[target.face]],
            children: vec![model.1.faces[target.face]],
        });
    }
    if source.1.shells[shell] != model.1.shells[shell] {
        model.1.lineage.push(TopologyLineageRecord {
            operation: "modify".into(),
            entity_kind: "shell".into(),
            parents: vec![source.1.shells[shell]],
            children: vec![model.1.shells[shell]],
        });
    }
    for (i, body) in source.bodies.iter().enumerate() {
        if (body.outer_shell == shell || body.inner_shells.contains(&shell))
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
    // Patch geometry is authored by this operation; history is relative to
    // the persistent source body, while inherited patch IDs remain stable.
    model.refresh_change_set(&[source]);
    model.validate()?;
    if max_cells - cells < 2 || pairs == 10000 {
        return Err(unproven("Final trim work exhausted"));
    }
    let stage = ((max_cells - cells) / 2).min(100000);
    let trims =
        crate::face_domain::audit_trim_regions(&model, tolerance_uv, 10000 - pairs, stage, stage)?;
    cells += trims.cells + trims.domain_cells;
    if !trims.all_valid {
        return Err(unproven(&format!(
            "Retained result trim regions remain unproven: {:?}",
            trims
                .faces
                .iter()
                .enumerate()
                .filter_map(
                    |(i, r)| r.as_ref().filter(|r| r.valid != Some(true)).map(|r| (
                        i,
                        r.reason,
                        r.problem_loops,
                        r.winding.clone(),
                        r.loops.iter().map(|l| l.outcome()).collect::<Vec<_>>()
                    ))
                )
                .collect::<Vec<_>>()
        )));
    }
    let closure =
        crate::watertight::inspect_with_exact(&model, max_cells - cells, max_exact - exact_work)?;
    cells += closure.closure.agreement.cells;
    exact_work += closure.exact.work;
    if closure.closure.status != crate::watertight::Closure::WithinTolerance {
        return Err(unproven("Closed retained boundary agreement is unproven"));
    }
    if !model.persistent_naming_complete() {
        return Err(unproven("Retained assembly naming is incomplete"));
    }
    Ok(Graft {
        model,
        closure,
        trims,
        source_vertices: vertex_map,
        patch_edges: (0..patch.edges.len())
            .map(|i| edge_map[edge_start + i].unwrap())
            .collect(),
        changed_faces: targets.iter().map(|t| t.face).collect(),
        patch_face,
        cells,
        exact_work,
    })
}
