//! Structural analysis request handlers (truss, bonded solid, print strength, sections).
//!
//! Routed by `geometry-bridge`: [`dispatch`] hands back operations this
//! domain does not own.

pub(crate) use bridge_codec::{
    Error, Result, Routed, Value, field, input, json, require_exact_fields,
};

pub mod bonded_solid;
pub mod print_strength;
pub mod structural_sections;
pub mod truss;

/// Handles the analysis operations; other operations are handed back.
pub fn dispatch(v: Value) -> Routed {
    Routed::Handled(match bridge_codec::op(&v) {
        "truss_solve" | "truss_solve_wrenches" => truss::solve(v),
        "bonded_solid_solve" => bonded_solid::solve(v),
        "truss_screen" => print_strength::screening(v),
        "print_strength_profile" => print_strength::profile(v),
        "thermal_strength" => print_strength::thermal(v),
        "structural_sections" => structural_sections::inspect(v),
        _ => return Routed::Unhandled(v),
    })
}

#[cfg(test)]
pub(crate) fn handle(v: Value) -> Result<Value> {
    match dispatch(v) {
        Routed::Handled(result) => result,
        Routed::Unhandled(_) => Err(input("Unknown operation")),
    }
}
