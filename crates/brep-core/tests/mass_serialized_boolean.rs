//! The serialized result of the public WASM Boolean route must retain the
//! native mass-analysis tolerance and budget after document reload.
#[test]
fn serialized_drilled_sphere_difference_fits_original_mass_budget() {
    let model: brep_core::Model = value_codec::from_str(include_str!(
        "fixtures/drilled_sphere_difference.json"
    ))
    .unwrap();
    model.validate().unwrap();
    let result = brep_core::analysis::mass_properties(&model, 1e-5, 2_000_000).unwrap();
    // Independent circular-section Simpson integration (10,000 and 100,000
    // panels agree within 8e-8 mm³): radius-5 sphere minus radius-1 drill,
    // drill axis offset sqrt(0.3² + 0.2²). This is a numerical oracle,
    // not a certificate of the fitted Boolean boundary.
    let reference = 492.582617279528_f64;
    assert!((result.signed_volume_mm3 - reference).abs() / reference <= 2e-3);
    assert!(result.evaluations <= 2_000_000);
}
