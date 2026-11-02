use brep_core::region_replacement as r;
#[test]
fn four_distinct_mid_radii_close_a_real_four_valent_vertex() {
    let source = r::four_edge_corner_stock().unwrap();
    let before = source.clone();
    let out = r::propose_four_edge_corner(&source, [2.6, 2.8, 3., 3.2]).unwrap();
    assert_eq!(source, before);
    assert_eq!(
        (
            out.model.faces.len(),
            out.model.edges.len(),
            out.model.vertices.len()
        ),
        (10, 20, 12)
    );
    assert_eq!(out.smooth_edges.len(), 12);
    assert!(
        out.radius_plan
            .corners
            .iter()
            .any(|c| c.incident.len() == 4 && c.selected.len() == 4 && c.unequal_radii)
    );
    // Independent original-surface shape operator, not the constructor's
    // profile formula. The arm midpoint is a unit world tangent axis.
    for i in 0..4 {
        let surface = &out.model.faces[out.strip_faces[i]].surface;
        for v in [0.125, 0.5, 0.875] {
            let shape = nurbs_core::surface_shape_operator::at(
                surface,
                [0., v],
                [nurbs_core::curve_differential::Side::Automatic; 2],
            )
            .unwrap();
            let k = if i % 2 == 0 { 0 } else { 1 };
            let interval = shape.tensor.unwrap()[k][k];
            let lo = if interval[0] > 0. {
                interval[0]
            } else {
                -interval[1]
            };
            let hi = interval[0].abs().max(interval[1].abs());
            assert!(lo > 0.);
            let radius = nurbs_core::interval_eval::Interval::point(1.)
                .div(nurbs_core::interval_eval::Interval::new(lo, hi).unwrap())
                .unwrap();
            assert!(
                radius.lo >= out.requested_mid_radii[i] - 1e-7
                    && radius.hi <= out.requested_mid_radii[i] + 1e-7,
                "arm {i} v={v} radius [{},{}]",
                radius.lo,
                radius.hi
            );
        }
    }
    for i in 0..5 {
        let old = source.1.faces[i];
        let current = out.model.1.faces[out.retained_faces[i]];
        assert!(
            old == current
                || out.model.1.lineage.iter().any(|r| r.entity_kind == "face"
                    && matches!(r.operation.as_str(), "persist" | "split" | "merge")
                    && r.parents.contains(&old)
                    && r.children.contains(&current))
        );
        assert_eq!(
            out.model.faces[out.retained_faces[i]].surface,
            source.faces[i].surface
        );
    }
    assert_eq!(
        brep_topology::explorer::euler_characteristic(&out.model.0),
        2
    );
}
#[test]
fn unsupported_radius_or_source_refuses_without_mutation() {
    let source = r::four_edge_corner_stock().unwrap();
    let before = source.clone();
    for radii in [
        [1., 2.8, 3., 3.2],
        [2.6, f64::NAN, 3., 3.2],
        [2.6, 2.8, 3., 10.],
    ] {
        assert!(r::propose_four_edge_corner(&source, radii).is_err());
    }
    assert_eq!(source, before);
    let cube = brep_core::cuboid([0.; 3], [10.; 3]).unwrap();
    assert!(r::propose_four_edge_corner(&cube, [2.6, 2.8, 3., 3.2]).is_err());
}
