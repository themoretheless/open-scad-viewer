//! Two bridges between different original loops form true simple sectors.
use super::*;
mod prepare;
#[cfg(test)]
mod tests;
pub struct BridgePath {
    pub endpoints: [usize; 2],
    pub segments: Vec<Segment>,
}
pub struct BridgePartition {
    pub model: Model,
    pub faces: [usize; 2],
    /// Native input path order and direction, independent of contour traversal.
    pub edges: [Vec<usize>; 2],
    pub vertices: [Vec<usize>; 2],
    pub source_uv_reference_exact: bool,
    pub source_identity: TopoId,
    pub face_identities: [TopoId; 2],
    pub cells: usize,
    pub pairs: usize,
}
fn arc(source: &Model, wire: usize, start: usize, end: usize) -> Vec<Coedge> {
    let old = &source.loops[wire].coedges;
    let mut at = start;
    let mut out = Vec::new();
    while at != end {
        out.push(old[at].clone());
        at = (at + 1) % old.len();
    }
    out
}
fn traversal(path: &BridgePath, offset: usize, forward: bool) -> Result<Vec<Coedge>> {
    if forward {
        Ok(path
            .segments
            .iter()
            .enumerate()
            .map(|(i, s)| Coedge::new(offset + i, false, s.pcurve.clone()))
            .collect())
    } else {
        path.segments
            .iter()
            .enumerate()
            .rev()
            .map(|(i, s)| Ok(Coedge::new(offset + i, true, s.pcurve.reverse()?)))
            .collect()
    }
}
/// Both paths connect the same two distinct loops. Their directions may differ.
/// The complete original region, both cycles, other hole assignments and both
/// saved lift/reference directions are verified before publishing a candidate.
/// A lone slit, unresolved contact, crossing or budget exhaustion cannot grant
/// a face subdivision. Both material sectors remain; no new Solid admission.
pub fn partition_bridges(
    source: &Model,
    face_id: usize,
    paths: &[BridgePath; 2],
    tolerance_uv: f64,
    limits: ClosedCutLimits,
) -> Result<BridgePartition> {
    partition_bridges_with_tolerances(
        source,
        face_id,
        paths,
        tolerance_uv,
        [source.tolerance_mm; 2],
        limits,
    )
}
pub(crate) fn partition_bridges_with_tolerances(
    source: &Model,
    face_id: usize,
    paths: &[BridgePath; 2],
    tolerance_uv: f64,
    world_tolerances: [f64; 2],
    limits: ClosedCutLimits,
) -> Result<BridgePartition> {
    if world_tolerances.iter().any(|t| !t.is_finite() || *t <= 0.) {
        return Err(invalid("Need finite positive bridge lift tolerances"));
    }
    let mut work = Work::default();
    let p = prepare::prepare(source, face_id, paths, tolerance_uv, limits, &mut work)?;
    let face = &source.faces[face_id];
    let offsets = [
        source.edges.len(),
        source.edges.len() + paths[0].segments.len(),
    ];
    let mut rings = [
        traversal(&paths[0], offsets[0], p.ab_forward[0])?,
        traversal(&paths[1], offsets[1], p.ab_forward[1])?,
    ];
    rings[0].extend(arc(source, p.loops[1], p.indices[0][1], p.indices[1][1]));
    rings[0].extend(traversal(&paths[1], offsets[1], !p.ab_forward[1])?);
    rings[0].extend(arc(source, p.loops[0], p.indices[1][0], p.indices[0][0]));
    rings[1].extend(arc(source, p.loops[1], p.indices[1][1], p.indices[0][1]));
    rings[1].extend(traversal(&paths[0], offsets[0], !p.ab_forward[0])?);
    rings[1].extend(arc(source, p.loops[0], p.indices[0][0], p.indices[1][0]));
    let curves = |ring: &[Coedge]| ring.iter().map(|c| c.pcurve.clone()).collect::<Vec<_>>();
    let signs = [
        audit(&[curves(&rings[0])], tolerance_uv, &mut work, limits)?,
        audit(&[curves(&rings[1])], tolerance_uv, &mut work, limits)?,
    ];
    let outer = p.loops[0] == face.outer;
    let ring_order = if outer {
        if signs != [Some(true), Some(true)] {
            return Err(unproven(
                "Outer-hole bridges need two simple positive sectors",
            ));
        }
        [0, 1]
    } else {
        match signs {
            [Some(false), Some(true)] => [0, 1],
            [Some(true), Some(false)] => [1, 0],
            _ => {
                return Err(unproven(
                    "Hole-hole bridges need a positive cap and negative merged boundary",
                ));
            }
        }
    };
    let mut regions = if outer {
        [vec![curves(&rings[0])], vec![curves(&rings[1])]]
    } else {
        [
            vec![
                curves(&source.loops[face.outer].coedges),
                curves(&rings[ring_order[0]]),
            ],
            vec![curves(&rings[ring_order[1]])],
        ]
    };
    for region in &regions {
        if audit(region, tolerance_uv, &mut work, limits)? != Some(true) {
            return Err(unproven(
                "Bridge sectors are not proven original-chart regions",
            ));
        }
    }
    let mut holes = [Vec::new(), Vec::new()];
    for &wire in face.holes.iter().filter(|w| !p.loops.contains(w)) {
        let h = curves(&source.loops[wire].coedges);
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
                    "Another hole crosses a bridge or has no unique material sector",
                ));
            }
        };
        regions[side].push(h);
        holes[side].push(wire);
    }
    let mut source_uv_reference_exact = true;
    for c in rings.iter().flatten().filter(|c| c.edge >= offsets[0]) {
        let (index, segment) = if c.edge < offsets[1] {
            (0, c.edge - offsets[0])
        } else {
            (1, c.edge - offsets[1])
        };
        let s = &paths[index].segments[segment];
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
            source.tolerance_mm.min(world_tolerances[index]),
            &mut work,
            limits,
        )?;
    }
    let mut model = source.clone();
    let mut vertices = [Vec::new(), Vec::new()];
    let mut edges = [Vec::new(), Vec::new()];
    for (j, path) in paths.iter().enumerate() {
        vertices[j].push(path.endpoints[0]);
        for s in path.segments.iter().skip(1) {
            vertices[j].push(model.vertices.len());
            model.vertices.push(Vertex {
                point: s.curve.control_points[0].clone().try_into().unwrap(),
            });
        }
        vertices[j].push(path.endpoints[1]);
        for (i, s) in path.segments.iter().enumerate() {
            edges[j].push(model.edges.len());
            model.edges.push(Edge {
                curve: s.curve.clone(),
                vertices: [vertices[j][i], vertices[j][i + 1]],
                degenerate: false,
            });
        }
    }
    model.loops[p.loops[0]] = Loop {
        coedges: rings[ring_order[0]].clone(),
    };
    model.loops[p.loops[1]] = Loop {
        coedges: rings[ring_order[1]].clone(),
    };
    model.faces[face_id].holes = holes[0].clone();
    if !outer {
        model.faces[face_id].holes.push(p.loops[0]);
    }
    let inner = model.faces.len();
    model.faces.push(Face {
        surface: face.surface.clone(),
        outer: p.loops[1],
        holes: holes[1].clone(),
    });
    model.shells[p.shell.0].faces.push(FaceUse {
        face: inner,
        reversed: p.shell.1,
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
        return Err(unproven("Bridge subdivision lost topology naming"));
    }
    Ok(BridgePartition {
        model,
        faces: [face_id, inner],
        edges,
        vertices,
        source_uv_reference_exact,
        source_identity,
        face_identities,
        cells: work.cells,
        pairs: work.pairs,
    })
}
