//! Partition an original face into two retained sectors along an authored cut.
//! Contact-curve generation and material removal are separate operations.
use crate::{Coedge, Edge, Face, FaceUse, Loop, Model, TopoId, TopologyLineageRecord, Vertex};
use nurbs_core::{
    Error, Result,
    curve::Curve,
    curve_surface_agreement::{self, Status},
};

#[path = "face_cut/retention_network.rs"]
mod retention_network;
pub use retention_network::{
    FaceRetention, RetentionNetwork, RetentionRegion, plan_retention_network,
};
#[path = "face_cut/rooted_retention.rs"]
mod rooted_retention;
pub use rooted_retention::{
    RootEnd, RootedCut, RootedFace, RootedRetention, plan_rooted_retention,
};
#[path = "face_cut/intersection_curve.rs"]
mod intersection_curve;
pub use intersection_curve::{IntersectionCurve, generate_intersection_curve};
#[path = "face_cut/iso_intersection.rs"]
mod iso_intersection;
pub use iso_intersection::{IsoIntersection, generate_iso_intersection_curve};
#[path = "face_cut/ruled_intersection.rs"]
mod ruled_intersection;
pub use ruled_intersection::{generate_ruled_plane_cut, generate_ruled_plane_cut_between};
#[path = "face_cut/bind_intersection.rs"]
mod bind_intersection;
pub use bind_intersection::bind_intersection_ends;
#[path = "face_cut/automatic_partition.rs"]
mod automatic_partition;
pub use automatic_partition::{AutomaticPartition, partition_ruled_at_face};
#[path = "face_cut/automatic_network.rs"]
mod automatic_network;
pub use automatic_network::{AutomaticNetwork, SectionFace, partition_ruled_network};
#[path = "face_cut/automatic_cap.rs"]
mod automatic_cap;
pub use automatic_cap::{AutomaticCap, CapSelection, trim_ruled_and_cap};
#[path = "face_cut/automatic_fiber.rs"]
mod automatic_fiber;
pub use automatic_fiber::{
    AutomaticFiberPartition, partition_fiber_at_face, partition_fiber_at_face_automatic,
};
#[path = "face_cut/fiber_branches.rs"]
mod fiber_branches;
#[path = "face_cut/fiber_network.rs"]
mod fiber_network;
pub use fiber_branches::{
    FiberBranch, FiberBranchRefusal, FiberBranches, partition_fiber_branches_at_face,
};
pub use fiber_network::{
    AutomaticFiberCapRequest, partition_fiber_network_automatic, trim_fiber_and_cap_automatic,
};
pub use fiber_network::{
    FiberCap, FiberCapRequest, FiberFaceRequest, FiberNetwork, partition_fiber_network,
    trim_fiber_and_cap,
};

mod endpoint_loop;
mod policy;
use endpoint_loop::owned_boundary;
mod closed;
pub use closed::{
    ClosedCutLimits, ClosedPartition, HolePartition, partition_closed, partition_same_hole,
};

