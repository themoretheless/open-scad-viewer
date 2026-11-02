//! Atomic transitions in original two-support/two-end-face topology.
//! Original edge binding, four face cuts and closure are shared; radius/G1
//! and whole chamfer setbacks have separate geometry admission owners.
use crate::{Error, Model, Result, TopoId, face_cut, region_replacement, solid_audit};
use nurbs_core::{
    curve::Curve,
    curve_pullback, curve_surface_agreement,
    surface::{Axis, Surface},
    surface_g1_pcurves,
    surface_join::Decision,
};
use std::collections::BTreeSet;
mod automatic;
mod binding;
mod chamfer;
mod circular;
mod circular_chamfer;
mod qualification;
pub use automatic::round_straight_edge;
pub use chamfer::{ChamferTransition, SetbackCertificate, chamfer_straight_edge};
pub use circular::round_circular_edge;
pub use circular_chamfer::{
    CircularChamferTransition, CircularSetbackCertificate, CircularSetbackLower,
    chamfer_circular_edge,
};
fn refused(message: &str) -> Error {
    Error::new("BREP_EDGE_TRANSITION_REFUSED", message)
}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub tolerance_uv: f64,
    pub normal_tolerance: f64,
    pub cells: usize,
    pub exact_work: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            tolerance_uv: 1e-8,
            normal_tolerance: 1e-5,
            cells: 100000,
            exact_work: 1000000,
        }
    }
}
/// Four clamped pcurves in one CCW outer loop. Sides 0/2 contact the original
/// supports, sides 1/3 contact the original end faces. Curved UV ends are allowed.
/// The center/radius laws use the supplied original surface's spine coordinate.
pub struct RadiusPatch<'a> {
    pub surface: &'a Surface,
    pub contour: [&'a Curve; 4],
    pub centers: &'a Curve,
    pub radius: &'a Curve,
    pub spine_axis: Axis,
}
/// Closed original-owned topology only. Geometry-specific wrappers supply
/// their own radius/G1 or setback evidence; closure does not imply embedding.
pub struct TopologyTransition {
    cap: region_replacement::SurfaceCap,
    source_edge: TopoId,
    support_faces: [TopoId; 2],
    end_faces: [TopoId; 2],
    roots: [usize; 4],
    rails: [Vec<usize>; 2],
    work: Ledger,
    audit: solid_audit::SolidAuditCertificate,
}
impl TopologyTransition {
    pub fn model(&self) -> &Model {
        &self.cap.replacement.model
    }
    pub fn into_model(self) -> Model {
        self.cap.replacement.model
    }
    pub fn transition_face(&self) -> usize {
        self.cap.replacement.face
    }
    pub fn source_edge(&self) -> TopoId {
        self.source_edge
    }
    pub fn source_supports(&self) -> [TopoId; 2] {
        self.support_faces
    }
    pub fn source_ends(&self) -> [TopoId; 2] {
        self.end_faces
    }
    pub fn corner_vertices(&self) -> [usize; 4] {
        self.roots
    }
    pub fn contact_edges(&self) -> &[Vec<usize>; 2] {
        &self.rails
    }
    pub fn closure(&self) -> &crate::watertight::QualifiedReport {
        &self.cap.replacement.closure
    }
    pub fn audit(&self) -> &solid_audit::SolidAuditCertificate {
        &self.audit
    }
    pub fn work(&self) -> (usize, u64) {
        (self.work.cells, self.work.exact)
    }
}
pub struct Transition {
    topology: TopologyTransition,
    seams: Vec<surface_g1_pcurves::Report>,
    envelope: nurbs_core::moving_envelope::Envelope,
}
impl std::ops::Deref for Transition {
    type Target = TopologyTransition;
    fn deref(&self) -> &Self::Target {
        &self.topology
    }
}
impl Transition {
    pub fn model(&self) -> &Model {
        self.topology.model()
    }
    pub fn transition_face(&self) -> usize {
        self.topology.transition_face()
    }
    pub fn source_edge(&self) -> TopoId {
        self.topology.source_edge()
    }
    pub fn source_supports(&self) -> [TopoId; 2] {
        self.topology.source_supports()
    }
    pub fn source_ends(&self) -> [TopoId; 2] {
        self.topology.source_ends()
    }
    pub fn corner_vertices(&self) -> [usize; 4] {
        self.topology.corner_vertices()
    }
    pub fn closure(&self) -> &crate::watertight::QualifiedReport {
        self.topology.closure()
    }
    pub fn audit(&self) -> &solid_audit::SolidAuditCertificate {
        self.topology.audit()
    }
    pub fn work(&self) -> (usize, u64) {
        self.topology.work()
    }
    pub fn into_model(self) -> Model {
        self.topology.into_model()
    }
    pub fn tangent_seams(&self) -> &[surface_g1_pcurves::Report] {
        &self.seams
    }
    pub fn radius_certificate(&self) -> &nurbs_core::moving_radius::Certificate {
        self.envelope.radius()
    }
    pub fn envelope_certificate(&self) -> &nurbs_core::moving_envelope::Envelope {
        &self.envelope
    }
}
struct Ledger {
    cells: usize,
    exact: u64,
    limits: Limits,
}
impl Ledger {
    fn allowance(&self, parts: usize) -> Result<usize> {
        let remaining = self.limits.cells.saturating_sub(self.cells);
        if remaining < parts {
            return Err(refused("Shared edge-transition work exhausted"));
        }
        Ok(remaining / parts)
    }
    fn exact_remaining(&self) -> u64 {
        self.limits.exact_work.saturating_sub(self.exact)
    }
    fn charge(&mut self, cells: usize, exact: u64) -> Result<()> {
        self.cells += cells;
        self.exact += exact;
        if self.cells > self.limits.cells || self.exact > self.limits.exact_work {
            return Err(refused("Edge-transition owner exceeded shared allowance"));
        }
        Ok(())
    }
}
fn edges(model: &Model, face: usize) -> BTreeSet<usize> {
    let f = &model.faces[face];
    std::iter::once(f.outer)
        .chain(f.holes.iter().copied())
        .flat_map(|w| model.loops[w].coedges.iter().map(|c| c.edge))
        .collect()
}
fn has_vertex(model: &Model, face: usize, vertex: usize) -> bool {
    edges(model, face)
        .iter()
        .any(|&e| model.edges[e].vertices.contains(&vertex))
}
fn topology(source: &Model, edge: usize) -> Result<([usize; 2], [usize; 2])> {
    if edge >= source.edges.len()
        || source.edges[edge].degenerate
        || source.bodies.len() != 1
        || source.shells.len() != 1
    {
        return Err(refused(
            "Need one actual nondegenerate edge in one closed body",
        ));
    }
    let supports = source
        .faces
        .iter()
        .enumerate()
        .filter(|(i, _)| edges(source, *i).contains(&edge))
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    if supports.len() != 2 {
        return Err(refused(
            "Selected edge must have exactly two original support faces",
        ));
    }
    let mut ends = Vec::new();
    for vertex in source.edges[edge].vertices {
        let candidates = source
            .faces
            .iter()
            .enumerate()
            .filter(|(i, _)| !supports.contains(i) && has_vertex(source, *i, vertex))
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        if candidates.len() != 1 {
            return Err(refused(
                "Each edge end needs one original end face; higher-valence corners need a network",
            ));
        }
        ends.push(candidates[0]);
    }
    if ends[0] == ends[1] {
        return Err(refused(
            "Closed edge-end topology requires a periodic transition constructor",
        ));
    }
    Ok(([supports[0], supports[1]], [ends[0], ends[1]]))
}
fn pullback(
    curve: &Curve,
    surface: &Surface,
    tol: f64,
    work: &mut Ledger,
    parts: usize,
) -> Result<Option<Curve>> {
    let report = curve_pullback::project(
        curve,
        surface,
        curve_pullback::Limits {
            tolerance: tol,
            max_segments: 128,
            max_iterations: 32,
            agreement_cells: work.allowance(parts)?,
        },
    )?;
    work.charge(report.agreement_cells, 0)?;
    Ok(if report.within_tolerance {
        report.pcurve
    } else {
        None
    })
}
/// Insert an authored radius transition on arbitrary original support/end
/// charts, including curved end contours. All four source faces are cut and
/// selected by original sharp-edge/vertex incidence, then replaced atomically.
/// The input patch is NOT admitted by its construction history or sample data.
/// Radius AND radial-normal sphere-envelope agreement cover the whole carrier.
/// Higher-valence corners, periodic edges and automatic patch generation remain
/// separate. Success proves closed topology/G1/radius/envelope, not embedding.
pub fn insert_radius_patch(
    source: &Model,
    edge: usize,
    patch: RadiusPatch<'_>,
    limits: Limits,
) -> Result<Transition> {
    insert_with_curves(source, edge, patch, None, limits)
}
fn insert_with_curves(
    source: &Model,
    edge: usize,
    patch: RadiusPatch<'_>,
    proposed_curves: Option<&[Curve; 4]>,
    limits: Limits,
) -> Result<Transition> {
    let topology = insert_four_faces(
        source,
        edge,
        patch.surface,
        patch.contour,
        proposed_curves,
        limits,
    )?;
    qualification::radius(topology, patch)
}
fn insert_four_faces(
    source: &Model,
    edge: usize,
    surface: &Surface,
    contour: [&Curve; 4],
    proposed_curves: Option<&[Curve; 4]>,
    limits: Limits,
) -> Result<TopologyTransition> {
    source.validate()?;
    surface.validate()?;
    if !source.persistent_naming_complete()
        || !(256..=100000).contains(&limits.cells)
        || !(1..=1000000).contains(&limits.exact_work)
        || !limits.tolerance_uv.is_finite()
        || limits.tolerance_uv <= 0.
        || !limits.normal_tolerance.is_finite()
        || limits.normal_tolerance <= 0.
        || limits.normal_tolerance > 2.
        || surface.periodic_u
        || surface.periodic_v
    {
        return Err(refused(
            "Need immutable named source and bounded nonperiodic transition work",
        ));
    }
    if !solid_audit::audit_solid(source)?.ok {
        return Err(refused("Original body lacks complete solid admission"));
    }
    let (supports, ends) = topology(source, edge)?;
    let mut work = Ledger {
        cells: 0,
        exact: 0,
        limits,
    };
    let contour = contour.iter().map(|c| (*c).clone()).collect::<Vec<_>>();
    let trim = nurbs_core::trim_region_audit::inspect(
        &[contour.clone()],
        limits.tolerance_uv,
        10000,
        work.allowance(12)?,
        work.allowance(12)?,
    )?;
    work.charge(trim.cells + trim.domain_cells, 0)?;
    if trim.valid != Some(true) || trim.winding != [Some(1)] {
        return Err(refused("Transition needs one proven CCW contour"));
    }
    let mut corner_points = Vec::new();
    for pc in &contour {
        pc.validate()?;
        let d = pc.domain();
        if pc.periodic
            || pc.control_points[0].len() != 2
            || !pc.knots[..=pc.degree].iter().all(|k| *k == d[0])
            || !pc.knots[pc.control_points.len()..]
                .iter()
                .all(|k| *k == d[1])
        {
            return Err(refused(
                "Transition contours require clamped original pcurves",
            ));
        }
        let uv = pc.evaluate(d[0])?.point;
        corner_points.push(surface.evaluate(uv[0], uv[1])?.point);
    }
    let mut curves = Vec::new();
    for i in 0..4 {
        if let Some(proposals) = proposed_curves {
            let mut c = proposals[i].clone();
            let last = c.control_points.len() - 1;
            c.control_points[0] = corner_points[i].to_vec();
            c.control_points[last] = corner_points[(i + 1) % 4].to_vec();
            let proof = curve_surface_agreement::verify(
                &c,
                &contour[i],
                surface,
                false,
                source.tolerance_mm / 4.,
                work.allowance(10 - i)?,
            )?;
            work.charge(proof.cells, 0)?;
            if proof.status != curve_surface_agreement::Status::WithinTolerance {
                return Err(refused(
                    "Compact boundary proposal fails the whole original skin lift",
                ));
            }
            curves.push(c);
            continue;
        }
        let report = curve_surface_agreement::lift(
            &contour[i],
            surface,
            Some([corner_points[i], corner_points[(i + 1) % 4]]),
            source.tolerance_mm / 4.,
            work.allowance(10 - i)?,
        )?;
        work.charge(report.cells, 0)?;
        curves.push(
            report
                .curve
                .ok_or_else(|| refused("Actual transition contour lift unproven"))?,
        );
    }
    // Choose chart assignment using complete original-source pullbacks. Two
    // permutations per opposing pair suffice; no UI orientation is assumed.
    let mut assigned = [usize::MAX; 4];
    for (sides, candidates) in [([0, 2], supports), ([1, 3], ends)] {
        let mut found = None;
        for order in [candidates, [candidates[1], candidates[0]]] {
            let mut okay = true;
            for k in 0..2 {
                if pullback(
                    &curves[sides[k]],
                    &source.faces[order[k]].surface,
                    source.tolerance_mm / 4.,
                    &mut work,
                    12,
                )?
                .is_none()
                {
                    okay = false;
                    break;
                }
            }
            if okay {
                found = Some(order);
                break;
            }
        }
        let order = found
            .ok_or_else(|| refused("Transition rails/ends do not lift on the original faces"))?;
        for k in 0..2 {
            assigned[sides[k]] = order[k];
        }
    }
    let source_edges = source
        .faces
        .iter()
        .enumerate()
        .map(|(i, _)| edges(source, i))
        .collect::<Vec<_>>();
    let mut staged = source.clone();
    let mut vertices = [usize::MAX; 4];
    let mut root_edges = BTreeSet::new();
    for i in 0..4 {
        let a = assigned[i];
        let b = assigned[(i + 3) % 4];
        let cap = if ends.contains(&a) { a } else { b };
        let endpoint = source.edges[edge].vertices[usize::from(cap == ends[1])];
        let candidates = source_edges[a]
            .intersection(&source_edges[b])
            .copied()
            .filter(|&e| e != edge && source.edges[e].vertices.contains(&endpoint))
            .collect::<Vec<_>>();
        if candidates.len() != 1 || !root_edges.insert(candidates[0]) {
            return Err(refused(
                "Each transition corner needs a distinct original support/end edge",
            ));
        }
        let result = binding::split(&staged, candidates[0], corner_points[i], &mut work)?;
        vertices[i] = result.vertex;
        staged = result.model;
    }
    let mut discard = Vec::new();
    let mut cut_edges = Vec::new();
    for i in 0..4 {
        let mut curve = curves[i].clone();
        curve.control_points[0] = staged.vertices[vertices[i]].point.to_vec();
        *curve.control_points.last_mut().unwrap() =
            staged.vertices[vertices[(i + 1) % 4]].point.to_vec();
        let proof = curve_surface_agreement::verify(
            &curve,
            &contour[i],
            surface,
            false,
            source.tolerance_mm / 2.,
            work.allowance(8 - i)?,
        )?;
        work.charge(proof.cells, 0)?;
        if proof.status != curve_surface_agreement::Status::WithinTolerance {
            return Err(refused(
                "Bound transition corners fail the complete original patch lift",
            ));
        }
        let part = face_cut::partition_projected_network(
            &staged,
            assigned[i],
            &[face_cut::OwnedCut {
                endpoints: [vertices[i], vertices[(i + 1) % 4]],
                curves: vec![curve],
            }],
            limits.tolerance_uv,
            work.allowance(7 - i)?,
            work.exact_remaining() / (8 - i) as u64,
        )
        .map_err(|e| {
            Error::new(
                e.code,
                format!(
                    "Transition side {i}, original face {}: {}",
                    assigned[i], e.message
                ),
            )
        })?;
        work.charge(part.cells, part.exact_work)?;
        let removed = part
            .faces
            .iter()
            .copied()
            .filter(|&f| {
                if i == 0 || i == 2 {
                    edges(&part.model, f).contains(&edge)
                } else {
                    let vertex = source.edges[edge].vertices[usize::from(assigned[i] == ends[1])];
                    has_vertex(&part.model, f, vertex)
                }
            })
            .collect::<Vec<_>>();
        if removed.len() != 1 {
            return Err(refused(
                "Original sharp-edge incidence does not uniquely select the discarded strip",
            ));
        }
        discard.push(removed[0]);
        cut_edges.push(part.edges);
        staged = part.model;
    }
    let mut cap_proposals = Vec::new();
    if cut_edges.iter().all(|e| e.len() == 1) {
        for i in 0..4 {
            let e = cut_edges[i][0];
            let mut pc = contour[i].clone();
            if staged.edges[e].vertices == [vertices[(i + 1) % 4], vertices[i]] {
                pc = pc.reverse()?;
            } else if staged.edges[e].vertices != [vertices[i], vertices[(i + 1) % 4]] {
                return Err(refused(
                    "Cap edge no longer owns its original transition corner pair",
                ));
            }
            cap_proposals.push((e, pc));
        }
    }
    let cap = if cap_proposals.len() == 4 {
        region_replacement::cap_region_with_pcurves(
            &staged,
            &discard,
            surface,
            &cap_proposals,
            limits.tolerance_uv,
            work.allowance(2)?,
            work.exact_remaining() / 2,
        )?
    } else {
        region_replacement::cap_region(
            &staged,
            &discard,
            surface,
            limits.tolerance_uv,
            work.allowance(2)?,
            // Preserve work for the two actual joins and the radius polynomial.
            // A nonidentical rounded lift may exhaust its optional exact attempt
            // before the mandatory continuous interval admission succeeds.
            work.exact_remaining() / 2,
        )?
    };
    work.charge(cap.cells, cap.replacement.exact_work)?;
    let model = &cap.replacement.model;
    if cap.replacement.source_edges[edge].is_some() {
        return Err(refused(
            "The selected sharp edge survived transition replacement",
        ));
    }
    let roots = vertices.map(|v| cap.replacement.source_vertices[v].unwrap_or(usize::MAX));
    if roots.contains(&usize::MAX) {
        return Err(refused("A bound transition corner was discarded"));
    }
    let mut rails = [Vec::new(), Vec::new()];
    for (k, side) in [0, 2].into_iter().enumerate() {
        for &e in &cut_edges[side] {
            rails[k].push(
                cap.replacement.source_edges[e]
                    .ok_or_else(|| refused("A transition rail was discarded"))?,
            );
        }
    }
    let audit = solid_audit::audit_solid(model)?;
    if !audit.ok {
        return Err(refused(
            "Actual transition body lacks complete solid admission",
        ));
    }
    Ok(TopologyTransition {
        source_edge: source.1.edges[edge],
        support_faces: [source.1.faces[assigned[0]], source.1.faces[assigned[2]]],
        end_faces: [source.1.faces[assigned[1]], source.1.faces[assigned[3]]],
        cap,
        roots,
        rails,
        work,
        audit,
    })
}
