use brep_core::gear::bevel_body_candidate;
use nurbs_core::involute::{BevelGearSpec as Spec, qualified_bevel_outline};
#[test]
fn homothetic_walls_and_fan_caps_form_a_continuously_closed_candidate() {
    for teeth in [6, 20] {
        let source = qualified_bevel_outline(
            Spec {
                teeth,
                ..Spec::default()
            },
            1e-5,
            16,
            4096,
            8192,
        )
        .unwrap();
        let candidate = bevel_body_candidate(&source, 1e-5, 1000000, 1000000).unwrap();
        let report = candidate.model.validate().unwrap();
        assert_eq!(report.boundary_edge_count, 0);
        assert_eq!(
            candidate.closure.closure.status,
            brep_core::watertight::Closure::WithinTolerance
        );
        assert!(candidate.world_error_upper <= 1e-5);
        assert!(!candidate.solid_geometry_certified);
        assert_eq!(candidate.model.faces.len(), 3 * source.curves().len());
        assert_eq!(
            candidate
                .model
                .edges
                .iter()
                .filter(|e| e.degenerate)
                .count(),
            2
        );
        let ranges = source.source().source().parameters().cone_distances;
        let outer = (ranges[1][0] + ranges[1][1]) / 2.;
        let inner = (ranges[0][0] + ranges[0][1]) / 2.;
        let mut twice_area = 0.;
        for c in source.curves() {
            let [a, b] = c.domain();
            let mut previous = c.evaluate_de_boor(a).unwrap();
            for j in 1..=64 {
                let t = if j == 64 {
                    b
                } else {
                    a + (b - a) * j as f64 / 64.
                };
                let p = c.evaluate_de_boor(t).unwrap();
                twice_area += previous[0] * p[1] - previous[1] * p[0];
                previous = p;
            }
        }
        let oracle = (1. - (inner / outer).powi(3)) * outer * twice_area / 6.;
        let mass = brep_core::analysis::mass_properties(&candidate.model, 1e-3, 2000000).unwrap();
        assert!(mass.signed_volume_mm3 > 0.);
        assert!((mass.signed_volume_mm3 - oracle).abs() < oracle * 1e-3);
    }
}
#[test]
fn damaged_pole_collapse_and_unproved_body_tolerance_are_refused() {
    let source = qualified_bevel_outline(Spec::default(), 1e-5, 16, 4096, 8192).unwrap();
    assert!(bevel_body_candidate(&source, 1e-20, 1000000, 1000000).is_err());
    let candidate = bevel_body_candidate(&source, 1e-5, 1000000, 1000000).unwrap();
    let mut damaged = candidate.model.clone();
    damaged.faces[1].surface.control_points[1][0][0] += 0.001;
    assert!(damaged.validate().is_err());
}
