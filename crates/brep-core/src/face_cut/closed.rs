//! Divide a retained face by a certified simple closed saved-curve contour.
use super::*;
mod hole;
pub use hole::{HolePartition, partition_same_hole};
mod proof;
mod reference;
use proof::{Work, audit, lift};
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug)]
pub struct ClosedCutLimits {
    pub cells: usize,
    pub pairs: usize,
}
impl Default for ClosedCutLimits {
    fn default() -> Self {
        Self {
            cells: 1000000,
            pairs: 100000,
        }
    }
}
pub struct ClosedPartition {
    /// Candidate subdivision only; preserves both sectors and original support.
    pub model: Model,
    /// Original outer sector and newly authored inner sector, respectively.
    pub faces: [usize; 2],
    /// Edges/vertices in the normalized positive UV contour order.
    pub edges: Vec<usize>,
    pub vertices: Vec<usize>,
    pub path_reversed: bool,
    /// Exact original saved-clock identity was proved for every new world edge.
    /// Otherwise direct continuous comparison proves `model.tolerance_mm`.
    pub source_world_reference_exact: bool,
    /// Exact original saved-clock identity was proved for both UV directions.
    /// Otherwise direct continuous comparison proves the requested UV tolerance.
    pub source_uv_reference_exact: bool,
    pub source_identity: TopoId,
    pub face_identities: [TopoId; 2],
    pub cells: usize,
    pub pairs: usize,
}
pub(super) fn verify_uv_reference(
    candidate: &Curve,
    source: &Curve,
    reversed: bool,
    tolerance_uv: f64,
    cells: &mut usize,
    max_cells: usize,
) -> Result<bool> {
    let mut work = Work {
        cells: *cells,
        pairs: 0,
    };
    let exact = reference::reference(
        candidate,
        source,
        reversed,
        tolerance_uv,
        &mut work,
        ClosedCutLimits {
            cells: max_cells,
            pairs: 1,
        },
    )?;
    *cells = work.cells;
    Ok(exact)
}
fn reverse(path: &[Segment]) -> Result<Vec<Segment>> {
    path.iter()
        .rev()
        .map(|s| {
            Ok(Segment {
                curve: s.curve.reverse()?,
                pcurve: s.pcurve.reverse()?,
            })
        })
        .collect()
}
/// Insert a simple closed contour wholly inside a nonperiodic original face.
/// Whole-loop separation/containment assigns all original holes. Both sectors
/// and both actual orientations of every new edge lift are proved before any
/// candidate is returned. No nearest-vertex welding or Solid admission occurs.
pub fn partition_closed(
    source: &Model,
    face_id: usize,
    path: &[Segment],
    tolerance_uv: f64,
    limits: ClosedCutLimits,
) -> Result<ClosedPartition> {
    source.validate()?;
    if !source.persistent_naming_complete()
        || !(2..=256).contains(&path.len())
        || !(1..=1000000).contains(&limits.cells)
        || !(1..=100000).contains(&limits.pairs)
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
    {
        return Err(invalid(
            "Need a closed saved-curve path, complete identities and bounded work",
        ));
    }
    let face = source
        .faces
        .get(face_id)
        .ok_or_else(|| invalid("Missing closed cut support face"))?;
    if face.surface.periodic_u || face.surface.periodic_v || face.holes.len() > 14 {
        return Err(unproven(
            "Closed cut needs a nonperiodic chart and space for the new hole",
        ));
    }
    let uses: Vec<_> = source
        .shells
        .iter()
        .enumerate()
        .flat_map(|(si, s)| {
            s.faces
                .iter()
                .filter(move |u| u.face == face_id)
                .map(move |u| (si, u.reversed))
        })
        .collect();
    if uses.len() != 1 {
        return Err(invalid("Closed cut support needs one oriented shell use"));
    }
    for s in path {
        s.curve.validate()?;
        s.pcurve.validate()?;
    }
    for (i, s) in path.iter().enumerate() {
        let next = &path[(i + 1) % path.len()];
        if !clamped(&s.curve)
            || !clamped(&s.pcurve)
            || s.curve.control_points[0].len() != 3
            || s.pcurve.control_points[0].len() != 2
            || s.curve.control_points.last() != Some(&next.curve.control_points[0])
            || s.pcurve.control_points.last() != Some(&next.pcurve.control_points[0])
        {
            return Err(invalid(
                "Closed cut needs exact cyclic 3D/UV endpoint ownership",
            ));
        }
    }
    let curves = |ring: &[Coedge]| ring.iter().map(|c| c.pcurve.clone()).collect::<Vec<_>>();
    let original_region: Vec<_> = std::iter::once(face.outer)
        .chain(face.holes.iter().copied())
        .map(|w| curves(&source.loops[w].coedges))
        .collect();
    let mut work = Work::default();
    if audit(&original_region, tolerance_uv, &mut work, limits)? != Some(true) {
        return Err(unproven("Original closed-cut face trim region is unproven"));
    }
    // Do not inherit sampled source validation as a continuous lift proof, or
    // replace partial/directed/lifted coedge metadata by stored whole curves.
    for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
        for c in &source.loops[wire].coedges {
            let edge = &source.edges[c.edge];
            if c.trim().is_some_and(|t| {
                t.curve_parameter != edge.curve.domain()
                    || t.pcurve_parameter != c.pcurve.domain()
                    || t.periodic_lift != [[0; 2]; 2]
            }) {
                return Err(unproven(
                    "Closed cut needs complete original coedge windows",
                ));
            }
            let tol = source.tolerance_mm.min(
                *source
                    .1
                    .local_tolerances_mm
                    .get(&source.1.edges[c.edge])
                    .unwrap_or(&source.tolerance_mm),
            );
            lift(
                &edge.curve,
                &c.pcurve,
                &face.surface,
                c.reversed,
                tol,
                &mut work,
                limits,
            )?;
        }
    }
    let ring: Vec<_> = path.iter().map(|s| s.pcurve.clone()).collect();
    let positive = audit(&[ring], tolerance_uv, &mut work, limits)?;
    let path_reversed = positive == Some(false);
    let owned = if path_reversed {
        reverse(path)?
    } else {
        path.iter()
            .map(|s| Segment {
                curve: s.curve.clone(),
                pcurve: s.pcurve.clone(),
            })
            .collect()
    };
    let positive_ring: Vec<_> = owned.iter().map(|s| s.pcurve.clone()).collect();
    if positive.is_none()
        || audit(&[positive_ring.clone()], tolerance_uv, &mut work, limits)? != Some(true)
    {
        return Err(unproven(
            "Closed cut contour simplicity or positive orientation is unproven",
        ));
    }
    let edge_start = source.edges.len();
    let forward: Vec<_> = owned
        .iter()
        .enumerate()
        .map(|(i, s)| Coedge::new(edge_start + i, false, s.pcurve.clone()))
        .collect();
    let backward = owned
        .iter()
        .enumerate()
        .rev()
        .map(|(i, s)| Ok(Coedge::new(edge_start + i, true, s.pcurve.reverse()?)))
        .collect::<Result<Vec<_>>>()?;
    let mut regions = [
        vec![curves(&source.loops[face.outer].coedges), curves(&backward)],
        vec![positive_ring],
    ];
    for region in &regions {
        if audit(region, tolerance_uv, &mut work, limits)? != Some(true) {
            return Err(unproven(
                "Closed cut is not proven strictly inside the original outer boundary",
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
            valid[side] = audit(&trial, tolerance_uv, &mut work, limits)?;
        }
        let side = match valid {
            [Some(true), Some(false)] => 0,
            [Some(false), Some(true)] => 1,
            _ => {
                return Err(unproven(
                    "Existing hole crosses the closed cut or has unresolved ownership",
                ));
            }
        };
        regions[side].push(h);
        holes[side].push(hole);
    }
    let mut source_world_reference_exact = true;
    let mut source_uv_reference_exact = true;
    for (i, segment) in owned.iter().enumerate() {
        let source_index = if path_reversed { path.len() - 1 - i } else { i };
        source_world_reference_exact &= reference::reference(
            &segment.curve,
            &path[source_index].curve,
            path_reversed,
            source.tolerance_mm,
            &mut work,
            limits,
        )?;
    }
    for c in forward.iter().chain(&backward) {
        let index = c.edge - edge_start;
        let source_index = if path_reversed {
            path.len() - 1 - index
        } else {
            index
        };
        source_uv_reference_exact &= reference::reference(
            &c.pcurve,
            &path[source_index].pcurve,
            path_reversed != c.reversed,
            tolerance_uv,
            &mut work,
            limits,
        )?;
        lift(
            &owned[c.edge - edge_start].curve,
            &c.pcurve,
            &face.surface,
            c.reversed,
            source.tolerance_mm,
            &mut work,
            limits,
        )?;
    }
    let mut model = source.clone();
    let vertices: Vec<_> = (model.vertices.len()..model.vertices.len() + owned.len()).collect();
    for s in &owned {
        model.vertices.push(Vertex {
            point: s.curve.control_points[0].clone().try_into().unwrap(),
        });
    }
    for (i, s) in owned.iter().enumerate() {
        model.edges.push(Edge {
            curve: s.curve.clone(),
            vertices: [vertices[i], vertices[(i + 1) % vertices.len()]],
            degenerate: false,
        });
    }
    let hole_loop = model.loops.len();
    model.loops.push(Loop { coedges: backward });
    let inner_loop = model.loops.len();
    model.loops.push(Loop { coedges: forward });
    model.faces[face_id].holes = holes[0].clone();
    model.faces[face_id].holes.push(hole_loop);
    let inner = model.faces.len();
    model.faces.push(Face {
        surface: face.surface.clone(),
        outer: inner_loop,
        holes: holes[1].clone(),
    });
    model.shells[uses[0].0].faces.push(FaceUse {
        face: inner,
        reversed: uses[0].1,
    });
    model.inherit_topology_ids(&[source]);
    super::policy::preserve(&mut model, source);
    let source_identity = source.1.faces[face_id];
    let face_identities = [model.1.faces[face_id], model.1.faces[inner]];
    model.1.lineage.push(TopologyLineageRecord {
        operation: "split".into(),
        entity_kind: "face".into(),
        parents: vec![source_identity],
        children: face_identities.to_vec(),
    });
    model.refresh_change_set(&[source]);
    model.validate()?;
    if !model.persistent_naming_complete() {
        return Err(unproven("Closed cut lost complete topology naming"));
    }
    Ok(ClosedPartition {
        model,
        faces: [face_id, inner],
        edges: (edge_start..edge_start + owned.len()).collect(),
        vertices,
        path_reversed,
        source_world_reference_exact,
        source_uv_reference_exact,
        source_identity,
        face_identities,
        cells: work.cells,
        pairs: work.pairs,
    })
}
