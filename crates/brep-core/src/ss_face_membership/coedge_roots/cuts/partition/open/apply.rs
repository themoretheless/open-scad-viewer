use super::*;
#[derive(Clone, Copy, Debug)]
pub struct OpenSSFaceLimits {
    pub cells: usize,
    pub pairs: usize,
    pub operations: usize,
}
impl Default for OpenSSFaceLimits {
    fn default() -> Self {
        Self {
            cells: 1000000,
            pairs: 100000,
            operations: 1000000,
        }
    }
}
/// Both sectors of the selected actual face remain. The other operand is
/// unchanged unless it is the same model. This is no Boolean or Solid owner.
pub struct QualifiedOpenSSFace<'s, 'i, 'p, 'r, 'a> {
    path: &'s QualifiedOpenSSPath<'i, 'p, 'r, 'a>,
    limits: OpenSSFaceLimits,
    models: Vec<Model>,
    model: usize,
    faces: [usize; 2],
    edges: Vec<usize>,
    vertices: Vec<usize>,
    cells: usize,
    pairs: usize,
    operations: usize,
}
impl QualifiedOpenSSFace<'_, '_, '_, '_, '_> {
    pub fn models(&self) -> &[Model] {
        &self.models
    }
    pub fn model(&self) -> usize {
        self.model
    }
    pub fn faces(&self) -> [usize; 2] {
        self.faces
    }
    pub fn edges(&self) -> &[usize] {
        &self.edges
    }
    pub fn vertices(&self) -> &[usize] {
        &self.vertices
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
        let fresh = partition_open_ss_path(self.path, self.limits)?;
        if self.models != fresh.models
            || self.model != fresh.model
            || self.faces != fresh.faces
            || self.edges != fresh.edges
            || self.vertices != fresh.vertices
            || self.cells != fresh.cells
            || self.pairs != fresh.pairs
            || self.operations != fresh.operations
        {
            return Err(unproven("Source-bound open SS face construction changed"));
        }
        Ok(())
    }
}
pub fn partition_open_ss_path<'s, 'i, 'p, 'r, 'a>(
    path: &'s QualifiedOpenSSPath<'i, 'p, 'r, 'a>,
    limits: OpenSSFaceLimits,
) -> Result<QualifiedOpenSSFace<'s, 'i, 'p, 'r, 'a>> {
    if !(1..=1000000).contains(&limits.cells)
        || !(1..=100000).contains(&limits.pairs)
        || !(1..=1000000).contains(&limits.operations)
    {
        return Err(invalid("Need bounded SS face construction work"));
    }
    let endpoints = [
        path.endpoints[0].ok_or_else(|| {
            unproven("An interior SS endpoint requires complete network assembly")
        })?,
        path.endpoints[1].ok_or_else(|| {
            unproven("An interior SS endpoint requires complete network assembly")
        })?,
    ];
    let mut cells = 0;
    let mut pairs = 0;
    let mut operations = 0;
    spend(&mut cells, limits.cells, path.cells)?;
    spend(&mut operations, limits.operations, path.operations)?;
    spend(&mut pairs, limits.pairs, path.input.source.roots.work.pairs)?;
    path.reverify()?;
    let slot = path.input.side_model[path.request.side];
    let source = &path.input.models[slot];
    let face = path.input.source.roots.faces[path.request.side].1;
    if cells == limits.cells || pairs == limits.pairs {
        return Err(unproven(
            "No actual SS face trim construction budget remains",
        ));
    }
    let segments: Vec<_> = path
        .segments
        .iter()
        .map(|s| crate::face_cut::Segment {
            curve: s.curve.clone(),
            pcurve: s.pcurves[path.request.side].clone(),
        })
        .collect();
    spend(&mut operations, limits.operations, path.segments.len())?;
    let result = crate::face_cut::partition_with_shared_limits(
        source,
        face,
        endpoints,
        &segments,
        path.uv_tolerance,
        limits.cells - cells,
        (limits.operations - operations) as u64,
        (limits.pairs - pairs).min(10000),
    )?;
    spend(
        &mut operations,
        limits.operations,
        usize::try_from(result.exact_work)
            .map_err(|_| unproven("SS face coefficient work overflow"))?,
    )?;
    spend(&mut cells, limits.cells, result.cells)?;
    spend(&mut pairs, limits.pairs, result.trim_pairs)?;
    let mut model = result.model;
    let mut vertices = vec![endpoints[0]];
    for (i, &edge) in result.edges.iter().enumerate() {
        let e = &model.edges[edge];
        if e.curve != path.segments[i].curve || e.vertices[0] != *vertices.last().unwrap() {
            return Err(unproven(
                "Actual SS cut lost its qualified source curve or directed ports",
            ));
        }
        vertices.push(e.vertices[1]);
        let id = model.1.edges[edge];
        model
            .1
            .local_tolerances_mm
            .insert(id, path.world_tolerance.max(1e-10));
    }
    if vertices.last() != Some(&endpoints[1]) {
        return Err(unproven(
            "Actual SS cut ends at an unrelated boundary vertex",
        ));
    }
    for (i, &v) in vertices.iter().enumerate() {
        let p = if i == 0 {
            &path.segments[0].curve.control_points[0]
        } else {
            path.segments[i - 1].curve.control_points.last().unwrap()
        };
        if model.vertices[v].point.as_slice() != p {
            return Err(unproven("Actual SS cut changed a canonical path vertex"));
        }
        let stamp = path.world_tolerance.max(1e-10);
        model
            .1
            .local_tolerances_mm
            .entry(model.1.vertices[v])
            .and_modify(|t| *t = t.min(stamp))
            .or_insert(stamp);
    }
    model.validate()?;
    model.refresh_change_set(&[source]);
    let mut models = path.input.models.clone();
    models[slot] = model;
    Ok(QualifiedOpenSSFace {
        path,
        limits,
        models,
        model: slot,
        faces: result.faces,
        edges: result.edges,
        vertices,
        cells,
        pairs,
        operations,
    })
}
