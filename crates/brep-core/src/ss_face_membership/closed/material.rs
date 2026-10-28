//! Actual source-body membership of complete authored roof sectors.
//! Boundary sewing and whole Boolean result admission remain separate.
use super::*;
use crate::graph_body::QualifiedGraphBody;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphMaterialFace {
    pub sector: GraphFaceSide,
    /// Opposite-solid strict membership of the sector relative interior.
    /// All sector boundary edges, including the shared contour, are excluded.
    pub inside_other: bool,
}
pub struct QualifiedClosedGraphMaterial<'s, 'q, 'p, 'a, 'b, 'm> {
    source: &'s QualifiedClosedGraphSides<'q, 'p, 'a>,
    bodies: [&'b QualifiedGraphBody<'m>; 2],
    limits: ClosedGraphSideLimits,
    faces: Vec<GraphMaterialFace>,
    operations: usize,
}
impl QualifiedClosedGraphMaterial<'_, '_, '_, '_, '_, '_> {
    pub fn source(&self) -> &QualifiedClosedGraphSides<'_, '_, '_> {
        self.source
    }
    pub fn bodies(&self) -> [&QualifiedGraphBody<'_>; 2] {
        self.bodies
    }
    pub fn faces(&self) -> &[GraphMaterialFace] {
        &self.faces
    }
    pub fn operations(&self) -> usize {
        self.operations
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = qualify_closed_graph_material(self.source, self.bodies, self.limits)?;
        if self.faces != fresh.faces || self.operations != fresh.operations {
            return Err(unproven("Original graph material sector ownership changed"));
        }
        Ok(())
    }
}
/// Both original roofs occupy the same complete XY rectangle, strictly above
/// the same floor. A roof-sector point therefore lies strictly inside the other
/// solid precisely when it is below the opposite roof. Only sector relative
/// interiors are classified; no source boundary edge gets a strict role. This uses the proven
/// column semantics of both actual Models, not support sign alone or a sample.
/// The zero contour itself is a common boundary, never strict material.
pub fn qualify_closed_graph_material<'s, 'q, 'p, 'a, 'b, 'm>(
    source: &'s QualifiedClosedGraphSides<'q, 'p, 'a>,
    bodies: [&'b QualifiedGraphBody<'m>; 2],
    limits: ClosedGraphSideLimits,
) -> Result<QualifiedClosedGraphMaterial<'s, 'q, 'p, 'a, 'b, 'm>> {
    if !(1..=1000000).contains(&limits.operations) {
        return Err(invalid("Need bounded graph material work"));
    }
    let mut operations = 0;
    charge(&mut operations, limits.operations, source.operations())?;
    source.reverify()?;
    for (side, body) in bodies.iter().enumerate() {
        charge(&mut operations, limits.operations, body.operations())?;
        body.reverify()
            .map_err(|_| unproven("Original graph body replay failed"))?;
        let (model, face) = source.source().membership.faces[side];
        if !std::ptr::eq(model, body.source()) || face != body.roof() {
            return Err(unproven(
                "SS sector is not the actual admitted original body roof",
            ));
        }
    }
    if std::ptr::eq(bodies[0].source(), bodies[1].source())
        || bodies[0].footprint() != bodies[1].footprint()
        || bodies[0].floor_height() != bodies[1].floor_height()
    {
        return Err(unproven(
            "Need two original graph bodies with common footprint and floor",
        ));
    }
    let mut faces = Vec::new();
    for sector in source.faces() {
        charge(&mut operations, limits.operations, 1)?;
        faces.push(GraphMaterialFace {
            inside_other: !sector.above,
            sector: sector.clone(),
        });
    }
    Ok(QualifiedClosedGraphMaterial {
        source,
        bodies,
        limits,
        faces,
        operations,
    })
}

#[cfg(test)]
mod tests;
