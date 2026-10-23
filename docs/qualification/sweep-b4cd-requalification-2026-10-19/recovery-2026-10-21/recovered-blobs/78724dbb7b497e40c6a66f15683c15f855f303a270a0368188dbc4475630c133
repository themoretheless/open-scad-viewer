use brep_core::{cuboid, cylinder, frustum, revolve_angle, sphere};
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

#[test]
fn certified_planar_and_rational_cells_publish_two_sided_coverage() {
    for model in [
        cuboid([0., 0., 0.], [3., 4., 5.]).unwrap(),
        cylinder(3., 7.).unwrap(),
    ] {
        let certified = geometry_bridge::brep::certified_nurbs(&model, 0.02, 20_000).unwrap();
        assert!(certified.surface_to_mesh_deviation_mm <= 0.02);
        assert_eq!(
            certified.surface_to_mesh_deviation_mm,
            certified.mesh_to_surface_deviation_mm
        );
        assert!(certified.audit.ok);
        assert!(certified.naming_complete);
        assert_eq!(certified.context, certified.evidence.context);
        let report = &certified.tessellation.built.report;
        assert!(report.closed);
        assert_eq!(report.orientation_conflicts, 0);
        assert_eq!(report.non_manifold_edges, 0);
    }
}

#[test]
fn certified_tessellation_has_typed_budget_mutation_and_sphere_successor() {
    let cylinder = cylinder(100., 10.).unwrap();
    let error = match geometry_bridge::brep::certified_nurbs(&cylinder, 1e-12, 20_000) {
        Ok(_) => panic!("tiny tolerance unexpectedly certified"),
        Err(error) => error,
    };
    assert_eq!(error.code, "BREP_TESSELLATION_BUDGET_EXHAUSTED");
    let mut mutated = cuboid([0.; 3], [1.; 3]).unwrap();
    mutated.1.change_set.changes.clear();
    let error = match geometry_bridge::brep::certified_nurbs(&mutated, 0.1, 20_000) {
        Ok(_) => panic!("mutated naming unexpectedly certified"),
        Err(error) => error,
    };
    assert_eq!(error.code, "BREP_CERTIFIED_TESSELLATION_REFUSED");
    let sphere = geometry_bridge::brep::certified_nurbs(&sphere(2.).unwrap(), 0.1, 20_000).unwrap();
    assert_eq!(sphere.capability, "certified-brep-tessellation/2");
    assert!(sphere.surface_to_mesh_deviation_mm <= 0.1);
    assert!(sphere.tessellation.built.report.closed);
}
