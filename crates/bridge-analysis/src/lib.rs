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
        "truss_diagnose" => truss::diagnose(v),
        "truss_buckling" => truss::buckling(v),
        "truss_modal" => truss::modal(v),
        "truss_nonlinear" => truss::nonlinear(v),
        "bonded_solid_solve" => bonded_solid::solve(v),
        "truss_screen" => print_strength::screening(v),
        "print_strength_profile" => print_strength::profile(v),
        "thermal_strength" => print_strength::thermal(v),
        "structural_sections" => structural_sections::inspect(v),
        "section_torsion" => structural_sections::torsion(v),
        "thin_walled_section" => structural_sections::thin_walled(v),
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

fn legacy_mesh_error(error: Error) -> Error {
    match error.code {
        "MESH_INVALID_INPUT" | "MESH_QUERY_INVALID_INPUT" | "MESH_SECTION_INVALID_INPUT" | "MESH_IO_INVALID_INPUT" => Error::new("POLYGON_INVALID_INPUT", error.message.replace("the mesh resource budget", "the polygon resource budget")),
        _ => error,
    }
}

fn mass_model(v: &Value) -> Result<mechanics_core::MassModel> {
    match v.as_str().unwrap_or("") {
        "lumped" => Ok(mechanics_core::MassModel::Lumped),
        "consistent" => Ok(mechanics_core::MassModel::Consistent),
        _ => Err(input("massModel must be lumped or consistent")),
    }
}

fn diagnosis_json(d: &mechanics_core::diagnostics::SingularityDiagnosis) -> Value {
    json!({
        "stable": d.is_stable(),
        "minNormalizedPivot": d.min_normalized_pivot,
        "issues": d.issues.iter().map(|i| json!({
            "node": i.node,
            "dof": i.dof,
            "dofName": i.dof_name,
            "issue": match i.issue {
                mechanics_core::diagnostics::DofIssue::Unrestrained => "unrestrained",
                mechanics_core::diagnostics::DofIssue::Mechanism => "mechanism",
            },
        })).collect::<Vec<_>>(),
    })
}
