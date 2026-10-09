//! STEP v6 keeps every persistent identity of a converted source body.
mod common;

/// Regenerate the fixture with
/// `CAD_SOURCE_MODEL_JSON_OUTPUT=<absolute path> cargo test -p brep-core --lib capped_body_recomputes_all_gates`
/// (from `crates/`; the test binary runs with the crate directory as cwd).
#[test]
fn capped_canal_body_keeps_identities_through_step_v6() {
    let model: brep_core::Model = value_codec::from_str(include_str!(
        "../../../tests/fixtures/cad-step/linear-canal-capped-body.model.json"
    ))
    .unwrap();
    model.validate().unwrap();
    let (step, _, _) = cad_step::export_step_v6(&model).unwrap();
    let (imported, _, _) = cad_step::import_step_v6(&step).unwrap();
    common::assert_step_identity(&model, &imported);
    if let Some(path) = std::env::var_os("CAD_SOURCE_MODEL_STEP_OUTPUT") {
        std::fs::write(path, step).unwrap();
    }
}
