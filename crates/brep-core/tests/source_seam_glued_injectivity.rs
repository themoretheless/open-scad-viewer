use nurbs_core::{primitives, surface_injectivity::certify_seam_glued_rectangle};
#[test]
fn one_actual_periodic_shore_has_one_common_injective_unwrapped_rectangle() {
    let s = primitives::torus([0.; 3], 3., 1., 1.).unwrap();
    let n = s.control_points.len();
    let uhi = s.knots_u[n];
    let ulo = s.knots_u[s.degree_u];
    let du = (uhi - ulo) / 32.;
    let report = certify_seam_glued_rectangle(
        &s,
        [vec![[uhi - du, uhi], [ulo, ulo + du]], vec![[0.25, 0.375]]],
        4096,
        2,
    )
    .unwrap();
    assert!(report.proven, "{} / {}", report.reason, report.spans);
    assert!(report.spans >= 2);
    assert!(report.contraction_upper.unwrap() < 1.);
}
#[test]
fn doubly_wrapped_corner_requires_all_four_actual_source_sections() {
    let s = primitives::torus([0.; 3], 3., 1., 1.).unwrap();
    let hi = [
        s.knots_u[s.control_points.len()],
        s.knots_v[s.control_points[0].len()],
    ];
    let lo = [s.knots_u[s.degree_u], s.knots_v[s.degree_v]];
    let axes = std::array::from_fn(|a| {
        let d = (hi[a] - lo[a]) / 32.;
        vec![[hi[a] - d, hi[a]], [lo[a], lo[a] + d]]
    });
    let partial = certify_seam_glued_rectangle(&s, axes.clone(), 1, 2).unwrap();
    assert!(!partial.proven);
    assert_eq!(partial.spans, 1);
    let report = certify_seam_glued_rectangle(&s, axes, 4096, 2).unwrap();
    assert!(report.proven, "{} / {}", report.reason, report.spans);
    assert!(report.spans >= 4);
}
#[test]
fn missing_actual_seam_identity_and_incomplete_source_sections_cannot_be_glued() {
    let mut s = primitives::torus([0.; 3], 3., 1., 1.).unwrap();
    let hi = s.knots_u[s.control_points.len()];
    let axes = [vec![[hi - 0.125, hi], [0., 0.125]], vec![[0.25, 0.375]]];
    let n = s.control_points.len();
    s.control_points[n - 1][0][0] += 2f64.powi(-30);
    let report = certify_seam_glued_rectangle(&s, axes.clone(), 4096, 2).unwrap();
    assert!(!report.proven);
    assert_eq!(report.reason, "periodic-seam-identity-unproven");
    let mut broken = axes;
    broken[0][0][1] -= 0.0625;
    assert!(certify_seam_glued_rectangle(&s, broken, 4096, 2).is_err());
}

#[test]
fn authored_parameter_unit_changes_preserve_actual_glued_geometry_proof() {
    let mut s = primitives::torus([0.; 3], 3., 1., 1.).unwrap();
    for u in &mut s.knots_u {
        *u *= 1024.;
    }
    for v in &mut s.knots_v {
        *v /= 1024.;
    }
    let hi = s.knots_u[s.control_points.len()];
    let axes = [
        vec![[hi - 32., hi], [0., 32.]],
        vec![[0.25 / 1024., 0.375 / 1024.]],
    ];
    let r = certify_seam_glued_rectangle(&s, axes, 4096, 2).unwrap();
    assert!(r.proven);
    assert!(r.parameter_scale.iter().all(|v| v.is_finite() && *v > 0.));
    assert!(r.projection.is_some());
}
