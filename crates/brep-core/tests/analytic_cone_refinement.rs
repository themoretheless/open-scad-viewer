use brep_core::feature_ops::edge_partition::{self, Limits};
#[test]
fn ordinary_native_constructor_cones_refine_real_radial_edges_at_either_pole_end() {
    for (bottom, top) in [(1., 0.), (0., 1.)] {
        let s = brep_core::frustum(bottom, top, 2.).unwrap();
        let saved = s.clone();
        let pole = s
            .vertices
            .iter()
            .position(|v| v.point[0] == 0. && v.point[1] == 0.)
            .unwrap();
        let edge = s
            .edges
            .iter()
            .position(|e| {
                !e.degenerate && e.vertices[0] != e.vertices[1] && e.vertices.contains(&pole)
            })
            .unwrap();
        let p = edge_partition::split(&s, edge, &[0.375], "native-cone-ray", Limits::default())
            .unwrap();
        assert!(p.reverify().unwrap().trim_regions_valid);
        assert_eq!(p.model().vertices[pole], s.vertices[pole]);
        assert_eq!(p.model().1.faces, s.1.faces);
        assert!((p.evidence().numerical_volume_mm3 - 2. * std::f64::consts::PI / 3.).abs() < 1e-5);
        assert_eq!(s, saved);
    }
}
#[test]
fn native_cone_rim_refinement_preserves_the_declared_zero_measure_pole_edge() {
    let s = brep_core::frustum(1., 0., 2.).unwrap();
    let pole = s.edges.iter().position(|e| e.degenerate).unwrap();
    let edge = s
        .edges
        .iter()
        .position(|e| {
            !e.degenerate
                && s.vertices[e.vertices[0]].point[2] == 0.
                && s.vertices[e.vertices[1]].point[2] == 0.
        })
        .unwrap();
    let p = edge_partition::split(
        &s,
        edge,
        &[0.25, 0.75],
        "native-cone-rim",
        Limits::default(),
    )
    .unwrap();
    assert_eq!(p.model().edges[pole], s.edges[pole]);
    assert_eq!(p.model().1.edges[pole], s.1.edges[pole]);
    assert!(p.reverify().unwrap().boundary_complete);
}
#[test]
fn strongly_sheared_native_cone_patches_refine_without_a_metric_ray_order_assumption() {
    for (bottom, top) in [(1., 0.), (0., 1.)] {
        let mut s = brep_core::frustum(bottom, top, 2.).unwrap();
        let transform = |p: &mut [f64]| {
            let [x, y, z] = [p[0], p[1], p[2]];
            p.copy_from_slice(&[x - 16. * y + 3., y - 7., z + 5.]);
        };
        for v in &mut s.vertices {
            transform(&mut v.point);
        }
        for e in &mut s.edges {
            for p in &mut e.curve.control_points {
                transform(p);
            }
        }
        for f in &mut s.faces {
            for row in &mut f.surface.control_points {
                for p in row {
                    transform(p);
                }
            }
        }
        s.validate().unwrap();
        let saved = s.clone();
        let pole = s
            .vertices
            .iter()
            .position(|v| v.point[0] == 3. && v.point[1] == -7.)
            .unwrap();
        let edge = s
            .edges
            .iter()
            .position(|e| !e.degenerate && e.vertices.contains(&pole))
            .unwrap();
        let p = edge_partition::split(&s, edge, &[0.375], "sheared-native-cone", Limits::default())
            .unwrap();
        assert!(p.reverify().unwrap().trim_regions_valid);
        assert_eq!(p.model().1.faces, s.1.faces);
        assert_eq!(p.model().vertices[pole], s.vertices[pole]);
        assert!((p.evidence().numerical_volume_mm3 - 2. * std::f64::consts::PI / 3.).abs() < 1e-5);
        assert_eq!(s, saved);
    }
}
