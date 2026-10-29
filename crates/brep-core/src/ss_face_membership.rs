//! Complete original SS clocks against actual retained face regions.
//! Membership is separate from original-face imprinting and Solid authorship.
use crate::{Model, TopoId, face_domain::FaceDomain};
use nurbs_core::{
    Error, Result, curve_surface_agreement as agreement,
    interval_eval::Interval as I,
    ss_intersection::{QuadraticGraphSSKind, QualifiedGraphCurves},
    trim_domain::Location,
    trim_region_audit,
};
mod closed;
pub use closed::{
    ClosedBranchFaces, ClosedBranchLimits, ClosedGraphEnvelope, ClosedGraphLens,
    ClosedGraphSideLimits, ClosedSourceEdge, GraphEnvelopeLimits, GraphEnvelopeOperation,
    GraphFaceSide, GraphLensLimits, GraphMaterialFace, QualifiedClosedBranches,
    QualifiedClosedGraphMaterial, QualifiedClosedGraphSides, assemble_closed_graph_envelope,
    assemble_closed_graph_lens, partition_closed_branches, qualify_closed_graph_material,
    qualify_closed_graph_sides,
};
mod coedge_roots;
#[cfg(test)]
mod tests;
pub use coedge_roots::{
    CoedgeBoundaryRole, CoedgeRootAddress, CoedgeRootEvent, CoedgeRootLimits, CutNode,
    CutPartitionLimits, CutRepresentative, CutRepresentativeLimits, OpenPathStep, OpenSSFaceLimits,
    OpenSSNetworkCut, OpenSSNetworkPort, OpenSSPairSide, OpenSSPathLimits, OpenSSPathRequest,
    OpenSSRootGuard, OpenSSSegment, PartitionedCutVertex, QualifiedCoedgeRoots,
    QualifiedCutPartitions, QualifiedCutRepresentatives, QualifiedFacePairRoots,
    QualifiedOpenSSFace, QualifiedOpenSSNetwork, QualifiedOpenSSPair, QualifiedOpenSSPath,
    partition_cut_representatives, partition_open_ss_network, partition_open_ss_pair,
    partition_open_ss_path, qualify_coedge_roots, qualify_cut_representatives,
    qualify_open_ss_path,
};
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub cells: usize,
    pub pairs: usize,
    pub range_operations: usize,
    pub depth: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            cells: 100000,
            pairs: 10000,
            range_operations: 100000,
            depth: 24,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct EdgeCell {
    pub original_edge: usize,
    pub clock: [f64; 2],
    /// These boxes contain BOTH original UVs and saved pcurve images.
    pub uv: [[[f64; 2]; 2]; 2],
    pub locations: [Location; 2],
}
#[derive(Clone, Debug, PartialEq)]
pub struct VertexMembership {
    pub original_vertex: usize,
    pub locations: [Location; 2],
}
/// Immutable borrowed sources cannot change while this owner is in use.
/// Neither this owner nor its public records authorize topology mutation.
pub struct QualifiedMembership<'a> {
    curves: &'a QualifiedGraphCurves,
    faces: [(&'a Model, usize); 2],
    tolerance_uv: f64,
    limits: Limits,
    edge_cells: Vec<EdgeCell>,
    vertices: Vec<VertexMembership>,
    face_ids: [TopoId; 2],
    reversed: [bool; 2],
    work: Work,
}
impl QualifiedMembership<'_> {
    pub fn edge_cells(&self) -> &[EdgeCell] {
        &self.edge_cells
    }
    pub fn vertices(&self) -> &[VertexMembership] {
        &self.vertices
    }
    pub fn face_ids(&self) -> [TopoId; 2] {
        self.face_ids
    }
    pub fn face_reversed(&self) -> [bool; 2] {
        self.reversed
    }
    pub fn cells(&self) -> usize {
        self.work.cells
    }
    pub fn pairs(&self) -> usize {
        self.work.pairs
    }
    pub fn range_operations(&self) -> usize {
        self.work.range_operations
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = qualify(self.curves, self.faces, self.tolerance_uv, self.limits)?;
        if fresh.edge_cells != self.edge_cells
            || fresh.vertices != self.vertices
            || fresh.face_ids != self.face_ids
            || fresh.reversed != self.reversed
            || fresh.work != self.work
        {
            return Err(unproven(
                "Saved SS face membership differs from fresh original proof",
            ));
        }
        Ok(())
    }
}
#[derive(Default, PartialEq)]
struct Work {
    cells: usize,
    pairs: usize,
    range_operations: usize,
}
fn invalid(s: &str) -> Error {
    Error::new("BREP_SS_FACE_MEMBERSHIP_INPUT", s)
}
fn unproven(s: &str) -> Error {
    Error::new("BREP_SS_FACE_MEMBERSHIP_UNPROVEN", s)
}
fn cells_left(w: &Work, l: Limits) -> Result<usize> {
    if w.cells >= l.cells {
        return Err(unproven("Shared SS face cell allowance exhausted"));
    }
    Ok((l.cells - w.cells).min(100000))
}
fn range_left(w: &Work, l: Limits) -> Result<usize> {
    if w.range_operations >= l.range_operations {
        return Err(unproven("Shared original range work exhausted"));
    }
    Ok((l.range_operations - w.range_operations).min(4096))
}
fn prepare(
    model: &Model,
    face: usize,
    tolerance: f64,
    w: &mut Work,
    l: Limits,
) -> Result<(FaceDomain, bool)> {
    model.validate_boundary_diagnostic_inputs()?;
    if !model.persistent_naming_complete() {
        return Err(invalid("Face identities are incomplete"));
    }
    let f = model
        .faces
        .get(face)
        .ok_or_else(|| invalid("Unknown support face"))?;
    let uses: Vec<_> = model
        .shells
        .iter()
        .flat_map(|s| &s.faces)
        .filter(|u| u.face == face)
        .collect();
    if uses.len() != 1 {
        return Err(invalid("Face needs one authored shell use"));
    }
    let loops: Vec<Vec<_>> = std::iter::once(f.outer)
        .chain(f.holes.iter().copied())
        .map(|i| {
            model.loops[i]
                .coedges
                .iter()
                .map(|c| c.pcurve.clone())
                .collect()
        })
        .collect();
    if w.pairs >= l.pairs || l.cells - w.cells < 2 {
        return Err(unproven("Shared face audit work exhausted"));
    }
    let half = ((l.cells - w.cells) / 2).min(100000);
    let audit = trim_region_audit::inspect(
        &loops,
        tolerance,
        (l.pairs - w.pairs).min(100000),
        half,
        half,
    )?;
    w.pairs += audit.pairs;
    w.cells += audit.cells + audit.domain_cells;
    if audit.valid != Some(true) {
        return Err(unproven("Original face trim region is unproven"));
    }
    for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
        for c in &model.loops[wire].coedges {
            let proof = agreement::verify(
                &model.edges[c.edge].curve,
                &c.pcurve,
                &f.surface,
                c.reversed,
                model.tolerance_mm,
                cells_left(w, l)?,
            )?;
            w.cells += proof.cells;
            if proof.status != agreement::Status::WithinTolerance {
                return Err(unproven("Actual face boundary lift is unproven"));
            }
        }
    }
    Ok((FaceDomain::new(model, face, tolerance)?, uses[0].reversed))
}
fn envelope(original: [[f64; 2]; 2], error: f64, domain: [[f64; 2]; 2]) -> Result<[[f64; 2]; 2]> {
    let mut result = original;
    for a in 0..2 {
        let q = I::new(original[a][0], original[a][1])?.add(I::new(-error, error)?)?;
        // Original owner and saved-pcurve proof independently establish natural
        // domain membership. Clipping an overestimate uses those facts only.
        result[a] = [q.lo.max(domain[a][0]), q.hi.min(domain[a][1])];
        I::new(result[a][0], result[a][1])?;
    }
    Ok(result)
}
fn classify(domain: &FaceDomain, uv: [[f64; 2]; 2], w: &mut Work, l: Limits) -> Result<Location> {
    if domain.contains_closed_rectangle(uv) || domain.on_linear_boundary(uv) {
        return Ok(Location::Inside);
    }
    let proof = domain.classify(uv, cells_left(w, l)?)?;
    w.cells += proof.cells;
    Ok(proof.location)
}
/// Prove complete membership/exclusion of every original edge clock and vertex
/// against both actual faces, including saved approximation displacement.
/// Crossing trim boundaries requires a later root-bound partition constructor;
/// unresolved clocks never yield a partial qualified owner.
pub fn qualify<'a>(
    curves: &'a QualifiedGraphCurves,
    faces: [(&'a Model, usize); 2],
    tolerance_uv: f64,
    limits: Limits,
) -> Result<QualifiedMembership<'a>> {
    if !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || !(1..=1000000).contains(&limits.cells)
        || !(1..=100000).contains(&limits.pairs)
        || !(1..=1000000).contains(&limits.range_operations)
        || limits.depth > 32
    {
        return Err(invalid(
            "Need finite positive tolerance and bounded shared work",
        ));
    }
    curves.reverify()?;
    let original = curves.original();
    if original.kind() == QuadraticGraphSSKind::CoincidentPatch {
        return Err(unproven(
            "Coincident patches require face-overlap region ownership",
        ));
    }
    let mut natural = [[[0.; 2]; 2]; 2];
    for (side, source) in [original.source().first(), original.source().second()]
        .into_iter()
        .enumerate()
    {
        let f = faces[side]
            .0
            .faces
            .get(faces[side].1)
            .ok_or_else(|| invalid("Unknown support face"))?;
        if f.surface != *source {
            return Err(invalid(
                "Face is not the original SS support with its native parameterization",
            ));
        }
        natural[side] = [
            [
                source.knots_u[source.degree_u],
                source.knots_u[source.control_points.len()],
            ],
            [
                source.knots_v[source.degree_v],
                source.knots_v[source.control_points[0].len()],
            ],
        ];
    }
    let mut work = Work::default();
    let (first, r0) = prepare(faces[0].0, faces[0].1, tolerance_uv, &mut work, limits)?;
    let (second, r1) = prepare(faces[1].0, faces[1].1, tolerance_uv, &mut work, limits)?;
    let domains = [first, second];
    let mut vertices = Vec::new();
    for v in curves.vertices() {
        let point = original.vertex_station(v.original_vertex, range_left(&work, limits)?)?;
        work.range_operations += point.operations;
        let mut locations = [Location::Unresolved; 2];
        for (side, bounds) in [point.first_uv, point.second_uv].into_iter().enumerate() {
            locations[side] = classify(
                &domains[side],
                envelope(bounds, v.uv_error_upper[side], natural[side])?,
                &mut work,
                limits,
            )?;
        }
        if locations.contains(&Location::Unresolved) {
            return Err(unproven("Original SS vertex trim membership unresolved"));
        }
        vertices.push(VertexMembership {
            original_vertex: v.original_vertex,
            locations,
        });
    }
    let mut edge_cells = Vec::new();
    for edge in curves.edges() {
        let mut pending = vec![([0., 1.], 0)];
        while let Some((clock, depth)) = pending.pop() {
            // Charge every visited clock, including exact rectangle shortcuts.
            cells_left(&work, limits)?;
            work.cells += 1;
            let range =
                original.edge_enclosure(edge.original_edge, clock, range_left(&work, limits)?)?;
            work.range_operations += range.operations;
            let mut uv = [[[0.; 2]; 2]; 2];
            let mut locations = [Location::Unresolved; 2];
            for (side, bounds) in [range.first_uv, range.second_uv].into_iter().enumerate() {
                uv[side] = envelope(bounds, edge.uv_error_upper[side], natural[side])?;
                locations[side] = classify(&domains[side], uv[side], &mut work, limits)?;
            }
            if locations.contains(&Location::Unresolved) {
                if depth == limits.depth {
                    return Err(unproven(
                        "Original SS clock meets an unresolved trim boundary",
                    ));
                }
                let mid = clock[0] + (clock[1] - clock[0]) * 0.5;
                if !(clock[0] < mid && mid < clock[1]) {
                    return Err(unproven("SS clock subdivision precision exhausted"));
                }
                pending.push(([mid, clock[1]], depth + 1));
                pending.push(([clock[0], mid], depth + 1));
            } else {
                edge_cells.push(EdgeCell {
                    original_edge: edge.original_edge,
                    clock,
                    uv,
                    locations,
                });
            }
        }
    }
    Ok(QualifiedMembership {
        curves,
        faces,
        tolerance_uv,
        limits,
        edge_cells,
        vertices,
        face_ids: [
            faces[0].0.1.faces[faces[0].1],
            faces[1].0.1.faces[faces[1].1],
        ],
        reversed: [r0, r1],
        work,
    })
}
