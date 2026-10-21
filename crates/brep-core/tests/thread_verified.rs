use brep_core::thread::{self, FiniteThreadLimits};
use nurbs_core::thread::{Kind, Spec};
#[test]
fn finite_external_body_binds_original_surfaces_and_complete_clipping() {
    let spec = Spec {
        turns: 2,
        error_budget: 1e-2,
        ..Spec::default()
    };
    let limits = FiniteThreadLimits::default();
    let result = thread::finite_external_with_certificates(spec, [1.13, 1.87], limits).unwrap();
    let candidate = result.candidate();
    let model = &candidate.body.model;
    assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(model.bodies.len(), 1);
    assert!(model.shells[0].closed);
    assert!(result.patch_certificate().rounding_included);
    assert!(result.patch_certificate().error_upper <= spec.error_budget);
    assert!(result.trim_certificate().stored_uv_isotopy_certified);
    assert!(result.trim_certificate().axial_error_band_upper <= limits.tolerance_mm);
    assert_eq!(result.original_wall_indices().len(), candidate.side_faces);
    assert!(result.boundary_agreement().complete);
    assert!(candidate.agreement_cells <= limits.agreement_cells);
    assert!(!result.solid_geometry_certified());
    assert_eq!(result.ideal_finite_body_error_upper(), None);
    let crest = spec.diameter / 2. - spec.clearance / 2.;
    let root = crest - 3_f64.sqrt() * spec.pitch * 5. / 16.;
    let ideal = std::f64::consts::PI
        * (0.125 * crest * crest
            + 0.25 * root * root
            + 0.625 * (crest * crest + crest * root + root * root) / 3.)
        * 0.74;
    assert!((candidate.body.mass.signed_volume_mm3 - ideal).abs() < ideal * 0.005);
}
#[test]
fn invalid_kind_and_missing_work_never_return_a_certified_thread() {
    let spec = Spec {
        turns: 2,
        error_budget: 1e-2,
        ..Spec::default()
    };
    assert!(
        thread::finite_external_with_certificates(
            Spec {
                kind: Kind::Internal,
                ..spec
            },
            [1.13, 1.87],
            FiniteThreadLimits::default()
        )
        .is_err()
    );
    assert!(
        thread::finite_external_with_certificates(
            spec,
            [1.13, 1.87],
            FiniteThreadLimits {
                agreement_cells: 0,
                ..FiniteThreadLimits::default()
            }
        )
        .is_err()
    );
}

#[test]
fn default_precision_left_thread_preserves_closed_certified_boundaries() {
    let spec = Spec {
        turns: 2,
        hand: nurbs_core::thread::Hand::Left,
        ..Spec::default()
    };
    let result = thread::finite_external_with_certificates(
        spec,
        [1.13, 1.87],
        FiniteThreadLimits::default(),
    )
    .unwrap();
    assert!(result.patch_certificate().error_upper <= spec.error_budget);
    assert!(result.boundary_agreement().complete);
    assert_eq!(
        result
            .candidate()
            .body
            .model
            .validate()
            .unwrap()
            .boundary_edge_count,
        0
    );
    assert!(
        result.candidate().body.mass.signed_volume_mm3
            > result.candidate().body.mass.volume_error_estimate_mm3
    );
}
