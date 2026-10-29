//! Annular original graph-body difference with roof-to-roof planar strips.
use super::*;
use crate::{trimmed_face_recipe as face, trimmed_shell_recipe as shell};
use cad_predicates::ToleranceContext;
mod walls;
pub type GraphAnnulusLimits = GraphLensLimits;
pub struct ClosedGraphAnnulus {
    boundary: shell::ClosedBoundary,
    minuend: usize,
    operations: usize,
}
impl ClosedGraphAnnulus {
    pub(super) fn into_boundary(self) -> shell::ClosedBoundary {
        self.boundary
    }

    pub fn model(&self) -> &Model {
        self.boundary.model()
    }
    pub fn minuend(&self) -> usize {
        self.minuend
    }
    pub fn operations(&self) -> usize {
        self.operations
    }
    pub fn audit_volume(
        self,
        tolerance_uv: f64,
        limits: crate::volume_validity::Limits,
    ) -> crate::Result<shell::VolumeReport> {
        self.boundary.audit_volume(tolerance_uv, limits)
    }
    pub fn audit_volume_with_linear(
        self,
        tolerance_uv: f64,
        limits: crate::volume_validity::Limits,
        max_linear_cells: usize,
    ) -> crate::Result<shell::VolumeReport> {
        self.boundary
            .audit_volume_with_linear(tolerance_uv, limits, max_linear_cells)
    }
}
pub(super) fn source_key(side: usize, edge: usize) -> u64 {
    ((side as u64 + 1) << 60) | edge as u64
}
pub fn assemble_closed_graph_annulus(
    material: &QualifiedClosedGraphMaterial<'_, '_, '_, '_, '_, '_>,
    minuend: usize,
    limits: GraphAnnulusLimits,
) -> Result<ClosedGraphAnnulus> {
    if minuend > 1
        || !(1..=1000000).contains(&limits.operations)
        || !limits.tolerance_uv.is_finite()
        || limits.tolerance_uv <= 0.
    {
        return Err(invalid("Choose bounded graph annulus work and an operand"));
    }
    let mut operations = 0;
    charge(&mut operations, limits.operations, material.operations())?;
    material.reverify()?;
    let bodies = material.bodies();
    let tolerance = bodies[0].source().tolerance_mm;
    if tolerance.to_bits() != bodies[1].source().tolerance_mm.to_bits() {
        return Err(unproven("Original annulus tolerances differ"));
    }
    let context = ToleranceContext::from_brep_tolerance_mm(tolerance)
        .map_err(|_| invalid("Invalid annulus tolerance"))?;
    let closed = material.source().source();
    let mut fragments = Vec::new();
    let mut keys = Vec::new();
    let mut senses = Vec::new();
    for side in 0..2 {
        let sector = material
            .faces()
            .iter()
            .find(|f| f.sector.side == side && !f.sector.inner)
            .ok_or_else(|| unproven("Missing original outer sector"))?;
        if sector.inside_other != (side != minuend) {
            return Err(unproven(
                "Difference selects the inner lens, not the annulus",
            ));
        }
        let region = closed
            .regions()
            .iter()
            .find(|r| r.side == side)
            .ok_or_else(|| unproven("Missing SS ownership"))?;
        let model = &closed.models()[sector.sector.model];
        let source = bodies[side].source();
        let support = &model.faces[sector.sector.face];
        let mut wires = Vec::new();
        let mut face_keys = Vec::new();
        for w in std::iter::once(support.outer).chain(support.holes.iter().copied()) {
            let mut wire = Vec::new();
            for c in &model.loops[w].coedges {
                charge(&mut operations, limits.operations, 1)?;
                if c.trim().is_some() {
                    return Err(unproven("Annulus requires complete source clocks"));
                }
                let edge = &model.edges[c.edge];
                let owner = region.edges.iter().find(|e| e.edge == c.edge);
                let (curve, reversed, key) = if let Some(owner) = owner {
                    (
                        if owner.reversed {
                            edge.curve.reverse()?
                        } else {
                            edge.curve.clone()
                        },
                        c.reversed ^ owner.reversed,
                        owner.original_edge as u64,
                    )
                } else {
                    if source.edges.get(c.edge) != Some(edge) {
                        return Err(unproven("Annulus lost original roof edge"));
                    }
                    (edge.curve.clone(), c.reversed, source_key(side, c.edge))
                };
                wire.push(face::Boundary {
                    curve,
                    pcurve: c.pcurve.clone(),
                    reversed,
                });
                face_keys.push(key);
            }
            wires.push(wire);
        }
        let report = face::assemble(
            &context,
            &support.surface,
            &wires,
            limits.tolerance_uv,
            limits.face,
        )?;
        fragments.push(report.face.ok_or_else(|| unproven(report.reason))?);
        keys.push(face_keys);
        senses.push(bodies[side].equivalent_body().reverse_orientation() ^ (side != minuend));
    }
    for strip in walls::build(bodies, minuend, &mut operations, limits.operations)? {
        let report = face::assemble(
            &context,
            &strip.surface,
            &[strip.wire],
            limits.tolerance_uv,
            limits.face,
        )?;
        fragments.push(report.face.ok_or_else(|| unproven(report.reason))?);
        keys.push(strip.keys);
        senses.push(bodies[minuend].equivalent_body().reverse_orientation());
    }
    let placements: Vec<_> = (0..fragments.len())
        .map(|i| shell::Placement {
            face: &fragments[i],
            reversed: senses[i],
            edge_keys: &keys[i],
        })
        .collect();
    let boundary = shell::assemble(
        &context,
        &placements,
        &[bodies[0].source(), bodies[1].source()],
    )?;
    Ok(ClosedGraphAnnulus {
        boundary,
        minuend,
        operations,
    })
}
#[cfg(test)]
mod tests;
