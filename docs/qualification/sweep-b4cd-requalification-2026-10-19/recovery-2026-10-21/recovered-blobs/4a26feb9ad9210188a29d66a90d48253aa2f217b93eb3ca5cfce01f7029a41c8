//! Partition an owned edge once and update every face-local use atomically.
//! Source-domain and face-lift checks qualify rounded authoring continuously.
//! Root isolation and global solid admission remain separate operations.
use crate::{Coedge, Edge, Model, Vertex};
use nurbs_core::{
    Error, Result,
    curve::Curve,
    curve_surface_agreement::{self, Status},
    surface::Surface,
};

#[derive(Clone, Copy, Debug)]
pub struct PcurveCut {
    pub wire: usize,
    pub coedge: usize,
    /// Natural parameter on this use's original pcurve, not on the 3D curve.
    pub parameter: f64,
}
pub struct Partition {
    /// Candidate only: fresh embedding/volume qualification is still required.
    pub model: Model,
    pub source_edge: usize,
    pub source_identity: crate::TopoId,
    pub child_identities: [crate::TopoId; 2],
    pub child_edges: [usize; 2],
    pub vertex: usize,
    pub source_parameter: f64,
    pub cells: usize,
}
fn invalid(message: &str) -> Error {
    Error::new("BREP_EDGE_PARTITION_REFUSED", message)
}
fn unresolved(message: &str) -> Error {
    Error::new("BREP_EDGE_PARTITION_UNPROVEN", message)
}
fn check_agreement(
    curve: &Curve,
    pcurve: &Curve,
    surface: &Surface,
    reversed: bool,
    tolerance: f64,
    cells: &mut usize,
    max_cells: usize,
) -> Result<()> {
    if *cells == max_cells {
        return Err(unresolved("Shared correspondence budget exhausted"));
    }
    let proof = curve_surface_agreement::verify(
        curve,
        pcurve,
        surface,
        reversed,
        tolerance,
        (max_cells - *cells).min(100000),
    )?;
    *cells += proof.cells;
    if proof.status != Status::WithinTolerance {
        return Err(unresolved(
            "Original-domain or face-lift agreement is unresolved or mismatching",
        ));
    }
    Ok(())
}

