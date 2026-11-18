//! An open retained-face cut whose ends belong to the same original hole.
use super::*;
#[cfg(test)]
mod tests;
pub struct HolePartition {
    /// Both retained sectors; no material deletion or new Solid admission.
    pub model: Model,
    /// Original outer sector with the enlarged hole, then the bounded cap.
    pub faces: [usize; 2],
    /// Input path order, including the two existing endpoint vertices.
    pub edges: Vec<usize>,
    pub vertices: Vec<usize>,
    pub cap_path_reversed: bool,
    pub source_uv_reference_exact: bool,
    pub source_identity: TopoId,
    pub face_identities: [TopoId; 2],
    pub cells: usize,
    pub pairs: usize,
}
/// A path outside one hole, between two owned vertices on that hole, divides
/// the original material into a bounded cap and an outer sector. Original UV
/// arcs choose their roles by complete simplicity/winding/containment proofs.
/// Paths through the hole, intersecting boundaries, or inconclusive assignments
/// refuse atomically. Different-loop bridge networks need a separate assembler.
pub fn partition_same_hole(
    source: &Model,
    face_id: usize,
    endpoints: [usize; 2],
    path: &[Segment],
    tolerance_uv: f64,
    limits: ClosedCutLimits,
) -> Result<HolePartition> {
    source.validate()?;
    if !source.persistent_naming_complete()
        || !(1..=256).contains(&path.len())
        || !(1..=1000000).contains(&limits.cells)
        || !(1..=100000).contains(&limits.pairs)
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || endpoints[0] == endpoints[1]
        || endpoints.iter().any(|&v| v >= source.vertices.len())
    {
        return Err(invalid(
            "Need distinct owned hole endpoints, a saved path and bounded work",
        ));
    }
    let face = source
        .faces
        .get(face_id)
        .ok_or_else(|| invalid("Missing hole-cut support face"))?;
    if face.surface.periodic_u || face.surface.periodic_v || face.holes.len() > 15 {
        return Err(unproven("Hole cut needs a nonperiodic original face chart"));
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
        return Err(invalid("Hole cut support needs one oriented shell use"));
    }
    let mut hits = [Vec::new(), Vec::new()];
    for (side, &v) in endpoints.iter().enumerate() {
        for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            for (i, c) in source.loops[wire].coedges.iter().enumerate() {
                if source.edges[c.edge].vertices[usize::from(c.reversed)] == v {
                    hits[side].push((wire, i));
                }
            }
        }
    }
    if hits.iter().any(|h| h.len() != 1)
        || hits[0][0].0 != hits[1][0].0
        || !face.holes.contains(&hits[0][0].0)
    {
        return Err(invalid(
            "Both endpoints need unique occurrences on one original hole",
        ));
    }
    let hole = hits[0][0].0;
    let indices = [hits[0][0].1, hits[1][0].1];
    let old = &source.loops[hole].coedges;
    for s in path {
        s.curve.validate()?;
        s.pcurve.validate()?;
    }
    for (i, s) in path.iter().enumerate() {
        if !clamped(&s.curve)
            || !clamped(&s.pcurve)
            || s.curve.control_points[0].len() != 3
            || s.pcurve.control_points[0].len() != 2
            || (i > 0
                && (path[i - 1].curve.control_points.last() != Some(&s.curve.control_points[0])
                    || path[i - 1].pcurve.control_points.last()
                        != Some(&s.pcurve.control_points[0])))
        {
            return Err(invalid("Hole path needs exact saved 3D/UV joins"));
        }
    }
    for (side, curve, pcurve) in [
        (
            0,
            &path[0].curve.control_points[0],
            &path[0].pcurve.control_points[0],
        ),
        (
            1,
            path.last().unwrap().curve.control_points.last().unwrap(),
            path.last().unwrap().pcurve.control_points.last().unwrap(),
        ),
    ] {
        if curve.as_slice() != source.vertices[endpoints[side]].point
            || *pcurve != old[indices[side]].pcurve.control_points[0]
        {
            return Err(invalid(
                "Hole path ends must literally match owned world and UV vertices",
            ));
        }
    }
    let curves = |ring: &[Coedge]| ring.iter().map(|c| c.pcurve.clone()).collect::<Vec<_>>();
    let mut work = Work::default();
    let original: Vec<_> = std::iter::once(face.outer)
        .chain(face.holes.iter().copied())
        .map(|w| curves(&source.loops[w].coedges))
        .collect();
    if audit(&original, tolerance_uv, &mut work, limits)? != Some(true) {
        return Err(unproven("Original hole-cut region is not proven valid"));
    }
    for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
        for c in &source.loops[wire].coedges {
            let e = &source.edges[c.edge];
            if c.trim().is_some_and(|t| {
                t.curve_parameter != e.curve.domain()
                    || t.pcurve_parameter != c.pcurve.domain()
                    || t.periodic_lift != [[0; 2]; 2]
            }) {
                return Err(unproven("Hole cut needs complete original coedge windows"));
            }
            let tol = source.tolerance_mm.min(
                *source
                    .1
                    .local_tolerances_mm
                    .get(&source.1.edges[c.edge])
                    .unwrap_or(&source.tolerance_mm),
            );
            lift(
                &e.curve,
                &c.pcurve,
                &face.surface,
                c.reversed,
                tol,
                &mut work,
                limits,
            )?;
        }
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
            result.push(old[i].clone());
            i = (i + 1) % old.len();
        }
        result
    };
    let mut rings = [forward, backward];
    rings[0].extend(arc(indices[1], indices[0]));
    rings[1].extend(arc(indices[0], indices[1]));
    let signs = [
        audit(&[curves(&rings[0])], tolerance_uv, &mut work, limits)?,
        audit(&[curves(&rings[1])], tolerance_uv, &mut work, limits)?,
    ];
    let (positive, negative) = match signs {
        [Some(true), Some(false)] => (0, 1),
        [Some(false), Some(true)] => (1, 0),
        _ => {
            return Err(unproven(
                "Hole path does not prove one material cap and one enlarged hole",
            ));
        }
    };
    let mut regions = [
        vec![
            curves(&source.loops[face.outer].coedges),
            curves(&rings[negative]),
        ],
        vec![curves(&rings[positive])],
    ];
    for r in &regions {
        if audit(r, tolerance_uv, &mut work, limits)? != Some(true) {
            return Err(unproven(
                "Hole-cut sectors are not valid original-chart regions",
            ));
        }
    }
    let mut holes = [Vec::new(), Vec::new()];
    for &w in face.holes.iter().filter(|&&w| w != hole) {
        let h = curves(&source.loops[w].coedges);
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
                    "Another original hole crosses the cut or lacks a unique sector",
                ));
            }
        };
        regions[side].push(h);
        holes[side].push(w);
    }
    let mut source_uv_reference_exact = true;
    for c in rings.iter().flatten().filter(|c| c.edge >= edge_start) {
        let s = &path[c.edge - edge_start];
        source_uv_reference_exact &= reference::reference(
            &c.pcurve,
            &s.pcurve,
            c.reversed,
            tolerance_uv,
            &mut work,
            limits,
        )?;
        lift(
            &s.curve,
            &c.pcurve,
            &face.surface,
            c.reversed,
            source.tolerance_mm,
            &mut work,
            limits,
        )?;
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
    model.loops[hole] = Loop {
        coedges: rings[negative].clone(),
    };
    let new_loop = model.loops.len();
    model.loops.push(Loop {
        coedges: rings[positive].clone(),
    });
    model.faces[face_id].holes = holes[0].clone();
    model.faces[face_id].holes.push(hole);
    let inner = model.faces.len();
    model.faces.push(Face {
        surface: face.surface.clone(),
        outer: new_loop,
        holes: holes[1].clone(),
    });
    model.shells[uses[0].0].faces.push(FaceUse {
        face: inner,
        reversed: uses[0].1,
    });
    model.inherit_topology_ids(&[source]);
    super::super::policy::preserve(&mut model, source);
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
        return Err(unproven("Hole cut lost complete topology naming"));
    }
    Ok(HolePartition {
        model,
        faces: [face_id, inner],
        edges: (edge_start..edge_start + path.len()).collect(),
        vertices,
        cap_path_reversed: positive == 1,
        source_uv_reference_exact,
        source_identity,
        face_identities,
        cells: work.cells,
        pairs: work.pairs,
    })
}
