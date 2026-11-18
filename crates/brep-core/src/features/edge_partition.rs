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

mod authored;
pub use authored::partition;
pub(crate) use authored::partition_with_position;
mod vertex_proposal;
pub use vertex_proposal::partition_to_vertex;

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
        ProvedRoot {
            parameter: root.curve_parameter,
            position_bounds: root.position_bounds,
            support_uv_bounds: root.uv_bounds,
            crossing_sign: None,
        },
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
        ProvedRoot {
            parameter: root.parameter_box[0],
            position_bounds: root.position_bounds,
            support_uv_bounds: [root.parameter_box[1], root.parameter_box[2]],
            crossing_sign: Some(root.crossing_sign),
        },
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
        ProvedRoot {
            parameter: root.root.parameter_box[0],
            position_bounds: root.root.position_bounds,
            support_uv_bounds: [root.root.parameter_box[1], root.root.parameter_box[2]],
            crossing_sign: Some(root.face_crossing_sign),
        },
        coverage.cells,
        max_cells,
    )?;
    result.root_cells = coverage.root_cells;
    Ok(FacePartition { result, coverage })
}

/// A proved edge root: its curve-parameter interval, world position box,
/// support UV box and, when known, the face crossing sign.
#[derive(Clone, Copy)]
struct ProvedRoot {
    parameter: [f64; 2],
    position_bounds: [[f64; 2]; 3],
    support_uv_bounds: [[f64; 2]; 2],
    crossing_sign: Option<i8>,
}
fn partition_at_proved_root(
    source: &Model,
    edge_id: usize,
    root: ProvedRoot,
    root_cells: usize,
    max_cells: usize,
) -> Result<RootPartition> {
    let ProvedRoot {
        parameter: root_parameter,
        position_bounds,
        support_uv_bounds,
        crossing_sign,
    } = root;
    use nurbs_core::interval_eval::Interval as I;
    if root_cells >= max_cells {
        return Err(unresolved("Root query exhausted the shared work budget"));
    }
    let edge = source
        .edges
        .get(edge_id)
        .ok_or_else(|| invalid("Edge index is out of range"))?;
    let parameter = root_representative(root_parameter, edge.curve.domain());
    let incident_uses = source
        .loops
        .iter()
        .flat_map(|w| &w.coedges)
        .filter(|c| c.edge == edge_id)
        .count();
    let mut result = if incident_uses == 2 {
        use crate::feature_ops::edge_partition as shared;
        let split = shared::split(
            source,
            edge_id,
            &[parameter],
            "root-bound-single",
            shared::Limits {
                max_cells: (max_cells - root_cells).min(100000),
                ..Default::default()
            },
        )?;
        if split.edges().len() != 2 || split.vertices().len() != 1 {
            return Err(unresolved(
                "Single shared root lost its topology-owned partition",
            ));
        }
        let children = [split.edges()[0], split.edges()[1]];
        Partition {
            source_edge: edge_id,
            source_identity: source.1.edges[edge_id],
            child_identities: children.map(|e| split.model().1.edges[e]),
            child_edges: children,
            vertex: split.vertices()[0],
            source_parameter: parameter,
            cells: split.evidence().cells,
            model: split.model().clone(),
        }
    } else {
        // Preserve the separate one-sided open-sheet operation. A failed
        // shared-edge proof never falls back to this path.
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
        partition(source, edge_id, parameter, &cuts, max_cells - root_cells)?
    };
    result.cells += root_cells;
    // Certify the saved common vertex, which knot insertion may round
    // differently from an independent evaluation of the same rational point.
    let point = result.model.vertices[result.vertex].point;
    let mut squared = I::point(0.);
    for k in 0..3 {
        let delta =
            I::point(point[k]).sub(I::new(position_bounds[k][0], position_bounds[k][1])?)?;
        squared = squared.add(delta.mul(delta)?)?;
    }
    let error_upper = squared.hi.max(0.).sqrt().next_up();
    let tolerance = source
        .1
        .local_tolerances_mm
        .get(&source.1.edges[edge_id])
        .copied()
        .unwrap_or(source.tolerance_mm)
        .min(source.tolerance_mm);
    if !error_upper.is_finite() || error_upper > tolerance {
        return Err(unresolved(
            "Saved shared root vertex is not bounded at the original local spatial tolerance",
        ));
    }
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
    let coverage =
        crate::face_crossings::inspect(source, edge_id, support, face_id, tolerance_uv, max_cells)?;
    if !coverage.complete {
        return Err(unresolved(
            "All trimmed-face root memberships must be resolved",
        ));
    }
    for pair in coverage.eligible.windows(2) {
        if coverage.roots[pair[0]].root.parameter_box[0][1]
            >= coverage.roots[pair[1]].root.parameter_box[0][0]
        {
            return Err(unresolved(
                "Retained root intervals must be strictly separated",
            ));
        }
    }
    // Closed/shared edges use one immutable-original partition for every root.
    // This preserves authored pcurve trim relations instead of recomputing
    // fractions on successively rounded child curves.
    let incident_uses = source
        .loops
        .iter()
        .flat_map(|w| &w.coedges)
        .filter(|c| c.edge == edge_id)
        .count();
    if incident_uses == 2 && !coverage.eligible.is_empty() {
        use crate::feature_ops::edge_partition as shared;
        if coverage.cells >= max_cells {
            return Err(unresolved("No shared work remains after root isolation"));
        }
        let original = &source.edges[edge_id];
        let domain = original.curve.domain();
        let parameters: Vec<_> = coverage
            .eligible
            .iter()
            .map(|&i| root_representative(coverage.roots[i].root.parameter_box[0], domain))
            .collect();
        let tolerance = source
            .1
            .local_tolerances_mm
            .get(&source.1.edges[edge_id])
            .copied()
            .unwrap_or(source.tolerance_mm)
            .min(source.tolerance_mm);
        let limits = shared::Limits {
            max_cells: (max_cells - coverage.cells).min(100000),
            ..Default::default()
        };
        let split = shared::split_with_uv_tolerance(
            source,
            edge_id,
            &parameters,
            "trimmed-face-root-batch",
            tolerance_uv,
            limits,
        )?;
        if split.vertices().len() != coverage.eligible.len() {
            return Err(unresolved("Shared root partition lost endpoint ownership"));
        }
        for (k, &root_index) in coverage.eligible.iter().enumerate() {
            use nurbs_core::interval_eval::Interval as I;
            let saved = split.model().vertices[split.vertices()[k]].point;
            let root = &coverage.roots[root_index].root;
            let mut squared = I::point(0.);
            for axis in 0..3 {
                let delta = I::point(saved[axis]).sub(I::new(
                    root.position_bounds[axis][0],
                    root.position_bounds[axis][1],
                )?)?;
                squared = squared.add(delta.mul(delta)?)?;
            }
            let error = squared.hi.max(0.).sqrt().next_up();
            if !error.is_finite() || error > tolerance {
                return Err(unresolved(
                    "Actual saved endpoint is not bounded to its original retained root",
                ));
            }
        }
        return Ok(FacePartitions {
            model: split.model().clone(),
            cells: coverage.cells + split.evidence().cells,
            coverage,
            edges: split.edges().to_vec(),
            vertices: split.vertices().to_vec(),
        });
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
            &model,
            edge_id,
            ProvedRoot {
                parameter: root.root.parameter_box[0],
                position_bounds: root.root.position_bounds,
                support_uv_bounds: [root.root.parameter_box[1], root.root.parameter_box[2]],
                crossing_sign: Some(root.face_crossing_sign),
            },
            0,
            max_cells - cells,
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
            degree_u: original.degree,
            degree_v: 1,
            knots_u: original.knots.clone(),
            knots_v: vec![0., 0., 1., 1.],
            control_points: original
                .control_points
                .iter()
                .map(|p| vec![p.clone(), p.clone()])
                .collect(),
            weights: original.weights.iter().map(|&w| vec![w, w]).collect(),
            periodic_u: false,
            periodic_v: false,
        };
        for &edge in &edges {
            let child = &model.edges[edge].curve;
            let [a, b] = child.domain();
            let pc = crate::line(vec![a, 0.], vec![b, 0.]);
            check_agreement(
                child,
                &pc,
                &carrier,
                false,
                source.tolerance_mm,
                &mut cells,
                max_cells,
            )?;
        }
    }
    Ok(FacePartitions {
        model,
        coverage,
        edges,
        vertices,
        cells,
    })
}

pub mod root_interval;

pub mod contact_interval;
