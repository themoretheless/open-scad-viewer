//! Multiple source-owned SS cuts with current-sector inference on both sides.
use super::*;
mod bridges;
#[derive(Clone, Debug, PartialEq)]
pub struct OpenSSNetworkCut {
    pub path: usize,
    /// Half-open range in the original qualified path's saved segments.
    pub segments: [usize; 2],
    pub sides: [OpenSSPairSide; 2],
    /// Side-local joint bridge batch IDs; these contributions split together.
    pub bridge_groups: [Option<usize>; 2],
}
#[derive(Clone, Debug, PartialEq)]
pub struct OpenSSNetworkPort {
    pub original_vertex: usize,
    pub vertices: [usize; 2],
}
pub struct QualifiedOpenSSNetwork<'s, 'i, 'p, 'r, 'a> {
    paths: Vec<&'s QualifiedOpenSSPath<'i, 'p, 'r, 'a>>,
    limits: OpenSSFaceLimits,
    models: Vec<Model>,
    faces: [Vec<usize>; 2],
    cuts: Vec<OpenSSNetworkCut>,
    ports: Vec<OpenSSNetworkPort>,
    cells: usize,
    pairs: usize,
    operations: usize,
}
impl QualifiedOpenSSNetwork<'_, '_, '_, '_, '_> {
    pub fn models(&self) -> &[Model] {
        &self.models
    }
    pub fn faces(&self) -> &[Vec<usize>; 2] {
        &self.faces
    }
    pub fn cuts(&self) -> &[OpenSSNetworkCut] {
        &self.cuts
    }
    pub fn ports(&self) -> &[OpenSSNetworkPort] {
        &self.ports
    }
    pub fn cells(&self) -> usize {
        self.cells
    }
    pub fn pairs(&self) -> usize {
        self.pairs
    }
    pub fn operations(&self) -> usize {
        self.operations
    }
    pub fn reverify(&self) -> Result<()> {
        let r = partition_open_ss_network(&self.paths, self.limits)?;
        if self.models != r.models
            || self.faces != r.faces
            || self.cuts != r.cuts
            || self.ports != r.ports
            || self.cells != r.cells
            || self.pairs != r.pairs
            || self.operations != r.operations
        {
            return Err(unproven("Source-owned paired SS network changed"));
        }
        Ok(())
    }
}
/// Each path must share the identical immutable cut-partition owner. Unique
/// actual endpoint ownership selects current descendants, never nearest faces.
/// Both sectors remain. Exact original junction ports split crossing paths;
/// unresolved crossings and ambiguous ownership refuse atomically.
pub fn partition_open_ss_network<'s, 'i, 'p, 'r, 'a>(
    paths: &[&'s QualifiedOpenSSPath<'i, 'p, 'r, 'a>],
    limits: OpenSSFaceLimits,
) -> Result<QualifiedOpenSSNetwork<'s, 'i, 'p, 'r, 'a>> {
    if !(1..=256).contains(&paths.len())
        || !(1..=1000000).contains(&limits.cells)
        || !(1..=100000).contains(&limits.pairs)
        || !(1..=1000000).contains(&limits.operations)
    {
        return Err(invalid("Need bounded source-owned SS network work"));
    }
    let input = paths[0].input;
    if paths.iter().any(|p| !std::ptr::eq(p.input, input)) {
        return Err(unproven(
            "SS network paths must share the same immutable partition owner",
        ));
    }
    if input.side_model[0] == input.side_model[1]
        && input.source.roots.faces[0].1 == input.source.roots.faces[1].1
    {
        return Err(unproven(
            "SS network requires distinct actual operand faces",
        ));
    }
    if let Some(result) = bridges::try_pair(paths, limits)? {
        return Ok(result);
    }
    let mut cells = 0;
    let mut pairs = 0;
    let mut operations = 0;
    let mut models = input.models.clone();
    let mut faces = [
        vec![input.source.roots.faces[0].1],
        vec![input.source.roots.faces[1].1],
    ];
    let mut cuts: Vec<OpenSSNetworkCut> = Vec::new();
    let mut ports = std::collections::BTreeMap::<usize, [usize; 2]>::new();
    for (path_index, path) in paths.iter().enumerate() {
        spend(&mut cells, limits.cells, path.cells)?;
        spend(&mut operations, limits.operations, path.operations)?;
        spend(&mut pairs, limits.pairs, input.source.roots.work.pairs)?;
        path.reverify()?;
        let mut source_ports = vec![None; path.segments.len() + 1];
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
                return Err(unproven("SS network lost an original branch port"));
            }
            source_ports[i] = Some(v);
        }
        let mut breaks = vec![0];
        for i in 1..path.segments.len() {
            if source_ports[i].is_some_and(|v| ports.contains_key(&v)) {
                breaks.push(i);
            }
        }
        breaks.push(path.segments.len());
        for window in breaks.windows(2) {
            let [start, end] = [window[0], window[1]];
            let mut sides = Vec::new();
            for side in 0..2 {
                let slot = input.side_model[side];
                let mut endpoints = [0; 2];
                for (i, index) in [start, end].into_iter().enumerate() {
                    spend(&mut operations, limits.operations, 1)?;
                    endpoints[i] = if index == 0 || index == path.segments.len() {
                        let node = if index == 0 {
                            path.request.start_node
                        } else {
                            path.request.end_node
                        };
                        route::anchor(input, node, side)?
                            .ok_or_else(|| {
                                unproven("Interior SS endpoint requires complete junction assembly")
                            })?
                            .0
                    } else {
                        let v = source_ports[index]
                            .ok_or_else(|| unproven("Missing original SS junction port"))?;
                        ports.get(&v).ok_or_else(|| {
                            unproven("Original SS junction has no actual paired vertex")
                        })?[side]
                    };
                }
                let mut candidates = Vec::new();
                for &face in &faces[side] {
                    spend(&mut operations, limits.operations, 1)?;
                    if crate::face_cut::endpoint_loop::owned_boundary(
                        &models[slot],
                        face,
                        endpoints,
                    )
                    .is_some()
                    {
                        candidates.push(face);
                    }
                }
                if candidates.len() != 1 {
                    return Err(unproven(
                        "SS network endpoints do not identify one current face sector",
                    ));
                }
                let r = apply::construct_range(
                    path,
                    side,
                    candidates[0],
                    &models[slot],
                    endpoints,
                    start..end,
                    limits,
                    &mut cells,
                    &mut pairs,
                    &mut operations,
                )?;
                faces[side].push(r.faces[1]);
                models[slot] = r.model;
                sides.push(OpenSSPairSide {
                    model: slot,
                    faces: r.faces,
                    edges: r.edges,
                    vertices: r.vertices,
                });
            }
            let sides: [OpenSSPairSide; 2] = sides.try_into().unwrap();
            for index in start..=end {
                if let Some(v) = source_ports[index] {
                    spend(&mut operations, limits.operations, 1)?;
                    let actual = [
                        sides[0].vertices[index - start],
                        sides[1].vertices[index - start],
                    ];
                    if let Some(previous) = ports.insert(v, actual) {
                        if previous != actual {
                            return Err(unproven(
                                "Original SS port acquired conflicting actual vertices",
                            ));
                        }
                    }
                }
            }
            cuts.push(OpenSSNetworkCut {
                path: path_index,
                segments: [start, end],
                sides,
                bridge_groups: [None; 2],
            });
        }
    }
    // Recheck all saved subpaths in the final topology, including aliased models.
    for cut in &cuts {
        let path = paths[cut.path];
        for side in 0..2 {
            let o = &cut.sides[side];
            let m = &models[o.model];
            for (i, &edge) in o.edges.iter().enumerate() {
                spend(&mut operations, limits.operations, 1)?;
                if m.edges[edge].curve != path.segments[cut.segments[0] + i].curve
                    || m.edges[edge].vertices != [o.vertices[i], o.vertices[i + 1]]
                {
                    return Err(unproven("SS network overwrote an earlier qualified path"));
                }
            }
        }
    }
    let ports = ports
        .into_iter()
        .map(|(original_vertex, vertices)| OpenSSNetworkPort {
            original_vertex,
            vertices,
        })
        .collect();
    for (i, m) in models.iter_mut().enumerate() {
        m.validate()?;
        m.refresh_change_set(&[&input.models[i]]);
    }
    Ok(QualifiedOpenSSNetwork {
        paths: paths.to_vec(),
        limits,
        models,
        faces,
        cuts,
        ports,
        cells,
        pairs,
        operations,
    })
}

#[cfg(test)]
mod tests;
