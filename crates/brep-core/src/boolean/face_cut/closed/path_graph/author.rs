use super::*;
pub(super) fn author(
    source: &Model,
    face: usize,
    paths: &[BridgePath],
    cycles: &cycles::Cycles,
    positive: &[usize],
    owners: &[usize],
    shell: (usize, bool),
) -> Result<PathGraphPartition> {
    let f = &source.faces[face];
    let pool: Vec<_> = std::iter::once(f.outer)
        .chain(f.holes.iter().copied())
        .collect();
    let mut model = source.clone();
    let mut vertices = Vec::new();
    let mut edges = Vec::new();
    for p in paths {
        let mut vs = vec![p.endpoints[0]];
        for s in p.segments.iter().skip(1) {
            vs.push(model.vertices.len());
            model.vertices.push(Vertex {
                point: s.curve.control_points[0].clone().try_into().unwrap(),
            });
        }
        vs.push(p.endpoints[1]);
        let mut es = Vec::new();
        for (i, s) in p.segments.iter().enumerate() {
            es.push(model.edges.len());
            model.edges.push(Edge {
                curve: s.curve.clone(),
                vertices: [vs[i], vs[i + 1]],
                degenerate: false,
            });
        }
        vertices.push(vs);
        edges.push(es);
    }
    // Reuse old owned loop slots where possible; remove only unused owned slots.
    // Every unrelated face receives the exact old-to-new loop index mapping.
    let mut order = positive.to_vec();
    order.extend((0..cycles.rings.len()).filter(|c| !positive.contains(c)));
    let mut old_to_new = vec![None; source.loops.len()];
    let mut cycle_loop = vec![usize::MAX; cycles.rings.len()];
    let mut loops = Vec::new();
    for old in 0..source.loops.len() {
        if let Some(slot) = pool.iter().position(|&l| l == old) {
            if let Some(&cycle) = order.get(slot) {
                cycle_loop[cycle] = loops.len();
                loops.push(Loop {
                    coedges: cycles.rings[cycle].clone(),
                });
            }
        } else {
            old_to_new[old] = Some(loops.len());
            loops.push(source.loops[old].clone());
        }
    }
    for &cycle in order.iter().skip(pool.len()) {
        cycle_loop[cycle] = loops.len();
        loops.push(Loop {
            coedges: cycles.rings[cycle].clone(),
        });
    }
    model.loops = loops;
    for (i, other) in model
        .faces
        .iter_mut()
        .enumerate()
        .filter(|(i, _)| *i != face)
    {
        let _ = i;
        other.outer = old_to_new[other.outer]
            .ok_or_else(|| unproven("Path graph removed another face's outer loop"))?;
        for h in &mut other.holes {
            *h =
                old_to_new[*h].ok_or_else(|| unproven("Path graph removed another face's hole"))?;
        }
    }
    let mut faces = vec![face];
    for _ in positive.iter().skip(1) {
        faces.push(model.faces.len());
        model.faces.push(f.clone());
    }
    for (p, &cycle) in positive.iter().enumerate() {
        model.faces[faces[p]] = Face {
            surface: f.surface.clone(),
            outer: cycle_loop[cycle],
            holes: (0..cycles.rings.len())
                .filter(|c| !positive.contains(c) && owners[*c] == p)
                .map(|c| cycle_loop[c])
                .collect(),
        };
    }
    for &new in faces.iter().skip(1) {
        model.shells[shell.0].faces.push(FaceUse {
            face: new,
            reversed: shell.1,
        });
    }
    let mut path_faces = Vec::new();
    for pair in &cycles.path_cycles {
        let actual = [faces[owners[pair[0]]], faces[owners[pair[1]]]];
        if actual[0] == actual[1] {
            return Err(unproven(
                "Path graph contains a lone slit without two material owners",
            ));
        }
        path_faces.push(actual);
    }
    model.inherit_topology_ids(&[source]);
    super::super::super::policy::preserve(&mut model, source);
    let source_identity = source.1.faces[face];
    let face_identities = faces.iter().map(|&f| model.1.faces[f]).collect::<Vec<_>>();
    model.1.lineage.push(TopologyLineageRecord {
        operation: "split".into(),
        entity_kind: "face".into(),
        parents: vec![source_identity],
        children: face_identities.clone(),
    });
    model.refresh_change_set(&[source]);
    model.validate()?;
    if !model.persistent_naming_complete() {
        return Err(unproven("Path graph lost persistent naming"));
    }
    Ok(PathGraphPartition {
        model,
        faces,
        edges,
        vertices,
        path_faces,
        source_identity,
        face_identities,
        cells: 0,
        pairs: 0,
    })
}
