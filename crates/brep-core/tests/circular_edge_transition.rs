use brep_core::{Model, feature_ops::edge_transition::{self, Limits}};
fn source(angle: f64) -> Model {
    brep_core::revolve_angle(&[[1., 0.], [4., 0.], [4., 3.], [1., 3.]], angle).unwrap()
}
fn edge(m: &Model, radius: f64) -> usize {
    m.edges
        .iter()
        .position(|e| {
            if e.curve.degree < 2 {
                return false;
            }
            let d = e.curve.domain();
            let p = e.curve.evaluate(d[0] * 0.5 + d[1] * 0.5).unwrap().point;
            p[2].abs() < 1e-7 && (p[0].hypot(p[1]) - radius).abs() < 1e-7
        })
        .unwrap()
}
fn volume(m: &Model) -> f64 {
    brep_core::analysis::mass_properties(m, 1e-6, 2000000)
        .unwrap()
        .signed_volume_mm3
}
#[test]
fn outer_and_inner_circular_rounds_close_the_original_sector_with_correct_removed_volume() {
    let m = source(90.);
    let before = m.clone();
    for (radius, r, sign) in [(4., 0.5, 1.), (1., 0.2, -1.)] {
        let out = edge_transition::round_circular_edge(&m, edge(&m, radius), r, Limits::default())
            .unwrap();
        assert!(out.audit().ok);
        assert_eq!(out.tangent_seams().len(), 2);
        assert!(out.radius_certificate().error_upper() <= m.tolerance_mm);
        assert!(
            out.envelope_certificate().sine_squared_bounds()[1]
                <= out.envelope_certificate().tolerance()
        );
        let removed = std::f64::consts::FRAC_PI_2
            * ((radius - sign * r) * r * r * (1. - std::f64::consts::FRAC_PI_4)
                + sign * r * r * r / 6.);
        assert!((volume(out.model()) - (volume(&m) - removed)).abs() < 1e-4);
    }
    assert_eq!(m, before);
}
#[test]
fn oblique_original_sector_needs_no_authored_axis_or_contact_rails() {
    let m = source(60.);
    let selected = edge(&m, 4.);
    let m = brep_core::transform::affine(
        &m,
        [
            [0.36, -0.8, 0.48, 11.],
            [0.48, 0.6, 0.64, -7.],
            [-0.8, 0., 0.6, 5.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let before = m.clone();
    let out = edge_transition::round_circular_edge(&m, selected, 0.4, Limits::default()).unwrap();
    assert!(out.audit().ok);
    assert_eq!(out.source_edge(), m.1.edges[selected]);
    assert!(out.work().0 <= 100000 && out.work().1 <= 1000000);
    assert_eq!(m, before);
}
#[test]
fn invalid_circular_frames_radii_and_exhausted_work_publish_no_partial_body() {
    let m = source(90.);
    let selected = edge(&m, 4.);
    let before = m.clone();
    for r in [0., -1., f64::NAN, 4.] {
        assert!(edge_transition::round_circular_edge(&m, selected, r, Limits::default()).is_err());
    }
    let mut o = Limits::default();
    o.cells = 256;
    assert!(edge_transition::round_circular_edge(&m, selected, 0.5, o).is_err());
    let straight = m.edges.iter().position(|e| e.curve.degree == 1).unwrap();
    assert!(edge_transition::round_circular_edge(&m, straight, 0.5, Limits::default()).is_err());
    assert_eq!(m, before);
}

#[test]
fn asymmetric_outer_and_inner_circular_chamfers_have_whole_nearest_setbacks_and_volume() {
    let m = source(90.);
    let before = m.clone();
    for (radius, d, sign) in [(4., [0.5, 0.25], 1.), (1., [0.2, 0.4], -1.)] {
        let selected = edge(&m, radius);
        let out =
            edge_transition::chamfer_circular_edge(&m, selected, d, Limits::default()).unwrap();
        assert!(out.audit().ok);
        for (side, certificates) in out.setbacks().iter().enumerate() {
            assert!(!certificates.is_empty());
            for c in certificates {
                assert_eq!(c.requested(), d[side]);
                assert!(c.error_upper() <= m.tolerance_mm);
                assert_eq!(c.original_curve(), &m.edges[selected].curve);
                assert_eq!(c.actual_curve(), &out.model().edges[c.edge_index()].curve);
                assert_eq!(&c.matched_distance().curves()[0], c.actual_curve());
                assert_eq!(&c.matched_distance().curves()[1], c.original_curve());
                assert!(c.distance_interval()[0] <= d[side] && d[side] <= c.distance_interval()[1]);
                assert!(matches!(
                    (side, c.lower_bound()),
                    (0, edge_transition::CircularSetbackLower::Radial { .. })
                        | (1, edge_transition::CircularSetbackLower::Axial { .. })
                ));
            }
        }
        let removed = std::f64::consts::FRAC_PI_2 * d[0] * d[1] * 0.5 * (radius - sign * d[0] / 3.);
        assert!((volume(out.model()) - (volume(&m) - removed)).abs() < 1e-4);
    }
    assert_eq!(m, before);
}
#[test]
fn circular_chamfer_uses_actual_independent_edge_clocks_after_oblique_placement() {
    let mut m = source(60.);
    let selected = edge(&m, 4.);
    for t in &mut m.edges[selected].curve.knots {
        *t = -7. + 20. * *t;
    }
    let m = brep_core::transform::affine(
        &m,
        [
            [0.36, -0.8, 0.48, 11.],
            [0.48, 0.6, 0.64, -7.],
            [-0.8, 0., 0.6, 5.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let before = m.clone();
    let out = edge_transition::chamfer_circular_edge(&m, selected, [0.4, 0.2], Limits::default())
        .unwrap();
    assert!(out.audit().ok);
    assert_eq!(out.source_edge(), m.1.edges[selected]);
    assert!(out.work().0 <= 100000 && out.work().1 <= 1000000);
    assert!(
        out.setbacks()
            .iter()
            .flatten()
            .all(|c| c.original_curve().domain() == [-7., 13.])
    );
    assert_eq!(m, before);
}
#[test]
fn circular_chamfer_refuses_unresolved_supports_bad_offsets_and_exhausted_work() {
    let m = source(90.);
    let selected = edge(&m, 4.);
    let before = m.clone();
    for d in [
        [0., 0.2],
        [-1., 0.2],
        [0.2, f64::NAN],
        [4., 0.2],
        [0.2, 4.],
        [3.5, 0.2],
    ] {
        assert!(
            edge_transition::chamfer_circular_edge(&m, selected, d, Limits::default()).is_err()
        );
    }
    let mut o = Limits::default();
    o.cells = 256;
    assert!(edge_transition::chamfer_circular_edge(&m, selected, [0.5, 0.25], o).is_err());
    let ellipse = brep_core::transform::affine(
        &m,
        [
            [1.02, 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    assert!(
        edge_transition::chamfer_circular_edge(&ellipse, selected, [0.5, 0.25], Limits::default())
            .is_err()
    );
    assert_eq!(m, before);
}