/// Split a nonperiodic 3D edge and all one or two incident face-local pcurves.
/// Independent natural pcurve cut parameters must be supplied for every use.
/// Children share one authored vertex, including both adjacent faces. Their
/// endpoints are assigned from one evaluation of the original 3D curve; every
/// resulting child is independently checked against the complete original
/// representation and each original support surface. No endpoint-only proof,
/// snapping authorization, exact-identity claim or solid certificate is implied.
/// Returns no model if any continuous proof or structural check fails.
pub fn partition(
    source: &Model,
    edge_id: usize,
    parameter: f64,
    cuts: &[PcurveCut],
    max_cells: usize,
) -> Result<Partition> {
    source.validate()?;
    if !source.persistent_naming_complete() {
        return Err(invalid("Source topology identities must be complete"));
    }
    if !(1..=1000000).contains(&max_cells) {
        return Err(invalid("Work must be in 1..1000000 cells"));
    }
    let edge = source
        .edges
        .get(edge_id)
        .ok_or_else(|| invalid("Edge index is out of range"))?;
    let [a, b] = edge.curve.domain();
    if edge.degenerate
        || edge.curve.periodic
        || !parameter.is_finite()
        || !(a < parameter && parameter < b)
    {
        return Err(invalid(
            "Need a nondegenerate nonperiodic edge and an interior natural parameter",
        ));
    }
    let mut uses = vec![];
    for (face, f) in source.faces.iter().enumerate() {
        for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            for (coedge, c) in source.loops[wire].coedges.iter().enumerate() {
                if c.edge == edge_id {
                    uses.push((face, wire, coedge));
                }
            }
        }
    }
    if !(1..=2).contains(&uses.len())
        || cuts.len() != uses.len()
        || cuts.iter().any(|c| {
            cuts.iter()
                .filter(|d| d.wire == c.wire && d.coedge == c.coedge)
                .count()
                != 1
        })
        || cuts
            .iter()
            .any(|c| !uses.iter().any(|&(_, w, k)| w == c.wire && k == c.coedge))
    {
        return Err(invalid(
            "Exactly one independent pcurve cut is required for every incident use",
        ));
    }

    let position: [f64; 3] = edge
        .curve
        .evaluate(parameter)?
        .point
        .try_into()
        .map_err(|_| invalid("Edge must be three-dimensional"))?;
    let mut children = edge.curve.split(parameter)?;
    let last = children[0].control_points.len() - 1;
    children[0].control_points[last] = position.to_vec();
    children[1].control_points[0] = position.to_vec();

    // An exact representation copy of C(u), independent of v. This proof-only
    // carrier retains the original knots, weights and poles, rather than an
    // extracted/rounded proxy for the original restriction.
    let carrier = Surface {
        degree_u: edge.curve.degree,
        degree_v: 1,
        knots_u: edge.curve.knots.clone(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: edge
            .curve
            .control_points
            .iter()
            .map(|p| vec![p.clone(), p.clone()])
            .collect(),
        weights: edge.curve.weights.iter().map(|&w| vec![w, w]).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let mut cells = 0;
    for (child, domain) in children.iter().zip([[a, parameter], [parameter, b]]) {
        let pcurve = crate::line(vec![domain[0], 0.], vec![domain[1], 0.]);
        check_agreement(
            child,
            &pcurve,
            &carrier,
            false,
            source.tolerance_mm,
            &mut cells,
            max_cells,
        )?;
    }
    let mut model = source.clone();
    let vertex = model.vertices.len();
    model.vertices.push(Vertex { point: position });
    let child_edges = [edge_id, model.edges.len()];
    model.edges[edge_id] = Edge {
        degenerate: false,
        vertices: [edge.vertices[0], vertex],
        curve: children[0].clone(),
    };
    model.edges.push(Edge {
        degenerate: false,
        vertices: [vertex, edge.vertices[1]],
        curve: children[1].clone(),
    });
    for (face, wire, coedge) in uses {
        let original = &source.loops[wire].coedges[coedge];
        let cut = cuts
            .iter()
            .find(|c| c.wire == wire && c.coedge == coedge)
            .unwrap();
        if original.pcurve.periodic {
            return Err(invalid(
                "Periodic pcurve partition needs explicit lifted chart ownership",
            ));
        }
        let [p0, p1] = original.pcurve.domain();
        if !cut.parameter.is_finite() || !(p0 < cut.parameter && cut.parameter < p1) {
            return Err(invalid(
                "Every pcurve cut must be strictly inside its original natural domain",
            ));
        }
        let pc = original.pcurve.split(cut.parameter)?;
        let order = if original.reversed { [1, 0] } else { [0, 1] };
        let replacements: Vec<_> = pc
            .into_iter()
            .zip(order)
            .map(|(pcurve, i)| Coedge {
                edge: child_edges[i],
                reversed: original.reversed,
                pcurve,
            })
            .collect();
        for c in &replacements {
            check_agreement(
                &model.edges[c.edge].curve,
                &c.pcurve,
                &source.faces[face].surface,
                c.reversed,
                source.tolerance_mm,
                &mut cells,
                max_cells,
            )?;
        }
        // A valid face uses this source edge at most once in each loop. Reject
        // ambiguous self-seams instead of shifting addresses during mutation.
        if model.loops[wire]
            .coedges
            .iter()
            .filter(|c| c.edge == edge_id)
            .count()
            != 1
        {
            return Err(invalid(
                "Repeated same-loop edge uses need explicit seam ownership",
            ));
        }
        model.loops[wire]
            .coedges
            .splice(coedge..=coedge, replacements);
    }
    model.inherit_topology_ids(&[source]);
    let source_identity = source.1.edges[edge_id];
    let child_identities = child_edges.map(|i| model.1.edges[i]);
    model.1.lineage.push(crate::TopologyLineageRecord {
        operation: "split".into(),
        entity_kind: "edge".into(),
        parents: vec![source_identity],
        children: child_identities.to_vec(),
    });
    model.refresh_change_set(&[source]);
    if !model.persistent_naming_complete() {
        return Err(unresolved(
            "Partition did not retain complete persistent topology identities",
        ));
    }
    model.validate()?;
    Ok(Partition {
        model,
        source_edge: edge_id,
        source_identity,
        child_identities,
        child_edges,
        vertex,
        source_parameter: parameter,
        cells,
    })
}

// Prefer a low-complexity binary fraction inside the certified band. This
// only selects a representative; original-domain, root-position and face-lift
// proofs remain mandatory. No root or support is moved to match the proposal.
pub(crate) fn root_representative(range: [f64; 2], domain: [f64; 2]) -> f64 {
    let middle = range[0] + (range[1] - range[0]) * 0.5;
    let magnitude = middle.abs().max((range[1] - range[0]).abs());
    let mut step = if magnitude > 0. {
        2_f64.powi((magnitude.log2().ceil() as i32).clamp(-1074, 1023))
    } else {
        f64::from_bits(1)
    };
    for _ in 0..2100 {
        let proposed = (middle / step).round() * step;
        if proposed.is_finite()
            && range[0] <= proposed
            && proposed <= range[1]
            && domain[0] < proposed
            && proposed < domain[1]
        {
            return proposed;
        }
        step *= 0.5;
        if step == 0. {
            break;
        }
    }
    middle
}

pub struct RootPartition {
    pub partition: Partition,
    /// Certified original natural-parameter root band, not a rounded trim proxy.
    pub root_parameter: [f64; 2],
    pub root_position_bounds: [[f64; 2]; 3],
    pub support_uv_bounds: [[f64; 2]; 2],
    /// Present when the full curve/surface operator proves oriented crossing.
    pub crossing_sign: Option<i8>,
    /// Distance from the one authored vertex to every point in the root box.
    pub root_position_error_upper: f64,
    pub root_cells: usize,
}

/// Automatically isolate a transverse crossing with an admitted finite planar
/// chart, then partition the owned edge and all incident pcurves. The root query
/// is recomputed on the immutable original representation; caller-supplied root
/// reports never authorize mutation. Current planar charts are axis-aligned
/// rectangles admitted by the native scalar-root prover. General SS cuts and
/// tangencies remain unresolved. The authored vertex approximates the true root
/// within the model tolerance; it is not relabeled an exact mathematical root.
/// Pcurve cut parameters are proposals from normalized original traversal and
/// must pass the same whole-child proof as explicit partitioning.
pub fn partition_at_plane(
    source: &Model,
    edge_id: usize,
    plane: &Surface,
    crossing: usize,
    max_cells: usize,
) -> Result<PlanePartition> {
    use nurbs_core::intersection::{self, ContactClass};
    source.validate()?;
    if !(1..=1000000).contains(&max_cells) {
        return Err(invalid("Work must be in 1..1000000 cells"));
    }
    let edge = source
        .edges
        .get(edge_id)
        .ok_or_else(|| invalid("Edge index is out of range"))?;
    let roots = intersection::intersect_curve_plane_certified_report(
        &edge.curve,
        plane,
        source.tolerance_mm,
        max_cells.min(100000),
    )?;
    if !roots.complete || roots.overlap.is_some() {
        return Err(unresolved(
            "Plane crossing coverage is unresolved or an overlap",
        ));
    }
    let root = roots
        .crossings
        .get(crossing)
        .ok_or_else(|| invalid("Certified crossing index is out of range"))?;
    if root.contact != ContactClass::Transverse {
        return Err(unresolved(
            "Partition requires an isolated transverse crossing",
        ));
    }
    if roots.cells >= max_cells {
        return Err(unresolved("Root query exhausted the shared work budget"));
    }
    partition_at_proved_root(
        source,
        edge_id,
        root.curve_parameter,
        root.position_bounds,
        root.uv_bounds,
        None,
        roots.cells,
        max_cells,
    )
}

/// Compatibility name for the previous plane-only result.
pub type PlanePartition = RootPartition;

/// Isolate original curve/surface roots with complete interval coverage before
/// atomically splitting all incident uses. The support is its full rectangular
/// NURBS parameter domain; trimming holes and boundary/tangent roots need separate
/// classification. No approximate legacy report or supplied Boolean grants
/// authoring. Positional precision and whole-child agreement remain mandatory.
pub fn partition_at_surface(
    source: &Model,
    edge_id: usize,
    support: &Surface,
    crossing: usize,
    max_cells: usize,
) -> Result<RootPartition> {
    source.validate()?;
    if !(1..=1000000).contains(&max_cells) {
        return Err(invalid("Work must be in 1..1000000 cells"));
    }
    let edge = source
        .edges
        .get(edge_id)
        .ok_or_else(|| invalid("Edge index is out of range"))?;
    let roots = nurbs_core::intersection::certified_curve_surface::intersections(
        &edge.curve,
        support,
        max_cells.min(100000),
    )?;
    if !roots.complete {
        return Err(unresolved(
            "Original curve/surface root coverage remains unresolved",
        ));
    }
    let root = roots
        .roots
        .get(crossing)
        .ok_or_else(|| invalid("Certified crossing index is out of range"))?;
    partition_at_proved_root(
        source,
        edge_id,
        root.parameter_box[0],
        root.position_bounds,
        [root.parameter_box[1], root.parameter_box[2]],
        Some(root.crossing_sign),
        roots.cells,
        max_cells,
    )
}

pub struct FacePartition {
    pub result: RootPartition,
    pub coverage: crate::face_crossings::Report,
}

/// Partition only at a root admitted inside the actual trimmed support face.
/// `crossing` selects the retained Inside roots in original parameter order.
/// All original surface roots and unresolved cells are classified first; no
/// supplied classification report or center-point test grants admission.
pub fn partition_at_face(
    source: &Model,
    edge_id: usize,
    support: &Model,
    face_id: usize,
    crossing: usize,
    tolerance_uv: f64,
    max_cells: usize,
) -> Result<FacePartition> {
    let coverage =
        crate::face_crossings::inspect(source, edge_id, support, face_id, tolerance_uv, max_cells)?;
    if !coverage.complete {
        return Err(unresolved(
            "Trimmed-face root coverage or boundary membership remains unresolved",
        ));
    }
    let index = *coverage
        .eligible
        .get(crossing)
        .ok_or_else(|| invalid("Retained face crossing index is out of range"))?;
    let root = &coverage.roots[index];
    let mut result = partition_at_proved_root(
        source,
        edge_id,
        root.root.parameter_box[0],
        root.root.position_bounds,
        [root.root.parameter_box[1], root.root.parameter_box[2]],
        Some(root.face_crossing_sign),
        coverage.cells,
        max_cells,
    )?;
    result.root_cells = coverage.root_cells;
    Ok(FacePartition { result, coverage })
}

fn partition_at_proved_root(
    source: &Model,
    edge_id: usize,
    root_parameter: [f64; 2],
    position_bounds: [[f64; 2]; 3],
    support_uv_bounds: [[f64; 2]; 2],
    crossing_sign: Option<i8>,
    root_cells: usize,
    max_cells: usize,
) -> Result<RootPartition> {
    use nurbs_core::interval_eval::Interval as I;
    if root_cells >= max_cells {
        return Err(unresolved("Root query exhausted the shared work budget"));
    }
    let edge = source
        .edges
        .get(edge_id)
        .ok_or_else(|| invalid("Edge index is out of range"))?;
    let parameter = root_representative(root_parameter, edge.curve.domain());
    let point = edge.curve.evaluate(parameter)?.point;
    let mut squared = I::point(0.);
    for k in 0..3 {
        let delta =
            I::point(point[k]).sub(I::new(position_bounds[k][0], position_bounds[k][1])?)?;
        squared = squared.add(delta.mul(delta)?)?;
    }
    let error_upper = squared.hi.max(0.).sqrt().next_up();
    if !error_upper.is_finite() || error_upper > source.tolerance_mm {
        return Err(unresolved(
            "Authored shared vertex is not proved within the certified root-position box tolerance",
        ));
    }
    let [a, b] = edge.curve.domain();
    let fraction = (parameter - a) / (b - a);
    let cuts: Vec<_> = source
        .loops
        .iter()
        .enumerate()
        .flat_map(|(wire, l)| {
            l.coedges
                .iter()
                .enumerate()
                .filter(move |(_, c)| c.edge == edge_id)
                .map(move |(coedge, c)| {
                    let [p0, p1] = c.pcurve.domain();
                    let t = if c.reversed { 1. - fraction } else { fraction };
                    PcurveCut {
                        wire,
                        coedge,
                        parameter: p0 + (p1 - p0) * t,
                    }
                })
        })
        .collect();
    let mut result = partition(source, edge_id, parameter, &cuts, max_cells - root_cells)?;
    result.cells += root_cells;
    Ok(RootPartition {
        partition: result,
        root_parameter: root_parameter,
        root_position_bounds: position_bounds,
        support_uv_bounds,
        crossing_sign,
        root_position_error_upper: error_upper,
        root_cells: root_cells,
    })
}

/// Atomic subdivision at every retained transverse root of one trimmed face.
/// This is the shared-edge stage of trimming, not a material-sector classifier.
pub struct FacePartitions {
    pub model: Model,
    pub coverage: crate::face_crossings::Report,
    /// Final children in increasing original natural parameter order.
    pub edges: Vec<usize>,
    /// Root vertices in the same order as coverage.eligible.
    pub vertices: Vec<usize>,
    pub cells: usize,
}

/// Compute coverage once on the immutable original edge, then consume roots
/// from right to left. Never rediscover roots on rounded children. Every final
/// child is checked against the original curve as well as both incident uses;
/// intermediate split errors cannot accumulate into an unchecked admission.
/// No roots is a successful unchanged candidate. Unknown membership, touching
/// root boxes, chart ambiguity or any exhausted proof rolls back the whole batch.
pub fn partition_all_at_face(
    source: &Model,
    edge_id: usize,
    support: &Model,
    face_id: usize,
    tolerance_uv: f64,
    max_cells: usize,
) -> Result<FacePartitions> {
    let coverage = crate::face_crossings::inspect(
        source, edge_id, support, face_id, tolerance_uv, max_cells,
    )?;
    if !coverage.complete {
        return Err(unresolved("All trimmed-face root memberships must be resolved"));
    }
    for pair in coverage.eligible.windows(2) {
        if coverage.roots[pair[0]].root.parameter_box[0][1]
            >= coverage.roots[pair[1]].root.parameter_box[0][0] {
            return Err(unresolved("Retained root intervals must be strictly separated"));
        }
    }
    let mut model = source.clone();
    let mut edges = Vec::with_capacity(coverage.eligible.len() + 1);
    let mut vertices = Vec::with_capacity(coverage.eligible.len());
    let mut cells = coverage.cells;
    for &index in coverage.eligible.iter().rev() {
        if cells >= max_cells {
            return Err(unresolved("Shared all-root partition budget exhausted"));
        }
        let root = &coverage.roots[index];
        let split = partition_at_proved_root(
            &model, edge_id, root.root.parameter_box[0], root.root.position_bounds,
            [root.root.parameter_box[1], root.root.parameter_box[2]],
            Some(root.face_crossing_sign), 0, max_cells - cells,
        )?;
        cells += split.partition.cells;
        edges.push(split.partition.child_edges[1]);
        vertices.push(split.partition.vertex);
        model = split.partition.model;
    }
    edges.push(edge_id);
    edges.reverse();
    vertices.reverse();
    if !vertices.is_empty() {
        let original = &source.edges[edge_id].curve;
        let carrier = Surface {
            degree_u: original.degree, degree_v: 1,
            knots_u: original.knots.clone(), knots_v: vec![0., 0., 1., 1.],
            control_points: original.control_points.iter().map(|p| vec![p.clone(), p.clone()]).collect(),
            weights: original.weights.iter().map(|&w| vec![w, w]).collect(),
            periodic_u: false, periodic_v: false,
        };
        for &edge in &edges {
            let child = &model.edges[edge].curve;
            let [a, b] = child.domain();
            let pc = crate::line(vec![a, 0.], vec![b, 0.]);
            check_agreement(child, &pc, &carrier, false, source.tolerance_mm, &mut cells, max_cells)?;
        }
    }
    Ok(FacePartitions { model, coverage, edges, vertices, cells })
}
