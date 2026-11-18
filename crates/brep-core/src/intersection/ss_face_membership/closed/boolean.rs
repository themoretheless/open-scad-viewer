//! One native entry point for complete closed graph Boolean components.
use super::*;
use crate::trimmed_shell_recipe as shell;
pub type GraphBooleanLimits = GraphLensLimits;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphBooleanOperation {
    Union,
    Intersection,
    Difference { minuend: usize },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphBooleanComponent {
    Envelope,
    InnerDifference,
    AnnularDifference,
}
pub struct ClosedGraphBoolean {
    boundary: shell::ClosedBoundary,
    operation: GraphBooleanOperation,
    component: GraphBooleanComponent,
    operations: usize,
}
impl ClosedGraphBoolean {
    pub fn model(&self) -> &Model {
        self.boundary.model()
    }
    pub fn operation(&self) -> GraphBooleanOperation {
        self.operation
    }
    pub fn component(&self) -> GraphBooleanComponent {
        self.component
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
/// Selection never authorizes a component: the chosen constructor freshly
/// replays original bodies, the complete SS partition and material roles before
/// constructing its immutable boundary. Result volume admission stays separate.
pub fn assemble_closed_graph_boolean(
    material: &QualifiedClosedGraphMaterial<'_, '_, '_, '_, '_, '_>,
    operation: GraphBooleanOperation,
    limits: GraphBooleanLimits,
) -> Result<ClosedGraphBoolean> {
    let (boundary, component, operations) = match operation {
        GraphBooleanOperation::Union | GraphBooleanOperation::Intersection => {
            let op = if operation == GraphBooleanOperation::Union {
                GraphEnvelopeOperation::Union
            } else {
                GraphEnvelopeOperation::Intersection
            };
            let result = assemble_closed_graph_envelope(
                material,
                op,
                GraphEnvelopeLimits {
                    operations: limits.operations,
                    tolerance_uv: limits.tolerance_uv,
                    face: limits.face,
                },
            )?;
            let operations = result.operations();
            (
                result.into_boundary(),
                GraphBooleanComponent::Envelope,
                operations,
            )
        }
        GraphBooleanOperation::Difference { minuend } => {
            if minuend > 1 {
                return Err(invalid("Choose original graph minuend 0 or 1"));
            }
            let inner = material
                .faces()
                .iter()
                .find(|f| f.sector.side == minuend && f.sector.inner)
                .ok_or_else(|| unproven("Missing complete original inner role"))?;
            if inner.inside_other {
                let result = assemble_closed_graph_annulus(material, minuend, limits)?;
                let operations = result.operations();
                (
                    result.into_boundary(),
                    GraphBooleanComponent::AnnularDifference,
                    operations,
                )
            } else {
                let result = assemble_closed_graph_lens(material, minuend, limits)?;
                let operations = result.operations();
                (
                    result.into_boundary(),
                    GraphBooleanComponent::InnerDifference,
                    operations,
                )
            }
        }
    };
    Ok(ClosedGraphBoolean {
        boundary,
        operation,
        component,
        operations,
    })
}
