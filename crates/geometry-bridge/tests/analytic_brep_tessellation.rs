use brep_core::{frustum, revolve_angle, sphere};
use geometry_bridge::brep::nurbs;

#[test]
fn sphere_and_both_cone_poles_tessellate_closed_at_each_lod() {
    for model in [
        sphere(3.).unwrap(),
        frustum(3., 0., 5.).unwrap(),
        frustum(0., 3., 5.).unwrap(),
    ] {
        for detail in [1, 2, 4, 8, 16, 32] {
            let t = nurbs(&model, detail)
                .unwrap_or_else(|e| panic!("LOD {detail}, faces {}: {e:?}", model.faces.len()));
            assert!(t.built.report.closed);
            assert_eq!(t.built.report.degenerate_triangles, 0);
            assert_eq!(t.built.report.non_manifold_edges, 0);
            assert_eq!(t.built.report.orientation_conflicts, 0);
            assert!(t.built.report.signed_volume_mm3 > 0.);
        }
    }
}

#[test]
fn partial_revolve_caps_share_side_tessellation_at_each_lod() {
    let profile = [[0., 0.], [3., 0.], [3., 4.], [0., 4.]];
    for angle in [0.01, 30., 90., 137., -30., -270.] {
        let model = revolve_angle(&profile, angle).unwrap();
        for detail in [1, 2, 4, 8, 16, 32] {
            let t = nurbs(&model, detail)
                .unwrap_or_else(|e| panic!("Angle {angle}, LOD {detail}: {e:?}"));
            assert!(t.built.report.closed);
            assert!(t.built.report.signed_volume_mm3 > 0.);
        }
    }
}

#[test]
fn very_small_partial_revolve_fails_explicitly_when_display_tolerance_collapses_sector() {
    let profile = [[0., 0.], [3., 0.], [3., 4.], [0., 4.]];
    let model = revolve_angle(&profile, 0.00001).unwrap();
    model.validate().unwrap();
    let error = match nurbs(&model, 32) {
        Ok(_) => return,
        Err(error) => error,
    };
    assert!(
        error.message.contains("manifold seams"),
        "Expected explicit display resolution refusal: {error:?}"
    );
}
