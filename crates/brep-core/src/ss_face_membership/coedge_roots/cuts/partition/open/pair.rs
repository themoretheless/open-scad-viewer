//! Atomic subdivision of both actual operand faces by one source-owned path.
use super::*;
#[derive(Clone, Debug, PartialEq)]
pub struct OpenSSPairSide {
    pub model: usize,
    pub faces: [usize; 2],
    pub edges: Vec<usize>,
    pub vertices: Vec<usize>,
}
/// Both material sectors survive on each operand. No Boolean deletion, welding
/// or Solid admission is implied by this atomic two-face construction.
pub struct QualifiedOpenSSPair<'s, 'i, 'p, 'r, 'a> {
    path: &'s QualifiedOpenSSPath<'i, 'p, 'r, 'a>,
    limits: OpenSSFaceLimits,
    models: Vec<Model>,
    sides: [OpenSSPairSide; 2],
    cells: usize,
    pairs: usize,
    operations: usize,
}
impl QualifiedOpenSSPair<'_, '_, '_, '_, '_> {
    pub fn models(&self) -> &[Model] {
        &self.models
    }
    pub fn sides(&self) -> &[OpenSSPairSide; 2] {
        &self.sides
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
        let r = partition_open_ss_pair(self.path, self.limits)?;
        if self.models != r.models
            || self.sides != r.sides
            || self.cells != r.cells
            || self.pairs != r.pairs
            || self.operations != r.operations
        {
            return Err(unproven("Atomic SS face pair changed"));
        }
        Ok(())
    }
}
pub fn partition_open_ss_pair<'s, 'i, 'p, 'r, 'a>(
    path: &'s QualifiedOpenSSPath<'i, 'p, 'r, 'a>,
    limits: OpenSSFaceLimits,
) -> Result<QualifiedOpenSSPair<'s, 'i, 'p, 'r, 'a>> {
    if !(1..=1000000).contains(&limits.cells)
        || !(1..=100000).contains(&limits.pairs)
        || !(1..=1000000).contains(&limits.operations)
    {
        return Err(invalid("Need bounded paired SS construction work"));
    }
    let mut cells = 0;
    let mut pairs = 0;
    let mut operations = 0;
    spend(&mut cells, limits.cells, path.cells)?;
    spend(&mut operations, limits.operations, path.operations)?;
    spend(&mut pairs, limits.pairs, path.input.source.roots.work.pairs)?;
    path.reverify()?;
    let mut endpoints = [[0; 2]; 2];
    for side in 0..2 {
        for (i, node) in [path.request.start_node, path.request.end_node]
            .into_iter()
            .enumerate()
        {
            spend(&mut operations, limits.operations, 1)?;
            let anchor=route::anchor(path.input,node,side)?.ok_or_else(|| {
                unproven("Paired SS path requires actual boundary ports on both faces; assemble an interior network first")
            })?;
            endpoints[side][i] = anchor.0;
        }
    }
    if path.input.side_model[0] == path.input.side_model[1]
        && path.input.source.roots.faces[0].1 == path.input.source.roots.faces[1].1
    {
        return Err(unproven(
            "Paired SS subdivision requires two distinct actual faces",
        ));
    }
    let mut models = path.input.models.clone();
    let mut outputs = Vec::new();
    for side in 0..2 {
        let slot = path.input.side_model[side];
        let r = apply::construct(
            path,
            side,
            &models[slot],
            endpoints[side],
            limits,
            &mut cells,
            &mut pairs,
            &mut operations,
        )?;
        models[slot] = r.model;
        outputs.push(OpenSSPairSide {
            model: slot,
            faces: r.faces,
            edges: r.edges,
            vertices: r.vertices,
        });
    }
    // The second operation on an aliased model must retain the first side's
    // literal source-owned world geometry and directed vertex chain.
    for side in 0..2 {
        let o = &outputs[side];
        let m = &models[o.model];
        for (i, &e) in o.edges.iter().enumerate() {
            spend(&mut operations, limits.operations, 1)?;
            if m.edges[e].curve != path.segments[i].curve
                || m.edges[e].vertices != [o.vertices[i], o.vertices[i + 1]]
            {
                return Err(unproven(
                    "Paired SS construction overwrote the other operand path",
                ));
            }
        }
    }
    for (i, m) in models.iter_mut().enumerate() {
        m.validate()?;
        m.refresh_change_set(&[&path.input.models[i]]);
    }
    Ok(QualifiedOpenSSPair {
        path,
        limits,
        models,
        sides: outputs.try_into().unwrap(),
        cells,
        pairs,
        operations,
    })
}
