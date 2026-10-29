//! Sew the bounded inner component of a graph-body difference.
//! Closed incidence is not geometric volume admission.
use super::*;
use crate::{trimmed_face_recipe as face, trimmed_shell_recipe as shell};
use cad_predicates::ToleranceContext;

#[derive(Clone, Copy)]
pub struct GraphLensLimits {
    pub operations: usize,
    pub tolerance_uv: f64,
    pub face: face::Limits,
}
pub struct ClosedGraphLens {
    boundary: shell::ClosedBoundary,
    /// Original intersection-edge identities, in assembled canonical order.
    original_edges: Vec<usize>,
    minuend: usize,
    operations: usize,
}
impl ClosedGraphLens {
    pub(super) fn into_boundary(self) -> shell::ClosedBoundary {
        self.boundary
    }

    pub fn model(&self) -> &Model {
        self.boundary.model()
    }
    pub fn original_edges(&self) -> &[usize] {
        &self.original_edges
    }
    pub fn minuend(&self) -> usize {
        self.minuend
    }
    pub fn operations(&self) -> usize {
        self.operations
    }
    /// Independent result geometry audit; no source-body certificate is reused.
    pub fn audit_volume(
        self,
        tolerance_uv: f64,
        limits: crate::volume_validity::Limits,
    ) -> crate::Result<shell::VolumeReport> {
        self.boundary.audit_volume(tolerance_uv, limits)
    }
}
/// Construct A\B when its selected material is the closed inner lens. The
/// complementary annular difference requires original wall strips and is an
/// explicit refusal here. Both complete source regions and their SS edge IDs
/// are retained; neither endpoint proximity nor matching samples weld a seam.
pub fn assemble_closed_graph_lens(
    material: &QualifiedClosedGraphMaterial<'_, '_, '_, '_, '_, '_>,
    minuend: usize,
    limits: GraphLensLimits,
) -> Result<ClosedGraphLens> {
    if minuend > 1
        || !(1..=1000000).contains(&limits.operations)
        || !limits.tolerance_uv.is_finite()
        || limits.tolerance_uv <= 0.
    {
        return Err(invalid(
            "Choose bounded graph lens work and a valid operand",
        ));
    }
    let mut operations = 0;
    charge(&mut operations, limits.operations, material.operations())?;
    material.reverify()?;
    let closed = material.source().source();
    let bodies = material.bodies();
    let tolerance = bodies[0].source().tolerance_mm;
    if tolerance.to_bits() != bodies[1].source().tolerance_mm.to_bits() {
        return Err(unproven("Original graph lens tolerances differ"));
    }
    let context = ToleranceContext::from_brep_tolerance_mm(tolerance)
        .map_err(|_| invalid("Invalid original graph lens tolerance"))?;
    let mut fragments = Vec::new();
    let mut keys = Vec::new();
    let mut senses = Vec::new();
    for side in 0..2 {
        let sector = material
            .faces()
            .iter()
            .find(|f| f.sector.side == side && f.sector.inner)
            .ok_or_else(|| unproven("Missing original inner material sector"))?;
        if sector.inside_other != (side != minuend) {
            return Err(unproven(
                "Difference selects the annulus; original wall strips are required",
            ));
        }
        let region = closed
            .regions()
            .iter()
            .find(|r| r.side == side)
            .ok_or_else(|| unproven("Missing original closed contour ownership"))?;
        let model = &closed.models()[sector.sector.model];
        let support = &model.faces[sector.sector.face];
        if !support.holes.is_empty() {
            return Err(unproven("Graph lens requires complete inner disks"));
        }
        let mut wire = Vec::new();
        let mut face_keys = Vec::new();
        for c in &model.loops[support.outer].coedges {
            charge(&mut operations, limits.operations, 1)?;
            let owner = region
                .edges
                .iter()
                .find(|e| e.edge == c.edge)
                .ok_or_else(|| unproven("Lens edge lacks original SS identity"))?;
            let original = &model.edges[c.edge].curve;
            let curve = if owner.reversed {
                original.reverse()?
            } else {
                original.clone()
            };
            wire.push(face::Boundary {
                curve,
                pcurve: c.pcurve.clone(),
                reversed: c.reversed ^ owner.reversed,
            });
            face_keys.push(owner.original_edge as u64);
        }
        let report = face::assemble(
            &context,
            &support.surface,
            &[wire],
            limits.tolerance_uv,
            limits.face,
        )?;
        fragments.push(report.face.ok_or_else(|| unproven(report.reason))?);
        keys.push(face_keys);
        senses.push(bodies[side].equivalent_body().reverse_orientation() ^ (side != minuend));
    }
    let placements: Vec<_> = (0..2)
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
    let mut original_edges: Vec<_> = keys[0].iter().map(|&k| k as usize).collect();
    original_edges.sort_unstable();
    Ok(ClosedGraphLens {
        boundary,
        original_edges,
        minuend,
        operations,
    })
}

#[cfg(test)]
mod tests;
