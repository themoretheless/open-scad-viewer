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
/// Upper bound on the operation budget a caller may request.
const MAX_ANNULUS_OPERATIONS: usize = 1_000_000;
/// Output faces, their edge keys and their placement senses, in shell order.
#[derive(Default)]
struct AnnulusFragments {
    faces: Vec<face::QualifiedFace>,
    keys: Vec<Vec<u64>>,
    senses: Vec<bool>,
}
impl AnnulusFragments {
    fn push(&mut self, report: face::Report, keys: Vec<u64>, reversed: bool) -> Result<()> {
        self.faces
            .push(report.face.ok_or_else(|| unproven(report.reason))?);
        self.keys.push(keys);
        self.senses.push(reversed);
        Ok(())
    }
}
/// Operation budget shared by every annulus stage.
struct AnnulusBudget {
    spent: usize,
    limit: usize,
}
impl AnnulusBudget {
    fn charge(&mut self, amount: usize) -> Result<()> {
        charge(&mut self.spent, self.limit, amount)
    }
}
fn validate_annulus_request(minuend: usize, limits: &GraphAnnulusLimits) -> Result<()> {
    if minuend > 1
        || !(1..=MAX_ANNULUS_OPERATIONS).contains(&limits.operations)
        || !limits.tolerance_uv.is_finite()
        || limits.tolerance_uv <= 0.
    {
        return Err(invalid("Choose bounded graph annulus work and an operand"));
    }
    Ok(())
}
/// Rebuilds one sector wire, mapping SS-owned edges to their original
/// identity and keeping untouched roof edges under their source key.
fn sector_wire(
    budget: &mut AnnulusBudget,
    model: &Model,
    source: &Model,
    region: &ClosedBranchFaces,
    side: usize,
    wire_index: usize,
) -> Result<(Vec<face::Boundary>, Vec<u64>)> {
    let mut wire = Vec::new();
    let mut keys = Vec::new();
    for c in &model.loops[wire_index].coedges {
        budget.charge(1)?;
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
        keys.push(key);
    }
    Ok((wire, keys))
}
pub fn assemble_closed_graph_annulus(
    material: &QualifiedClosedGraphMaterial<'_, '_, '_, '_, '_, '_>,
    minuend: usize,
    limits: GraphAnnulusLimits,
) -> Result<ClosedGraphAnnulus> {
    validate_annulus_request(minuend, &limits)?;
    let mut budget = AnnulusBudget {
        spent: 0,
        limit: limits.operations,
    };
    budget.charge(material.operations())?;
    material.reverify()?;
    let bodies = material.bodies();
    let tolerance = bodies[0].source().tolerance_mm;
    if tolerance.to_bits() != bodies[1].source().tolerance_mm.to_bits() {
        return Err(unproven("Original annulus tolerances differ"));
    }
    let context = ToleranceContext::from_brep_tolerance_mm(tolerance)
        .map_err(|_| invalid("Invalid annulus tolerance"))?;
    let closed = material.source().source();
    let mut fragments = AnnulusFragments::default();
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
            let (wire, keys) = sector_wire(&mut budget, model, source, region, side, w)?;
            wires.push(wire);
            face_keys.extend(keys);
        }
        let report = face::assemble(
            &context,
            &support.surface,
            &wires,
            limits.tolerance_uv,
            limits.face,
        )?;
        let reversed = bodies[side].equivalent_body().reverse_orientation() ^ (side != minuend);
        fragments.push(report, face_keys, reversed)?;
    }
    for strip in walls::build(bodies, minuend, &mut budget.spent, limits.operations)? {
        let report = face::assemble(
            &context,
            &strip.surface,
            &[strip.wire],
            limits.tolerance_uv,
            limits.face,
        )?;
        let reversed = bodies[minuend].equivalent_body().reverse_orientation();
        fragments.push(report, strip.keys, reversed)?;
    }
    let placements: Vec<_> = (0..fragments.faces.len())
        .map(|i| shell::Placement {
            face: &fragments.faces[i],
            reversed: fragments.senses[i],
            edge_keys: &fragments.keys[i],
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
        operations: budget.spent,
    })
}
#[cfg(test)]
mod tests;
