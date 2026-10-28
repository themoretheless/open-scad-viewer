//! Original region inputs; cached qualification never becomes replay authority.
use super::*;
#[derive(Clone)]
pub(super) struct Original {
    pub context: cad_predicates::ToleranceSpec,
    pub surface: Surface,
    pub wires: Vec<Vec<Boundary>>,
    pub tolerance: f64,
}
impl Original {
    pub fn new(
        context: &ToleranceContext,
        surface: &Surface,
        wires: &[Vec<Boundary>],
        tolerance: f64,
    ) -> Self {
        Self {
            context: context.specification().clone(),
            surface: surface.clone(),
            wires: wires.to_vec(),
            tolerance,
        }
    }
}
#[derive(Clone, Copy)]
pub(super) enum CutKind {
    Linear,
    Curved,
}
#[derive(Clone)]
pub(super) enum Recipe {
    Original(Original),
    Cut {
        original: Original,
        kind: CutKind,
        contact: Curve,
        address: [usize; 3],
        target_width: [f64; 2],
        axis: usize,
    },
    SplitRoot {
        parent: Box<Recipe>,
        address: [usize; 2],
        point: crate::source_contact_point::SourcePoint,
        role: Role,
    },
    SplitParameter {
        parent: Box<Recipe>,
        address: [usize; 2],
        parameter: f64,
    },
}
impl Recipe {
    #[cfg(feature = "codec")]
    pub fn definition(&self) -> value_codec::Value {
        use value_codec::json;
        match self {
            Self::Original(o) => {
                let context = ToleranceContext::new(o.context.clone())
                    .expect("Stored valid original tolerance");
                crate::source_region_restore::original_definition(
                    &context,
                    &o.surface,
                    &o.wires,
                    o.tolerance,
                )
            }
            Self::Cut {
                original: o,
                kind,
                contact,
                address,
                target_width,
                axis,
            } => {
                let context = ToleranceContext::new(o.context.clone())
                    .expect("Stored valid original tolerance");
                crate::source_region_restore::cut_definition(
                    &context,
                    match kind {
                        CutKind::Linear => "linear",
                        CutKind::Curved => "curved",
                    },
                    &o.surface,
                    &o.wires,
                    contact,
                    *address,
                    o.tolerance,
                    *target_width,
                    *axis,
                )
            }
            Self::SplitRoot {
                parent,
                address,
                point,
                role,
            } => {
                json!({"version":1,"kind":"splitRoot","parent":parent.definition(),"address":address,"point":point.definition(),"role":match role {Role::Boundary=>"boundary",Role::Contact=>"contact"}})
            }
            Self::SplitParameter {
                parent,
                address,
                parameter,
            } => {
                json!({"version":1,"kind":"splitParameter","parent":parent.definition(),"address":address,"parameter":parameter})
            }
        }
    }
}
