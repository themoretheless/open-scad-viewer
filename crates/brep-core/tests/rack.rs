use brep_core::{analysis, rack};
use nurbs_core::rack::Spec;
#[test]
fn rack_is_one_closed_body_with_analytic_volume() {
    for teeth in [1, 12, 50] {
        let spec = Spec {
            teeth,
            module: 2.,
            backlash: 0.1,
            addendum: 2.,
            dedendum: 2.5,
            backing_height: 3.,
            ..Spec::default()
        };
        let r = rack::extrude(spec, -2., 3.).unwrap();
        let validation = r.model.validate().unwrap();
        assert_eq!(validation.boundary_edge_count, 0);
        assert_eq!(r.model.bodies.len(), 1);
        assert_eq!(r.model.faces.len(), 4 * teeth + 6);
        let expected_area = r.profile.width * spec.backing_height
            + teeth as f64
                * (spec.addendum + spec.dedendum)
                * (r.profile.tooth_thickness_at_pitch
                    + (spec.dedendum - spec.addendum) * spec.pressure_angle_radians.tan());
        let mass = analysis::mass_properties(&r.model, 1e-6, 1_000_000).unwrap();
        assert!((mass.signed_volume_mm3 - expected_area * 5.).abs() < 1e-7 * expected_area);
        // Every face is a planar NURBS patch of the prismatic outline.
        for face in &r.model.faces {
            assert!(face.surface.degree_u <= 1 && face.surface.degree_v <= 1);
        }
        assert_eq!(r.z_bounds, [-2., 3.]);
    }
}
#[test]
fn rack_body_refuses_invalid_extrusion_and_keeps_profile_limits() {
    for (a, b) in [(0., 0.), (1., 0.), (0., f64::NAN), (0., 1e7)] {
        assert!(rack::extrude(Spec::default(), a, b).is_err());
    }
    assert!(
        rack::extrude(
            Spec {
                teeth: 51,
                ..Spec::default()
            },
            0.,
            3.
        )
        .is_err()
    );
}
