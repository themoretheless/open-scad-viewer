use super::*;

mod booleans;
mod lofts;
mod signature_ops;
mod sweeps;

const LOFT_OPS: &[&str] = &[
    "boundary_fill",
    "gordon_surface",
    "loft_match_surface",
    "brep_natural_loft",
    "brep_capped_loft",
    "loft_embedding_limits",
    "loft_parameter_map",
    "guided_loft_surface",
    "auto_guided_loft_surface",
    "control_tangent_loft_surface",
    "natural_loft_surface",
    "clamped_loft_surface",
    "closed_loft_surface",
];

const SWEEP_OPS: &[&str] = &[
    "brep_miter_sweep",
    "brep_progressive_miter_sweep",
    "brep_progressive_sweep",
    "progressive_sweep",
    "profile_sweep",
    "framed_sweep",
    "scaled_sweep",
    "two_guide_sweep",
    "twist_sweep",
];

pub(super) fn take_optional(args: &mut Vec<(Option<String>, V)>, key: &str) -> Option<V> {
    args.iter()
        .position(|(n, _)| n.as_deref() == Some(key))
        .map(|idx| args.remove(idx).1)
}

pub(super) fn take_named(
    args: &mut Vec<(Option<String>, V)>,
    key: &str,
    err: &str,
) -> R<V> {
    take_optional(args, key).ok_or_else(|| crate::error(err))
}

impl Compiler {
    pub(super) fn operation(
        &mut self,
        name: &str,
        input: Option<String>,
        args: Vec<(Option<String>, V)>,
    ) -> R<V> {
        let name = match name {
            "move" => "translate",
            "rect" => "rectangle",
            _ => name,
        };
        if LOFT_OPS.contains(&name) {
            return self.loft_operation(name, input, args);
        }
        if SWEEP_OPS.contains(&name) {
            return self.sweep_operation(name, input, args);
        }
        if booleans::POSITIONAL_AND_BOOLEAN_OPS.contains(&name) {
            return self.boolean_or_patch_operation(name, input, args);
        }
        self.signature_operation(name, input, args)
    }

    pub(super) fn collect_positional_geometry(
        &mut self,
        input: Option<String>,
        args: Vec<(Option<String>, V)>,
        err: &str,
    ) -> R<Vec<String>> {
        if args.iter().any(|(n, _)| n.is_some()) {
            return Err(crate::error(err));
        }
        let mut inputs = Vec::with_capacity(args.len() + usize::from(input.is_some()));
        if let Some(id) = input {
            inputs.push(id);
        }
        for (_, value) in args {
            inputs.push(self.geometry(value)?);
        }
        Ok(inputs)
    }
}
