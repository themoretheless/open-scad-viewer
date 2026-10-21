use brep_core::thread::{self, FiniteThreadLimits};
use nurbs_core::thread::{Kind, Spec};
#[test]
fn finite_internal_ring_has_owned_annular_caps_and_independent_volume() {
    let spec = Spec {
        turns: 2,
        kind: Kind::Internal,
        error_budget: 1e-2,
        ..Spec::default()
    };
    let radius = 5.;
    let result = thread::finite_internal_with_certificates(
        spec,
        [1.13, 1.87],
        radius,
        FiniteThreadLimits::default(),
    )
    .unwrap();
    let c = result.candidate();
    let m = &c.body.model;
    assert_eq!(m.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(m.bodies.len(), 1);
    assert!(m.shells[0].closed);
    assert_eq!(c.outer_wall_faces, 4);
    assert_eq!(m.faces.len(), c.side_faces + 2);
    assert_eq!(result.original_wall_indices().len() + 4, c.side_faces);
    assert_eq!(result.cap_profiles().len(), 2);
    for cap in result.cap_profiles() {
        assert_eq!(cap.verified().nesting_depth(), &[0, 1]);
    }
    assert!(m.faces.iter().rev().take(2).all(|f| f.holes.len() == 1));
    let range = c.outer_wall_radius_bounds.unwrap();
    assert!(range[0] <= radius && radius <= range[1]);
    assert!(range[1] - range[0] < 1e-10);
    assert!(result.boundary_agreement().complete);
    assert!(!result.solid_geometry_certified());
    let crest = spec.diameter / 2. + spec.clearance / 2.;
    let root = crest - 3_f64.sqrt() * spec.pitch * 5. / 16.;
    let hole = std::f64::consts::PI
        * (0.125 * crest * crest
            + 0.25 * root * root
            + 0.625 * (crest * crest + crest * root + root * root) / 3.)
        * 0.74;
    let expected = std::f64::consts::PI * radius * radius * 0.74 - hole;
    assert!(
        (c.body.mass.signed_volume_mm3 - expected).abs() < expected * 0.005,
        "{} vs {expected}",
        c.body.mass.signed_volume_mm3
    );
}
#[test]
fn outer_wall_separation_is_cylindrical_even_at_nonzero_z() {
    let spec = Spec {
        turns: 2,
        kind: Kind::Internal,
        error_budget: 1e-2,
        ..Spec::default()
    };
    let e = thread::finite_internal_with_certificates(
        spec,
        [1.13, 1.87],
        3.,
        FiniteThreadLimits::default(),
    )
    .unwrap_err();
    assert!(e.message.contains("not separated"));
    assert!(
        thread::finite_internal_with_certificates(
            Spec {
                kind: Kind::External,
                ..spec
            },
            [1.13, 1.87],
            5.,
            FiniteThreadLimits::default()
        )
        .is_err()
    );
    assert!(
        thread::finite_internal_with_certificates(
            spec,
            [1.13, 1.87],
            5.,
            FiniteThreadLimits {
                agreement_cells: 0,
                ..FiniteThreadLimits::default()
            }
        )
        .is_err()
    );
}

#[test]
fn default_precision_left_internal_thread_keeps_annular_material_roles() {
    let spec = Spec {
        turns: 2,
        kind: Kind::Internal,
        hand: nurbs_core::thread::Hand::Left,
        ..Spec::default()
    };
    let result = thread::finite_internal_with_certificates(
        spec,
        [1.13, 1.87],
        5.,
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
    for cap in result.cap_profiles() {
        assert_eq!(cap.verified().nesting_depth(), &[0, 1]);
    }
    assert!(
        result.candidate().body.mass.signed_volume_mm3
            > result.candidate().body.mass.volume_error_estimate_mm3
    );
}
