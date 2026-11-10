//! Joint bridge batches retain immutable original SS path ownership.
use super::*;
#[cfg(test)]
mod tests;
struct Layout {
    endpoints: [[usize; 2]; 2],
    bridge: bool,
}
fn layout(
    input: &QualifiedCutPartitions<'_, '_, '_>,
    side: usize,
    paths: &[&QualifiedOpenSSPath<'_, '_, '_, '_>],
) -> Result<Option<Layout>> {
    let m = &input.models[input.side_model[side]];
    let f = &m.faces[input.source.roots.faces[side].1];
    let mut endpoints = [[0; 2]; 2];
    let mut loops = [[0; 2]; 2];
    for (p, path) in paths.iter().enumerate() {
        for (i, node) in [path.request.start_node, path.request.end_node]
            .into_iter()
            .enumerate()
        {
            let Some((v, _)) = route::anchor(input, node, side)? else {
                return Ok(None);
            };
            endpoints[p][i] = v;
            let mut hits = Vec::new();
            for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
                for c in &m.loops[wire].coedges {
                    if m.edges[c.edge].vertices[usize::from(c.reversed)] == v {
                        hits.push(wire);
                    }
                }
            }
            if hits.len() != 1 {
                return Ok(None);
            }
            loops[p][i] = hits[0];
        }
    }
    let bridge = loops.iter().all(|l| l[0] != l[1]);
    if bridge {
        let mut a = loops[0];
        let mut b = loops[1];
        a.sort();
        b.sort();
        if a != b {
            return Ok(None);
        }
    } else if loops.iter().any(|l| l[0] != l[1]) {
        return Ok(None);
    }
    Ok(Some(Layout { endpoints, bridge }))
}
pub(super) fn try_pair<'s, 'i, 'p, 'r, 'a>(
    paths: &[&'s QualifiedOpenSSPath<'i, 'p, 'r, 'a>],
    limits: OpenSSFaceLimits,
) -> Result<Option<QualifiedOpenSSNetwork<'s, 'i, 'p, 'r, 'a>>> {
    if paths.len() != 2 {
        return Ok(None);
    }
    let input = paths[0].input;
    let (Some(a), Some(b)) = (layout(input, 0, paths)?, layout(input, 1, paths)?) else {
        return Ok(None);
    };
    let layouts = [a, b];
    if !layouts.iter().any(|l| l.bridge) {
        return Ok(None);
    }
    let mut cells = 0;
    let mut pairs = 0;
    let mut operations = 0;
    spend(&mut operations, limits.operations, 8)?;
    for p in paths {
        spend(&mut cells, limits.cells, p.cells)?;
        spend(&mut pairs, limits.pairs, input.source.roots.work.pairs)?;
        spend(&mut operations, limits.operations, p.operations)?;
        p.reverify()?;
    }
    let mut models = input.models.clone();
    let mut faces = [
        vec![input.source.roots.faces[0].1],
        vec![input.source.roots.faces[1].1],
    ];
    let mut sides: [Vec<OpenSSPairSide>; 2] = [vec![], vec![]];
    for side in 0..2 {
        let slot = input.side_model[side];
        let original = input.source.roots.faces[side].1;
        if layouts[side].bridge {
            let saved: Vec<_> = paths
                .iter()
                .enumerate()
                .map(|(p, path)| crate::face_cut::BridgePath {
                    endpoints: layouts[side].endpoints[p],
                    segments: path
                        .segments
                        .iter()
                        .map(|s| crate::face_cut::Segment {
                            curve: s.curve.clone(),
                            pcurve: s.pcurves[side].clone(),
                        })
                        .collect(),
                })
                .collect();
            spend(
                &mut operations,
                limits.operations,
                paths.iter().map(|p| p.segments.len()).sum(),
            )?;
            if cells == limits.cells || pairs == limits.pairs {
                return Err(unproven("SS bridge batch exhausted shared trim work"));
            }
            let saved: [crate::face_cut::BridgePath; 2] = saved.try_into().ok().unwrap();
            let result = crate::face_cut::partition_bridges_with_tolerances(
                &models[slot],
                original,
                &saved,
                paths[0].uv_tolerance.min(paths[1].uv_tolerance),
                [paths[0].world_tolerance, paths[1].world_tolerance],
                crate::face_cut::ClosedCutLimits {
                    cells: limits.cells - cells,
                    pairs: (limits.pairs - pairs).min(10000),
                },
            )?;
            spend(&mut cells, limits.cells, result.cells)?;
            spend(&mut pairs, limits.pairs, result.pairs)?;
            faces[side] = result.faces.to_vec();
            models[slot] = result.model;
            for p in 0..2 {
                sides[p].push(OpenSSPairSide {
                    model: slot,
                    faces: result.faces,
                    edges: result.edges[p].clone(),
                    vertices: result.vertices[p].clone(),
                });
            }
        } else {
            for (p, path) in paths.iter().enumerate() {
                let endpoints = layouts[side].endpoints[p];
                let candidates: Vec<_> = faces[side]
                    .iter()
                    .copied()
                    .filter(|&f| {
                        crate::face_cut::endpoint_loop::owned_boundary(&models[slot], f, endpoints)
                            .is_some()
                    })
                    .collect();
                spend(&mut operations, limits.operations, faces[side].len())?;
                if candidates.len() != 1 {
                    return Err(unproven(
                        "Mixed SS bridge batch has no unique current sector",
                    ));
                }
                let result = apply::construct(
                    path,
                    side,
                    candidates[0],
                    &models[slot],
                    endpoints,
                    limits,
                    &mut cells,
                    &mut pairs,
                    &mut operations,
                )?;
                faces[side].push(result.faces[1]);
                models[slot] = result.model;
                sides[p].push(OpenSSPairSide {
                    model: slot,
                    faces: result.faces,
                    edges: result.edges,
                    vertices: result.vertices,
                });
            }
        }
    }
    let mut ports = std::collections::BTreeMap::new();
    let mut cuts = Vec::new();
    for (p, path) in paths.iter().enumerate() {
        let output: [OpenSSPairSide; 2] = std::mem::take(&mut sides[p]).try_into().unwrap();
        for side in 0..2 {
            let m = &mut models[output[side].model];
            let o = &output[side];
            for (i, &e) in o.edges.iter().enumerate() {
                spend(&mut operations, limits.operations, 1)?;
                if m.edges[e].curve != path.segments[i].curve
                    || m.edges[e].vertices != [o.vertices[i], o.vertices[i + 1]]
                {
                    return Err(unproven("SS bridge batch changed its original saved path"));
                }
                m.1.local_tolerances_mm
                    .insert(m.1.edges[e], path.world_tolerance.max(1e-10));
            }
            for &v in &o.vertices {
                let stamp = path.world_tolerance.max(1e-10);
                m.1.local_tolerances_mm
                    .entry(m.1.vertices[v])
                    .and_modify(|t| *t = t.min(stamp))
                    .or_insert(stamp);
            }
        }
        for i in 1..path.segments.len() {
            spend(&mut operations, limits.operations, 1)?;
            let a = &path.segments[i - 1];
            let b = &path.segments[i];
            let v = input.source.roots.original.edges()[a.original_edge].vertices
                [usize::from(!a.reversed)];
            if input.source.roots.original.edges()[b.original_edge].vertices
                [usize::from(b.reversed)]
                != v
            {
                return Err(unproven("SS bridge batch lost an original source port"));
            }
            let native = [output[0].vertices[i], output[1].vertices[i]];
            if ports.insert(v, native).is_some_and(|old| old != native) {
                return Err(unproven("SS bridge batch conflicts at an original port"));
            }
        }
        cuts.push(OpenSSNetworkCut {
            path: p,
            segments: [0, path.segments.len()],
            sides: output,
            bridge_groups: layouts.each_ref().map(|l| l.bridge.then_some(0)),
        });
    }
    for (i, m) in models.iter_mut().enumerate() {
        m.validate()?;
        m.refresh_change_set(&[&input.models[i]]);
    }
    let ports = ports
        .into_iter()
        .map(|(original_vertex, vertices)| OpenSSNetworkPort {
            original_vertex,
            vertices,
        })
        .collect();
    Ok(Some(QualifiedOpenSSNetwork {
        paths: paths.to_vec(),
        limits,
        models,
        faces,
        cuts,
        ports,
        cells,
        pairs,
        operations,
    }))
}