pub struct Segment {
    pub curve: Curve,
    /// Original surface chart, increasing in the same direction as curve.
    pub pcurve: Curve,
}
pub struct Partition {
    pub model: Model,
    pub faces: [usize; 2],
    pub edges: Vec<usize>,
    pub source_identity: TopoId,
    pub face_identities: [TopoId; 2],
    pub cells: usize,
    pub exact_work: u64,
    pub trim_pairs: usize,
}
fn invalid(s: &str) -> Error {
    Error::new("BREP_FACE_CUT_INPUT", s)
}
fn unproven(s: &str) -> Error {
    Error::new("BREP_FACE_CUT_UNPROVEN", s)
}
fn clamped(c: &Curve) -> bool {
    !c.periodic
        && c.knots[..=c.degree].iter().all(|&k| k == c.domain()[0])
        && c.knots[c.control_points.len()..]
            .iter()
            .all(|&k| k == c.domain()[1])
}
/// Project authored 3D cuts into the immutable original face chart, then split
/// with fresh saved-curve agreement and region audits. Projection only proposes
/// pcurves: snapping to owned endpoint UVs never inherits its proof. Endpoints
/// must already be bound on one shared outer or hole boundary. No nearest-vertex
/// welding or choice of the material sector to discard is performed here.
pub fn partition_projected(
    source: &Model,
    face_id: usize,
    endpoints: [usize; 2],
    curves: &[Curve],
    tolerance_uv: f64,
    max_cells: usize,
) -> Result<Partition> {
    projected_with_budget(
        source,
        face_id,
        endpoints,
        curves,
        tolerance_uv,
        max_cells,
        1000000,
    )
}
fn projected_with_budget(
    source: &Model,
    face_id: usize,
    endpoints: [usize; 2],
    curves: &[Curve],
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<Partition> {
    source.validate()?;
    if !(1..=256).contains(&curves.len())
        || !(1..=100000).contains(&max_cells)
        || endpoints[0] == endpoints[1]
        || endpoints.iter().any(|&v| v >= source.vertices.len())
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
    {
        return Err(invalid(
            "Need owned endpoints, 3D cuts and bounded projection work",
        ));
    }
    let face = source
        .faces
        .get(face_id)
        .ok_or_else(|| invalid("Missing source face"))?;
    let (wire, indices) = owned_boundary(source, face_id, endpoints).ok_or_else(|| {
        invalid("Projected cut needs unique endpoints on one original boundary loop")
    })?;
    let endpoint_uv: Vec<_> = indices
        .iter()
        .map(|&i| source.loops[wire].coedges[i].pcurve.control_points[0].clone())
        .collect();
    let mut path: Vec<Segment> = Vec::with_capacity(curves.len());
    let mut cells = 0;
    for (i, curve) in curves.iter().enumerate() {
        curve.validate()?;
        if curve.control_points[0].len() != 3 || !clamped(curve) || cells == max_cells {
            return Err(invalid(
                "Need clamped 3D curves and remaining projection work",
            ));
        }
        let proposal = nurbs_core::curve_pullback::project(
            curve,
            &face.surface,
            nurbs_core::curve_pullback::Limits {
                tolerance: source.tolerance_mm * 0.5,
                max_segments: 256,
                max_iterations: 32,
                agreement_cells: max_cells - cells,
            },
        )?;
        cells += proposal.agreement_cells;
        let mut pcurve = proposal
            .pcurve
            .ok_or_else(|| unproven("Cut projection is unresolved"))?;
        pcurve.control_points[0] = if i == 0 {
            endpoint_uv[0].clone()
        } else {
            path[i - 1].pcurve.control_points.last().unwrap().clone()
        };
        if i + 1 == curves.len() {
            *pcurve.control_points.last_mut().unwrap() = endpoint_uv[1].clone();
        }
        path.push(Segment {
            curve: curve.clone(),
            pcurve,
        });
    }
    if cells >= max_cells {
        return Err(unproven("Projection exhausted shared cut work"));
    }
    let mut result = partition_with_budget(
        source,
        face_id,
        endpoints,
        &path,
        tolerance_uv,
        max_cells - cells,
        max_exact,
    )?;
    result.cells += cells;
    Ok(result)
}
pub struct OwnedCut {
    pub endpoints: [usize; 2],
    pub curves: Vec<Curve>,
}
pub struct Network {
    pub model: Model,
    /// All descendants of the original face; both material sectors are retained.
    pub faces: Vec<usize>,
    pub edges: Vec<usize>,
    pub cells: usize,
    pub exact_work: u64,
}
pub struct Retention {
    /// Complete partition; no selected sector has yet been deleted or replaced.
    pub network: Network,
    pub retained_faces: Vec<usize>,
    /// Sectors planned for removal; still present in network.model.
    pub discard_faces: Vec<usize>,
    pub seed_faces: Vec<usize>,
    pub domain_cells: usize,
}
/// Build a fresh network and select material sectors by strict original-chart
/// interior witnesses. Every seed must be inside exactly one descendant and
/// outside every other descendant. Boundary bands and incomplete winding proofs
/// refuse; no nearest-face choice or caller-supplied classification is trusted.
/// This is a material plan, not permission to export an open body as a solid.
pub fn plan_projected_retention(
    source: &Model,
    face: usize,
    cuts: &[OwnedCut],
    keep_uv: &[[f64; 2]],
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
    max_domain_cells: usize,
) -> Result<Retention> {
    if !(1..=256).contains(&keep_uv.len())
        || keep_uv.iter().flatten().any(|x| !x.is_finite())
        || !(1..=1000000).contains(&max_domain_cells)
    {
        return Err(invalid(
            "Need finite original-chart interior seeds and bounded classification work",
        ));
    }
    let network =
        partition_projected_network(source, face, cuts, tolerance_uv, max_cells, max_exact)?;
    let domains = network
        .faces
        .iter()
        .map(|&f| crate::face_domain::FaceDomain::new(&network.model, f, tolerance_uv))
        .collect::<Result<Vec<_>>>()?;
    let mut retained = std::collections::BTreeSet::new();
    let mut seed_faces = Vec::new();
    let mut domain_cells = 0;
    let mut remaining = keep_uv.len() * domains.len();
    for &uv in keep_uv {
        let mut owner = None;
        for (i, domain) in domains.iter().enumerate() {
            if domain_cells == max_domain_cells {
                return Err(unproven("Retained-sector classification work exhausted"));
            }
            let budget = ((max_domain_cells - domain_cells) / remaining).clamp(1, 100000);
            let r = domain.classify(uv.map(|x| [x, x]), budget)?;
            domain_cells += r.cells;
            remaining -= 1;
            match r.location {
                nurbs_core::trim_domain::Location::Inside if r.winding == Some(1) => {
                    if owner.replace(network.faces[i]).is_some() {
                        return Err(unproven("Retention seed belongs to overlapping sectors"));
                    }
                }
                nurbs_core::trim_domain::Location::Outside => {}
                _ => {
                    return Err(unproven(
                        "Retention seed is on a boundary or its winding is unresolved",
                    ));
                }
            }
        }
        let f =
            owner.ok_or_else(|| unproven("Retention seed is outside every descendant sector"))?;
        retained.insert(f);
        seed_faces.push(f);
    }
    let discard_faces = network
        .faces
        .iter()
        .copied()
        .filter(|f| !retained.contains(f))
        .collect();
    Ok(Retention {
        network,
        retained_faces: retained.into_iter().collect(),
        discard_faces,
        seed_faces,
        domain_cells,
    })
}
/// Apply several 3D cuts to descendants of one original face. Each next face
/// is inferred from unique endpoint ownership in the current topology. This
/// prepares a trim network without caller-managed descendant face indices.
/// Ambiguous ownership, crossing cuts and exhausted work refuse atomically.
pub fn partition_projected_network(
    source: &Model,
    face: usize,
    cuts: &[OwnedCut],
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<Network> {
    source.validate()?;
    if face >= source.faces.len()
        || !(1..=256).contains(&cuts.len())
        || !(1..=100000).contains(&max_cells)
        || max_exact > 1000000
    {
        return Err(invalid(
            "Need one original face, owned cuts and bounded shared work",
        ));
    }
    let mut out = Network {
        model: source.clone(),
        faces: vec![face],
        edges: vec![],
        cells: 0,
        exact_work: 0,
    };
    for cut in cuts {
        let candidates = out
            .faces
            .iter()
            .copied()
            .filter(|&f| owned_boundary(&out.model, f, cut.endpoints).is_some())
            .collect::<Vec<_>>();
        if candidates.len() != 1 {
            return Err(unproven(
                "Cut endpoints do not identify one current face sector",
            ));
        }
        if out.cells == max_cells {
            return Err(unproven("Trim network exhausted shared work"));
        }
        let result = projected_with_budget(
            &out.model,
            candidates[0],
            cut.endpoints,
            &cut.curves,
            tolerance_uv,
            max_cells - out.cells,
            max_exact - out.exact_work,
        )?;
        out.cells += result.cells;
        out.exact_work += result.exact_work;
        out.faces.push(result.faces[1]);
        out.edges.extend(result.edges);
        out.model = result.model;
    }
    // Preserve the complete accumulated change set relative to the input,
    // rather than exposing only the last incremental split.
    out.model.refresh_change_set(&[source]);
    Ok(out)
}
fn audit(
    loops: &[Vec<Curve>],
    uv: f64,
    cells: &mut usize,
    pairs: &mut usize,
    max: usize,
) -> Result<Option<bool>> {
    if max - *cells < 2 || *pairs == 10000 {
        return Err(unproven("Shared trim budget exhausted"));
    }
    let stage = ((max - *cells) / 2).min(100000);
    let r = nurbs_core::trim_region_audit::inspect(loops, uv, 10000 - *pairs, stage, stage)?;
    *cells += r.cells + r.domain_cells;
    *pairs += r.pairs;
    if r.valid.is_none() {
        return Err(unproven(&format!(
            "Trim proof unresolved: {} loops={:?} winding={:?}",
            r.reason, r.problem_loops, r.winding
        )));
    }
    Ok(if r.valid == Some(true) && r.winding[0] != Some(1) {
        Some(false)
    } else {
        r.valid
    })
}
/// Split along a simple open path between two vertices on one original loop.
/// Same-hole paths retain an outer sector and a bounded material cap.
/// Original surfaces, outer boundary definitions and hole definitions are kept.
/// Both sectors and the actual reversed cut pcurves are independently verified.
/// Existing holes are assigned automatically only after full-loop trim audits;
/// touching/crossed holes and inconclusive assignments refuse atomically.
/// For cuts terminating inside existing edges, partition those shared edges first.
/// Returns a topological candidate, not a fillet/radius/embedding certificate.
pub fn partition(
    source: &Model,
    face_id: usize,
    endpoints: [usize; 2],
    path: &[Segment],
    tolerance_uv: f64,
    max_cells: usize,
) -> Result<Partition> {
    partition_with_budget(
        source,
        face_id,
        endpoints,
        path,
        tolerance_uv,
        max_cells,
        1000000,
    )
}
fn partition_with_budget(
    source: &Model,
    face_id: usize,
    endpoints: [usize; 2],
    path: &[Segment],
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<Partition> {
    source.validate()?;
    if !source.persistent_naming_complete()
        || !(1..=256).contains(&path.len())
        || !(1..=1000000).contains(&max_cells)
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || endpoints[0] == endpoints[1]
        || endpoints.iter().any(|&v| v >= source.vertices.len())
    {
        return Err(invalid(
            "Need complete identities, distinct existing vertices and bounded cut work",
        ));
    }
    let face = source
        .faces
        .get(face_id)
        .ok_or_else(|| invalid("Missing source face"))?;
    if face.surface.periodic_u || face.surface.periodic_v || face.holes.len() > 15 {
        return Err(invalid(
            "Periodic chart ownership or more than 15 holes is unsupported",
        ));
    }
    let uses: Vec<_> = source
        .shells
        .iter()
        .enumerate()
        .flat_map(|(s, shell)| {
            shell
                .faces
                .iter()
                .filter(move |u| u.face == face_id)
                .map(move |u| (s, u.reversed))
        })
        .collect();
    if uses.len() != 1 {
        return Err(invalid("Source face needs one oriented shell use"));
    }
    for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
        let owners = source
            .faces
            .iter()
            .flat_map(|f| std::iter::once(f.outer).chain(f.holes.iter().copied()))
            .filter(|&w| w == wire)
            .count();
        if owners != 1 {
            return Err(invalid("Source trim loops must have unique face ownership"));
        }
    }
    if owned_boundary(source, face_id, endpoints).is_some_and(|(wire, _)| wire != face.outer) {
        let result = partition_same_hole(
            source,
            face_id,
            endpoints,
            path,
            tolerance_uv,
            ClosedCutLimits {
                cells: max_cells,
                pairs: 10000,
            },
        )?;
        return Ok(Partition {
            model: result.model,
            faces: result.faces,
            edges: result.edges,
            source_identity: result.source_identity,
            face_identities: result.face_identities,
            cells: result.cells,
            exact_work: 0,
            trim_pairs: result.pairs,
        });
    }
    let outer = &source.loops[face.outer].coedges;
    let mut indices = [0; 2];
    for (k, &vertex) in endpoints.iter().enumerate() {
        let hits: Vec<_> = outer
            .iter()
            .enumerate()
            .filter(|(_, c)| source.edges[c.edge].vertices[usize::from(c.reversed)] == vertex)
            .map(|(i, _)| i)
            .collect();
        if hits.len() != 1 {
            return Err(invalid("Cut endpoints need unique outer-loop occurrences"));
        }
        indices[k] = hits[0];
    }
    for s in path {
        s.curve.validate()?;
        s.pcurve.validate()?;
    }
    for (i, s) in path.iter().enumerate() {
        if s.curve.control_points[0].len() != 3
            || s.pcurve.control_points[0].len() != 2
            || !clamped(&s.curve)
            || !clamped(&s.pcurve)
            || s.curve.control_points[0] == *s.curve.control_points.last().unwrap()
            || (i > 0
                && path[i - 1].curve.control_points.last() != Some(&s.curve.control_points[0]))
        {
            return Err(invalid(
                "Cut needs clamped 3D/2D segments with exactly shared distinct 3D endpoints",
            ));
        }
    }
    if path[0].curve.control_points[0].as_slice() != source.vertices[endpoints[0]].point
        || path
            .last()
            .unwrap()
            .curve
            .control_points
            .last()
            .unwrap()
            .as_slice()
            != source.vertices[endpoints[1]].point
    {
        return Err(invalid(
            "Cut endpoints must equal the existing owned vertices",
        ));
    }
    let edge_start = source.edges.len();
    let forward: Vec<_> = path
        .iter()
        .enumerate()
        .map(|(i, s)| Coedge::new(edge_start + i, false, s.pcurve.clone()))
        .collect();
    let backward = path
        .iter()
        .enumerate()
        .rev()
        .map(|(i, s)| Ok(Coedge::new(edge_start + i, true, s.pcurve.reverse()?)))
        .collect::<Result<Vec<_>>>()?;
    let arc = |start: usize, end: usize| {
        let mut result = Vec::new();
        let mut i = start;
        while i != end {
            result.push(outer[i].clone());
            i = (i + 1) % outer.len();
        }
        result
    };
    let mut rings = [forward, backward];
    rings[0].extend(arc(indices[1], indices[0]));
    rings[1].extend(arc(indices[0], indices[1]));
    let curves = |ring: &[Coedge]| ring.iter().map(|c| c.pcurve.clone()).collect::<Vec<_>>();
    let mut regions = [vec![curves(&rings[0])], vec![curves(&rings[1])]];
    let mut cells = 0;
    let mut pairs = 0;
    let mut exact_work = 0;
    let source_region = std::iter::once(face.outer)
        .chain(face.holes.iter().copied())
        .map(|w| curves(&source.loops[w].coedges))
        .collect::<Vec<_>>();
    if audit(
        &source_region,
        tolerance_uv,
        &mut cells,
        &mut pairs,
        max_cells,
    )? != Some(true)
    {
        return Err(unproven(
            "Original trim region is not proven valid and positive",
        ));
    }
    for region in &regions {
        if audit(region, tolerance_uv, &mut cells, &mut pairs, max_cells)? != Some(true) {
            return Err(unproven(
                "Cut sectors are not both proven simple positive trim regions",
            ));
        }
    }
    let mut holes = [Vec::new(), Vec::new()];
    for &hole in &face.holes {
        let h = curves(&source.loops[hole].coedges);
        let mut valid = [None; 2];
        for side in 0..2 {
            let mut trial = regions[side].clone();
            trial.push(h.clone());
            valid[side] = audit(&trial, tolerance_uv, &mut cells, &mut pairs, max_cells)?;
        }
        let side = match valid {
            [Some(true), Some(false)] => 0,
            [Some(false), Some(true)] => 1,
            _ => {
                return Err(unproven(
                    "Hole assignment crosses the cut or remains unresolved",
                ));
            }
        };
        regions[side].push(h);
        holes[side].push(hole);
    }
    for c in rings
        .iter()
        .flat_map(|r| r.iter())
        .filter(|c| c.edge >= edge_start)
    {
        let segment = &path[c.edge - edge_start];
        closed::verify_uv_reference(
            &c.pcurve,
            &segment.pcurve,
            c.reversed,
            tolerance_uv,
            &mut cells,
            max_cells,
        )?;
        let mut proven = false;
        if exact_work < max_exact {
            if let Some(r) = curve_surface_agreement::verify_exact(
                &segment.curve,
                &c.pcurve,
                &face.surface,
                c.reversed,
                max_exact - exact_work,
            )? {
                exact_work += r.work_used;
                proven = r.outcome == cad_predicates::BezierIdentity::Equal;
            }
        }
        if !proven {
            if cells == max_cells {
                return Err(unproven("Cut lift budget exhausted"));
            }
            let r = curve_surface_agreement::verify(
                &segment.curve,
                &c.pcurve,
                &face.surface,
                c.reversed,
                source.tolerance_mm,
                (max_cells - cells).min(100000),
            )?;
            cells += r.cells;
            if r.status != Status::WithinTolerance {
                return Err(unproven("Actual cut lift disagrees or is unresolved"));
            }
        }
    }
    let mut model = source.clone();
    let mut vertices = vec![endpoints[0]];
    for s in path.iter().skip(1) {
        vertices.push(model.vertices.len());
        model.vertices.push(Vertex {
            point: s.curve.control_points[0].clone().try_into().unwrap(),
        });
    }
    vertices.push(endpoints[1]);
    for (i, s) in path.iter().enumerate() {
        model.edges.push(Edge {
            curve: s.curve.clone(),
            vertices: [vertices[i], vertices[i + 1]],
            degenerate: false,
        });
    }
    // Reuse the original outer-loop slot so no abandoned loop is manufactured.
    model.loops[face.outer] = Loop {
        coedges: rings[0].clone(),
    };
    let new_loop = model.loops.len();
    model.loops.push(Loop {
        coedges: rings[1].clone(),
    });
    model.faces[face_id].holes = holes[0].clone();
    let second = model.faces.len();
    model.faces.push(Face {
        surface: face.surface.clone(),
        outer: new_loop,
        holes: holes[1].clone(),
    });
    model.shells[uses[0].0].faces.push(FaceUse {
        face: second,
        reversed: uses[0].1,
    });
    model.inherit_topology_ids(&[source]);
    policy::preserve(&mut model, source);
    let source_identity = source.1.faces[face_id];
    let face_identities = [model.1.faces[face_id], model.1.faces[second]];
    model.1.lineage.push(TopologyLineageRecord {
        operation: "split".into(),
        entity_kind: "face".into(),
        parents: vec![source_identity],
        children: face_identities.to_vec(),
    });
    model.refresh_change_set(&[source]);
    model.validate()?;
    if !model.persistent_naming_complete() {
        return Err(unproven("Cut face naming is incomplete"));
    }
    Ok(Partition {
        model,
        faces: [face_id, second],
        edges: (edge_start..edge_start + path.len()).collect(),
        source_identity,
        face_identities,
        cells,
        exact_work,
        trim_pairs: pairs,
    })
}
