use super::*;
#[cfg(test)]
mod tests;
pub(super) fn try_batch<'s, 'i, 'p, 'r, 'a>(
    paths: &[&'s QualifiedOpenSSPath<'i, 'p, 'r, 'a>],
    limits: OpenSSFaceLimits,
) -> Result<Option<QualifiedOpenSSNetwork<'s, 'i, 'p, 'r, 'a>>> {
    if paths.len() <= 2 {
        return Ok(None);
    }
    let input = paths[0].input;
    let mut endpoints = [Vec::new(), Vec::new()];
    let mut bridge = [false; 2];
    for side in 0..2 {
        let m = &input.models[input.side_model[side]];
        let f = &m.faces[input.source.roots.faces[side].1];
        for path in paths {
            let mut vs = [0; 2];
            let mut ws = [0; 2];
            for (i, node) in [path.request.start_node, path.request.end_node]
                .into_iter()
                .enumerate()
            {
                let Some((v, _)) = route::anchor(input, node, side)? else {
                    return Ok(None);
                };
                vs[i] = v;
                let mut owned = Vec::new();
                for l in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
                    for c in &m.loops[l].coedges {
                        if m.edges[c.edge].vertices[usize::from(c.reversed)] == v {
                            owned.push(l);
                        }
                    }
                }
                if owned.len() != 1 {
                    return Ok(None);
                }
                ws[i] = owned[0];
            }
            bridge[side] |= ws[0] != ws[1];
            endpoints[side].push(vs);
        }
    }
    if !bridge.iter().any(|&b| b) {
        return Ok(None);
    }
    let mut cells = 0;
    let mut pairs = 0;
    let mut operations = 0;
    spend(&mut operations, limits.operations, paths.len() * 4)?;
    for path in paths {
        spend(&mut cells, limits.cells, path.cells)?;
        spend(&mut pairs, limits.pairs, input.source.roots.work.pairs)?;
        spend(&mut operations, limits.operations, path.operations)?;
        path.reverify()?;
    }
    let mut models = input.models.clone();
    let mut faces = [Vec::new(), Vec::new()];
    let mut contributions: Vec<Vec<OpenSSPairSide>> =
        (0..paths.len()).map(|_| Vec::new()).collect();
    for side in 0..2 {
        let slot = input.side_model[side];
        let face = input.source.roots.faces[side].1;
        let saved: Vec<_> = paths
            .iter()
            .enumerate()
            .map(|(i, p)| crate::face_cut::BridgePath {
                endpoints: endpoints[side][i],
                segments: p
                    .segments
                    .iter()
                    .map(|s| crate::face_cut::Segment {
                        curve: s.curve.clone(),
                        pcurve: s.pcurves[side].clone(),
                    })
                    .collect(),
            })
            .collect();
        let uv = paths
            .iter()
            .map(|p| p.uv_tolerance)
            .fold(f64::INFINITY, f64::min);
        let tols: Vec<_> = paths.iter().map(|p| p.world_tolerance).collect();
        spend(
            &mut operations,
            limits.operations,
            paths.iter().map(|p| p.segments.len()).sum(),
        )?;
        if cells == limits.cells || pairs == limits.pairs {
            return Err(unproven(
                "SS path graph exhausted shared actual contour work",
            ));
        }
        let r = crate::face_cut::partition_path_graph_with_tolerances(
            &models[slot],
            face,
            &saved,
            uv,
            &tols,
            crate::face_cut::ClosedCutLimits {
                cells: limits.cells - cells,
                pairs: (limits.pairs - pairs).min(10000),
            },
        )?;
        spend(&mut cells, limits.cells, r.cells)?;
        spend(&mut pairs, limits.pairs, r.pairs)?;
        faces[side] = r.faces;
        models[slot] = r.model;
        for i in 0..paths.len() {
            contributions[i].push(OpenSSPairSide {
                model: slot,
                faces: r.path_faces[i],
                edges: r.edges[i].clone(),
                vertices: r.vertices[i].clone(),
            });
        }
    }
    let mut ports = std::collections::BTreeMap::new();
    let mut cuts = Vec::new();
    for (i, path) in paths.iter().enumerate() {
        let sides: [OpenSSPairSide; 2] = std::mem::take(&mut contributions[i]).try_into().unwrap();
        for side in 0..2 {
            let m = &mut models[sides[side].model];
            for (j, &edge) in sides[side].edges.iter().enumerate() {
                spend(&mut operations, limits.operations, 1)?;
                if m.edges[edge].curve != path.segments[j].curve
                    || m.edges[edge].vertices
                        != [sides[side].vertices[j], sides[side].vertices[j + 1]]
                {
                    return Err(unproven(
                        "SS path graph changed an original saved curve or directed port",
                    ));
                }
                m.1.local_tolerances_mm
                    .insert(m.1.edges[edge], path.world_tolerance.max(1e-10));
            }
            for &v in &sides[side].vertices {
                let stamp = path.world_tolerance.max(1e-10);
                m.1.local_tolerances_mm
                    .entry(m.1.vertices[v])
                    .and_modify(|t| *t = t.min(stamp))
                    .or_insert(stamp);
            }
        }
        for j in 1..path.segments.len() {
            spend(&mut operations, limits.operations, 1)?;
            let a = &path.segments[j - 1];
            let b = &path.segments[j];
            let v = input.source.roots.original.edges()[a.original_edge].vertices
                [usize::from(!a.reversed)];
            if input.source.roots.original.edges()[b.original_edge].vertices
                [usize::from(b.reversed)]
                != v
            {
                return Err(unproven("SS path graph lost an original source port"));
            }
            let actual = [sides[0].vertices[j], sides[1].vertices[j]];
            if ports.insert(v, actual).is_some_and(|old| old != actual) {
                return Err(unproven(
                    "SS path graph has an unresolved internal junction",
                ));
            }
        }
        cuts.push(OpenSSNetworkCut {
            path: i,
            segments: [0, path.segments.len()],
            sides,
            bridge_groups: bridge.map(|b| b.then_some(0)),
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
