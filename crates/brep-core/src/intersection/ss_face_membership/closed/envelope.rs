//! Actual upper/lower roof envelopes with original column walls and floor.
use super::*;
use crate::{trimmed_face_recipe as face, trimmed_shell_recipe as shell};
use cad_predicates::ToleranceContext;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphEnvelopeOperation {
    Union,
    Intersection,
}
#[derive(Clone, Copy)]
pub struct GraphEnvelopeLimits {
    pub operations: usize,
    pub tolerance_uv: f64,
    /// Independently bounded audits for each of the seven retained fragments.
    pub face: face::Limits,
}
pub struct ClosedGraphEnvelope {
    boundary: shell::ClosedBoundary,
    operation: GraphEnvelopeOperation,
    outer_side: usize,
    operations: usize,
}
impl ClosedGraphEnvelope {
    pub(super) fn into_boundary(self) -> shell::ClosedBoundary {
        self.boundary
    }

    pub fn model(&self) -> &Model {
        self.boundary.model()
    }
    pub fn operation(&self) -> GraphEnvelopeOperation {
        self.operation
    }
    pub fn outer_side(&self) -> usize {
        self.outer_side
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
}
pub fn assemble_closed_graph_envelope(
    material: &QualifiedClosedGraphMaterial<'_, '_, '_, '_, '_, '_>,
    operation: GraphEnvelopeOperation,
    limits: GraphEnvelopeLimits,
) -> Result<ClosedGraphEnvelope> {
    if !(1..=1000000).contains(&limits.operations)
        || !limits.tolerance_uv.is_finite()
        || limits.tolerance_uv <= 0.
    {
        return Err(invalid("Choose bounded graph envelope work"));
    }
    let mut operations = 0;
    charge(&mut operations, limits.operations, material.operations())?;
    material.reverify()?;
    let inside = operation == GraphEnvelopeOperation::Intersection;
    let selected: Vec<_> = material
        .faces()
        .iter()
        .filter(|f| f.inside_other == inside)
        .collect();
    let outer = selected
        .iter()
        .find(|f| !f.sector.inner)
        .ok_or_else(|| unproven("Missing outer envelope roof"))?;
    let inner = selected
        .iter()
        .find(|f| f.sector.inner)
        .ok_or_else(|| unproven("Missing inner envelope roof"))?;
    if selected.len() != 2 || outer.sector.side == inner.sector.side {
        return Err(unproven(
            "Envelope needs complementary complete roof sectors",
        ));
    }
    let bodies = material.bodies();
    let base = bodies[outer.sector.side].source();
    let tolerance = base.tolerance_mm;
    if bodies
        .iter()
        .any(|b| b.source().tolerance_mm.to_bits() != tolerance.to_bits())
    {
        return Err(unproven("Original envelope tolerances differ"));
    }
    let context = ToleranceContext::from_brep_tolerance_mm(tolerance)
        .map_err(|_| invalid("Invalid envelope tolerance"))?;
    let closed = material.source().source();
    let mut fragments = Vec::new();
    let mut keys = Vec::new();
    let mut senses = Vec::new();
    let mut inputs = Vec::new();
    for f in [outer, inner] {
        let region = closed
            .regions()
            .iter()
            .find(|r| r.side == f.sector.side)
            .ok_or_else(|| unproven("Missing original envelope contour"))?;
        inputs.push((
            &closed.models()[f.sector.model],
            f.sector.face,
            Some(region),
            f.sector.side,
        ));
    }
    for f in 0..base.faces.len() {
        if f != bodies[outer.sector.side].roof() {
            inputs.push((base, f, None, outer.sector.side));
        }
    }
    for (model, index, region, side) in inputs {
        let support = &model.faces[index];
        let mut wires = Vec::new();
        let mut face_keys = Vec::new();
        for wire in std::iter::once(support.outer).chain(support.holes.iter().copied()) {
            let mut boundaries = Vec::new();
            for c in &model.loops[wire].coedges {
                charge(&mut operations, limits.operations, 1)?;
                if c.trim().is_some() {
                    return Err(unproven("Envelope requires complete original clocks"));
                }
                let edge = &model.edges[c.edge];
                let owner = region.and_then(|r| r.edges.iter().find(|e| e.edge == c.edge));
                let (curve, reversed, key) = if let Some(owner) = owner {
                    let curve = if owner.reversed {
                        edge.curve.reverse()?
                    } else {
                        edge.curve.clone()
                    };
                    (
                        curve,
                        c.reversed ^ owner.reversed,
                        owner.original_edge as u64,
                    )
                } else {
                    if side != outer.sector.side || base.edges.get(c.edge) != Some(edge) {
                        return Err(unproven("Envelope edge lost original wall/floor identity"));
                    }
                    (edge.curve.clone(), c.reversed, (1u64 << 63) | c.edge as u64)
                };
                boundaries.push(face::Boundary {
                    curve,
                    pcurve: c.pcurve.clone(),
                    reversed,
                });
                face_keys.push(key);
            }
            wires.push(boundaries);
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
        senses.push(bodies[side].equivalent_body().reverse_orientation());
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
    Ok(ClosedGraphEnvelope {
        boundary,
        operation,
        outer_side: outer.sector.side,
        operations,
    })
}
#[cfg(test)]
mod tests;
