//! Bind original SS events to audited actual model/loop/coedge definitions.
use super::*;
use nurbs_core::ss_intersection::{
    self as ss, QualifiedGraphBoundaryBindings, QualifiedQuadraticGraphSS,
};
mod cuts;
pub use cuts::{
    CutNode, CutPartitionLimits, CutRepresentative, CutRepresentativeLimits, OpenPathStep,
    OpenSSFaceLimits, OpenSSPairSide, OpenSSPathLimits, OpenSSPathRequest, OpenSSRootGuard,
    OpenSSSegment, PartitionedCutVertex, QualifiedCutPartitions, QualifiedCutRepresentatives,
    QualifiedOpenSSFace, QualifiedOpenSSPair, QualifiedOpenSSPath, partition_cut_representatives,
    partition_open_ss_pair, partition_open_ss_path, qualify_cut_representatives,
    qualify_open_ss_path,
};
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug)]
pub struct CoedgeRootLimits {
    pub face_cells: usize,
    pub face_pairs: usize,
    /// Shared source replay, root, binding and event-transport operations.
    pub operations: usize,
    pub coedges: usize,
    pub root_depth: usize,
    pub clock_tolerance: f64,
    pub world_tolerance: f64,
}
impl Default for CoedgeRootLimits {
    fn default() -> Self {
        Self {
            face_cells: 100000,
            face_pairs: 10000,
            operations: 1000000,
            coedges: 512,
            root_depth: 64,
            clock_tolerance: 1e-8,
            world_tolerance: 1e-7,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoedgeBoundaryRole {
    Outer,
    Hole(usize),
}
#[derive(Clone, Debug, PartialEq)]
pub struct CoedgeRootAddress {
    pub side: usize,
    pub shell: usize,
    pub shell_identity: TopoId,
    pub face_use: usize,
    pub face: usize,
    pub face_identity: TopoId,
    pub face_reversed: bool,
    pub wire: usize,
    pub wire_identity: TopoId,
    pub role: CoedgeBoundaryRole,
    pub coedge: usize,
    pub edge: usize,
    pub edge_identity: TopoId,
    pub edge_reversed: bool,
    pub edge_degenerate: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CoedgeRootEvent {
    pub original_root: usize,
    pub pcurve_parameter: [f64; 2],
    /// Increasing enclosure; actual coedge reversal remains in its address.
    pub edge_parameter: [f64; 2],
    pub source_world: [[f64; 2]; 3],
    /// Contains the actual saved 3D edge position at the original event clock,
    /// using its freshly verified whole-clock model-tolerance lift agreement.
    pub actual_edge_world: [[f64; 2]; 3],
    pub other_face_uv: [[f64; 2]; 2],
    pub other_face_location: Location,
    /// (other loop index, coedge ordinal), only after a literal pcurve match
    /// in identical original native domains and fresh other-face auditing.
    pub other_boundary: Option<(usize, usize)>,
}
pub struct QualifiedCoedgeRoots {
    address: CoedgeRootAddress,
    bindings: QualifiedGraphBoundaryBindings,
    events: Vec<CoedgeRootEvent>,
}
impl QualifiedCoedgeRoots {
    pub fn address(&self) -> &CoedgeRootAddress {
        &self.address
    }
    pub fn bindings(&self) -> &QualifiedGraphBoundaryBindings {
        &self.bindings
    }
    pub fn events(&self) -> &[CoedgeRootEvent] {
        &self.events
    }
}
/// Borrowed models/source cannot change while this nonserializable owner lives.
/// Complete boundary coverage is not complete intersection-region coverage.
pub struct QualifiedFacePairRoots<'a> {
    original: &'a QualifiedQuadraticGraphSS,
    faces: [(&'a Model, usize); 2],
    tolerance_uv: f64,
    limits: CoedgeRootLimits,
    records: Vec<QualifiedCoedgeRoots>,
    work: Work,
    operations: usize,
}
impl QualifiedFacePairRoots<'_> {
    pub fn original(&self) -> &QualifiedQuadraticGraphSS {
        self.original
    }
    pub fn coedges(&self) -> &[QualifiedCoedgeRoots] {
        &self.records
    }
    pub fn operations(&self) -> usize {
        self.operations
    }
    pub fn cells(&self) -> usize {
        self.work.cells
    }
    pub fn pairs(&self) -> usize {
        self.work.pairs
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh =
            qualify_coedge_roots(self.original, self.faces, self.tolerance_uv, self.limits)?;
        let same = self.records.len() == fresh.records.len()
            && self.records.iter().zip(&fresh.records).all(|(a, b)| {
                let ca = a.bindings.census();
                let cb = b.bindings.census();
                a.address == b.address
                    && a.events == b.events
                    && a.bindings.bindings() == b.bindings.bindings()
                    && a.bindings.operations() == b.bindings.operations()
                    && ca.roots() == cb.roots()
                    && ca.boundary() == cb.boundary()
                    && ca.side() == cb.side()
                    && ca.operations() == cb.operations()
                    && ca.original().source().first() == cb.original().source().first()
                    && ca.original().source().second() == cb.original().source().second()
            });
        if !same || self.work != fresh.work || self.operations != fresh.operations {
            return Err(unproven("Actual face/coedge root ownership changed"));
        }
        Ok(())
    }
}
fn spend(used: &mut usize, max: usize, n: usize) -> Result<()> {
    if n > max - *used {
        return Err(unproven(
            "Shared actual coedge root operation allowance exhausted",
        ));
    }
    *used += n;
    Ok(())
}
fn edge_parameter(domain: [f64; 2], clock: [f64; 2], reversed: bool) -> Result<[f64; 2]> {
    let t = I::new(clock[0], clock[1])?;
    let t = if reversed {
        I::point(1.).sub(t)?.intersect(0., 1.)?
    } else {
        t
    };
    let span = I::point(domain[1]).sub(I::point(domain[0]))?;
    let p = I::point(domain[0])
        .add(span.mul(t)?)?
        .intersect(domain[0], domain[1])?;
    Ok([p.lo, p.hi])
}
fn literal_boundary(
    query: &nurbs_core::curve::Curve,
    model: &Model,
    face: usize,
    operations: &mut usize,
    max: usize,
) -> Result<Option<(usize, usize)>> {
    let f = &model.faces[face];
    for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
        for (coedge, c) in model.loops[wire].coedges.iter().enumerate() {
            spend(operations, max, 1)?;
            if c.pcurve == *query {
                return Ok(Some((wire, coedge)));
            }
        }
    }
    Ok(None)
}
fn record_equal_surface(
    original: &QualifiedQuadraticGraphSS,
    faces: [(&Model, usize); 2],
) -> Result<()> {
    for (side, source) in [original.source().first(), original.source().second()]
        .into_iter()
        .enumerate()
    {
        let f = faces[side]
            .0
            .faces
            .get(faces[side].1)
            .ok_or_else(|| invalid("Unknown actual SS support face"))?;
        if f.surface != *source {
            return Err(invalid(
                "Actual face differs from the original SS support/native parameterization",
            ));
        }
    }
    Ok(())
}
/// Fresh complete simple-root census for EVERY actual boundary coedge of both
/// selected faces. Preserve excluded roots and full source addresses. Unsupported
/// windows, unresolved contacts/membership or insufficient shared work return no
/// partial owner. No topology, material-side or Solid authority is granted.
pub fn qualify_coedge_roots<'a>(
    original: &'a QualifiedQuadraticGraphSS,
    faces: [(&'a Model, usize); 2],
    tolerance_uv: f64,
    limits: CoedgeRootLimits,
) -> Result<QualifiedFacePairRoots<'a>> {
    if !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || !(1..=1000000).contains(&limits.face_cells)
        || !(1..=100000).contains(&limits.face_pairs)
        || !(1..=1000000).contains(&limits.operations)
        || !(1..=512).contains(&limits.coedges)
        || limits.root_depth > 64
        || !limits.clock_tolerance.is_finite()
        || limits.clock_tolerance <= 0.
        || limits.clock_tolerance > 1.
        || !limits.world_tolerance.is_finite()
        || limits.world_tolerance <= 0.
    {
        return Err(invalid(
            "Need positive actual coedge root tolerances and bounded shared work",
        ));
    }
    record_equal_surface(original, faces)?;
    let mut operations = 0;
    spend(&mut operations, limits.operations, original.operations())?;
    original.reverify()?;
    if original.kind() == QuadraticGraphSSKind::CoincidentPatch {
        return Err(unproven(
            "Coincident patches require actual overlap-region ownership",
        ));
    }
    let face_limits = Limits {
        cells: limits.face_cells,
        pairs: limits.face_pairs,
        range_operations: 1,
        depth: 0,
    };
    let mut work = Work::default();
    let (d0, _) = prepare(faces[0].0, faces[0].1, tolerance_uv, &mut work, face_limits)?;
    let (d1, _) = prepare(faces[1].0, faces[1].1, tolerance_uv, &mut work, face_limits)?;
    let domains = [d0, d1];
    let natural_of = |s: &nurbs_core::surface::Surface| {
        [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ]
    };
    let same_native =
        natural_of(original.source().first()) == natural_of(original.source().second());
    let mut records = Vec::new();
    for side in 0..2 {
        let (model, face) = faces[side];
        let f = &model.faces[face];
        let (shell, face_use, use_) = model
            .shells
            .iter()
            .enumerate()
            .flat_map(|(s, sh)| sh.faces.iter().enumerate().map(move |(i, u)| (s, i, u)))
            .find(|(_, _, u)| u.face == face)
            .unwrap();
        for (wire_order, wire) in std::iter::once(f.outer)
            .chain(f.holes.iter().copied())
            .enumerate()
        {
            for (coedge, c) in model.loops[wire].coedges.iter().enumerate() {
                if records.len() == limits.coedges {
                    return Err(unproven(
                        "Complete actual boundary exceeds shared coedge allowance",
                    ));
                }
                let edge = &model.edges[c.edge];
                if c.trim().is_some_and(|t| {
                    t.curve_parameter != edge.curve.domain()
                        || t.pcurve_parameter != c.pcurve.domain()
                        || t.periodic_lift != [[0; 2]; 2]
                }) {
                    return Err(unproven(
                        "Partial/directed/periodic coedge windows need original window-clock ownership",
                    ));
                }
                let remaining = limits.operations - operations;
                if remaining == 0 {
                    return Err(unproven("No actual boundary root query work remains"));
                }
                let census = ss::qualify_graph_boundary_roots(
                    original,
                    &c.pcurve,
                    side,
                    ss::GraphBoundaryRootLimits {
                        max_operations: remaining,
                        max_depth: limits.root_depth,
                        clock_tolerance: limits.clock_tolerance,
                        world_tolerance: limits.world_tolerance,
                    },
                )?;
                spend(&mut operations, limits.operations, census.operations())?;
                // Binding freshly replays the immutable root census and original
                // source. Successful replay must reproduce their exact counters;
                // reserve and charge that work rather than hiding it per coedge.
                let replay = census.operations() + original.operations();
                if replay >= limits.operations - operations {
                    return Err(unproven(
                        "No shared work remains for root replay and branch binding",
                    ));
                }
                let bindings = ss::bind_graph_boundary_roots(
                    &census,
                    limits.operations - operations - replay,
                )?;
                spend(
                    &mut operations,
                    limits.operations,
                    replay + bindings.operations(),
                )?;
                let other = 1 - side;
                let s = &faces[other].0.faces[faces[other].1].surface;
                let natural = [
                    [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
                    [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
                ];
                let mut events = Vec::new();
                for (root, r) in bindings.census().roots().iter().enumerate() {
                    spend(
                        &mut operations,
                        limits.operations,
                        8 + usize::from(c.reversed),
                    )?;
                    let edge_parameter = edge_parameter(edge.curve.domain(), r.clock, c.reversed)?;
                    let mut actual_edge_world = r.world;
                    for a in 0..3 {
                        let p = I::new(r.world[a][0], r.world[a][1])?
                            .add(I::new(-model.tolerance_mm, model.tolerance_mm)?)?;
                        actual_edge_world[a] = [p.lo, p.hi];
                    }
                    let mut other_face_uv = r.native_uv[other];
                    for a in 0..2 {
                        let p = I::new(other_face_uv[a][0], other_face_uv[a][1])?
                            .intersect(natural[a][0], natural[a][1])?;
                        other_face_uv[a] = [p.lo, p.hi];
                    }
                    cells_left(&work, face_limits)?;
                    work.cells += 1;
                    let mut other_face_location =
                        classify(&domains[other], other_face_uv, &mut work, face_limits)?;
                    let other_boundary =
                        if other_face_location == Location::Unresolved && same_native {
                            literal_boundary(
                                &c.pcurve,
                                faces[other].0,
                                faces[other].1,
                                &mut operations,
                                limits.operations,
                            )?
                        } else {
                            None
                        };
                    if other_boundary.is_some() {
                        other_face_location = Location::Inside;
                    }
                    if other_face_location == Location::Unresolved {
                        return Err(unproven(
                            "Root membership in the other actual face remains unresolved",
                        ));
                    }
                    events.push(CoedgeRootEvent {
                        original_root: root,
                        pcurve_parameter: r.boundary_parameter,
                        edge_parameter,
                        source_world: r.world,
                        actual_edge_world,
                        other_face_uv,
                        other_face_location,
                        other_boundary,
                    });
                }
                let address = CoedgeRootAddress {
                    side,
                    shell,
                    shell_identity: model.1.shells[shell],
                    face_use,
                    face,
                    face_identity: model.1.faces[face],
                    face_reversed: use_.reversed,
                    wire,
                    wire_identity: model.1.loops[wire],
                    role: if wire_order == 0 {
                        CoedgeBoundaryRole::Outer
                    } else {
                        CoedgeBoundaryRole::Hole(wire_order - 1)
                    },
                    coedge,
                    edge: c.edge,
                    edge_identity: model.1.edges[c.edge],
                    edge_reversed: c.reversed,
                    edge_degenerate: edge.degenerate,
                };
                records.push(QualifiedCoedgeRoots {
                    address,
                    bindings,
                    events,
                });
            }
        }
    }
    Ok(QualifiedFacePairRoots {
        original,
        faces,
        tolerance_uv,
        limits,
        records,
        work,
        operations,
    })
}
